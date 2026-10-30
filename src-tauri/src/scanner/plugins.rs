//! Plugin inventory, memory listing and credentials presence. Read-only.

use super::collect::path_str;
use crate::models::*;
use crate::parser::{self, mask_secrets};
use crate::paths::Paths;
use serde_json::{json, Value};
use std::fs;
use walkdir::WalkDir;

pub fn collect_plugins(c: &mut Collection, paths: &Paths) {
    let dir = paths.config_dir.join("plugins");
    if !dir.is_dir() {
        return;
    }

    let installed = dir.join("installed_plugins.json");
    let parsed = parser::read_json_file(&installed);
    match parsed.status {
        DiscoveryStatus::Missing => {}
        DiscoveryStatus::Present => {
            if let Some(v) = &parsed.value {
                // Tolerate both `{ plugins: { "name@mkt": [..] | {..} } }` and a flat map.
                let map = v.get("plugins").and_then(|p| p.as_object()).or_else(|| v.as_object());
                for (key, records) in map.into_iter().flatten() {
                    if !(records.is_array() || records.is_object()) {
                        continue;
                    }
                    let list: Vec<&Value> = match records {
                        Value::Array(a) => a.iter().collect(),
                        other => vec![other],
                    };
                    for (i, rec) in list.into_iter().enumerate() {
                        let mut a = ConfigArtifact::new(
                            ArtifactKind::Plugin,
                            Scope::Plugin,
                            key.clone(),
                            path_str(&installed),
                            None,
                        );
                        a.id = stable_id(&["plugin", &path_str(&installed), key, &i.to_string()]);
                        let version = rec.get("version").and_then(|x| x.as_str());
                        let install_scope = rec.get("scope").and_then(|x| x.as_str());
                        a.description = Some(
                            [version.map(|v| format!("v{v}")), install_scope.map(|s| format!("{s} scope"))]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                                .join(" · "),
                        )
                        .filter(|s| !s.is_empty());
                        a.metadata = json!({ "installed": true, "record": mask_secrets(rec) });
                        c.artifacts.push(a);
                    }
                }
            }
        }
        status => {
            let mut a = ConfigArtifact::new(
                ArtifactKind::Plugin,
                Scope::Plugin,
                "installed_plugins.json",
                path_str(&installed),
                None,
            );
            a.status = status;
            a.error = parsed.error.clone();
            c.errors.push(ScanError {
                path: path_str(&installed),
                scope: Some(Scope::Plugin),
                status,
                message: parsed.error.unwrap_or_default(),
            });
            c.artifacts.push(a);
        }
    }

    let known = dir.join("known_marketplaces.json");
    if known.is_file() {
        let parsed = parser::read_json_file(&known);
        if let Some(obj) = parsed.value.as_ref().and_then(|v| v.as_object()) {
            for (name, rec) in obj {
                let mut a = ConfigArtifact::new(
                    ArtifactKind::Plugin,
                    Scope::Plugin,
                    name.clone(),
                    path_str(&known),
                    None,
                );
                a.id = stable_id(&["marketplace", &path_str(&known), name]);
                a.description = Some("Marketplace".into());
                a.metadata = json!({ "marketplace": true, "record": mask_secrets(rec) });
                c.artifacts.push(a);
            }
        }
    }

    let marketplaces = dir.join("marketplaces");
    if let Ok(rd) = fs::read_dir(&marketplaces) {
        let mut dirs: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
        dirs.sort();
        for d in dirs {
            let name = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let mut a = ConfigArtifact::new(
                ArtifactKind::Plugin,
                Scope::Plugin,
                name,
                path_str(&d),
                None,
            );
            a.description = Some("Marketplace directory".into());
            a.metadata = json!({ "marketplace": true, "directory": true });
            c.artifacts.push(a);
        }
    }
}

/// `projects/*/memory/**` — list only (never parsed).
pub fn collect_memory(c: &mut Collection, paths: &Paths) {
    let projects = paths.config_dir.join("projects");
    let Ok(rd) = fs::read_dir(&projects) else { return };
    let mut dirs: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for pd in dirs {
        let memory = pd.join("memory");
        if !memory.is_dir() {
            continue;
        }
        let encoded = pd.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut files: Vec<_> = WalkDir::new(&memory)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .collect();
        files.sort();
        files.truncate(200);
        for f in files {
            let rel = f.strip_prefix(&memory).map(|r| r.to_string_lossy().to_string()).unwrap_or_default();
            let mut a = ConfigArtifact::new(
                ArtifactKind::Memory,
                Scope::User,
                format!("{encoded}/{rel}"),
                path_str(&f),
                None,
            );
            if let Ok(m) = fs::metadata(&f) {
                a.size_bytes = Some(m.len());
                a.modified_at = m
                    .modified()
                    .ok()
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
            }
            a.metadata = json!({ "projectKey": encoded });
            c.artifacts.push(a);
        }
    }
}

/// Note presence of `.credentials.json` without opening it.
pub fn credentials_artifact(paths: &Paths) -> Option<ConfigArtifact> {
    let p = paths.credentials_file();
    if fs::symlink_metadata(&p).is_err() {
        return None;
    }
    let mut a = ConfigArtifact::new(ArtifactKind::Other, Scope::User, ".credentials.json", path_str(&p), None);
    a.description = Some("Credentials file present (contents hidden)".into());
    a.metadata = json!({ "sensitive": true, "contentHidden": true });
    Some(a)
}
