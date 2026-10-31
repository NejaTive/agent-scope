//! Scanner tests — always against tempdirs, never the real ~/.claude.

use super::*;
use crate::parser::MASK;
use crate::tooling::ToolingFilter;
use std::fs;
use tempfile::tempdir;

fn all_filter() -> ToolingFilter {
    ToolingFilter::all()
}

fn w(p: &Path, s: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, s).unwrap();
}

fn find<'a>(c: &'a Collection, kind: ArtifactKind, name: &str) -> &'a ConfigArtifact {
    c.artifacts
        .iter()
        .find(|a| a.kind == kind && a.name == name)
        .unwrap_or_else(|| panic!("no {kind:?} {name}"))
}

#[test]
fn empty_home_is_ok() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    let g = scan_global_with(&ScanContext::load(&paths), &all_filter());
    assert!(g.items.errors.is_empty());
    assert!(!g.credentials_present);
    // Absent optional files are not listed as missing placeholders.
    assert!(!g.items.artifacts.iter().any(|a| a.status == DiscoveryStatus::Missing));
    assert!(g
        .items
        .artifacts
        .iter()
        .all(|a| a.kind != ArtifactKind::Settings && a.kind != ArtifactKind::ClaudeMd));
    assert!(g.items.skills.is_empty());
}

#[test]
fn corrupted_json_reported() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    w(&paths.config_dir.join("settings.json"), "{ not json");
    w(&paths.claude_json, "[[[");
    let g = scan_global_with(&ScanContext::load(&paths), &all_filter());
    assert_eq!(find(&g.items, ArtifactKind::Settings, "settings.json").status, DiscoveryStatus::InvalidJson);
    assert_eq!(find(&g.items, ArtifactKind::Other, ".claude.json").status, DiscoveryStatus::InvalidJson);
    assert_eq!(g.items.errors.len(), 2);
    assert!(g.items.settings_layers.is_empty());
}

