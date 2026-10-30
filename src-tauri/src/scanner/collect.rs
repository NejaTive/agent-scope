//! Per-artifact collectors. All functions are read-only.

use crate::models::*;
use crate::parser::{self, mask_secrets};
use crate::paths::is_skipped_dir;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MAX_TREE_ENTRIES: usize = 200;
const MAX_FILES_PER_DIR: usize = 500;

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

fn stat(path: &Path) -> (Option<u64>, Option<String>) {
    match fs::metadata(path) {
        Ok(m) => {
            let modified = m
                .modified()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
            (Some(m.len()), modified)
        }
        Err(_) => (None, None),
    }
}

fn artifact(
    kind: ArtifactKind,
    scope: Scope,
    name: &str,
    path: &Path,
    project: Option<&str>,
) -> ConfigArtifact {
    let mut a = ConfigArtifact::new(kind, scope, name, path_str(path), project.map(|s| s.to_string()));
    let (size, modified) = stat(path);
    a.size_bytes = size;
    a.modified_at = modified;
    a
}

fn report(c: &mut Collection, path: &Path, scope: Scope, status: DiscoveryStatus, msg: &str) {
    c.errors.push(ScanError {
        path: path_str(path),
        scope: Some(scope),
        status,
        message: msg.to_string(),
    });
}

/// Read a text file. `Err((status, message))` on failure.
fn read_text(path: &Path) -> Result<String, (DiscoveryStatus, String)> {
    let bytes = fs::read(path).map_err(|e| (DiscoveryStatus::Unreadable, e.to_string()))?;
    String::from_utf8(bytes)
        .map_err(|_| (DiscoveryStatus::Unreadable, "file is not valid UTF-8".to_string()))
}

pub fn missing_artifact(
    kind: ArtifactKind,
    scope: Scope,
    name: &str,
    path: &Path,
    project: Option<&str>,
) -> ConfigArtifact {
    let mut a = ConfigArtifact::new(kind, scope, name, path_str(path), project.map(|s| s.to_string()));
    a.status = DiscoveryStatus::Missing;
    a
}

// ------------------------------------------------------------ settings

/// Collect a settings JSON file (settings.json / settings.local.json / managed-settings.json).
pub fn collect_settings_file(
    c: &mut Collection,
    path: &Path,
    scope: Scope,
    project: Option<&str>,
    report_missing: bool,
) {
    let name = file_name(path);
    let parsed = parser::read_json_file(path);
    match parsed.status {
        DiscoveryStatus::Missing => {
            if report_missing {
                c.artifacts
                    .push(missing_artifact(ArtifactKind::Settings, scope, &name, path, project));
            }
        }
        status => {
            let mut a = artifact(ArtifactKind::Settings, scope, &name, path, project);
            a.status = status;
            a.error = parsed.error.clone();
            if let Some(v) = &parsed.value {
                let keys: Vec<String> = v
                    .as_object()
                    .map(|o| o.keys().cloned().collect())
                    .unwrap_or_default();
                a.description = Some(format!("{} top-level key(s)", keys.len()));
                a.metadata = json!({ "keys": keys });
                if !v.is_object() {
                    a.status = DiscoveryStatus::Partial;
                    a.error = Some("settings file must contain a JSON object".into());
                }
            }
            if let Some(err) = &a.error {
                report(c, path, scope, a.status, err);
            }
            c.artifacts.push(a);
            if let Some(v) = parsed.value.filter(|v| v.is_object()) {
                super::hooks::collect_hooks(c, &v, scope, path, project);
                c.settings_layers.push(SettingsLayer {
                    scope,
                    path: path_str(path),
                    project_path: project.map(|s| s.to_string()),
                    value: mask_secrets(&v),
                });
            }
        }
    }
}

// ------------------------------------------------------------ markdown memory files

pub fn collect_claude_md(
    c: &mut Collection,
    path: &Path,
    scope: Scope,
    project: Option<&str>,
    report_missing: bool,
) {
    if !path.is_file() {
        if report_missing {
            c.artifacts.push(missing_artifact(
                ArtifactKind::ClaudeMd,
                scope,
                &file_name(path),
                path,
                project,
            ));
        }
        return;
    }
    let mut a = artifact(ArtifactKind::ClaudeMd, scope, &file_name(path), path, project);
    match read_text(path) {
        Ok(text) => {
            a.description = parser::first_paragraph_line(&text);
            let imports = text
                .lines()
                .flat_map(|l| l.split_whitespace())
                .filter(|w| w.starts_with('@') && w.len() > 1 && (w.contains('/') || w.contains('.')))
                .map(|w| w.to_string())
                .take(50)
                .collect::<Vec<_>>();
            if !imports.is_empty() {
                a.metadata = json!({ "imports": imports });
            }
        }
        Err((status, msg)) => {
            a.status = status;
            a.error = Some(msg.clone());
            report(c, path, scope, status, &msg);
        }
    }
    c.artifacts.push(a);
}

