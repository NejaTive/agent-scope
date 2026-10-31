//! Debounced file watching (notify). Observes only; never writes to watched files.

use crate::errors::AppResult;
use crate::models::ChangeEvent;
use crate::paths::{is_skipped_dir, Paths, PROJECT_FILE_NAMES};
use crate::tooling::ToolingFilter;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const DEBOUNCE: Duration = Duration::from_millis(400);

pub type Sink = Arc<dyn Fn(ChangeEvent) + Send + Sync + 'static>;

/// Dropping the handle stops watching (the event thread exits when the channel closes).
pub struct WatcherHandle {
    _watcher: RecommendedWatcher,
    pub targets: Vec<PathBuf>,
}

pub fn kind_str(kind: &EventKind) -> Option<&'static str> {
    match kind {
        EventKind::Create(_) => Some("created"),
        EventKind::Remove(_) => Some("removed"),
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => Some("renamed"),
        EventKind::Modify(_) => Some("modified"),
        // Access / Any / Other are noise.
        _ => None,
    }
}

/// Claude-ish project root files (not shared with Cursor).
const CLAUDE_PROJECT_FILES: &[&str] = &["CLAUDE.md", "CLAUDE.local.md", ".mcp.json"];

/// Which changed paths are worth reporting.
pub fn is_relevant(path: &Path, paths: &Paths, project_roots: &[PathBuf], filter: &ToolingFilter) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if name == ".DS_Store" || name == ".credentials.json" || name.ends_with('~') || name.ends_with(".swp")
        || name.ends_with(".tmp") || name.starts_with(".#")
    {
        return false;
    }
    if path.components().any(|c| is_skipped_dir(&c.as_os_str().to_string_lossy())) {
        return false;
    }
    if filter.claude {
        if path == paths.claude_json {
            return true;
        }
        if let Ok(rel) = path.strip_prefix(&paths.config_dir) {
            let comps: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
            return match comps.first().map(|s| s.as_str()) {
                Some("projects") => comps.iter().any(|c| c == "memory"),
                Some("settings.json") | Some("CLAUDE.md") | Some("rules") | Some("skills") | Some("commands")
                | Some("agents") | Some("output-styles") | Some("workflows") | Some("plugins") => true,
                _ => false,
            };
        }
        if path.starts_with(&paths.managed_dir) {
            return true;
        }
    }
    if filter.cursor {
        if path == paths.cursor_hooks_json || path == paths.cursor_mcp_json || path == paths.cursor_managed_hooks {
            return true;
        }
        if let Ok(rel) = path.strip_prefix(&paths.cursor_dir) {
            let comps: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
            return matches!(
                comps.first().map(|s| s.as_str()),
                Some("rules") | Some("skills") | Some("commands") | Some("agents") | Some("hooks")
                    | Some("hooks.json") | Some("mcp.json")
            );
        }
    }
    // User-level `~/.agents/skills` — shared; keep if either product is enabled.
    if filter.claude || filter.cursor {
        if let Ok(rel) = path.strip_prefix(&paths.home.join(".agents")) {
            let comps: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
            if comps.first().map(|s| s.as_str()) == Some("skills") {
                return true;
            }
        }
    }
    for root in project_roots {
        if let Ok(rel) = path.strip_prefix(root) {
            let comps: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
            let first = comps.first().map(|s| s.as_str());
            if first == Some(".claude") {
                return filter.claude;
            }
            if first == Some(".cursor") {
                return filter.cursor;
            }
            if first == Some(".agents") && comps.get(1).map(|s| s.as_str()) == Some("skills") {
                return filter.claude || filter.cursor;
            }
            if comps.len() == 1 && PROJECT_FILE_NAMES.contains(&name.as_str()) {
                if CLAUDE_PROJECT_FILES.contains(&name.as_str()) {
                    return filter.claude;
                }
                // AGENTS.md is shared.
                return filter.claude || filter.cursor;
            }
        }
    }
    false
}

