//! Read-only scanner for global and per-project Claude Code configuration.

pub mod collect;
pub mod discovery;
pub mod hooks;
pub mod plugins;

pub use discovery::{discover_projects, discover_projects_excluding, info_for_dir};

use crate::models::*;
use crate::parser::{self, mask_secrets, JsonFile};
use crate::paths::{is_skipped_dir, Paths};
use collect::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const DEFAULT_MAX_DEPTH: usize = 4;
const MAX_NESTED_MEMORY_FILES: usize = 200;

/// `~/.claude.json` is parsed once per scan and shared.
pub struct ScanContext {
    pub paths: Paths,
    pub claude_json: JsonFile,
    /// canonical project path → its `mcpServers` object (from `projects` map)
    project_mcp: HashMap<String, Value>,
}

impl ScanContext {
    pub fn load(paths: &Paths) -> Self {
        let claude_json = parser::read_json_file(&paths.claude_json);
        let mut project_mcp = HashMap::new();
        if let Some(projects) = claude_json
            .value
            .as_ref()
            .and_then(|v| v.get("projects"))
            .and_then(|p| p.as_object())
        {
            for (k, v) in projects {
                if let Some(m) = v.get("mcpServers").filter(|m| m.as_object().map(|o| !o.is_empty()).unwrap_or(false)) {
                    let canon = discovery::canonical(Path::new(k)).to_string_lossy().to_string();
                    project_mcp.insert(canon, m.clone());
                }
            }
        }
        Self { paths: paths.clone(), claude_json, project_mcp }
    }
}

/// Redacted rendering of `~/.claude.json` — only MCP server definitions, masked.
pub fn claude_json_view(v: &Value) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(m) = v.get("mcpServers") {
        out.insert("mcpServers".into(), mask_secrets(m));
    }
    if let Some(projects) = v.get("projects").and_then(|p| p.as_object()) {
        let mut pm = serde_json::Map::new();
        for (k, pv) in projects {
            if let Some(m) = pv.get("mcpServers") {
                pm.insert(k.clone(), json!({ "mcpServers": mask_secrets(m) }));
            }
        }
        out.insert("projects".into(), Value::Object(pm));
    }
    Value::Object(out)
}

// ---------------------------------------------------------------- global

pub fn scan_global() -> GlobalBundle {
    scan_global_with(&ScanContext::load(&Paths::resolve()))
}

