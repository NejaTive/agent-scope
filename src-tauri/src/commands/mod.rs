//! Tauri commands. Config reads are always available; writing existing allowlisted
//! files is opt-in via `Preferences.allow_editing` and `check_writable`.

use crate::errors::{AppError, AppResult};
use crate::models::*;
use crate::parser;
use crate::paths::language_for;
use crate::resolver;
use crate::scanner::{self, discovery};
use crate::state::{self, AppState};
use std::io::Write;
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

fn require_editing_enabled(state: &AppState) -> AppResult<()> {
    let prefs = state.db.lock().get_preferences()?;
    if !prefs.allow_editing {
        return Err(AppError::AccessDenied(
            "editing is disabled; enable allowEditing in preferences".into(),
        ));
    }
    Ok(())
}

/// Raw (unmasked) content for Edit mode. Requires `allowEditing` and a writable path.
pub fn read_artifact_content_for_edit(state: &AppState, path: &str) -> AppResult<ArtifactContentForEdit> {
    require_editing_enabled(state)?;
    let roots = state.project_roots();
    let canon = state.paths.check_writable(Path::new(path), &roots)?;
    let meta = std::fs::metadata(&canon)?;
    if meta.len() > MAX_CONTENT_BYTES {
        return Err(AppError::InvalidPath(format!(
            "file is too large to edit ({} bytes)",
            meta.len()
        )));
    }
    let bytes = std::fs::read(&canon)?;
    let text = String::from_utf8(bytes).map_err(|_| AppError::Io("file is not valid UTF-8".into()))?;
    let content_hash = state::hash_file(&canon).ok_or_else(|| {
        AppError::Other(format!("could not hash {}", canon.display()))
    })?;
    Ok(ArtifactContentForEdit {
        text,
        language: language_for(&canon).into(),
        content_hash,
        writable: true,
    })
}

#[tauri::command]
pub async fn get_artifact_content_for_edit(
    path: String,
    state: State<'_, AppState>,
) -> Result<ArtifactContentForEdit, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || read_artifact_content_for_edit(&st, &path)).await
}

/// Atomically write allowlisted config. Returns previous content for UI undo (no `.bak`).
pub fn write_artifact_content(
    state: &AppState,
    path: &str,
    content: &str,
    expected_hash: Option<&str>,
) -> AppResult<SaveArtifactResult> {
    require_editing_enabled(state)?;
    let roots = state.project_roots();
    let canon = state.paths.check_writable(Path::new(path), &roots)?;

    if content.len() as u64 > MAX_CONTENT_BYTES {
        return Err(AppError::InvalidPath(format!(
            "content is too large to write ({} bytes)",
            content.len()
        )));
    }

    if language_for(&canon) == "json" {
        serde_json::from_str::<serde_json::Value>(content)?;
    }

    let current_hash = state::hash_file(&canon).ok_or_else(|| {
        AppError::Other(format!("could not hash {}", canon.display()))
    })?;
    if let Some(expected) = expected_hash {
        if expected != current_hash {
            return Err(AppError::Conflict(
                "file changed on disk; reload and try again".into(),
            ));
        }
    }

    let previous_bytes = std::fs::read(&canon)?;
    let previous_content = String::from_utf8(previous_bytes)
        .map_err(|_| AppError::Io("file is not valid UTF-8".into()))?;

    let parent = canon.parent().ok_or_else(|| {
        AppError::InvalidPath(format!("{} has no parent directory", canon.display()))
    })?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(content.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(&canon).map_err(|e| AppError::Io(e.to_string()))?;

    let content_hash = state::hash_file(&canon).ok_or_else(|| {
        AppError::Other(format!("could not hash after write {}", canon.display()))
    })?;
    let _ = state.db.lock().record_change(
        &canon.to_string_lossy(),
        "modified",
        Some(&content_hash),
    );
    state.invalidate();

    Ok(SaveArtifactResult {
        previous_content,
        content_hash,
    })
}

#[tauri::command]
pub async fn save_artifact_content(
    path: String,
    content: String,
    expected_hash: Option<String>,
    state: State<'_, AppState>,
) -> Result<SaveArtifactResult, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        write_artifact_content(&st, &path, &content, expected_hash.as_deref())
    })
    .await
}

