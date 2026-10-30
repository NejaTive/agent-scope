//! Cursor config collectors (`~/.cursor`, `<root>/.cursor`, `.agents/skills`).
//! Read-only: hooks are inventoried only — never executed.

use super::collect::{
    artifact, collect_agents, collect_commands, collect_mcp_json, path_str, report,
};
use crate::models::*;
use crate::parser::{self, mask_secrets};
use crate::paths::{is_skipped_dir, Paths};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MAX_SKILL_DEPTH: usize = 8;
const MAX_FILES_PER_DIR: usize = 500;
const MAX_TREE_ENTRIES: usize = 200;

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

fn read_text(path: &Path) -> Result<String, (DiscoveryStatus, String)> {
    let bytes = fs::read(path).map_err(|e| (DiscoveryStatus::Unreadable, e.to_string()))?;
    String::from_utf8(bytes)
        .map_err(|_| (DiscoveryStatus::Unreadable, "file is not valid UTF-8".to_string()))
}

fn rel_name(base: &Path, p: &Path) -> String {
    p.strip_prefix(base)
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|_| file_name(p))
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

fn mark_cursor(a: &mut ConfigArtifact) {
    a.product = Tooling::Cursor;
}

fn already_seen(seen: &HashSet<String>, path: &Path) -> bool {
    seen.contains(&path_str(path))
}

fn remember(seen: &mut HashSet<String>, path: &Path) {
    seen.insert(path_str(path));
}

fn string_list(v: Option<&Value>, split_commas: bool) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::String(s)) if split_commas => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => Vec::new(),
    }
}