pub fn scan_global_with(ctx: &ScanContext) -> GlobalBundle {
    let paths = &ctx.paths;
    let mut c = Collection::default();

    // Managed (enterprise) — optional, may be unreadable.
    collect_settings_file(&mut c, &paths.managed_dir.join("managed-settings.json"), Scope::Managed, None, true);
    let drop_in = paths.managed_dir.join("managed-settings.d");
    if let Ok(rd) = std::fs::read_dir(&drop_in) {
        let mut files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
            .collect();
        files.sort();
        for f in files {
            collect_settings_file(&mut c, &f, Scope::Managed, None, false);
        }
    }
    collect_claude_md(&mut c, &paths.managed_dir.join("CLAUDE.md"), Scope::Managed, None, false);
    collect_mcp_json(&mut c, &paths.managed_dir.join("managed-mcp.json"), Scope::Managed, None, "managed-mcp.json");

    // User scope
    collect_settings_file(&mut c, &paths.config_dir.join("settings.json"), Scope::User, None, true);
    collect_claude_md(&mut c, &paths.config_dir.join("CLAUDE.md"), Scope::User, None, true);
    collect_claude_dir(&mut c, &paths.config_dir, Scope::User, None);
    plugins::collect_plugins(&mut c, paths);
    plugins::collect_memory(&mut c, paths);
    let credentials_present = if let Some(a) = plugins::credentials_artifact(paths) {
        c.artifacts.push(a);
        true
    } else {
        false
    };

    // ~/.claude.json: top-level (user-scope) MCP servers only. The `projects` map is
    // attributed to each project's own bundle.
    let cj = &ctx.claude_json;
    if cj.status != DiscoveryStatus::Missing {
        let mut a = ConfigArtifact::new(
            ArtifactKind::Other,
            Scope::GlobalConfig,
            ".claude.json",
            path_str(&paths.claude_json),
            None,
        );
        a.status = cj.status;
        a.error = cj.error.clone();
        if let Ok(m) = std::fs::metadata(&paths.claude_json) {
            a.size_bytes = Some(m.len());
            a.modified_at = m
                .modified()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
        }
        if let Some(v) = &cj.value {
            let n_proj = v.get("projects").and_then(|p| p.as_object()).map(|o| o.len()).unwrap_or(0);
            a.description = Some(format!("{n_proj} project entr{}", if n_proj == 1 { "y" } else { "ies" }));
            a.metadata = json!({ "projectEntries": n_proj });
            if let Some(s) = v.get("mcpServers") {
                collect_mcp_servers(&mut c, s, Scope::User, &paths.claude_json, None, "~/.claude.json");
            }
        }
        if matches!(cj.status, DiscoveryStatus::InvalidJson | DiscoveryStatus::Unreadable) {
            c.errors.push(ScanError {
                path: path_str(&paths.claude_json),
                scope: Some(Scope::GlobalConfig),
                status: cj.status,
                message: cj.error.clone().unwrap_or_default(),
            });
        }
        c.artifacts.push(a);
    }

    GlobalBundle {
        config_dir: path_str(&paths.config_dir),
        managed_dir: path_str(&paths.managed_dir),
        claude_json_path: path_str(&paths.claude_json),
        credentials_present,
        items: c,
    }
}

// ---------------------------------------------------------------- project

pub fn scan_project(root: &Path) -> ProjectBundle {
    scan_project_with(&ScanContext::load(&Paths::resolve()), root)
}

pub fn scan_project_with(ctx: &ScanContext, root: &Path) -> ProjectBundle {
    let root = discovery::canonical(root);
    let project = path_str(&root);
    let p = Some(project.as_str());
    let mut c = Collection::default();
    let claude = root.join(".claude");

    collect_settings_file(&mut c, &claude.join("settings.json"), Scope::Project, p, true);
    collect_settings_file(&mut c, &claude.join("settings.local.json"), Scope::ProjectLocal, p, true);

    collect_claude_md(&mut c, &root.join("CLAUDE.md"), Scope::Project, p, true);
    collect_claude_md(&mut c, &claude.join("CLAUDE.md"), Scope::Project, p, false);
    collect_claude_md(&mut c, &root.join("CLAUDE.local.md"), Scope::ProjectLocal, p, false);
    collect_claude_md(&mut c, &root.join("AGENTS.md"), Scope::Project, p, false);

    collect_mcp_json(&mut c, &root.join(".mcp.json"), Scope::Project, p, ".mcp.json");
    if let Some(servers) = ctx.project_mcp.get(&project) {
        collect_mcp_servers(
            &mut c,
            servers,
            Scope::ProjectLocal,
            &ctx.paths.claude_json,
            p,
            "~/.claude.json#projects",
        );
    }

    collect_claude_dir(&mut c, &claude, Scope::Project, p);
    collect_nested_memory_files(&mut c, &root, p);

    let mut info = discovery::info_for_dir(&root);
    info.artifact_count = c.artifacts.iter().filter(|a| a.status != DiscoveryStatus::Missing).count();
    ProjectBundle { project: info, items: c }
}