fn check_expected_hash(canon: &Path, expected_hash: Option<&str>) -> AppResult<()> {
    if let Some(expected) = expected_hash {
        let current = state::hash_file(canon).ok_or_else(|| {
            AppError::Other(format!("could not hash {}", canon.display()))
        })?;
        if expected != current {
            return Err(AppError::Conflict(
                "file changed on disk; reload and try again".into(),
            ));
        }
    }
    Ok(())
}

/// Rename within the same directory. `new_name` is a basename only.
pub fn rename_artifact_file(
    state: &AppState,
    path: &str,
    new_name: &str,
    expected_hash: Option<&str>,
) -> AppResult<RenameArtifactResult> {
    require_editing_enabled(state)?;
    let roots = state.project_roots();
    let canon = state.paths.check_writable(Path::new(path), &roots)?;
    check_expected_hash(&canon, expected_hash)?;

    let parent = canon.parent().ok_or_else(|| {
        AppError::InvalidPath(format!("{} has no parent directory", canon.display()))
    })?;
    let dest = state
        .paths
        .check_writable_destination(parent, new_name, &roots)?;

    std::fs::rename(&canon, &dest)?;
    let new_path = dest.to_string_lossy().to_string();
    let _ = state
        .db
        .lock()
        .record_change(&new_path, "renamed", state::hash_file(&dest).as_deref());
    state.invalidate();
    Ok(RenameArtifactResult { new_path })
}

#[tauri::command]
pub async fn rename_artifact(
    path: String,
    new_name: String,
    expected_hash: Option<String>,
    state: State<'_, AppState>,
) -> Result<RenameArtifactResult, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || {
        rename_artifact_file(&st, &path, &new_name, expected_hash.as_deref())
    })
    .await
}

/// Delete a single allowlisted file (not recursive).
pub fn delete_artifact_file(
    state: &AppState,
    path: &str,
    expected_hash: Option<&str>,
) -> AppResult<DeleteArtifactResult> {
    require_editing_enabled(state)?;
    let roots = state.project_roots();
    let canon = state.paths.check_writable(Path::new(path), &roots)?;
    check_expected_hash(&canon, expected_hash)?;

    let deleted_path = canon.to_string_lossy().to_string();
    std::fs::remove_file(&canon)?;
    let _ = state.db.lock().record_change(&deleted_path, "deleted", None);
    state.invalidate();
    Ok(DeleteArtifactResult { deleted_path })
}

