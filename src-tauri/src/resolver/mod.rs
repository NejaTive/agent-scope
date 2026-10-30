//! Effective settings resolution and conflict detection.
//!
//! Precedence (highest first): Managed > ProjectLocal > Project > User.
//! Session `--settings` / CLI flags are not scanned and therefore not modelled.

use crate::models::*;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

/// Scalar keys whose precedence behaviour is documented.
const KNOWN_SCALAR_KEYS: &[&str] = &[
    "model",
    "permissions.defaultMode",
    "permissions.disableBypassPermissionsMode",
    "outputStyle",
    "language",
    "cleanupPeriodDays",
    "includeCoAuthoredBy",
    "includeGitInstructions",
    "apiKeyHelper",
    "forceLoginMethod",
    "forceLoginOrgUUID",
    "enableAllProjectMcpServers",
    "disableAllHooks",
    "awsAuthRefresh",
    "awsCredentialExport",
    "otelHeadersHelper",
    "alwaysThinkingEnabled",
    "autoUpdatesChannel",
    "theme",
    "respectGitignore",
    "defaultShell",
    "spinnerTipsEnabled",
    "statusLine",
];

/// List keys that are merged (union, de-duplicated) across scopes.
const KNOWN_LIST_KEYS: &[&str] = &[
    "permissions.allow",
    "permissions.deny",
    "permissions.ask",
    "permissions.additionalDirectories",
    "enabledMcpjsonServers",
    "disabledMcpjsonServers",
    "allowedMcpServers",
    "deniedMcpServers",
];

/// Dotted prefixes under which keys are free-form but have a well-understood
/// "highest precedence wins" behaviour.
const INFERRED_PREFIXES: &[&str] = &[
    "env.",
    "enabledPlugins.",
    "extraKnownMarketplaces.",
    "statusLine.",
    "sandbox.",
    "attribution.",
    "permissions.",
];

fn is_list_key(key: &str) -> bool {
    KNOWN_LIST_KEYS.contains(&key) || (key.starts_with("hooks.") && key.matches('.').count() == 1)
}

fn classify(key: &str, value_is_array: bool) -> Confidence {
    if KNOWN_SCALAR_KEYS.contains(&key) || is_list_key(key) {
        Confidence::Computed
    } else if INFERRED_PREFIXES.iter().any(|p| key.starts_with(p)) && !value_is_array {
        Confidence::Inferred
    } else {
        Confidence::Unknown
    }
}

