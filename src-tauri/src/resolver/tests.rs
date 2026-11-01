use super::*;
use serde_json::json;

fn get<'a>(r: &'a [ResolvedSetting], key: &str) -> &'a ResolvedSetting {
    r.iter().find(|s| s.key == key).unwrap_or_else(|| panic!("missing {key}"))
}

#[test]
fn scalar_precedence_managed_local_project_user() {
    let layers = vec![
        layer(Scope::User, "/u", json!({"model": "user-m", "theme": "dark", "language": "en"})),
        layer(Scope::Project, "/p", json!({"model": "proj-m", "language": "fr"})),
        layer(Scope::ProjectLocal, "/l", json!({"model": "local-m"})),
        layer(Scope::Managed, "/m", json!({"language": "de"})),
    ];
    let r = resolve(&layers);
    let model = get(&r, "model");
    assert_eq!(model.value, "local-m");
    assert_eq!(model.scope, Scope::ProjectLocal);
    assert_eq!(model.confidence, Confidence::Computed);
    assert_eq!(model.sources.len(), 3);
    assert!(model.explanation.contains("projectLocal"));

    let lang = get(&r, "language");
    assert_eq!(lang.value, "de");
    assert_eq!(lang.scope, Scope::Managed);

    let theme = get(&r, "theme");
    assert_eq!(theme.value, "dark");
    assert_eq!(theme.scope, Scope::User);
}

#[test]
fn list_merge_permissions() {
    let layers = vec![
        layer(Scope::User, "/u", json!({"permissions": {"allow": ["Read", "Bash(ls)"], "deny": ["Bash(rm:*)"]}})),
        layer(Scope::Project, "/p", json!({"permissions": {"allow": ["Bash(ls)", "Edit"], "defaultMode": "plan"}})),
    ];
    let r = resolve(&layers);
    let allow = get(&r, "permissions.allow");
    assert!(allow.merged);
    assert_eq!(allow.value, json!(["Bash(ls)", "Edit", "Read"]));
    assert_eq!(allow.confidence, Confidence::Computed);
    let deny = get(&r, "permissions.deny");
    assert!(!deny.merged);
    assert_eq!(deny.value, json!(["Bash(rm:*)"]));
    assert_eq!(get(&r, "permissions.defaultMode").value, "plan");
}

#[test]
fn unknown_and_inferred_confidence() {
    let layers = vec![
        layer(Scope::User, "/u", json!({"futureKey": 1, "env": {"FOO": "a"}, "weirdList": [1]})),
        layer(Scope::Project, "/p", json!({"futureKey": 2, "env": {"FOO": "b", "BAR": "c"}})),
    ];
    let r = resolve(&layers);
    let f = get(&r, "futureKey");
    assert_eq!(f.confidence, Confidence::Unknown);
    assert_eq!(f.value, 2);
    assert_eq!(get(&r, "env.FOO").confidence, Confidence::Inferred);
    assert_eq!(get(&r, "env.FOO").value, "b");
    assert_eq!(get(&r, "env.BAR").value, "c");
    assert_eq!(get(&r, "weirdList").confidence, Confidence::Unknown);
}

#[test]
fn hooks_arrays_merge() {
    let layers = vec![
        layer(Scope::User, "/u", json!({"hooks": {"Stop": [{"hooks": [{"command": "a"}]}]}})),
        layer(Scope::Project, "/p", json!({"hooks": {"Stop": [{"hooks": [{"command": "b"}]}]}})),
    ];
    let r = resolve(&layers);
    let h = get(&r, "hooks.Stop");
    assert!(h.merged);
    assert_eq!(h.value.as_array().unwrap().len(), 2);
}

#[test]
fn non_settings_scopes_ignored() {
    let layers = vec![layer(Scope::Plugin, "/x", json!({"model": "x"}))];
    assert!(resolve(&layers).is_empty());
}

