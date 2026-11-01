//! Project discovery under user-chosen scan roots.

use crate::models::ProjectInfo;
use crate::paths::is_skipped_dir;
use crate::tooling::ToolingFilter;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn canonical(p: &Path) -> PathBuf {
    dunce::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Describe `dir` as a project if it has markers of at least one **enabled** product.
///
/// Discovery rule: a directory is a project if it has markers of at least one ENABLED
/// product. Markers of disabled products alone do not make it "discovered".
/// Pinned projects still scan regardless.
///
/// `has_*` flags always reflect what is on disk (truthful for UI display), even when
/// that product is disabled — we only return `Some` when an enabled marker exists.
pub fn probe_project(dir: &Path, filter: &ToolingFilter) -> Option<ProjectInfo> {
    let has_claude_dir = dir.join(".claude").is_dir();
    let has_claude_md = dir.join("CLAUDE.md").is_file();
    let has_mcp_json = dir.join(".mcp.json").is_file();
    let has_cursor_dir = dir.join(".cursor").is_dir();
    let has_cursor_mcp_json = dir.join(".cursor/mcp.json").is_file();
    let has_codex_dir = dir.join(".codex").is_dir();

    // Claude-ish discovery markers: .claude, CLAUDE.md, .mcp.json
    let claude_marker = has_claude_dir || has_claude_md || has_mcp_json;
    // Cursor discovery markers: .cursor / .cursor/mcp.json
    let cursor_marker = has_cursor_dir || has_cursor_mcp_json;

    // Codex discovery marker: `.codex` only (bare AGENTS.md never discovers a project).
    let codex_marker = has_codex_dir;

    let enabled_marker = (filter.claude && claude_marker)
        || (filter.cursor && cursor_marker)
        || (filter.codex && codex_marker);
    if !enabled_marker {
        return None;
    }
    Some(project_info(
        dir,
        has_claude_dir,
        has_claude_md,
        has_mcp_json,
        has_cursor_dir,
        has_cursor_mcp_json,
        has_codex_dir,
    ))
}

pub fn project_info(
    dir: &Path,
    has_claude_dir: bool,
    has_claude_md: bool,
    has_mcp_json: bool,
    has_cursor_dir: bool,
    has_cursor_mcp_json: bool,
    has_codex_dir: bool,
) -> ProjectInfo {
    ProjectInfo {
        path: dir.to_string_lossy().to_string(),
        name: dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.to_string_lossy().to_string()),
        has_claude_dir,
        has_claude_md,
        has_mcp_json,
        has_cursor_dir,
        has_cursor_mcp_json,
        has_codex_dir,
        pinned: false,
        discovered: true,
        artifact_count: 0,
    }
}

/// Build info for any directory (pinned projects need not have markers).
pub fn info_for_dir(dir: &Path) -> ProjectInfo {
    project_info(
        dir,
        dir.join(".claude").is_dir(),
        dir.join("CLAUDE.md").is_file(),
        dir.join(".mcp.json").is_file(),
        dir.join(".cursor").is_dir(),
        dir.join(".cursor/mcp.json").is_file(),
        dir.join(".codex").is_dir(),
    )
}

pub fn discover_projects(roots: &[PathBuf], max_depth: usize, filter: &ToolingFilter) -> Vec<ProjectInfo> {
    discover_projects_excluding(roots, max_depth, &[], filter)
}

/// Walk `roots` up to `max_depth` levels. Directory identity is the canonical
/// path, so symlink loops terminate and duplicate roots are deduplicated.
pub fn discover_projects_excluding(
    roots: &[PathBuf],
    max_depth: usize,
    exclude: &[PathBuf],
    filter: &ToolingFilter,
) -> Vec<ProjectInfo> {
    let exclude: HashSet<PathBuf> = exclude.iter().map(|p| canonical(p)).collect();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut found: BTreeMap<PathBuf, ProjectInfo> = BTreeMap::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        walk(
            &canonical(root),
            0,
            max_depth,
            &exclude,
            filter,
            &mut visited,
            &mut found,
        );
    }
    found.into_values().collect()
}