/// Directories/files to subscribe to. Only paths that exist are returned.
pub fn watch_targets(
    paths: &Paths,
    project_roots: &[PathBuf],
    filter: &ToolingFilter,
) -> Vec<(PathBuf, RecursiveMode)> {
    let mut v: Vec<(PathBuf, RecursiveMode)> = Vec::new();
    let mut push = |p: PathBuf, m: RecursiveMode| {
        if p.exists() && !v.iter().any(|(q, _)| q == &p) {
            v.push((p, m));
        }
    };
    if filter.claude {
        // Config dir itself (non-recursive: settings.json, CLAUDE.md) + specific subtrees.
        // `projects/` is deliberately skipped — transcripts churn constantly.
        push(paths.config_dir.clone(), RecursiveMode::NonRecursive);
        for sub in ["rules", "skills", "commands", "agents", "output-styles", "workflows", "plugins"] {
            push(paths.config_dir.join(sub), RecursiveMode::Recursive);
        }
        if let Some(parent) = paths.claude_json.parent() {
            push(parent.to_path_buf(), RecursiveMode::NonRecursive);
        }
        push(paths.managed_dir.clone(), RecursiveMode::NonRecursive);
    }
    if filter.cursor {
        // Cursor user config
        push(paths.cursor_dir.clone(), RecursiveMode::NonRecursive);
        for sub in ["rules", "skills", "commands", "agents", "hooks"] {
            push(paths.cursor_dir.join(sub), RecursiveMode::Recursive);
        }
        if let Some(parent) = paths.cursor_managed_hooks.parent() {
            push(parent.to_path_buf(), RecursiveMode::NonRecursive);
        }
    }
    // Shared agents skills — keep if either product is enabled.
    if filter.claude || filter.cursor {
        push(paths.home.join(".agents/skills"), RecursiveMode::Recursive);
    }
    for root in project_roots {
        push(root.clone(), RecursiveMode::NonRecursive);
        if filter.claude {
            push(root.join(".claude"), RecursiveMode::Recursive);
        }
        if filter.cursor {
            push(root.join(".cursor"), RecursiveMode::Recursive);
        }
        if filter.claude || filter.cursor {
            push(root.join(".agents/skills"), RecursiveMode::Recursive);
        }
    }
    v
}

/// Coalesces bursts of events per path; emits once a path has been quiet for `delay`.
#[derive(Debug)]
pub struct Debouncer {
    delay: Duration,
    pending: HashMap<PathBuf, (&'static str, Instant)>,
}

impl Debouncer {
    pub fn new(delay: Duration) -> Self {
        Self { delay, pending: HashMap::new() }
    }

    pub fn push(&mut self, path: PathBuf, kind: &'static str, now: Instant) {
        let merged = match self.pending.get(&path).map(|(k, _)| *k) {
            // created-then-modified is still "created"; removed-then-created is a modification.
            Some("created") if kind == "modified" => "created",
            Some("removed") if kind == "created" => "modified",
            _ => kind,
        };
        self.pending.insert(path, (merged, now));
    }

