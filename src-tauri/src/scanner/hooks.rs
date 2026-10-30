//! Hook inventory from settings `hooks`. Hooks are parsed only — never executed.

use super::collect::path_str;
use crate::models::*;
use crate::parser::mask_secrets;
use serde_json::{json, Value};
use std::path::Path;

pub fn parse_hooks(
    settings: &Value,
    scope: Scope,
    source: &Path,
    project: Option<&str>,
) -> Vec<HookEntry> {
    let mut out = Vec::new();
    let Some(events) = settings.get("hooks").and_then(|h| h.as_object()) else {
        return out;
    };
    for (event, groups) in events {
        let Some(groups) = groups.as_array() else { continue };
        for group in groups {
            let matcher = group.get("matcher").and_then(|m| m.as_str()).map(String::from);
            let handlers: Vec<&Value> = match group.get("hooks").and_then(|h| h.as_array()) {
                Some(list) => list.iter().collect(),
                // Tolerate a group that is itself a handler.
                None if group.get("type").is_some() || group.get("command").is_some() => vec![group],
                None => Vec::new(),
            };
            for h in handlers {
                let hook_type = h
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("command")
                    .to_string();
                out.push(HookEntry {
                    event: event.clone(),
                    matcher: matcher.clone(),
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
    }
    out
}

pub fn collect_hooks(
    c: &mut Collection,
    settings: &Value,
    scope: Scope,
    source: &Path,
    project: Option<&str>,
) {
    let hooks = parse_hooks(settings, scope, source, project);
    for (i, h) in hooks.iter().enumerate() {
        let name = match &h.matcher {
            Some(m) if !m.is_empty() => format!("{} [{}]", h.event, m),
            _ => h.event.clone(),
        };
        let mut a = ConfigArtifact::new(
            ArtifactKind::Hook,
            scope,
            name,
            path_str(source),
            project.map(String::from),
        );
        a.id = stable_id(&["hook", scope.as_str(), project.unwrap_or(""), &path_str(source), &i.to_string()]);
        a.description = h.command.clone().or_else(|| h.prompt.clone());
        a.metadata = json!({
            "event": h.event,
            "matcher": h.matcher,
            "type": h.hook_type,
            "timeout": h.timeout,
        });
        c.artifacts.push(a);
    }
    c.hooks.extend(hooks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_hooks_structure() {
        let v = json!({"hooks": {
            "PreToolUse": [{"matcher": "Bash", "hooks": [
                {"type": "command", "command": "./check.sh", "timeout": 30},
                {"type": "prompt", "prompt": "ok?"}
            ]}],
            "Stop": [{"hooks": [{"type": "command", "command": "echo done"}]}]
        }});
        let h = parse_hooks(&v, Scope::User, &PathBuf::from("/x/settings.json"), None);
        assert_eq!(h.len(), 3);
        let pre: Vec<_> = h.iter().filter(|h| h.event == "PreToolUse").collect();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0].matcher.as_deref(), Some("Bash"));
        assert_eq!(pre[0].timeout, Some(30));
        assert_eq!(pre[1].prompt.as_deref(), Some("ok?"));
        assert!(h.iter().any(|h| h.event == "Stop" && h.matcher.is_none()));
    }
}