/// Flatten an object into dotted leaf keys. Arrays and scalars are leaves; the
/// `statusLine` object is a known scalar-like unit and stays whole.
pub fn flatten(value: &Value) -> BTreeMap<String, Value> {
    fn rec(prefix: &str, v: &Value, out: &mut BTreeMap<String, Value>) {
        match v {
            Value::Object(m) if !m.is_empty() && prefix != "statusLine" => {
                for (k, child) in m {
                    let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                    rec(&key, child, out);
                }
            }
            _ => {
                if !prefix.is_empty() {
                    out.insert(prefix.to_string(), v.clone());
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    rec("", value, &mut out);
    out
}

/// Layers sorted highest-precedence first, restricted to scopes that take part in settings.
fn ordered(layers: &[SettingsLayer]) -> Vec<&SettingsLayer> {
    let mut v: Vec<&SettingsLayer> = layers.iter().filter(|l| l.scope.settings_rank().is_some()).collect();
    // stable: keeps declaration order within a scope (later managed drop-ins override earlier ones)
    v.sort_by_key(|l| l.scope.settings_rank());
    v
}

pub fn resolve(layers: &[SettingsLayer]) -> Vec<ResolvedSetting> {
    let layers = ordered(layers);
    let flat: Vec<BTreeMap<String, Value>> = layers.iter().map(|l| flatten(&l.value)).collect();
    let mut keys: Vec<&String> = flat.iter().flat_map(|m| m.keys()).collect();
    keys.sort();
    keys.dedup();

    let mut out = Vec::new();
    for key in keys {
        let sources: Vec<SettingSource> = layers
            .iter()
            .zip(&flat)
            .filter_map(|(l, m)| {
                m.get(key).map(|v| SettingSource {
                    scope: l.scope,
                    path: l.path.clone(),
                    value: v.clone(),
                })
            })
            .collect();
        let top = &sources[0];
        let any_array = sources.iter().any(|s| s.value.is_array());
        let confidence = classify(key, any_array);

        let (value, merged, explanation) = if is_list_key(key) && sources.iter().all(|s| s.value.is_array()) {
            let mut seen = HashSet::new();
            let mut items = Vec::new();
            for s in &sources {
                for it in s.value.as_array().into_iter().flatten() {
                    if seen.insert(serde_json::to_string(it).unwrap_or_default()) {
                        items.push(it.clone());
                    }
                }
            }
            let scopes: Vec<&str> = sources.iter().map(|s| s.scope.as_str()).collect();
            (
                Value::Array(items),
                sources.len() > 1,
                format!("List merged (union) across scopes: {}", scopes.join(", ")),
            )
        } else {
            let explanation = match confidence {
                Confidence::Unknown => format!(
                    "Unrecognized key; showing the highest-precedence value ({}). Merge behaviour is not verified.",
                    top.scope.as_str()
                ),
                _ if sources.len() == 1 => format!("Defined only in {}", top.scope.as_str()),
                _ => {
                    let overridden: Vec<&str> = sources[1..].iter().map(|s| s.scope.as_str()).collect();
                    format!(
                        "{} takes precedence over {}",
                        top.scope.as_str(),
                        overridden.join(", ")
                    )
                }
            };
            (top.value.clone(), false, explanation)
        };

        out.push(ResolvedSetting {
            key: key.clone(),
            value,
            scope: top.scope,
            source_path: Some(top.path.clone()),
            confidence,
            explanation,
            merged,
            sources,
        });
    }
    out
}

/// Layers relevant for a project: global (managed + user) plus that project's own.
pub fn layers_for(global: &GlobalBundle, project: Option<&ProjectBundle>) -> Vec<SettingsLayer> {
    let mut v: Vec<SettingsLayer> = global.items.settings_layers.clone();
    if let Some(p) = project {
        v.extend(p.items.settings_layers.iter().cloned());
    }
    v
}

pub fn settings_diff(global: &GlobalBundle, project: &ProjectBundle) -> SettingsDiff {
    let find = |layers: &[SettingsLayer], scope: Scope| layers.iter().find(|l| l.scope == scope).cloned();
    let user = find(&global.items.settings_layers, Scope::User);
    let proj = find(&project.items.settings_layers, Scope::Project);
    let local = find(&project.items.settings_layers, Scope::ProjectLocal);

    let fu = user.as_ref().map(|l| flatten(&l.value)).unwrap_or_default();
    let fp = proj.as_ref().map(|l| flatten(&l.value)).unwrap_or_default();
    let fl = local.as_ref().map(|l| flatten(&l.value)).unwrap_or_default();
    let mut keys: Vec<&String> = fu.keys().chain(fp.keys()).chain(fl.keys()).collect();
    keys.sort();
    keys.dedup();
    let rows = keys
        .into_iter()
        .map(|k| {
            let (u, p, l) = (fu.get(k).cloned(), fp.get(k).cloned(), fl.get(k).cloned());
            let defined: Vec<&Value> = [&u, &p, &l].into_iter().flatten().collect();
            let differs = defined.len() > 1 && defined.iter().any(|v| *v != defined[0]);
            SettingsDiffRow { key: k.clone(), user: u, project: p, local: l, differs }
        })
        .collect();

    SettingsDiff {
        project_path: project.project.path.clone(),
        user_path: user.as_ref().map(|l| l.path.clone()),
        project_settings_path: proj.as_ref().map(|l| l.path.clone()),
        local_path: local.as_ref().map(|l| l.path.clone()),
        user: user.map(|l| l.value),
        project: proj.map(|l| l.value),
        local: local.map(|l| l.value),
        rows,
    }
}

// ---------------------------------------------------------------- conflicts

fn conflict_id(kind: &str, key: &str, project: Option<&str>) -> String {
    stable_id(&["conflict", kind, key, project.unwrap_or("")])
}

fn duplicate_names(
    skills: &[&SkillEntry],
    project: Option<&str>,
    require_project_member: bool,
    out: &mut Vec<Conflict>,
) {
    let mut groups: BTreeMap<String, Vec<&SkillEntry>> = BTreeMap::new();
    for s in skills {
        groups.entry(s.name.to_lowercase()).or_default().push(s);
    }
    for (name, entries) in groups {
        if entries.len() < 2 {
            continue;
        }
        if require_project_member && !entries.iter().any(|e| e.project_path.is_some()) {
            continue;
        }
        out.push(Conflict {
            id: conflict_id("dup", &name, project),
            kind: ConflictKind::DuplicateName,
            severity: ConflictSeverity::Warning,
            key: name.clone(),
            message: format!(
                "`{}` is defined {} times across scopes ({})",
                name,
                entries.len(),
                entries.iter().map(|e| e.scope.as_str()).collect::<Vec<_>>().join(", ")
            ),
            project_path: project.map(String::from),
            entries: entries
                .iter()
                .map(|e| ConflictEntry {
                    scope: e.scope,
                    path: e.path.clone(),
                    kind: Some(e.kind),
                    value: None,
                })
                .collect(),
        });
    }
}

fn mcp_mismatches(
    servers: &[&McpServerEntry],
    project: Option<&str>,
    require_project_member: bool,
    out: &mut Vec<Conflict>,
) {
    let mut groups: BTreeMap<&str, Vec<&McpServerEntry>> = BTreeMap::new();
    for s in servers {
        groups.entry(s.name.as_str()).or_default().push(s);
    }
    for (name, entries) in groups {
        if entries.len() < 2 {
            continue;
        }
        if require_project_member && !entries.iter().any(|e| e.project_path.is_some()) {
            continue;
        }
        let first = &entries[0].config_hash;
        if entries.iter().all(|e| &e.config_hash == first) {
            continue;
        }
        out.push(Conflict {
            id: conflict_id("mcp", name, project),
            kind: ConflictKind::McpMismatch,
            severity: ConflictSeverity::Warning,
            key: name.to_string(),
            message: format!("MCP server `{name}` is configured differently in different scopes"),
            project_path: project.map(String::from),
            entries: entries
                .iter()
                .map(|e| ConflictEntry {
                    scope: e.scope,
                    path: e.source_path.clone(),
                    kind: Some(ArtifactKind::McpServer),
                    value: Some(e.config.clone()),
                })
                .collect(),
        });
    }
}

fn setting_overrides(
    layers: &[SettingsLayer],
    project: Option<&str>,
    require_project_layer: bool,
    out: &mut Vec<Conflict>,
) {
    for r in resolve(layers) {
        if r.merged || r.sources.len() < 2 || is_list_key(&r.key) {
            continue;
        }
        let top = &r.sources[0].value;
        if r.sources.iter().all(|s| &s.value == top) {
            continue;
        }
        if require_project_layer
            && !r.sources.iter().any(|s| matches!(s.scope, Scope::Project | Scope::ProjectLocal))
        {
            continue;
        }
        out.push(Conflict {
            id: conflict_id("set", &r.key, project),
            kind: ConflictKind::SettingOverride,
            severity: ConflictSeverity::Info,
            key: r.key.clone(),
            message: format!("`{}` is overridden: {}", r.key, r.explanation),
            project_path: project.map(String::from),
            entries: r
                .sources
                .iter()
                .map(|s| ConflictEntry {
                    scope: s.scope,
                    path: s.path.clone(),
                    kind: Some(ArtifactKind::Settings),
                    value: Some(s.value.clone()),
                })
                .collect(),
        });
    }
}

/// Detect duplicate skill/command names, MCP mismatches and scalar overrides.
/// Global-only findings are reported once (project_path = None); per-project
/// findings are only those involving at least one project-level item.
pub fn detect_conflicts(global: &GlobalBundle, projects: &[ProjectBundle]) -> Vec<Conflict> {
    let mut out = Vec::new();
    let g_skills: Vec<&SkillEntry> = global.items.skills.iter().collect();
    let g_mcp: Vec<&McpServerEntry> = global.items.mcp_servers.iter().collect();

    duplicate_names(&g_skills, None, false, &mut out);
    mcp_mismatches(&g_mcp, None, false, &mut out);
    setting_overrides(&global.items.settings_layers, None, false, &mut out);

    for p in projects {
        let pp = Some(p.project.path.as_str());
        let skills: Vec<&SkillEntry> = g_skills.iter().copied().chain(p.items.skills.iter()).collect();
        duplicate_names(&skills, pp, true, &mut out);
        let mcp: Vec<&McpServerEntry> = g_mcp.iter().copied().chain(p.items.mcp_servers.iter()).collect();
        mcp_mismatches(&mcp, pp, true, &mut out);
        setting_overrides(&layers_for(global, Some(p)), pp, true, &mut out);
    }
    out
}

/// Convenience for tests / callers that build layers by hand.
pub fn layer(scope: Scope, path: &str, value: Value) -> SettingsLayer {
    SettingsLayer { scope, path: path.to_string(), project_path: None, value }
}

#[cfg(test)]
mod tests;
