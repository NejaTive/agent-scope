//! Tauri commands. Everything here is read-only with respect to Claude Code config.

use crate::errors::{AppError, AppResult};
use crate::models::*;
use crate::parser;
use crate::paths::language_for;
use crate::resolver;
use crate::scanner::{self, discovery};
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

const MAX_CONTENT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SEARCH_RESULTS: usize = 500;

fn spawn_blocking<T, F>(f: F) -> impl std::future::Future<Output = AppResult<T>>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    let h = tauri::async_runtime::spawn_blocking(f);
    async move {
        h.await
            .map_err(|e| AppError::Other(format!("background task failed: {e}")))?
    }
}

fn validate_dir(path: &str) -> AppResult<PathBuf> {
    let p = PathBuf::from(path);
    if !p.is_dir() {
        return Err(AppError::InvalidPath(format!("{path} is not a directory")));
    }
    Ok(discovery::canonical(&p))
}

// ---------------------------------------------------------------- app

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo { name: "Agent Scope".into(), version: env!("CARGO_PKG_VERSION").into() }
}

#[tauri::command]
pub async fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardStats, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        let mut stats = st.with_snapshot(|s| s.stats.clone())?;
        stats.recent_changes = st.db.lock().recent_changes(10)?;
        Ok(stats)
    })
    .await
}

#[tauri::command]
pub async fn run_scan(state: State<'_, AppState>) -> Result<ScanSnapshot, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || st.fresh_snapshot()).await
}

// ---------------------------------------------------------------- roots & projects

#[tauri::command]
pub fn list_scan_roots(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    state.db.lock().list_scan_roots()
}

#[tauri::command]
pub fn add_scan_root(path: String, state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let canon = validate_dir(&path)?;
    {
        let db = state.db.lock();
        db.add_scan_root(&canon.to_string_lossy())?;
    }
    state.invalidate();
    state.db.lock().list_scan_roots()
}

#[tauri::command]
pub fn remove_scan_root(path: String, state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    {
        let db = state.db.lock();
        // Accept either the stored string or its canonical form.
        let canon = discovery::canonical(Path::new(&path)).to_string_lossy().to_string();
        if !db.remove_scan_root(&path)? {
            db.remove_scan_root(&canon)?;
        }
    }
    state.invalidate();
    state.db.lock().list_scan_roots()
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || st.with_snapshot(|s| s.projects.iter().map(|p| p.project.clone()).collect())).await
}

#[tauri::command]
pub async fn add_project(path: String, state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        let canon = validate_dir(&path)?;
        st.db.lock().add_project(&canon.to_string_lossy())?;
        st.invalidate();
        st.with_snapshot(|s| s.projects.iter().map(|p| p.project.clone()).collect())
    })
    .await
}

#[tauri::command]
pub async fn remove_project(path: String, state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        {
            let db = st.db.lock();
            let canon = discovery::canonical(Path::new(&path)).to_string_lossy().to_string();
            if !db.remove_project(&path)? {
                db.remove_project(&canon)?;
            }
        }
        st.invalidate();
        st.with_snapshot(|s| s.projects.iter().map(|p| p.project.clone()).collect())
    })
    .await
}

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    spawn_blocking(move || {
        match app.dialog().file().blocking_pick_folder() {
            None => Ok(None),
            Some(fp) => fp
                .into_path()
                .map(|p| Some(p.to_string_lossy().to_string()))
                .map_err(|e| AppError::InvalidPath(e.to_string())),
        }
    })
    .await
}

// ---------------------------------------------------------------- content