// ------------------------------------------------------------ directory walkers

fn files_with_ext(dir: &Path, exts: &[&str], max_depth: usize) -> Vec<PathBuf> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out: Vec<PathBuf> = WalkDir::new(dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !(e.file_type().is_dir() && is_skipped_dir(&file_name(e.path()))))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| exts.iter().any(|w| w.eq_ignore_ascii_case(x)))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect();
    out.sort();
    out.truncate(MAX_FILES_PER_DIR);
    out
}

fn rel_name(base: &Path, p: &Path) -> String {
    p.strip_prefix(base)
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|_| file_name(p))
}

fn strip_ext(s: &str) -> String {
    s.rsplit_once('.').map(|(a, _)| a.to_string()).unwrap_or_else(|| s.to_string())
}

pub fn collect_rules(c: &mut Collection, dir: &Path, scope: Scope, project: Option<&str>) {
    for file in files_with_ext(dir, &["md"], 8) {
        let name = rel_name(dir, &file);
        let mut a = artifact(ArtifactKind::Rule, scope, &name, &file, project);
        let mut entry = RuleEntry {
            name: name.clone(),
            scope,
            path: path_str(&file),
            project_path: project.map(|s| s.to_string()),
            paths: Vec::new(),
            conditional: false,
            status: DiscoveryStatus::Present,
        };
        match read_text(&file) {
            Ok(text) => {
                let meta = parser::parse_rule(&text);
                entry.paths = meta.paths.clone();
                entry.conditional = !meta.paths.is_empty();
                entry.status = meta.status;
                a.status = meta.status;
                a.error = meta.error.clone();
                a.description = Some(if meta.paths.is_empty() {
                    "Always loaded".to_string()
                } else {
                    format!("Applies to: {}", meta.paths.join(", "))
                });
                a.metadata = json!({ "paths": meta.paths });
                if let Some(e) = &meta.error {
                    report(c, &file, scope, meta.status, e);
                }
            }
            Err((status, msg)) => {
                entry.status = status;
                a.status = status;
                a.error = Some(msg.clone());
                report(c, &file, scope, status, &msg);
            }
        }
        c.rules.push(entry);
        c.artifacts.push(a);
    }
}

fn tree_listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = WalkDir::new(dir)
        .min_depth(1)
        .max_depth(5)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !(e.file_type().is_dir() && is_skipped_dir(&file_name(e.path()))))
        .filter_map(|e| e.ok())
        .map(|e| {
            let mut s = rel_name(dir, e.path());
            if e.file_type().is_dir() {
                s.push('/');
            }
            s
        })
        .collect();
    v.sort();
    v.truncate(MAX_TREE_ENTRIES);
    v
}

/// `skills/*/SKILL.md` + file tree listing.
pub fn collect_skills(c: &mut Collection, dir: &Path, scope: Scope, project: Option<&str>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut dirs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for skill_dir in dirs {
        let dir_name = file_name(&skill_dir);
        if is_skipped_dir(&dir_name) {
            continue;
        }
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.is_file() {
            continue;
        }
        let files = tree_listing(&skill_dir);
        let mut a = artifact(ArtifactKind::Skill, scope, &dir_name, &skill_md, project);
        a.children = files.clone();
        let mut entry = SkillEntry {
            name: dir_name.clone(),
            kind: ArtifactKind::Skill,
            description: None,
            scope,
            path: path_str(&skill_md),
            project_path: project.map(|s| s.to_string()),
            status: DiscoveryStatus::Present,
            frontmatter: Value::Null,
            files,
        };
        match read_text(&skill_md) {
            Ok(text) => {
                let meta = parser::parse_skill(&text);
                if let Some(n) = meta.name {
                    // The directory name is the invocation name; keep frontmatter name as metadata.
                    a.metadata = json!({ "frontmatterName": n, "frontmatter": meta.frontmatter });
                } else {
                    a.metadata = json!({ "frontmatter": meta.frontmatter });
                }
                entry.description = meta.description.clone();
                entry.frontmatter = meta.frontmatter;
                entry.status = meta.status;
                a.description = meta.description;
                a.status = meta.status;
                a.error = meta.error.clone();
                if let Some(e) = &meta.error {
                    report(c, &skill_md, scope, meta.status, e);
                }
            }
            Err((status, msg)) => {
                entry.status = status;
                a.status = status;
                a.error = Some(msg.clone());
                report(c, &skill_md, scope, status, &msg);
            }
        }
        c.skills.push(entry);
        c.artifacts.push(a);
    }
}