fn bool_field(fm: &Value, key: &str) -> Option<bool> {
    match fm.get(key) {
        Some(Value::Bool(b)) => Some(*b),
        Some(Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Cursor rules: `.mdc` with `globs` / `alwaysApply` / `description` frontmatter.
pub fn collect_cursor_rules(
    c: &mut Collection,
    dir: &Path,
    scope: Scope,
    project: Option<&str>,
    seen: &mut HashSet<String>,
) {
    if !dir.is_dir() {
        return;
    }
    let mut files: Vec<PathBuf> = WalkDir::new(dir)
        .max_depth(8)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !(e.file_type().is_dir() && is_skipped_dir(&file_name(e.path()))))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("mdc"))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect();
    files.sort();
    files.truncate(MAX_FILES_PER_DIR);

    for file in files {
        if already_seen(seen, &file) {
            continue;
        }
        remember(seen, &file);
        let name = rel_name(dir, &file);
        let mut a = artifact(ArtifactKind::Rule, scope, &name, &file, project);
        mark_cursor(&mut a);
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
                let fm = parser::parse_frontmatter(&text);
                let globs = string_list(fm.data.get("globs"), true);
                let always = bool_field(&fm.data, "alwaysApply").unwrap_or(false);
                let description = fm
                    .data
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                entry.paths = globs.clone();
                entry.conditional = !always && !globs.is_empty();
                let status =
                    if fm.error.is_some() && description.is_none() && globs.is_empty() && !always {
                        DiscoveryStatus::InvalidFrontmatter
                    } else {
                        DiscoveryStatus::Present
                    };
                entry.status = status;
                a.status = status;
                a.error = if status == DiscoveryStatus::InvalidFrontmatter {
                    fm.error.clone()
                } else {
                    None
                };
                a.description = description.or_else(|| {
                    Some(if always {
                        "Always apply".into()
                    } else if globs.is_empty() {
                        "Manual / intelligent".into()
                    } else {
                        format!("Applies to: {}", globs.join(", "))
                    })
                });
                a.metadata = json!({
                    "globs": globs,
                    "alwaysApply": always,
                    "frontmatter": fm.data,
                });
                if let Some(e) = &a.error {
                    report(c, &file, scope, status, e);
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

/// Nested `**/SKILL.md` under a Cursor/agents skills root (identity = parent folder).
pub fn collect_cursor_skills(
    c: &mut Collection,
    dir: &Path,
    scope: Scope,
    project: Option<&str>,
    seen: &mut HashSet<String>,
) {
    if !dir.is_dir() {
        return;
    }
    let mut skill_mds: Vec<PathBuf> = WalkDir::new(dir)
        .max_depth(MAX_SKILL_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !(e.file_type().is_dir() && is_skipped_dir(&file_name(e.path()))))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name() == "SKILL.md")
        .map(|e| e.into_path())
        .collect();
    skill_mds.sort();
    skill_mds.truncate(MAX_FILES_PER_DIR);

    for skill_md in skill_mds {
        if already_seen(seen, &skill_md) {
            continue;
        }
        remember(seen, &skill_md);
        let skill_dir = skill_md.parent().unwrap_or(&skill_md);
        let dir_name = file_name(skill_dir);
        let files = tree_listing(skill_dir);
        let mut a = artifact(ArtifactKind::Skill, scope, &dir_name, &skill_md, project);
        mark_cursor(&mut a);
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

/// Collect commands/agents under `.cursor`, tagging product and skipping known paths.
fn collect_cursor_commands_agents(
    c: &mut Collection,
    cursor_dir: &Path,
    scope: Scope,
    project: Option<&str>,
    seen: &mut HashSet<String>,
) {
    for (subdir, kind) in [("commands", ArtifactKind::Command), ("agents", ArtifactKind::Agent)] {
        let dir = cursor_dir.join(subdir);
        if !dir.is_dir() {
            continue;
        }
        let art_before = c.artifacts.len();
        let skill_before = c.skills.len();
        let agent_before = c.agents.len();
        match kind {
            ArtifactKind::Command => collect_commands(c, &dir, scope, project),
            ArtifactKind::Agent => collect_agents(c, &dir, scope, project),
            _ => {}
        }

        let mut drop_paths: HashSet<String> = HashSet::new();
        let mut kept_arts: Vec<ConfigArtifact> = c.artifacts.drain(art_before..).collect();
        for a in &mut kept_arts {
            if already_seen(seen, Path::new(&a.path)) {
                drop_paths.insert(a.path.clone());
            } else {
                remember(seen, Path::new(&a.path));
                mark_cursor(a);
            }
        }
        kept_arts.retain(|a| !drop_paths.contains(&a.path));
        c.artifacts.extend(kept_arts);

        if kind == ArtifactKind::Command {
            let mut kept: Vec<SkillEntry> = c.skills.drain(skill_before..).collect();
            kept.retain(|s| !drop_paths.contains(&s.path));
            c.skills.extend(kept);
        } else {
            let mut kept: Vec<AgentEntry> = c.agents.drain(agent_before..).collect();
            kept.retain(|a| !drop_paths.contains(&a.path));
            c.agents.extend(kept);
        }
    }
}

/// Cursor `hooks.json`: `{ "version": 1, "hooks": { "event": [{ "command": "..." }] } }`.
/// Never executes commands — records script paths only.
pub fn collect_cursor_hooks_json(
    c: &mut Collection,
    path: &Path,
    scope: Scope,
    project: Option<&str>,
    seen: &mut HashSet<String>,
) {
    if already_seen(seen, path) {
        return;
    }
    let parsed = parser::read_json_file(path);
    match parsed.status {
        DiscoveryStatus::Missing => {}
        status => {
            remember(seen, path);
            let mut a = artifact(ArtifactKind::Other, scope, &file_name(path), path, project);
            mark_cursor(&mut a);
            a.status = status;
            a.error = parsed.error.clone();
            if let Some(e) = &parsed.error {
                report(c, path, scope, status, e);
            }
            if let Some(v) = &parsed.value {
                let hooks = parse_cursor_hooks(v, scope, path, project);
                a.description = Some(format!("{} hook(s)", hooks.len()));
                a.metadata = json!({ "version": v.get("version"), "hookCount": hooks.len() });
                for (i, h) in hooks.iter().enumerate() {
                    let name = match &h.matcher {
                        Some(m) if !m.is_empty() => format!("{} [{}]", h.event, m),
                        _ => h.event.clone(),
                    };
                    let mut ha = ConfigArtifact::new(
                        ArtifactKind::Hook,
                        scope,
                        name,
                        path_str(path),
                        project.map(String::from),
                    );
                    mark_cursor(&mut ha);
                    ha.id = stable_id(&[
                        "cursor-hook",
                        scope.as_str(),
                        project.unwrap_or(""),
                        &path_str(path),
                        &i.to_string(),
                    ]);
                    ha.description = h.command.clone();
                    ha.metadata = json!({
                        "event": h.event,
                        "matcher": h.matcher,
                        "type": h.hook_type,
                        "timeout": h.timeout,
                        "command": h.command,
                    });
                    c.artifacts.push(ha);
                }
                c.hooks.extend(hooks);
            }
            c.artifacts.push(a);
        }
    }
}

pub fn parse_cursor_hooks(
    root: &Value,
    scope: Scope,
    source: &Path,
    project: Option<&str>,
) -> Vec<HookEntry> {
    let mut out = Vec::new();
    let Some(events) = root.get("hooks").and_then(|h| h.as_object()) else {
        return out;
    };
    for (event, handlers) in events {
        let Some(handlers) = handlers.as_array() else { continue };
        for h in handlers {
            let hook_type = h
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("command")
                .to_string();
            out.push(HookEntry {
                event: event.clone(),
                matcher: h.get("matcher").and_then(|m| m.as_str()).map(String::from),
                hook_type,
                command: h.get("command").and_then(|v| v.as_str()).map(String::from),
                prompt: h.get("prompt").and_then(|v| v.as_str()).map(String::from),
                timeout: h.get("timeout").and_then(|v| v.as_u64()),
                scope,
                source_path: path_str(source),
                project_path: project.map(String::from),
                details: mask_secrets(h),
            });
        }
    }
    out
}

fn collect_cursor_mcp(
    c: &mut Collection,
    path: &Path,
    scope: Scope,
    project: Option<&str>,
    origin: &str,
    seen: &mut HashSet<String>,
) {
    if already_seen(seen, path) {
        return;
    }
    let art_before = c.artifacts.len();
    let mcp_before = c.mcp_servers.len();
    collect_mcp_json(c, path, scope, project, origin);
    if c.artifacts.len() == art_before && c.mcp_servers.len() == mcp_before {
        return; // missing
    }
    remember(seen, path);
    for a in c.artifacts.iter_mut().skip(art_before) {
        mark_cursor(a);
    }
}

fn seed_seen(c: &Collection) -> HashSet<String> {
    let mut seen: HashSet<String> = c.artifacts.iter().map(|a| a.path.clone()).collect();
    for s in &c.skills {
        seen.insert(s.path.clone());
    }
    seen
}

/// User-scope Cursor inventory (`~/.cursor`, `~/.agents/skills`, managed hooks).
pub fn collect_cursor_user(c: &mut Collection, paths: &Paths) {
    let mut seen = seed_seen(c);
    let dir = &paths.cursor_dir;
    collect_cursor_rules(c, &dir.join("rules"), Scope::User, None, &mut seen);
    collect_cursor_skills(c, &dir.join("skills"), Scope::User, None, &mut seen);
    collect_cursor_skills(c, &paths.home.join(".agents/skills"), Scope::User, None, &mut seen);
    collect_cursor_commands_agents(c, dir, Scope::User, None, &mut seen);
    collect_cursor_hooks_json(c, &paths.cursor_hooks_json, Scope::User, None, &mut seen);
    collect_cursor_hooks_json(c, &paths.cursor_managed_hooks, Scope::Managed, None, &mut seen);
    collect_cursor_mcp(c, &paths.cursor_mcp_json, Scope::User, None, "~/.cursor/mcp.json", &mut seen);
}

/// Project-scope Cursor inventory (`<root>/.cursor`, `<root>/.agents/skills`).
/// Does not re-collect `.claude/**` or duplicate AGENTS.md already owned by Claude.
pub fn collect_cursor_project(c: &mut Collection, root: &Path, project: Option<&str>) {
    let mut seen = seed_seen(c);
    let cursor = root.join(".cursor");
    collect_cursor_rules(c, &cursor.join("rules"), Scope::Project, project, &mut seen);
    collect_cursor_skills(c, &cursor.join("skills"), Scope::Project, project, &mut seen);
    collect_cursor_skills(c, &root.join(".agents/skills"), Scope::Project, project, &mut seen);
    collect_cursor_commands_agents(c, &cursor, Scope::Project, project, &mut seen);
    collect_cursor_hooks_json(c, &cursor.join("hooks.json"), Scope::Project, project, &mut seen);
    collect_cursor_mcp(
        c,
        &cursor.join("mcp.json"),
        Scope::Project,
        project,
        ".cursor/mcp.json",
        &mut seen,
    );

    // AGENTS.md: only if Claude did not already collect this path.
    let agents = root.join("AGENTS.md");
    if agents.is_file() && !already_seen(&seen, &agents) {
        remember(&mut seen, &agents);
        let mut a = artifact(ArtifactKind::ClaudeMd, Scope::Project, "AGENTS.md", &agents, project);
        mark_cursor(&mut a);
        if let Ok(text) = read_text(&agents) {
            a.description = parser::first_paragraph_line(&text);
        }
        c.artifacts.push(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn parse_cursor_hooks_shape() {
        let v = json!({
            "version": 1,
            "hooks": {
                "afterFileEdit": [{ "command": ".cursor/hooks/format.sh" }],
                "beforeShellExecution": [{
                    "command": "./scripts/validate.sh",
                    "matcher": "rm|curl",
                    "failClosed": true
                }]
            }
        });
        let h = parse_cursor_hooks(&v, Scope::Project, &PathBuf::from("/p/.cursor/hooks.json"), Some("/p"));
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].event, "afterFileEdit");
        assert_eq!(h[0].command.as_deref(), Some(".cursor/hooks/format.sh"));
        let shell = h.iter().find(|x| x.event == "beforeShellExecution").unwrap();
        assert_eq!(shell.matcher.as_deref(), Some("rm|curl"));
        assert!(shell.details.get("failClosed").is_some());
    }
}