fn skill(name: &str, scope: Scope, project: Option<&str>) -> SkillEntry {
    SkillEntry {
        name: name.into(),
        kind: ArtifactKind::Skill,
        description: None,
        scope,
        path: format!("/{}/{}", scope.as_str(), name),
        project_path: project.map(String::from),
        status: DiscoveryStatus::Present,
        frontmatter: Value::Null,
        files: vec![],
    }
}

fn mcp(name: &str, scope: Scope, project: Option<&str>, cfg: Value) -> McpServerEntry {
    McpServerEntry {
        name: name.into(),
        scope,
        source_path: "/s".into(),
        project_path: project.map(String::from),
        origin: "t".into(),
        server_type: "stdio".into(),
        command: None,
        args: vec![],
        url: None,
        config_hash: crate::scanner::collect::config_hash(&cfg),
        config: cfg,
    }
}

fn empty_global() -> GlobalBundle {
    GlobalBundle {
        config_dir: "/c".into(),
        managed_dir: "/m".into(),
        claude_json_path: "/j".into(),
        cursor_config_dir: Some("/.cursor".into()),
        codex_config_dir: Some("/.codex".into()),
        credentials_present: false,
        items: Collection::default(),
    }
}

fn empty_project(path: &str) -> ProjectBundle {
    ProjectBundle {
        project: crate::scanner::info_for_dir(std::path::Path::new(path)),
        items: Collection::default(),
    }
}

#[test]
fn conflicts_detected() {
    let mut g = empty_global();
    g.items.skills.push(skill("deploy", Scope::User, None));
    g.items.skills.push(skill("lonely", Scope::User, None));
    g.items.mcp_servers.push(mcp("gh", Scope::User, None, json!({"command": "a"})));
    g.items.mcp_servers.push(mcp("same", Scope::User, None, json!({"command": "x"})));
    g.items.settings_layers.push(layer(Scope::User, "/u", json!({"model": "a"})));

    let mut p = empty_project("/tmp/nonexistent-proj");
    p.items.skills.push(skill("Deploy", Scope::Project, Some("/tmp/nonexistent-proj")));
    p.items.mcp_servers.push(mcp("gh", Scope::Project, Some("/tmp/nonexistent-proj"), json!({"command": "b"})));
    p.items.mcp_servers.push(mcp("same", Scope::Project, Some("/tmp/nonexistent-proj"), json!({"command": "x"})));
    p.items.settings_layers.push(layer(Scope::Project, "/p", json!({"model": "b"})));

    let c = detect_conflicts(&g, &[p]);
    assert!(c.iter().any(|c| c.kind == ConflictKind::DuplicateName && c.key == "deploy"));
    assert!(!c.iter().any(|c| c.key == "lonely"));
    assert!(c.iter().any(|c| c.kind == ConflictKind::McpMismatch && c.key == "gh"));
    assert!(!c.iter().any(|c| c.kind == ConflictKind::McpMismatch && c.key == "same"));
    let ov = c.iter().find(|c| c.kind == ConflictKind::SettingOverride).unwrap();
    assert_eq!(ov.key, "model");
    assert_eq!(ov.entries.len(), 2);
    // ids are unique
    let ids: HashSet<_> = c.iter().map(|c| &c.id).collect();
    assert_eq!(ids.len(), c.len());
}

#[test]
fn settings_diff_rows() {
    let mut g = empty_global();
    g.items.settings_layers.push(layer(Scope::User, "/u", json!({"model": "a", "x": 1})));
    let mut p = empty_project("/tmp/nonexistent-proj");
    p.items.settings_layers.push(layer(Scope::Project, "/p", json!({"model": "b", "x": 1})));
    p.items.settings_layers.push(layer(Scope::ProjectLocal, "/l", json!({"y": true})));
    let d = settings_diff(&g, &p);
    let model = d.rows.iter().find(|r| r.key == "model").unwrap();
    assert!(model.differs && model.user.is_some() && model.local.is_none());
    assert!(!d.rows.iter().find(|r| r.key == "x").unwrap().differs);
    assert!(!d.rows.iter().find(|r| r.key == "y").unwrap().differs);
    assert_eq!(d.local_path.as_deref(), Some("/l"));
}