    pub fn drain_ready(&mut self, now: Instant) -> Vec<ChangeEvent> {
        let ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (_, t))| now.duration_since(*t) >= self.delay)
            .map(|(p, _)| p.clone())
            .collect();
        let mut out: Vec<ChangeEvent> = ready
            .into_iter()
            .filter_map(|p| {
                self.pending.remove(&p).map(|(k, _)| ChangeEvent {
                    path: p.to_string_lossy().to_string(),
                    kind: k.to_string(),
                })
            })
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// FSEvents reports resolved paths, so match against canonical locations.
fn canonical_paths(p: &Paths) -> Paths {
    let canon = |x: &Path| dunce::canonicalize(x).unwrap_or_else(|_| x.to_path_buf());
    let claude_json = match (p.claude_json.parent(), p.claude_json.file_name()) {
        (Some(parent), Some(name)) => canon(parent).join(name),
        _ => p.claude_json.clone(),
    };
    let cursor_dir = canon(&p.cursor_dir);
    let cursor_mcp_json = match (p.cursor_mcp_json.parent(), p.cursor_mcp_json.file_name()) {
        (Some(parent), Some(name)) => canon(parent).join(name),
        _ => p.cursor_mcp_json.clone(),
    };
    let cursor_hooks_json = match (p.cursor_hooks_json.parent(), p.cursor_hooks_json.file_name()) {
        (Some(parent), Some(name)) => canon(parent).join(name),
        _ => p.cursor_hooks_json.clone(),
    };
    let cursor_managed_hooks = match (p.cursor_managed_hooks.parent(), p.cursor_managed_hooks.file_name()) {
        (Some(parent), Some(name)) => canon(parent).join(name),
        _ => p.cursor_managed_hooks.clone(),
    };
    Paths {
        home: canon(&p.home),
        config_dir: canon(&p.config_dir),
        claude_json,
        managed_dir: canon(&p.managed_dir),
        cursor_dir,
        cursor_mcp_json,
        cursor_hooks_json,
        cursor_managed_hooks,
    }
}

pub fn start(
    paths: Paths,
    project_roots: Vec<PathBuf>,
    filter: ToolingFilter,
    sink: Sink,
) -> AppResult<WatcherHandle> {
    let paths = canonical_paths(&paths);
    let project_roots: Vec<PathBuf> =
        project_roots.iter().map(|r| dunce::canonicalize(r).unwrap_or_else(|_| r.clone())).collect();
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    let targets = watch_targets(&paths, &project_roots, &filter);
    let mut watched = Vec::new();
    for (p, mode) in &targets {
        // Individual failures (permissions, vanished dirs) shouldn't stop the whole watcher.
        if watcher.watch(p, *mode).is_ok() {
            watched.push(p.clone());
        }
    }

    std::thread::Builder::new()
        .name("agent-scope-watcher".into())
        .spawn(move || {
            let mut deb = Debouncer::new(DEBOUNCE);
            loop {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(ev)) => {
                        if let Some(kind) = kind_str(&ev.kind) {
                            let now = Instant::now();
                            for p in ev.paths {
                                if is_relevant(&p, &paths, &project_roots, &filter) {
                                    deb.push(p, kind, now);
                                }
                            }
                        }
                    }
                    Ok(Err(_)) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
                if !deb.is_empty() {
                    for ev in deb.drain_ready(Instant::now()) {
                        sink(ev);
                    }
                }
            }
        })
        .map_err(|e| crate::errors::AppError::Watcher(e.to_string()))?;