#[test]
fn global_collects_everything() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    let cfg = &paths.config_dir;
    w(&cfg.join("settings.json"), r#"{"model":"opus","env":{"MY_TOKEN":"abc","A":"b"},
        "hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"./c.sh"}]}]}}"#);
    w(&cfg.join("CLAUDE.md"), "# Mine\nHello there\n");
    w(&cfg.join("rules/style.md"), "---\npaths:\n  - \"src/**/*.ts\"\n---\nUse tabs\n");
    w(&cfg.join("rules/sub/always.md"), "Always\n");
    w(&cfg.join("skills/pdf/SKILL.md"), "---\nname: pdf\ndescription: Make pdfs\n---\nbody");
    w(&cfg.join("skills/pdf/scripts/run.py"), "print(1)");
    w(&cfg.join("skills/nodoc/README.md"), "not a skill");
    w(&cfg.join("commands/deploy.md"), "---\ndescription: Deploy it\n---\nrun");
    w(&cfg.join("agents/rev.md"), "---\nname: rev\ndescription: Reviews code\ntools: Read, Grep\n---\nprompt");
    w(&cfg.join("agents/bad.md"), "no frontmatter");
    w(&cfg.join("output-styles/terse.md"), "---\nname: Terse\n---\nbe short");
    w(&cfg.join("workflows/ship.js"), "module.exports={}");
    w(&cfg.join(".credentials.json"), r#"{"claudeAiOauth":{"accessToken":"TOP-SECRET"}}"#);
    w(&cfg.join("projects/-tmp-x/memory/MEMORY.md"), "- note");
    w(&cfg.join("projects/-tmp-x/transcript.jsonl"), "{}");
    w(&cfg.join("plugins/installed_plugins.json"),
        r#"{"version":2,"plugins":{"fmt@market":[{"scope":"user","version":"1.2.3","installPath":"/x"}]}}"#);
    w(&cfg.join("plugins/marketplaces/market/README.md"), "x");
    w(&paths.claude_json, r#"{"userID":"zzz","mcpServers":{"gh":{"command":"npx","args":["gh"],"env":{"GITHUB_TOKEN":"ghp_x"}}},
        "projects":{"/p":{"mcpServers":{}}}}"#);
    w(&paths.managed_dir.join("managed-settings.json"), r#"{"model":"sonnet"}"#);

    let g = scan_global_with(&ScanContext::load(&paths), &all_filter());
    let c = &g.items;
    assert!(g.credentials_present);
    assert!(c.errors.iter().any(|e| e.path.ends_with("bad.md")) );

    // settings + masking
    let layer = c.settings_layers.iter().find(|l| l.scope == Scope::User).unwrap();
    assert_eq!(layer.value["env"]["MY_TOKEN"], MASK);
    assert_eq!(layer.value["env"]["A"], "b");
    assert!(c.settings_layers.iter().any(|l| l.scope == Scope::Managed));

    // hooks parsed, not executed
    assert_eq!(c.hooks.len(), 1);
    assert_eq!(c.hooks[0].event, "PreToolUse");
    assert!(c.artifacts.iter().any(|a| a.kind == ArtifactKind::Hook));

    // rules
    assert_eq!(c.rules.len(), 2);
    let styled = c.rules.iter().find(|r| r.name == "style.md").unwrap();
    assert_eq!(styled.paths, vec!["src/**/*.ts"]);
    assert!(styled.conditional);

    // skills / commands
    assert_eq!(c.skills.iter().filter(|s| s.kind == ArtifactKind::Skill).count(), 1);
    let pdf = find(c, ArtifactKind::Skill, "pdf");
    assert!(pdf.children.iter().any(|f| f == "scripts/run.py"));
    assert_eq!(pdf.description.as_deref(), Some("Make pdfs"));
    assert_eq!(find(c, ArtifactKind::Command, "deploy").description.as_deref(), Some("Deploy it"));

    // agents validity
    assert!(c.agents.iter().find(|a| a.name == "rev").unwrap().valid);
    assert!(!c.agents.iter().find(|a| a.name == "bad").unwrap().valid);

    find(c, ArtifactKind::OutputStyle, "terse");
    find(c, ArtifactKind::Workflow, "ship");

    // credentials: presence only
    let cred = find(c, ArtifactKind::Other, ".credentials.json");
    assert_eq!(cred.metadata["contentHidden"], true);
    let dump = serde_json::to_string(&g).unwrap();
    assert!(!dump.contains("TOP-SECRET"));
    assert!(!dump.contains("ghp_x"));
    assert!(!dump.contains("zzz"));

    // memory lists file only; transcripts ignored
    assert_eq!(c.artifacts.iter().filter(|a| a.kind == ArtifactKind::Memory).count(), 1);

    // plugins
    assert!(c.artifacts.iter().any(|a| a.kind == ArtifactKind::Plugin && a.name == "fmt@market"));
    assert!(c.artifacts.iter().any(|a| a.kind == ArtifactKind::Plugin && a.name == "market"));

    // user MCP from ~/.claude.json, masked
    assert_eq!(c.mcp_servers.len(), 1);
    assert_eq!(c.mcp_servers[0].scope, Scope::User);
    assert_eq!(c.mcp_servers[0].config["env"]["GITHUB_TOKEN"], MASK);
}

#[test]
fn project_scan_collects_scopes() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    let proj = d.path().join("work/app");
    w(&proj.join("CLAUDE.md"), "# App\nproject notes");
    w(&proj.join("CLAUDE.local.md"), "local");
    w(&proj.join("AGENTS.md"), "agents");
    w(&proj.join("sub/pkg/CLAUDE.md"), "nested");
    w(&proj.join("node_modules/x/CLAUDE.md"), "ignored");
    w(&proj.join(".claude/settings.json"), r#"{"model":"p"}"#);
    w(&proj.join(".claude/settings.local.json"), r#"{"model":"l"}"#);
    w(&proj.join(".claude/skills/s/SKILL.md"), "# s\nhi");
    w(&proj.join(".mcp.json"), r#"{"mcpServers":{"db":{"type":"http","url":"https://x","headers":{"Authorization":"Bearer q"}}}}"#);
    let canon = discovery::canonical(&proj);
    w(&paths.claude_json, &format!(
        r#"{{"projects":{{"{}":{{"mcpServers":{{"loc":{{"command":"run","env":{{"SECRET":"s"}}}}}}}}}}}}"#,
        canon.display()
    ));

    let b = scan_project_with(&ScanContext::load(&paths), &proj, &all_filter());
    let c = &b.items;
    assert_eq!(b.project.path, canon.to_string_lossy());
    assert!(b.project.has_claude_dir && b.project.has_claude_md && b.project.has_mcp_json);

    assert_eq!(find(c, ArtifactKind::Settings, "settings.json").scope, Scope::Project);
    assert_eq!(find(c, ArtifactKind::Settings, "settings.local.json").scope, Scope::ProjectLocal);
    assert_eq!(find(c, ArtifactKind::ClaudeMd, "CLAUDE.local.md").scope, Scope::ProjectLocal);
    find(c, ArtifactKind::ClaudeMd, "AGENTS.md");
    let nested: Vec<_> = c.artifacts.iter().filter(|a| a.scope == Scope::Nested).collect();
    assert_eq!(nested.len(), 1, "{:?}", nested.iter().map(|a| &a.path).collect::<Vec<_>>());
    assert!(nested[0].path.contains("sub/pkg"));

    let names: Vec<_> = c.mcp_servers.iter().map(|m| (m.name.as_str(), m.scope)).collect();
    assert!(names.contains(&("db", Scope::Project)));
    assert!(names.contains(&("loc", Scope::ProjectLocal)));
    let db = c.mcp_servers.iter().find(|m| m.name == "db").unwrap();
    assert_eq!(db.config["headers"]["Authorization"], MASK);
    assert_eq!(db.server_type, "http");
    let loc = c.mcp_servers.iter().find(|m| m.name == "loc").unwrap();
    assert_eq!(loc.config["env"]["SECRET"], MASK);

    assert_eq!(c.skills.len(), 1);
    assert_eq!(c.skills[0].scope, Scope::Project);
    assert!(b.project.artifact_count > 5);
}

#[test]
fn scan_all_dedupes_and_resolves() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    let proj = d.path().join("code/a");
    w(&proj.join(".claude/settings.json"), r#"{"model":"p"}"#);
    w(&proj.join(".claude/skills/deploy/SKILL.md"), "x");
    w(&paths.config_dir.join("settings.json"), r#"{"model":"u"}"#);
    w(&paths.config_dir.join("skills/deploy/SKILL.md"), "x");

    let input = ScanInput {
        scan_roots: vec![d.path().join("code"), d.path().join("code")],
        pinned: vec![proj.clone(), d.path().join("gone")],
        max_depth: 3,
        filter: all_filter(),
    };
    let snap = scan_all(&paths, &input);
    assert_eq!(snap.projects.len(), 1);
    assert!(snap.projects[0].project.pinned && snap.projects[0].project.discovered);
    // home dir (has .claude) must not be treated as a project
    assert!(snap.errors.iter().any(|e| e.status == DiscoveryStatus::Missing && e.path.ends_with("gone")));
    assert!(snap.conflicts.iter().any(|c| c.kind == ConflictKind::DuplicateName));
    assert!(snap.conflicts.iter().any(|c| c.kind == ConflictKind::SettingOverride));
    assert_eq!(snap.stats.project_count, 1);
    assert_eq!(snap.stats.skills, 2);
    assert_eq!(snap.stats.conflict_count, snap.conflicts.len());
    let eff = crate::resolver::resolve(&crate::resolver::layers_for(&snap.global, snap.projects.first()));
    assert_eq!(eff.iter().find(|r| r.key == "model").unwrap().value, "p");
}

#[test]
fn claude_json_view_is_redacted() {
    let v = serde_json::json!({
        "userID": "secret-id", "oauthAccount": {"email": "a@b"},
        "mcpServers": {"s": {"env": {"API_KEY": "k"}}},
        "projects": {"/p": {"allowedTools": [], "mcpServers": {"tsrv": {"env": {"X_TOKEN": "tv"}}}}}
    });
    let out = serde_json::to_string(&claude_json_view(&v)).unwrap();
    assert!(!out.contains("secret-id") && !out.contains("a@b"));
    assert!(!out.contains("\"k\"") && !out.contains("\"tv\""));
    assert!(out.contains(MASK));
}

#[test]
fn cursor_global_collects_rules_skills_hooks_mcp() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    let cur = &paths.cursor_dir;
    w(
        &cur.join("rules/style.mdc"),
        "---\ndescription: Style\nalwaysApply: false\nglobs:\n  - \"**/*.ts\"\n---\nUse const\n",
    );
    w(&cur.join("rules/always.mdc"), "---\nalwaysApply: true\n---\nAlways\n");
    w(&cur.join("skills/pdf/SKILL.md"), "---\nname: pdf\ndescription: Make pdfs\n---\nbody");
    w(
        &cur.join("skills/nested/group/land/SKILL.md"),
        "---\nname: land\ndescription: Land it\n---\n",
    );
    w(&cur.join("commands/ship.md"), "---\ndescription: Ship it\n---\nrun");
    w(
        &cur.join("agents/rev.md"),
        "---\nname: rev\ndescription: Reviews\ntools: Read\n---\nprompt",
    );
    w(
        &cur.join("hooks.json"),
        r#"{"version":1,"hooks":{"afterFileEdit":[{"command":".cursor/hooks/format.sh"}]}}"#,
    );
    w(
        &cur.join("mcp.json"),
        r#"{"mcpServers":{"gh":{"command":"npx","args":["gh"],"env":{"GITHUB_TOKEN":"ghp_SECRET"}}}}"#,
    );
    w(
        &paths.home.join(".agents/skills/agent-skill/SKILL.md"),
        "---\nname: agent-skill\ndescription: From agents\n---\n",
    );
    // Claude skill must stay Claude product and must not be duplicated as Cursor.
    w(
        &paths.config_dir.join("skills/shared/SKILL.md"),
        "---\nname: shared\ndescription: Claude owned\n---\n",
    );

    let g = scan_global_with(&ScanContext::load(&paths), &all_filter());
    let c = &g.items;

    let rules: Vec<_> = c.artifacts.iter().filter(|a| a.kind == ArtifactKind::Rule && a.product == Tooling::Cursor).collect();
    assert_eq!(rules.len(), 2);
    let styled = c.rules.iter().find(|r| r.name == "style.mdc").unwrap();
    assert_eq!(styled.paths, vec!["**/*.ts"]);
    assert!(styled.conditional);

    let cursor_skills: Vec<_> = c
        .artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Skill && a.product == Tooling::Cursor)
        .collect();
    assert!(cursor_skills.iter().any(|a| a.name == "pdf"));
    assert!(cursor_skills.iter().any(|a| a.name == "land"));
    assert!(cursor_skills.iter().any(|a| a.name == "agent-skill"));

    let shared = find(c, ArtifactKind::Skill, "shared");
    assert_eq!(shared.product, Tooling::Claude);
    assert_eq!(
        c.artifacts.iter().filter(|a| a.path == shared.path).count(),
        1,
        "claude skill path must not be re-collected as Cursor"
    );

    find(c, ArtifactKind::Command, "ship");
    assert!(c.artifacts.iter().any(|a| a.kind == ArtifactKind::Command && a.product == Tooling::Cursor));
    assert!(c.agents.iter().any(|a| a.name == "rev"));

    assert!(c.hooks.iter().any(|h| h.event == "afterFileEdit" && h.command.as_deref() == Some(".cursor/hooks/format.sh")));
    assert!(c.artifacts.iter().any(|a| a.kind == ArtifactKind::Hook && a.product == Tooling::Cursor));

    let mcp = c.mcp_servers.iter().find(|m| m.name == "gh").unwrap();
    assert_eq!(mcp.config["env"]["GITHUB_TOKEN"], MASK);
    let dump = serde_json::to_string(&g).unwrap();
    assert!(!dump.contains("ghp_SECRET"));
}

#[test]
fn cursor_project_dedupes_agents_md_and_claude_skills() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    let proj = d.path().join("work/app");
    w(&proj.join("AGENTS.md"), "# Agents\nshared instructions");
    w(&proj.join(".claude/skills/deploy/SKILL.md"), "---\nname: deploy\ndescription: Claude deploy\n---\n");
    w(&proj.join(".cursor/rules/api.mdc"), "---\ndescription: API\nalwaysApply: true\n---\nrules\n");
    w(&proj.join(".cursor/skills/ship/SKILL.md"), "---\nname: ship\ndescription: Cursor ship\n---\n");
    w(&proj.join(".agents/skills/extra/SKILL.md"), "---\nname: extra\ndescription: Extra\n---\n");
    w(
        &proj.join(".cursor/mcp.json"),
        r#"{"mcpServers":{"db":{"type":"http","url":"https://x","headers":{"Authorization":"Bearer tok"}}}}"#,
    );
    w(
        &proj.join(".cursor/hooks.json"),
        r#"{"version":1,"hooks":{"sessionEnd":[{"command":".cursor/hooks/audit.sh"}]}}"#,
    );

    let b = scan_project_with(&ScanContext::load(&paths), &proj, &all_filter());
    let c = &b.items;

    let agents: Vec<_> = c.artifacts.iter().filter(|a| a.name == "AGENTS.md").collect();
    assert_eq!(agents.len(), 1, "AGENTS.md collected once");
    assert_eq!(agents[0].product, Tooling::Claude, "Claude collector owns AGENTS.md when present");

    let deploy = find(c, ArtifactKind::Skill, "deploy");
    assert_eq!(deploy.product, Tooling::Claude);
    assert_eq!(c.artifacts.iter().filter(|a| a.path == deploy.path).count(), 1);

    let ship = find(c, ArtifactKind::Skill, "ship");
    assert_eq!(ship.product, Tooling::Cursor);
    let extra = find(c, ArtifactKind::Skill, "extra");
    assert_eq!(extra.product, Tooling::Cursor);
    assert!(c.artifacts.iter().any(|a| a.kind == ArtifactKind::Rule && a.product == Tooling::Cursor && a.name == "api.mdc"));

    let db = c.mcp_servers.iter().find(|m| m.name == "db").unwrap();
    assert_eq!(db.config["headers"]["Authorization"], MASK);
    assert!(c.hooks.iter().any(|h| h.event == "sessionEnd"));

    // Unique paths across all artifacts (hook entries may share hooks.json path with Other + Hook kinds —
    // allow same path for different kinds/ids, but skill/rule file paths must be unique).
    let mut skill_paths: Vec<_> = c
        .artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Skill)
        .map(|a| a.path.clone())
        .collect();
    skill_paths.sort();
    let before = skill_paths.len();
    skill_paths.dedup();
    assert_eq!(skill_paths.len(), before);
}

#[test]
fn cursor_only_project_picks_up_agents_md() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    let proj = d.path().join("cursor-app");
    // No CLAUDE.md / .claude — but Claude collector still tries AGENTS.md first.
    // With Claude collector owning AGENTS.md, product stays Claude when the file exists.
    // This fixture verifies Cursor still scans .cursor when Claude markers are absent.
    w(&proj.join("AGENTS.md"), "cursor agents");
    w(&proj.join(".cursor/rules/r.mdc"), "---\nalwaysApply: true\n---\nx\n");

    let b = scan_project_with(&ScanContext::load(&paths), &proj, &all_filter());
    assert!(b.items.artifacts.iter().any(|a| a.product == Tooling::Cursor && a.kind == ArtifactKind::Rule));
    let agents = b.items.artifacts.iter().filter(|a| a.name == "AGENTS.md").count();
    assert_eq!(agents, 1);
}

#[test]
fn filter_skips_disabled_product_collectors() {
    let d = tempdir().unwrap();
    let paths = Paths::for_root(d.path());
    w(&paths.config_dir.join("settings.json"), r#"{"model":"opus"}"#);
    w(&paths.config_dir.join("skills/pdf/SKILL.md"), "---\nname: pdf\ndescription: Claude\n---\n");
    w(&paths.cursor_dir.join("rules/style.mdc"), "---\nalwaysApply: true\n---\nx\n");
    w(&paths.cursor_dir.join("skills/ship/SKILL.md"), "---\nname: ship\ndescription: Cursor\n---\n");
    w(&paths.claude_json, r#"{"mcpServers":{"gh":{"command":"npx"}}}"#);

    let proj = d.path().join("app");
    w(&proj.join(".claude/settings.json"), r#"{"model":"p"}"#);
    w(&proj.join(".claude/skills/c/SKILL.md"), "---\nname: c\ndescription: C\n---\n");
    w(&proj.join(".cursor/rules/r.mdc"), "---\nalwaysApply: true\n---\nr\n");
    w(&proj.join(".cursor/skills/s/SKILL.md"), "---\nname: s\ndescription: S\n---\n");

    let ctx = ScanContext::load(&paths);

    let claude_only = ToolingFilter {
        claude: true,
        cursor: false,
    };
    let g = scan_global_with(&ctx, &claude_only);
    assert!(g.items.artifacts.iter().any(|a| a.product == Tooling::Claude && a.kind == ArtifactKind::Skill));
    assert!(!g.items.artifacts.iter().any(|a| a.product == Tooling::Cursor));
    assert!(g.items.mcp_servers.iter().any(|m| m.name == "gh"));
    let b = scan_project_with(&ctx, &proj, &claude_only);
    assert!(b.items.artifacts.iter().any(|a| a.product == Tooling::Claude));
    assert!(!b.items.artifacts.iter().any(|a| a.product == Tooling::Cursor));

    let cursor_only = ToolingFilter {
        claude: false,
        cursor: true,
    };
    let g = scan_global_with(&ctx, &cursor_only);
    assert!(!g.items.artifacts.iter().any(|a| a.product == Tooling::Claude));
    assert!(g.items.artifacts.iter().any(|a| a.product == Tooling::Cursor && a.kind == ArtifactKind::Rule));
    assert!(g.items.mcp_servers.is_empty());
    assert!(!g.credentials_present);
    let b = scan_project_with(&ctx, &proj, &cursor_only);
    assert!(!b.items.artifacts.iter().any(|a| a.product == Tooling::Claude));
    assert!(b.items.artifacts.iter().any(|a| a.product == Tooling::Cursor));

    let both = scan_all(
        &paths,
        &ScanInput {
            scan_roots: vec![d.path().to_path_buf()],
            pinned: vec![],
            max_depth: 3,
            filter: all_filter(),
        },
    );
    assert!(both.global.items.artifacts.iter().any(|a| a.product == Tooling::Claude));
    assert!(both.global.items.artifacts.iter().any(|a| a.product == Tooling::Cursor));
}
