//! OpenAI Codex config collectors (`$CODEX_HOME`, `<root>/.codex`, `.agents/skills`).
//! Read-only: hooks are inventoried only — never executed. `auth.json` is never read.

use super::collect::{artifact, path_str, report};
use super::cursor::{already_seen, collect_skills_for, file_name, read_text, remember, seed_seen};
use crate::models::*;
use crate::parser;
use crate::paths::Paths;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;

fn mark_codex(a: &mut ConfigArtifact) {
    a.product = Tooling::Codex;
}

/// Merge `legacy: true` into the metadata of artifacts collected since `art_before`.
fn mark_legacy(c: &mut Collection, art_before: usize) {
    for a in c.artifacts.iter_mut().skip(art_before) {
        match &mut a.metadata {
            Value::Object(m) => {
                m.insert("legacy".into(), Value::Bool(true));
            }
            other => *other = json!({ "legacy": true }),
        }
    }
}

fn collect_skills(
    c: &mut Collection,
    dir: &Path,
    scope: Scope,
    project: Option<&str>,
    seen: &mut HashSet<String>,
    legacy: bool,
) {
    let before = c.artifacts.len();
    collect_skills_for(c, dir, scope, project, seen, Tooling::Codex);
    if legacy {
        mark_legacy(c, before);
    }
}

/// `config.toml` as a Settings artifact (TOML is shown as text; no key merge).
fn collect_config_toml(
    c: &mut Collection,
    path: &Path,
    scope: Scope,
    project: Option<&str>,
    seen: &mut HashSet<String>,
) {
    if !path.is_file() || already_seen(seen, path) {
        return;
    }
    remember(seen, path);
    let mut a = artifact(ArtifactKind::Settings, scope, &file_name(path), path, project);
    mark_codex(&mut a);
    a.metadata = json!({ "language": "toml" });
    match read_text(path) {
        Ok(_) => a.description = Some("Codex config (TOML)".into()),
        Err((status, msg)) => {
            a.status = status;
            a.error = Some(msg.clone());
            report(c, path, scope, status, &msg);
        }
    }
    c.artifacts.push(a);
}

/// `AGENTS.md` / `AGENTS.override.md` as memory-like artifacts.
fn collect_agents_md(
    c: &mut Collection,
    path: &Path,
    scope: Scope,
    project: Option<&str>,
    seen: &mut HashSet<String>,
) {
    if !path.is_file() || already_seen(seen, path) {
        return;
    }
    remember(seen, path);
    let name = file_name(path);
    let mut a = artifact(ArtifactKind::ClaudeMd, scope, &name, path, project);
    mark_codex(&mut a);
    if name == "AGENTS.override.md" {
        a.metadata = json!({ "override": true });
    }
    match read_text(path) {
        Ok(text) => a.description = parser::first_paragraph_line(&text),
        Err((status, msg)) => {
            a.status = status;
            a.error = Some(msg.clone());
            report(c, path, scope, status, &msg);
        }
    }
    c.artifacts.push(a);
}

/// Codex `hooks.json`: `{ "hooks": { "Event": [{ "matcher": "...", "hooks": [{ "command": "..." }] }] } }`.
/// Parsing is best-effort (nested Claude-like shape); the file artifact is always recorded.
fn collect_codex_hooks_json(
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
    if parsed.status == DiscoveryStatus::Missing {
        return;
    }
    remember(seen, path);
    let mut a = artifact(ArtifactKind::Other, scope, &file_name(path), path, project);
    mark_codex(&mut a);
    a.status = parsed.status;
    a.error = parsed.error.clone();
    if let Some(e) = &parsed.error {
        report(c, path, scope, parsed.status, e);
    }
    if let Some(v) = &parsed.value {
        let hooks = super::hooks::parse_hooks(v, scope, path, project);
        a.description = Some(format!("{} hook(s)", hooks.len()));
        a.metadata = json!({ "hookCount": hooks.len() });
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
            mark_codex(&mut ha);
            ha.id = stable_id(&[
                "codex-hook",
                scope.as_str(),
                project.unwrap_or(""),
                &path_str(path),
                &i.to_string(),
            ]);
            ha.description = h.command.clone().or_else(|| h.prompt.clone());
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

/// User-scope Codex inventory (`$CODEX_HOME`, `~/.agents/skills`, admin skills).
pub fn collect_codex_user(c: &mut Collection, paths: &Paths) {
    let mut seen = seed_seen(c);
    let dir = &paths.codex_dir;
    collect_config_toml(c, &dir.join("config.toml"), Scope::User, None, &mut seen);
    collect_agents_md(c, &dir.join("AGENTS.md"), Scope::User, None, &mut seen);
    collect_agents_md(c, &dir.join("AGENTS.override.md"), Scope::User, None, &mut seen);
    collect_codex_hooks_json(c, &dir.join("hooks.json"), Scope::User, None, &mut seen);
    collect_skills(c, &dir.join("skills"), Scope::User, None, &mut seen, true);
    collect_skills(c, &paths.home.join(".agents/skills"), Scope::User, None, &mut seen, false);
    collect_skills(c, &paths.codex_admin_skills, Scope::Managed, None, &mut seen, false);
}

/// Project-scope Codex inventory (`<root>/.codex`, `<root>/.agents/skills`, root `AGENTS*.md`).
/// Paths already claimed by Claude / Cursor are skipped (one path = one artifact).
pub fn collect_codex_project(c: &mut Collection, root: &Path, project: Option<&str>) {
    let mut seen = seed_seen(c);
    let codex = root.join(".codex");
    collect_config_toml(c, &codex.join("config.toml"), Scope::Project, project, &mut seen);
    collect_codex_hooks_json(c, &codex.join("hooks.json"), Scope::Project, project, &mut seen);
    collect_skills(c, &codex.join("skills"), Scope::Project, project, &mut seen, false);
    collect_skills(c, &root.join(".agents/skills"), Scope::Project, project, &mut seen, false);
    collect_agents_md(c, &root.join("AGENTS.md"), Scope::Project, project, &mut seen);
    collect_agents_md(c, &root.join("AGENTS.override.md"), Scope::Project, project, &mut seen);
}