/// `commands/**/*.md` — slash commands (siblings of skills).
pub fn collect_commands(c: &mut Collection, dir: &Path, scope: Scope, project: Option<&str>) {
    for file in files_with_ext(dir, &["md"], 5) {
        let rel = rel_name(dir, &file);
        let name = strip_ext(&rel);
        let mut a = artifact(ArtifactKind::Command, scope, &name, &file, project);
        let mut entry = SkillEntry {
            name: file_name(&file).trim_end_matches(".md").to_string(),
            kind: ArtifactKind::Command,
            description: None,
            scope,
            path: path_str(&file),
            project_path: project.map(|s| s.to_string()),
            status: DiscoveryStatus::Present,
            frontmatter: Value::Null,
            files: Vec::new(),
        };
        match read_text(&file) {
            Ok(text) => {
                let meta = parser::parse_skill(&text);
                entry.description = meta.description.clone();
                entry.frontmatter = meta.frontmatter.clone();
                entry.status = meta.status;
                a.description = meta.description;
                a.status = meta.status;
                a.error = meta.error.clone();
                a.metadata = json!({ "frontmatter": meta.frontmatter });
                if let Some(e) = &meta.error {
                    report(c, &file, scope, meta.status, e);
                }
            }
            Err((status, msg)) => {
                entry.status = status;
                a.status = status;
                a.error = Some(msg.clone());
                report(c, &file, scope, status, &msg);
            }
        }
        c.skills.push(entry);
        c.artifacts.push(a);
    }
}

pub fn collect_agents(c: &mut Collection, dir: &Path, scope: Scope, project: Option<&str>) {
    for file in files_with_ext(dir, &["md"], 5) {
        let stem = strip_ext(&rel_name(dir, &file));
        let mut a = artifact(ArtifactKind::Agent, scope, &stem, &file, project);
        let mut entry = AgentEntry {
            name: stem.clone(),
            description: None,
            scope,
            path: path_str(&file),
            project_path: project.map(|s| s.to_string()),
            tools: Vec::new(),
            model: None,
            valid: false,
            status: DiscoveryStatus::Present,
            frontmatter: Value::Null,
        };
        match read_text(&file) {
            Ok(text) => {
                let meta = parser::parse_agent(&text);
                if let Some(n) = &meta.name {
                    entry.name = n.clone();
                    a.name = n.clone();
                }
                entry.description = meta.description.clone();
                entry.tools = meta.tools.clone();
                entry.model = meta.model.clone();
                entry.valid = meta.valid;
                entry.status = meta.status;
                entry.frontmatter = meta.frontmatter.clone();
                a.description = meta.description;
                a.status = meta.status;
                a.error = meta.error.clone();
                a.metadata = json!({
                    "valid": meta.valid,
                    "tools": meta.tools,
                    "model": meta.model,
                });
                if let Some(e) = &meta.error {
                    report(c, &file, scope, meta.status, e);
                }
            }
            Err((status, msg)) => {
                entry.status = status;
                a.status = status;
                a.error = Some(msg.clone());
                report(c, &file, scope, status, &msg);
            }
        }
        // Keep ids stable even when the frontmatter name changes the display name.
        a.id = stable_id(&["agent", scope.as_str(), project.unwrap_or(""), &path_str(&file)]);
        c.agents.push(entry);
        c.artifacts.push(a);
    }
}

pub fn collect_output_styles(c: &mut Collection, dir: &Path, scope: Scope, project: Option<&str>) {
    for file in files_with_ext(dir, &["md"], 3) {
        let name = strip_ext(&rel_name(dir, &file));
        let mut a = artifact(ArtifactKind::OutputStyle, scope, &name, &file, project);
        match read_text(&file) {
            Ok(text) => {
                let meta = parser::parse_skill(&text);
                a.description = meta.description;
                a.status = meta.status;
                a.error = meta.error.clone();
                if let Some(n) = meta.name {
                    a.metadata = json!({ "name": n });
                }
                if let Some(e) = &meta.error {
                    report(c, &file, scope, meta.status, e);
                }
            }
            Err((status, msg)) => {
                a.status = status;
                a.error = Some(msg.clone());
                report(c, &file, scope, status, &msg);
            }
        }
        c.artifacts.push(a);
    }
}