#[tauri::command]
pub async fn delete_artifact(
    path: String,
    expected_hash: Option<String>,
    state: State<'_, AppState>,
) -> Result<DeleteArtifactResult, AppError> {
    let st = state.inner().clone();
    spawn_blocking(move || delete_artifact_file(&st, &path, expected_hash.as_deref())).await
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

    fn enable_editing(st: &AppState) {
        let mut p = st.db.lock().get_preferences().unwrap();
        p.allow_editing = true;
        st.db.lock().set_preferences(&p).unwrap();
    }

    #[test]
    fn edit_requires_opt_in_and_writable() {
        let (d, st, proj) = fixture();
        st.with_snapshot(|_| ()).unwrap();
        let settings = proj.join(".claude/settings.json");
        let settings_s = settings.to_str().unwrap();

        assert!(read_artifact_content_for_edit(&st, settings_s).is_err());
        assert!(write_artifact_content(&st, settings_s, "{}", None).is_err());

        enable_editing(&st);

        let raw = read_artifact_content_for_edit(&st, settings_s).unwrap();
        assert!(raw.text.contains("supersecret"));
        assert!(raw.writable);
        let masked = read_artifact_content(&st, settings_s).unwrap();
        assert!(!masked.text.contains("supersecret"));
        assert!(masked.text.contains(parser::MASK));

        assert!(read_artifact_content_for_edit(
            &st,
            st.paths.credentials_file().to_str().unwrap()
        )
        .is_err());
        assert!(write_artifact_content(
            &st,
            d.path().join("outside.json").to_str().unwrap(),
            "{}",
            None
        )
        .is_err());
        assert!(write_artifact_content(
            &st,
            st.paths.claude_json.to_str().unwrap(),
            "{}",
            None
        )
        .is_err());
    }

    #[test]
    fn save_markdown_and_json() {
        let (_d, st, proj) = fixture();
        enable_editing(&st);
        let skill = proj.join(".claude/skills/alpha/SKILL.md");
        let skill_s = skill.to_str().unwrap();
        let settings = proj.join(".claude/settings.json");
        let settings_s = settings.to_str().unwrap();

        let before = read_artifact_content_for_edit(&st, skill_s).unwrap();
        let saved = write_artifact_content(
            &st,
            skill_s,
            "---\ndescription: Updated\n---\nbody\n",
            Some(&before.content_hash),
        )
        .unwrap();
        assert!(saved.previous_content.contains("Alpha thing"));
        assert_eq!(
            fs::read_to_string(&skill).unwrap(),
            "---\ndescription: Updated\n---\nbody\n"
        );

        let bad = write_artifact_content(&st, settings_s, "{not-json", None);
        assert!(bad.is_err());
        assert!(fs::read_to_string(&settings).unwrap().contains("supersecret"));

        let ok = write_artifact_content(
            &st,
            settings_s,
            r#"{"env":{"MY_TOKEN":"newsecret"}}"#,
            None,
        )
        .unwrap();
        assert!(ok.previous_content.contains("supersecret"));
        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            r#"{"env":{"MY_TOKEN":"newsecret"}}"#
        );
    }

    #[test]
    fn save_hash_conflict() {
        let (_d, st, proj) = fixture();
        enable_editing(&st);
        let settings = proj.join(".claude/settings.json");
        let settings_s = settings.to_str().unwrap();
        let before = read_artifact_content_for_edit(&st, settings_s).unwrap();
        fs::write(&settings, r#"{"env":{"A":"changed"}}"#).unwrap();
        let err = write_artifact_content(
            &st,
            settings_s,
            r#"{"env":{"A":"mine"}}"#,
            Some(&before.content_hash),
        );
        assert!(matches!(err, Err(AppError::Conflict(_))));
        assert!(fs::read_to_string(&settings).unwrap().contains("changed"));
    }

    #[test]
    fn rename_and_delete_artifact() {
        let (d, st, proj) = fixture();
        let skill_dir = proj.join(".claude/skills/alpha");
        fs::write(skill_dir.join("notes.md"), "note").unwrap();
        let notes = skill_dir.join("notes.md");
        let notes_s = notes.to_str().unwrap();

        assert!(rename_artifact_file(&st, notes_s, "renamed.md", None).is_err());
        enable_editing(&st);

        let before = read_artifact_content_for_edit(&st, notes_s).unwrap();
        let renamed = rename_artifact_file(&st, notes_s, "renamed.md", Some(&before.content_hash))
            .unwrap();
        assert!(!notes.exists());
        let new_path = PathBuf::from(&renamed.new_path);
        assert!(new_path.exists());
        assert_eq!(fs::read_to_string(&new_path).unwrap(), "note");

        assert!(rename_artifact_file(
            &st,
            new_path.to_str().unwrap(),
            "renamed.md",
            None
        )
        .is_err()); // dest exists (same name) — conflict if we try another file; rename to existing:
        fs::write(skill_dir.join("other.md"), "x").unwrap();
        assert!(matches!(
            rename_artifact_file(&st, skill_dir.join("other.md").to_str().unwrap(), "renamed.md", None),
            Err(AppError::Conflict(_))
        ));

        // Outside allowlist dest (project root FOO.md)
        let claude = proj.join("CLAUDE.md");
        fs::write(&claude, "x").unwrap();
        assert!(rename_artifact_file(&st, claude.to_str().unwrap(), "FOO.md", None).is_err());

        assert!(delete_artifact_file(
            &st,
            st.paths.credentials_file().to_str().unwrap(),
            None
        )
        .is_err());
        assert!(delete_artifact_file(
            &st,
            d.path().join("outside.json").to_str().unwrap(),
            None
        )
        .is_err());

        let hash = read_artifact_content_for_edit(&st, new_path.to_str().unwrap())
            .unwrap()
            .content_hash;
        fs::write(&new_path, "changed").unwrap();
        assert!(matches!(
            delete_artifact_file(&st, new_path.to_str().unwrap(), Some(&hash)),
            Err(AppError::Conflict(_))
        ));
        assert!(new_path.exists());

        delete_artifact_file(&st, new_path.to_str().unwrap(), None).unwrap();
        assert!(!new_path.exists());
    }
}
