//! Shared application state (cheap to clone; all fields are `Arc`).

use crate::errors::{AppError, AppResult};
use crate::models::*;
use crate::paths::Paths;
use crate::scanner::{self, discovery, ScanContext, ScanInput};
use crate::storage::Storage;
use crate::tooling::ToolingFilter;
use crate::watcher::{self, WatcherHandle};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub const CONFIG_CHANGED_EVENT: &str = "config-changed";
const MAX_HASH_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub paths: Paths,
    pub db: Arc<Mutex<Storage>>,
    pub snapshot: Arc<Mutex<Option<ScanSnapshot>>>,
    pub watcher: Arc<Mutex<Option<WatcherHandle>>>,
    scan_lock: Arc<Mutex<()>>,
}

impl AppState {
    pub fn new(paths: Paths, storage: Storage) -> Self {
        Self {
            paths,
            db: Arc::new(Mutex::new(storage)),
            snapshot: Arc::new(Mutex::new(None)),
            watcher: Arc::new(Mutex::new(None)),
            scan_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn invalidate(&self) {
        *self.snapshot.lock() = None;
    }

    pub fn tooling_filter(&self) -> ToolingFilter {
        self.db
            .lock()
            .get_preferences()
            .map(|p| ToolingFilter::from_prefs(&p))
            .unwrap_or_default()
    }

    fn scan_input(&self) -> AppResult<ScanInput> {
        let db = self.db.lock();
        let prefs = db.get_preferences()?;
        Ok(ScanInput {
            scan_roots: db.list_scan_roots()?.into_iter().map(PathBuf::from).collect(),
            pinned: db.list_projects()?.into_iter().map(PathBuf::from).collect(),
            max_depth: scanner::DEFAULT_MAX_DEPTH,
            filter: ToolingFilter::from_prefs(&prefs),
        })
    }

    /// Full rescan; replaces the cached snapshot.
    pub fn scan(&self) -> AppResult<()> {
        let _guard = self.scan_lock.lock();
        self.scan_locked()
    }

    fn scan_locked(&self) -> AppResult<()> {
        let input = self.scan_input()?;
        let mut snap = scanner::scan_all(&self.paths, &input);
        {
            let db = self.db.lock();
            snap.stats.recent_changes = db.recent_changes(10)?;
            db.save_last_scan(&snap.stats)?;
        }
        *self.snapshot.lock() = Some(snap);
        Ok(())
    }

    /// Run `f` against the cached snapshot, scanning first if there is none.
    pub fn with_snapshot<T>(&self, f: impl FnOnce(&ScanSnapshot) -> T) -> AppResult<T> {
        if let Some(s) = self.snapshot.lock().as_ref() {
            return Ok(f(s));
        }
        let _guard = self.scan_lock.lock();
        if self.snapshot.lock().is_none() {
            self.scan_locked()?;
        }
        let g = self.snapshot.lock();
        let s = g.as_ref().ok_or_else(|| AppError::Other("scan produced no snapshot".into()))?;
        Ok(f(s))
    }

    pub fn fresh_snapshot(&self) -> AppResult<ScanSnapshot> {
        self.scan()?;
        self.with_snapshot(|s| s.clone())
    }

    /// Known project roots (canonical): scanned + pinned.
    pub fn project_roots(&self) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = self
            .snapshot
            .lock()
            .as_ref()
            .map(|s| s.projects.iter().map(|p| PathBuf::from(&p.project.path)).collect())
            .unwrap_or_default();
        if let Ok(pinned) = self.db.lock().list_projects() {
            for p in pinned {
                let c = discovery::canonical(Path::new(&p));
                if !roots.contains(&c) {
                    roots.push(c);
                }
            }
        }
        roots
    }

    /// Project bundle from the snapshot, or scanned on the fly for unknown directories.
    pub fn project_bundle(&self, path: &str) -> AppResult<ProjectBundle> {
        let canon = discovery::canonical(Path::new(path));
        let canon_s = canon.to_string_lossy().to_string();
        if let Some(b) = self.with_snapshot(|s| s.find_project(&canon_s).cloned())? {
            return Ok(b);
        }
        if !canon.is_dir() {
            return Err(AppError::NotFound(path.to_string()));
        }
        let filter = self.tooling_filter();
        Ok(scanner::scan_project_with(
            &ScanContext::load(&self.paths),
            &canon,
            &filter,
        ))
    }

    pub fn start_watching(&self, app: &AppHandle) -> AppResult<Vec<String>> {
        let roots = self.project_roots();
        let filter = self.tooling_filter();
        let state = self.clone();
        let app = app.clone();
        let sink: watcher::Sink = Arc::new(move |ev: ChangeEvent| {
            let hash = hash_file(Path::new(&ev.path));
            let notify_enabled = {
                let db = state.db.lock();
                let _ = db.record_change(&ev.path, &ev.kind, hash.as_deref());
                db.get_preferences().map(|p| p.notifications).unwrap_or(false)
            };
            state.invalidate();
            let _ = app.emit(CONFIG_CHANGED_EVENT, &ev);
            if notify_enabled && !ev.path.ends_with(".claude.json") {
                use tauri_plugin_notification::NotificationExt;
                let name = Path::new(&ev.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let _ = app
                    .notification()
                    .builder()
                    .title("Agent Scope")
                    .body(format!("{name} {}", ev.kind))
                    .show();
            }
        });
        let handle = watcher::start(self.paths.clone(), roots, filter, sink)?;
        let targets = handle.targets.iter().map(|p| p.to_string_lossy().to_string()).collect();
        *self.watcher.lock() = Some(handle);
        Ok(targets)
    }

    pub fn stop_watching(&self) -> bool {
        self.watcher.lock().take().is_some()
    }
}

/// SHA-256 of a file's bytes (read-only); `None` for missing/oversized files.
pub fn hash_file(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_HASH_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Some(hex::encode(h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn state_scan_and_cache() {
        let d = tempdir().unwrap();
        let paths = Paths::for_root(d.path());
        let proj = d.path().join("proj");
        fs::create_dir_all(proj.join(".claude")).unwrap();
        fs::write(proj.join(".claude/settings.json"), r#"{"model":"x"}"#).unwrap();

        let st = AppState::new(paths, Storage::open_in_memory().unwrap());
        st.db.lock().add_project(proj.to_str().unwrap()).unwrap();
        let n = st.with_snapshot(|s| s.projects.len()).unwrap();
        assert_eq!(n, 1);
        assert!(st.db.lock().load_last_scan().unwrap().is_some());
        assert_eq!(st.project_roots().len(), 1);
        let b = st.project_bundle(proj.to_str().unwrap()).unwrap();
        assert_eq!(b.items.settings_layers.len(), 1);
        st.invalidate();
        assert!(st.snapshot.lock().is_none());
        assert!(hash_file(&proj.join(".claude/settings.json")).is_some());
        assert!(hash_file(&proj.join("nope")).is_none());
    }
}