    Ok(WatcherHandle { _watcher: watcher, targets: watched })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn debounce_coalesces() {
        let mut d = Debouncer::new(Duration::from_millis(400));
        let t0 = Instant::now();
        let p = PathBuf::from("/a/settings.json");
        d.push(p.clone(), "modified", t0);
        d.push(p.clone(), "modified", t0 + Duration::from_millis(300));
        assert!(d.drain_ready(t0 + Duration::from_millis(500)).is_empty());
        let out = d.drain_ready(t0 + Duration::from_millis(800));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "modified");
        assert!(d.is_empty());
    }

    #[test]
    fn kind_merging() {
        let mut d = Debouncer::new(Duration::ZERO);
        let t = Instant::now();
        d.push("/x".into(), "created", t);
        d.push("/x".into(), "modified", t);
        d.push("/y".into(), "removed", t);
        d.push("/y".into(), "created", t);
        let out = d.drain_ready(t + Duration::from_millis(1));
        assert_eq!(out[0].kind, "created");
        assert_eq!(out[1].kind, "modified");
    }

    fn all() -> ToolingFilter {
        ToolingFilter::all()
    }

    #[test]
    fn relevance_filter() {
        let d = tempdir().unwrap();
        let paths = Paths::for_root(d.path());
        let proj = d.path().join("proj");
        let roots = vec![proj.clone()];
        let f = all();
        assert!(is_relevant(&paths.config_dir.join("settings.json"), &paths, &roots, &f));
        assert!(is_relevant(&paths.config_dir.join("skills/x/SKILL.md"), &paths, &roots, &f));
        assert!(is_relevant(&paths.config_dir.join("projects/a/memory/MEMORY.md"), &paths, &roots, &f));
        assert!(!is_relevant(&paths.config_dir.join("projects/a/t.jsonl"), &paths, &roots, &f));
        assert!(!is_relevant(&paths.config_dir.join(".credentials.json"), &paths, &roots, &f));
        assert!(!is_relevant(&paths.config_dir.join("history.jsonl"), &paths, &roots, &f));
        assert!(is_relevant(&paths.claude_json, &paths, &roots, &f));
        assert!(is_relevant(&proj.join("CLAUDE.md"), &paths, &roots, &f));
        assert!(is_relevant(&proj.join(".claude/settings.json"), &paths, &roots, &f));
        assert!(!is_relevant(&proj.join("src/main.rs"), &paths, &roots, &f));
        assert!(!is_relevant(&proj.join(".claude/node_modules/x"), &paths, &roots, &f));
        assert!(!is_relevant(&d.path().join("random.txt"), &paths, &roots, &f));
        // Cursor
        assert!(is_relevant(&paths.cursor_dir.join("rules/a.mdc"), &paths, &roots, &f));
        assert!(is_relevant(&paths.cursor_mcp_json, &paths, &roots, &f));
        assert!(is_relevant(&paths.cursor_hooks_json, &paths, &roots, &f));
        assert!(is_relevant(&proj.join(".cursor/skills/x/SKILL.md"), &paths, &roots, &f));
        assert!(is_relevant(&proj.join(".agents/skills/y/SKILL.md"), &paths, &roots, &f));
        assert!(!is_relevant(&paths.cursor_dir.join("secrets.txt"), &paths, &roots, &f));

        let cursor_off = ToolingFilter {
            claude: true,
            cursor: false,
        };
        assert!(!is_relevant(&paths.cursor_dir.join("rules/a.mdc"), &paths, &roots, &cursor_off));
        assert!(!is_relevant(&proj.join(".cursor/skills/x/SKILL.md"), &paths, &roots, &cursor_off));
        assert!(is_relevant(&proj.join(".agents/skills/y/SKILL.md"), &paths, &roots, &cursor_off));
        assert!(is_relevant(&paths.config_dir.join("settings.json"), &paths, &roots, &cursor_off));
    }

    #[test]
    fn targets_only_existing() {
        let d = tempdir().unwrap();
        let paths = Paths::for_root(d.path());
        std::fs::create_dir_all(paths.config_dir.join("rules")).unwrap();
        std::fs::create_dir_all(paths.cursor_dir.join("skills")).unwrap();
        let proj = d.path().join("p");
        std::fs::create_dir_all(proj.join(".claude")).unwrap();
        std::fs::create_dir_all(proj.join(".cursor")).unwrap();
        let t = watch_targets(&paths, &[proj.clone()], &all());
        assert!(t.iter().any(|(p, m)| p == &paths.config_dir.join("rules") && *m == RecursiveMode::Recursive));
        assert!(t.iter().any(|(p, _)| p == &proj.join(".claude")));
        assert!(!t.iter().any(|(p, _)| p == &paths.config_dir.join("skills")));
        assert!(t.iter().any(|(p, m)| p == &paths.cursor_dir.join("skills") && *m == RecursiveMode::Recursive));
        assert!(t.iter().any(|(p, _)| p == &proj.join(".cursor")));

        let claude_only = ToolingFilter {
            claude: true,
            cursor: false,
        };
        let t = watch_targets(&paths, &[proj.clone()], &claude_only);
        assert!(t.iter().any(|(p, _)| p == &proj.join(".claude")));
        assert!(!t.iter().any(|(p, _)| p == &proj.join(".cursor")));
        assert!(!t.iter().any(|(p, _)| p == &paths.cursor_dir.join("skills")));
    }

    #[test]
    fn live_watch_smoke() {
        // Starts and stops cleanly on a tempdir; event delivery itself is OS dependent.
        let d = tempdir().unwrap();
        let paths = Paths::for_root(d.path());
        std::fs::create_dir_all(&paths.config_dir).unwrap();
        let h = start(paths, vec![], ToolingFilter::all(), Arc::new(|_| {})).unwrap();
        assert!(!h.targets.is_empty());
        drop(h);
    }
}