/// Read an artifact's text, only from allowlisted locations. JSON is secret-masked.
pub fn read_artifact_content(state: &AppState, path: &str) -> AppResult<ArtifactContent> {
    let roots = state.project_roots();
    let canon = state.paths.check_readable(Path::new(path), &roots)?;
    let meta = std::fs::metadata(&canon)?;
    if meta.len() > MAX_CONTENT_BYTES {
        return Err(AppError::InvalidPath(format!(
            "file is too large to display ({} bytes)",
            meta.len()
        )));
    }
    let bytes = std::fs::read(&canon)?;
    let raw = String::from_utf8(bytes).map_err(|_| AppError::Io("file is not valid UTF-8".into()))?;

    if state.paths.is_claude_json(&canon) {
        let v = parser::parse_json_str(&raw).map_err(AppError::Json)?;
        let text = serde_json::to_string_pretty(&scanner::claude_json_view(&v))?;
        return Ok(ArtifactContent { text, language: "json".into() });
    }
    let language = language_for(&canon);
    let text = if language == "json" {
        parser::mask_json_text(&raw).unwrap_or_else(|| parser::mask_text_lenient(&raw))
    } else {
        raw
    };
    Ok(ArtifactContent { text, language: language.into() })
}

#[tauri::command]
pub async fn get_artifact_content(path: String, state: State<'_, AppState>) -> Result<ArtifactContent, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || read_artifact_content(&st, &path)).await
}

// ---------------------------------------------------------------- settings & conflicts

#[tauri::command]
pub async fn get_effective_settings(
    project_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ResolvedSetting>, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        let project = match &project_path {
            Some(p) => Some(st.project_bundle(p)?),
            None => None,
        };
        st.with_snapshot(|s| resolver::resolve(&resolver::layers_for(&s.global, project.as_ref())))
    })
    .await
}

#[tauri::command]
pub async fn get_conflicts(state: State<'_, AppState>) -> Result<Vec<Conflict>, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || st.with_snapshot(|s| s.conflicts.clone())).await
}

#[tauri::command]
pub async fn get_settings_diff(project_path: String, state: State<'_, AppState>) -> Result<SettingsDiff, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        let project = st.project_bundle(&project_path)?;
        st.with_snapshot(|s| resolver::settings_diff(&s.global, &project))
    })
    .await
}

// ---------------------------------------------------------------- changes & watching

#[tauri::command]
pub fn get_recent_changes(limit: Option<usize>, state: State<'_, AppState>) -> Result<Vec<RecentChange>, AppError> {
    state.db.lock().recent_changes(limit.unwrap_or(50).min(500))
}

#[tauri::command]
pub async fn start_watching(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        // Make sure the snapshot (and thus the project list) exists before subscribing.
        st.with_snapshot(|_| ())?;
        st.start_watching(&app)
    })
    .await
}

#[tauri::command]
pub fn stop_watching(state: State<'_, AppState>) -> bool {
    state.stop_watching()
}

// ---------------------------------------------------------------- search

pub fn search(
    snapshot: &ScanSnapshot,
    query: &str,
    kind: Option<ArtifactKind>,
    scope: Option<Scope>,
) -> Vec<ConfigArtifact> {
    let q = query.trim().to_lowercase();
    snapshot
        .all_artifacts()
        .filter(|a| a.status != DiscoveryStatus::Missing)
        .filter(|a| kind.map(|k| a.kind == k).unwrap_or(true))
        .filter(|a| scope.map(|s| a.scope == s).unwrap_or(true))
        .filter(|a| {
            q.is_empty()
                || a.name.to_lowercase().contains(&q)
                || a.path.to_lowercase().contains(&q)
                || a.description.as_deref().map(|d| d.to_lowercase().contains(&q)).unwrap_or(false)
        })
        .take(MAX_SEARCH_RESULTS)
        .cloned()
        .collect()
}

#[tauri::command]
pub async fn search_artifacts(
    query: String,
    kind: Option<ArtifactKind>,
    scope: Option<Scope>,
    state: State<'_, AppState>,
) -> Result<Vec<ConfigArtifact>, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || st.with_snapshot(|s| search(s, &query, kind, scope))).await
}

// ---------------------------------------------------------------- preferences

#[tauri::command]
pub fn get_preferences(state: State<'_, AppState>) -> Result<Preferences, AppError> {
    state.db.lock().get_preferences()
}