fn walk(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    exclude: &HashSet<PathBuf>,
    filter: &ToolingFilter,
    visited: &mut HashSet<PathBuf>,
    found: &mut BTreeMap<PathBuf, ProjectInfo>,
) {
    if !visited.insert(dir.to_path_buf()) {
        return;
    }
    if !exclude.contains(dir) {
        if let Some(info) = probe_project(dir, filter) {
            found.insert(dir.to_path_buf(), info);
        }
    }
    if depth >= max_depth {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut children: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    children.sort();
    for child in children {
        let name = child.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if is_skipped_dir(&name) || name.starts_with('.') {
            continue;
        }
        // `metadata` follows symlinks; loops are cut by the canonical `visited` set.
        if !fs::metadata(&child).map(|m| m.is_dir()).unwrap_or(false) {
            continue;
        }
        let canon = canonical(&child);
        walk(&canon, depth + 1, max_depth, exclude, filter, visited, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn mk(p: &Path) {
        fs::create_dir_all(p).unwrap();
    }

    fn all() -> ToolingFilter {
        ToolingFilter::all()
    }

    #[test]
    fn discovers_markers_and_skips_excluded() {
        let d = tempdir().unwrap();
        let r = d.path();
        mk(&r.join("a/.claude"));
        mk(&r.join("b"));
        fs::write(r.join("b/CLAUDE.md"), "x").unwrap();
        mk(&r.join("c"));
        fs::write(r.join("c/.mcp.json"), "{}").unwrap();
        mk(&r.join("plain/src"));
        mk(&r.join("node_modules/pkg/.claude"));
        mk(&r.join("target/x/.claude"));
        mk(&r.join(".hidden/.claude"));

        let found = discover_projects(&[r.to_path_buf()], 4, &all());
        let names: Vec<_> = found.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        assert!(found[0].has_claude_dir && found[1].has_claude_md && found[2].has_mcp_json);
    }

    #[test]
    fn discovers_cursor_only_project() {
        let d = tempdir().unwrap();
        let r = d.path();
        mk(&r.join("cursor-only/.cursor"));
        fs::write(r.join("cursor-only/.cursor/mcp.json"), "{}").unwrap();
        mk(&r.join("cursor-mcp"));
        // `.cursor/mcp.json` without a `.cursor` dir still counts via parent create
        fs::create_dir_all(r.join("cursor-mcp/.cursor")).unwrap();
        fs::write(r.join("cursor-mcp/.cursor/mcp.json"), "{}").unwrap();
        // Only a nested mcp under a sibling should not invent extra projects
        mk(&r.join("plain/src"));

        let found = discover_projects(&[r.to_path_buf()], 4, &all());
        let names: Vec<_> = found.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["cursor-mcp", "cursor-only"]);
        assert!(found.iter().all(|p| p.has_cursor_dir && p.has_cursor_mcp_json));
        assert!(found.iter().all(|p| !p.has_claude_dir && !p.has_claude_md && !p.has_mcp_json));
    }

    #[test]
    fn probe_project_with_only_cursor_dir() {
        let d = tempdir().unwrap();
        let proj = d.path().join("p");
        mk(&proj.join(".cursor"));
        let info = probe_project(&proj, &all()).expect("cursor dir should mark a project");
        assert!(info.has_cursor_dir);
        assert!(!info.has_cursor_mcp_json);
        assert!(!info.has_claude_dir);
        assert!(!info.has_claude_md);
        assert!(!info.has_mcp_json);
    }

    #[test]
    fn disabled_product_markers_do_not_discover() {
        let d = tempdir().unwrap();
        let r = d.path();
        mk(&r.join("claude-only/.claude"));
        mk(&r.join("cursor-only/.cursor"));
        mk(&r.join("both/.claude"));
        mk(&r.join("both/.cursor"));

        let claude_only = ToolingFilter {
            claude: true,
            cursor: false,
            codex: false,
        };
        let names: Vec<_> = discover_projects(&[r.to_path_buf()], 3, &claude_only)
            .iter()
            .map(|p| p.name.clone())
            .collect();
        assert_eq!(names, vec!["both", "claude-only"]);

        let cursor_only = ToolingFilter {
            claude: false,
            cursor: true,
            codex: false,
        };
        let names: Vec<_> = discover_projects(&[r.to_path_buf()], 3, &cursor_only)
            .iter()
            .map(|p| p.name.clone())
            .collect();
        assert_eq!(names, vec!["both", "cursor-only"]);

        // has_* still truthful when only Cursor markers are enabled for discovery
        let both = probe_project(&r.join("both"), &cursor_only).unwrap();
        assert!(both.has_claude_dir && both.has_cursor_dir);
    }

    #[test]
    fn nested_claude_dirs_found_and_depth_limited() {
        let d = tempdir().unwrap();
        let r = d.path();
        mk(&r.join("mono/.claude"));
        mk(&r.join("mono/packages/web/.claude"));
        mk(&r.join("mono/packages/deep/x/y/z/.claude"));
        let found = discover_projects(&[r.to_path_buf()], 3, &all());
        let names: Vec<_> = found.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["mono", "web"]);
    }

    #[test]
    fn duplicate_roots_deduped() {
        let d = tempdir().unwrap();
        let r = d.path();
        mk(&r.join("p/.claude"));
        let roots = vec![r.to_path_buf(), r.to_path_buf(), r.join("p"), r.join("p/../p")];
        let found = discover_projects(&roots, 3, &all());
        assert_eq!(found.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_loop_terminates() {
        use std::os::unix::fs::symlink;
        let d = tempdir().unwrap();
        let r = d.path();
        mk(&r.join("a/b"));
        mk(&r.join("a/.claude"));
        symlink(r.join("a"), r.join("a/b/loop")).unwrap();
        symlink(r, r.join("a/b/toroot")).unwrap();
        // alias to same project is deduped by canonical path
        symlink(r.join("a"), r.join("alias")).unwrap();
        let found = discover_projects(&[r.to_path_buf()], 50, &all());
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn excluded_dir_not_reported() {
        let d = tempdir().unwrap();
        mk(&d.path().join(".claude"));
        mk(&d.path().join("p/.claude"));
        let found = discover_projects_excluding(
            &[d.path().to_path_buf()],
            3,
            &[d.path().to_path_buf()],
            &all(),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "p");
    }

    #[test]
    fn discovers_codex_only_project() {
        let d = tempdir().unwrap();
        let r = d.path();
        mk(&r.join("codex-app/.codex"));
        mk(&r.join("agents-only"));
        fs::write(r.join("agents-only/AGENTS.md"), "x").unwrap();
        fs::write(r.join("agents-only/AGENTS.override.md"), "x").unwrap();

        let found = discover_projects(&[r.to_path_buf()], 3, &all());
        let names: Vec<_> = found.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["codex-app"], "bare AGENTS*.md must not discover a project");
        assert!(found[0].has_codex_dir);
        assert!(!found[0].has_claude_dir && !found[0].has_cursor_dir);

        let codex_only = ToolingFilter { claude: false, cursor: false, codex: true };
        assert_eq!(discover_projects(&[r.to_path_buf()], 3, &codex_only).len(), 1);

        // Codex off: `.codex` alone does not discover; `has_codex_dir` stays truthful.
        let no_codex = ToolingFilter { claude: true, cursor: true, codex: false };
        assert!(discover_projects(&[r.to_path_buf()], 3, &no_codex).is_empty());
        mk(&r.join("codex-app/.claude"));
        let both = probe_project(&r.join("codex-app"), &no_codex).unwrap();
        assert!(both.has_codex_dir && both.has_claude_dir);
        assert!(info_for_dir(&r.join("codex-app")).has_codex_dir);
    }
}