/// `CLAUDE.md` / `CLAUDE.local.md` in subdirectories (loaded on demand by Claude Code).
fn collect_nested_memory_files(c: &mut Collection, root: &Path, project: Option<&str>) {
    let mut found = 0usize;
    let walker = WalkDir::new(root)
        .max_depth(6)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !(e.file_type().is_dir() && (is_skipped_dir(&name) || name.starts_with('.')))
        });
    for e in walker.filter_map(|e| e.ok()) {
        // depth 1 is the project root's own CLAUDE.md, handled separately.
        if !e.file_type().is_file() || e.depth() < 2 {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if name == "CLAUDE.md" || name == "CLAUDE.local.md" {
            collect_claude_md(c, e.path(), Scope::Nested, project, false);
            found += 1;
            if found >= MAX_NESTED_MEMORY_FILES {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------- full scan

#[derive(Debug, Clone, Default)]
pub struct ScanInput {
    pub scan_roots: Vec<PathBuf>,
    pub pinned: Vec<PathBuf>,
    pub max_depth: usize,
}

pub fn scan_all(paths: &Paths, input: &ScanInput) -> ScanSnapshot {
    let started = std::time::Instant::now();
    let ctx = ScanContext::load(paths);
    let global = scan_global_with(&ctx);
    let mut errors: Vec<ScanError> = Vec::new();

    let depth = if input.max_depth == 0 { DEFAULT_MAX_DEPTH } else { input.max_depth };
    let discovered =
        discover_projects_excluding(&input.scan_roots, depth, &[paths.home.clone()]);

    // canonical path → (pinned, discovered)
    let mut all: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for d in &discovered {
        all.entry(d.path.clone()).or_default().1 = true;
    }
    for pin in &input.pinned {
        if !pin.is_dir() {
            errors.push(ScanError {
                path: path_str(pin),
                scope: Some(Scope::Project),
                status: DiscoveryStatus::Missing,
                message: "Pinned project directory does not exist".into(),
            });
            continue;
        }
        all.entry(path_str(&discovery::canonical(pin))).or_default().0 = true;
    }

    let mut projects: Vec<ProjectBundle> = all
        .iter()
        .map(|(path, (pinned, disc))| {
            let mut b = scan_project_with(&ctx, Path::new(path));
            b.project.pinned = *pinned;
            b.project.discovered = *disc;
            b
        })
        .collect();
    projects.sort_by(|a, b| a.project.path.cmp(&b.project.path));

    errors.extend(global.items.errors.iter().cloned());
    for p in &projects {
        errors.extend(p.items.errors.iter().cloned());
    }

    let conflicts = crate::resolver::detect_conflicts(&global, &projects);
    let scanned_at = chrono::Utc::now().to_rfc3339();
    let stats = build_stats(&global, &projects, conflicts.len(), errors.len(), &scanned_at);

    ScanSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        scanned_at,
        duration_ms: started.elapsed().as_millis() as u64,
        global,
        projects,
        conflicts,
        errors,
        stats,
    }
}

pub fn build_stats(
    global: &GlobalBundle,
    projects: &[ProjectBundle],
    conflicts: usize,
    errors: usize,
    scanned_at: &str,
) -> DashboardStats {
    let mut s = DashboardStats {
        project_count: projects.len(),
        conflict_count: conflicts,
        error_count: errors,
        last_scan_at: Some(scanned_at.to_string()),
        ..Default::default()
    };
    let all = std::iter::once(&global.items).chain(projects.iter().map(|p| &p.items));
    for items in all {
        for a in items.artifacts.iter().filter(|a| a.status != DiscoveryStatus::Missing) {
            s.total_artifacts += 1;
            *s.by_kind.entry(a.kind.as_str().to_string()).or_default() += 1;
            *s.by_scope.entry(a.scope.as_str().to_string()).or_default() += 1;
            match a.kind {
                ArtifactKind::Skill => s.skills += 1,
                ArtifactKind::Command => s.commands += 1,
                ArtifactKind::Agent => s.agents += 1,
                ArtifactKind::Rule => s.rules += 1,
                ArtifactKind::Hook => s.hooks += 1,
                ArtifactKind::McpServer => s.mcp_servers += 1,
                ArtifactKind::Plugin => s.plugins += 1,
                ArtifactKind::Memory => s.memories += 1,
                _ => {}
            }
        }
    }
    s
}

#[cfg(test)]
mod tests;