#[tauri::command]
pub fn set_preferences(prefs: Preferences, state: State<'_, AppState>) -> Result<Preferences, AppError> {
    if !matches!(prefs.theme.as_str(), "light" | "dark" | "system") {
        return Err(AppError::Other(format!("unknown theme `{}`", prefs.theme)));
    }
    let db = state.db.lock();
    db.set_preferences(&prefs)?;
    db.get_preferences()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Paths;
    use crate::storage::Storage;
    use std::fs;
    use tempfile::tempdir;

    fn fixture() -> (tempfile::TempDir, AppState, PathBuf) {
        let d = tempdir().unwrap();
        let paths = Paths::for_root(d.path());
        let proj = d.path().join("proj");
        fs::create_dir_all(proj.join(".claude/skills/alpha")).unwrap();
        fs::write(proj.join(".claude/skills/alpha/SKILL.md"), "---\ndescription: Alpha thing\n---\n").unwrap();
        fs::write(proj.join(".claude/settings.json"), r#"{"env":{"MY_TOKEN":"supersecret","A":"1"}}"#).unwrap();
        fs::create_dir_all(&paths.config_dir).unwrap();
        fs::write(paths.config_dir.join(".credentials.json"), "{\"t\":\"x\"}").unwrap();
        fs::write(&paths.claude_json, r#"{"userID":"uid-123","mcpServers":{"s":{"env":{"X_KEY":"kk"}}}}"#).unwrap();
        fs::write(d.path().join("outside.json"), "{}").unwrap();
        let st = AppState::new(paths, Storage::open_in_memory().unwrap());
        st.db.lock().add_project(proj.to_str().unwrap()).unwrap();
        (d, st, proj)
    }

    #[test]
    fn content_allowlist_and_masking() {
        let (d, st, proj) = fixture();
        st.with_snapshot(|_| ()).unwrap();

        let c = read_artifact_content(&st, proj.join(".claude/settings.json").to_str().unwrap()).unwrap();
        assert_eq!(c.language, "json");
        assert!(!c.text.contains("supersecret") && c.text.contains(parser::MASK));

        let md = read_artifact_content(&st, proj.join(".claude/skills/alpha/SKILL.md").to_str().unwrap()).unwrap();
        assert_eq!(md.language, "markdown");

        assert!(read_artifact_content(&st, d.path().join("outside.json").to_str().unwrap()).is_err());
        assert!(read_artifact_content(&st, st.paths.credentials_file().to_str().unwrap()).is_err());
        assert!(read_artifact_content(&st, "/etc/passwd").is_err());
        assert!(read_artifact_content(&st, d.path().join("proj/../outside.json").to_str().unwrap()).is_err());

        let cj = read_artifact_content(&st, st.paths.claude_json.to_str().unwrap()).unwrap();
        assert!(!cj.text.contains("uid-123") && !cj.text.contains("\"kk\""));
        assert!(cj.text.contains("mcpServers"));
    }

    #[test]
    fn search_filters() {
        let (_d, st, _p) = fixture();
        let r = st
            .with_snapshot(|s| {
                (
                    search(s, "alpha", None, None).len(),
                    search(s, "", Some(ArtifactKind::Skill), None).len(),
                    search(s, "alpha thing", Some(ArtifactKind::Rule), None).len(),
                    search(s, "", None, Some(Scope::Project)).len(),
                    search(s, "settings.json", None, None).iter().all(|a| a.status != DiscoveryStatus::Missing),
                )
            })
            .unwrap();
        assert_eq!(r.0, 1);
        assert_eq!(r.1, 1);
        assert_eq!(r.2, 0);
        assert!(r.3 >= 2);
        assert!(r.4);
    }

    #[test]
    fn effective_settings_for_project() {
        let (_d, st, proj) = fixture();
        let b = st.project_bundle(proj.to_str().unwrap()).unwrap();
        let r = st.with_snapshot(|s| resolver::resolve(&resolver::layers_for(&s.global, Some(&b)))).unwrap();
        let tok = r.iter().find(|x| x.key == "env.MY_TOKEN").unwrap();
        assert_eq!(tok.value, parser::MASK);
    }

    #[test]
    fn app_info_name() {
        assert_eq!(get_app_info().name, "Agent Scope");
    }
}