pub fn collect_workflows(c: &mut Collection, dir: &Path, scope: Scope, project: Option<&str>) {
    for file in files_with_ext(dir, &["js", "mjs", "cjs"], 3) {
        let name = strip_ext(&rel_name(dir, &file));
        c.artifacts
            .push(artifact(ArtifactKind::Workflow, scope, &name, &file, project));
    }
}

/// Everything that lives inside a `.claude` directory and is common to user and project scope.
pub fn collect_claude_dir(c: &mut Collection, dir: &Path, scope: Scope, project: Option<&str>) {
    collect_rules(c, &dir.join("rules"), scope, project);
    collect_skills(c, &dir.join("skills"), scope, project);
    collect_commands(c, &dir.join("commands"), scope, project);
    collect_agents(c, &dir.join("agents"), scope, project);
    collect_output_styles(c, &dir.join("output-styles"), scope, project);
    collect_workflows(c, &dir.join("workflows"), scope, project);
}

// ------------------------------------------------------------ MCP

pub fn config_hash(v: &Value) -> String {
    let mut h = Sha256::new();
    h.update(serde_json::to_string(v).unwrap_or_default().as_bytes());
    hex::encode(h.finalize())[..16].to_string()
}

/// Parse an `mcpServers` object into entries (secrets masked) and artifacts.
pub fn collect_mcp_servers(
    c: &mut Collection,
    servers: &Value,
    scope: Scope,
    source: &Path,
    project: Option<&str>,
    origin: &str,
) {
    let Some(map) = servers.as_object() else { return };
    for (name, cfg) in map {
        let masked = mask_secrets(cfg);
        let obj = cfg.as_object();
        let command = obj.and_then(|o| o.get("command")).and_then(|v| v.as_str()).map(String::from);
        let url = obj.and_then(|o| o.get("url")).and_then(|v| v.as_str()).map(String::from);
        let server_type = obj
            .and_then(|o| o.get("type"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| {
                if url.is_some() {
                    "http".into()
                } else {
                    "stdio".into()
                }
            });
        let args: Vec<String> = obj
            .and_then(|o| o.get("args"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        c.mcp_servers.push(McpServerEntry {
            name: name.clone(),
            scope,
            source_path: path_str(source),
            project_path: project.map(String::from),
            origin: origin.to_string(),
            server_type: server_type.clone(),
            command: command.clone(),
            args: args.clone(),
            url: url.clone(),
            config: masked.clone(),
            config_hash: config_hash(cfg),
        });
        let mut a = ConfigArtifact::new(
            ArtifactKind::McpServer,
            scope,
            name.clone(),
            path_str(source),
            project.map(String::from),
        );
        a.id = stable_id(&["mcp", scope.as_str(), project.unwrap_or(""), &path_str(source), origin, name]);
        a.description = Some(match (&command, &url) {
            (Some(cmd), _) => format!("{server_type}: {cmd} {}", args.join(" ")).trim().to_string(),
            (None, Some(u)) => format!("{server_type}: {u}"),
            _ => server_type.clone(),
        });
        a.metadata = json!({ "origin": origin, "config": masked });
        c.artifacts.push(a);
    }
}

/// Read a standalone `.mcp.json` / `managed-mcp.json`.
pub fn collect_mcp_json(
    c: &mut Collection,
    path: &Path,
    scope: Scope,
    project: Option<&str>,
    origin: &str,
) {
    let parsed = parser::read_json_file(path);
    match parsed.status {
        DiscoveryStatus::Missing => {}
        status => {
            let mut a = artifact(ArtifactKind::Other, scope, &file_name(path), path, project);
            a.status = status;
            a.error = parsed.error.clone();
            if let Some(e) = &parsed.error {
                report(c, path, scope, status, e);
            }
            let servers = parsed.value.as_ref().and_then(|v| v.get("mcpServers")).cloned();
            if let Some(s) = servers {
                a.description = Some(format!(
                    "{} MCP server(s)",
                    s.as_object().map(|o| o.len()).unwrap_or(0)
                ));
                collect_mcp_servers(c, &s, scope, path, project, origin);
            }
            c.artifacts.push(a);
        }
    }
}
