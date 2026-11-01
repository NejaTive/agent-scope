//! Resolution of Claude Code / Cursor config locations and the read allowlist.

use crate::errors::{AppError, AppResult};
use crate::tooling::ToolingFilter;
use std::path::{Component, Path, PathBuf};

/// How a path maps to an AI tooling product for enable/disable gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathProduct {
    Claude,
    Cursor,
    Codex,
    /// Shared across products (e.g. `.agents/skills`, bare `AGENTS.md`).
    Shared,
}

pub const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".git",
    ".next",
    "coverage",
    "__pycache__",
    ".turbo",
    ".cache",
];

pub fn is_skipped_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Project-level file names that may be read through `get_artifact_content`.
pub const PROJECT_FILE_NAMES: &[&str] = &[
    "CLAUDE.md",
    "CLAUDE.local.md",
    "AGENTS.md",
    "AGENTS.override.md",
    ".mcp.json",
];

/// Top-level entries of the user Claude config dir that may be read.
const CONFIG_DIR_ALLOWED: &[&str] = &[
    "settings.json",
    "CLAUDE.md",
    "rules",
    "skills",
    "commands",
    "agents",
    "output-styles",
    "workflows",
    "plugins",
    "projects", // only `*/memory/**`, enforced below
];

/// Top-level entries of `~/.cursor` that may be read (no credentials / secrets).
const CURSOR_DIR_ALLOWED: &[&str] = &[
    "rules",
    "skills",
    "commands",
    "agents",
    "hooks.json",
    "mcp.json",
    "hooks", // adjacent hook scripts
];

/// Top-level entries of `CODEX_HOME` / `~/.codex` that may be read (never `auth.json`).
const CODEX_DIR_ALLOWED: &[&str] = &[
    "config.toml",
    "AGENTS.md",
    "AGENTS.override.md",
    "hooks.json",
    "hooks", // adjacent hook scripts
    "skills", // legacy skill root
];

#[derive(Debug, Clone)]
pub struct Paths {
    pub home: PathBuf,
    /// `CLAUDE_CONFIG_DIR` or `~/.claude`
    pub config_dir: PathBuf,
    /// `~/.claude.json` (inside `CLAUDE_CONFIG_DIR` when that is set)
    pub claude_json: PathBuf,
    pub managed_dir: PathBuf,
    /// `~/.cursor`
    pub cursor_dir: PathBuf,
    /// `~/.cursor/mcp.json`
    pub cursor_mcp_json: PathBuf,
    /// `~/.cursor/hooks.json`
    pub cursor_hooks_json: PathBuf,
    /// Managed Cursor hooks (macOS Application Support / Windows ProgramData / Linux `/etc`).
    pub cursor_managed_hooks: PathBuf,
    /// `CODEX_HOME` or `~/.codex`
    pub codex_dir: PathBuf,
    /// Admin Codex skills (`/etc/codex/skills`) when present.
    pub codex_admin_skills: PathBuf,
}

fn expand_tilde(s: &str, home: &Path) -> PathBuf {
    if s == "~" {
        home.to_path_buf()
    } else if let Some(rest) = s.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(s)
    }
}

pub fn managed_dir_default() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/ClaudeCode")
    } else if cfg!(target_os = "windows") {
        PathBuf::from("C:\\Program Files\\ClaudeCode")
    } else {
        PathBuf::from("/etc/claude-code")
    }
}

pub fn cursor_managed_hooks_default() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/Cursor/hooks.json")
    } else if cfg!(target_os = "windows") {
        PathBuf::from("C:\\ProgramData\\Cursor\\hooks.json")
    } else {
        PathBuf::from("/etc/cursor/hooks.json")
    }
}

pub fn codex_admin_skills_default() -> PathBuf {
    PathBuf::from("/etc/codex/skills")
}

impl Paths {
    pub fn resolve() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let env_dir = std::env::var("CLAUDE_CONFIG_DIR")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let (config_dir, claude_json) = match env_dir {
            Some(d) => {
                let dir = expand_tilde(&d, &home);
                let json = dir.join(".claude.json");
                (dir, json)
            }
            None => (home.join(".claude"), home.join(".claude.json")),
        };
        let cursor_dir = home.join(".cursor");
        let codex_env = std::env::var("CODEX_HOME")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let codex_dir = match codex_env {
            Some(d) => expand_tilde(&d, &home),
            None => home.join(".codex"),
        };
        Self {
            home,
            config_dir,
            claude_json,
            managed_dir: managed_dir_default(),
            cursor_mcp_json: cursor_dir.join("mcp.json"),
            cursor_hooks_json: cursor_dir.join("hooks.json"),
            cursor_dir,
            cursor_managed_hooks: cursor_managed_hooks_default(),
            codex_dir,
            codex_admin_skills: codex_admin_skills_default(),
        }
    }

    /// Fully sandboxed layout under `root` (used by tests; never touches real dirs).
    pub fn for_root(root: &Path) -> Self {
        let cursor_dir = root.join(".cursor");
        Self {
            home: root.to_path_buf(),
            config_dir: root.join(".claude"),
            claude_json: root.join(".claude.json"),
            managed_dir: root.join("managed"),
            cursor_mcp_json: cursor_dir.join("mcp.json"),
            cursor_hooks_json: cursor_dir.join("hooks.json"),
            cursor_dir,
            cursor_managed_hooks: root.join("managed-cursor").join("hooks.json"),
            codex_dir: root.join(".codex"),
            codex_admin_skills: root.join("etc-codex").join("skills"),
        }
    }

    pub fn credentials_file(&self) -> PathBuf {
        self.config_dir.join(".credentials.json")
    }

    fn canon_of(p: &Path) -> PathBuf {
        dunce::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
    }

    fn normal_components(rel: &Path) -> Vec<String> {
        rel.components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                _ => None,
            })
            .collect()
    }

    fn classify_path(&self, path: &Path, project_roots: &[PathBuf]) -> Option<PathProduct> {
        if self.is_claude_json(path) {
            return Some(PathProduct::Claude);
        }
        let cfg = Self::canon_of(&self.config_dir);
        if path.starts_with(&cfg) {
            return Some(PathProduct::Claude);
        }
        let managed = Self::canon_of(&self.managed_dir);
        if path.starts_with(&managed) {
            return Some(PathProduct::Claude);
        }
        let cursor = Self::canon_of(&self.cursor_dir);
        if path.starts_with(&cursor) {
            return Some(PathProduct::Cursor);
        }
        let cursor_managed = Self::canon_of(&self.cursor_managed_hooks);
        if path == cursor_managed {
            return Some(PathProduct::Cursor);
        }
        if let Some(parent) = self.cursor_managed_hooks.parent() {
            let parent = Self::canon_of(parent);
            if path.starts_with(&parent) {
                return Some(PathProduct::Cursor);
            }
        }
        let codex = Self::canon_of(&self.codex_dir);
        if path.starts_with(&codex) {
            return Some(PathProduct::Codex);
        }
        let admin_skills = Self::canon_of(&self.codex_admin_skills);
        if path.starts_with(&admin_skills) {
            return Some(PathProduct::Codex);
        }
        // User shared skills: `~/.agents/skills`
        let agents_home = Self::canon_of(&self.home.join(".agents"));
        if let Ok(rel) = path.strip_prefix(&agents_home) {
            let comps = Self::normal_components(rel);
            if comps.first().map(|s| s.as_str()) == Some("skills") {
                return Some(PathProduct::Shared);
            }
        }
        for root in project_roots {
            let root = Self::canon_of(root);
            if let Ok(rel) = path.strip_prefix(&root) {
                let comps = Self::normal_components(rel);
                let first = comps.first().map(|s| s.as_str());
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if first == Some(".claude") {
                    return Some(PathProduct::Claude);
                }
                if first == Some(".cursor") {
                    return Some(PathProduct::Cursor);
                }
                if first == Some(".codex") {
                    return Some(PathProduct::Codex);
                }
                if first == Some(".agents") && comps.get(1).map(|s| s.as_str()) == Some("skills") {
                    return Some(PathProduct::Shared);
                }
                if comps.len() == 1 {
                    if matches!(name.as_str(), "CLAUDE.md" | "CLAUDE.local.md" | ".mcp.json") {
                        return Some(PathProduct::Claude);
                    }
                    if matches!(name.as_str(), "AGENTS.md" | "AGENTS.override.md") {
                        return Some(PathProduct::Shared);
                    }
                }
            }
        }
        None
    }

    fn assert_product_enabled(
        &self,
        path: &Path,
        project_roots: &[PathBuf],
        filter: &ToolingFilter,
    ) -> AppResult<()> {
        match self.classify_path(path, project_roots) {
            Some(PathProduct::Claude) if !filter.claude => Err(AppError::AccessDenied(
                "отслеживание Claude Code выключено".into(),
            )),
            Some(PathProduct::Cursor) if !filter.cursor => Err(AppError::AccessDenied(
                "отслеживание Cursor выключено".into(),
            )),
            Some(PathProduct::Codex) if !filter.codex => Err(AppError::AccessDenied(
                "отслеживание Codex выключено".into(),
            )),
            Some(PathProduct::Shared) if !filter.allows_shared() => Err(AppError::AccessDenied(
                "отслеживание AI-инструментов выключено".into(),
            )),
            _ => Ok(()),
        }
    }

    /// Read allowlist for an absolute path. File need not exist (used for rename dest).
    fn assert_in_read_allowlist(&self, path: &Path, project_roots: &[PathBuf]) -> AppResult<()> {
        if path
            .file_name()
            .map(|n| n == ".credentials.json" || n == "auth.json")
            .unwrap_or(false)
        {
            return Err(AppError::AccessDenied("credentials are never displayed".into()));
        }
        if self.is_claude_json(path) {
            return Ok(());
        }

        let cfg = Self::canon_of(&self.config_dir);
        if let Ok(rel) = path.strip_prefix(&cfg) {
            let comps = Self::normal_components(rel);
            if let Some(first) = comps.first() {
                if CONFIG_DIR_ALLOWED.contains(&first.as_str()) {
                    if first == "projects" && !comps.iter().skip(1).any(|c| c == "memory") {
                        return Err(AppError::AccessDenied(
                            "only project memory may be read from projects/".into(),
                        ));
                    }
                    return Ok(());
                }
            }
            return Err(AppError::AccessDenied(format!(
                "{} is not a known Claude Code config file",
                path.display()
            )));
        }

        let cursor = Self::canon_of(&self.cursor_dir);
        if let Ok(rel) = path.strip_prefix(&cursor) {
            let comps = Self::normal_components(rel);
            if let Some(first) = comps.first() {
                if CURSOR_DIR_ALLOWED.contains(&first.as_str()) {
                    return Ok(());
                }
            }
            return Err(AppError::AccessDenied(format!(
                "{} is not a known Cursor config file",
                path.display()
            )));
        }

        let codex = Self::canon_of(&self.codex_dir);
        if let Ok(rel) = path.strip_prefix(&codex) {
            let comps = Self::normal_components(rel);
            if let Some(first) = comps.first() {
                if CODEX_DIR_ALLOWED.contains(&first.as_str()) {
                    return Ok(());
                }
            }
            return Err(AppError::AccessDenied(format!(
                "{} is not a known Codex config file",
                path.display()
            )));
        }

        let admin_skills = Self::canon_of(&self.codex_admin_skills);
        if path.starts_with(&admin_skills) {
            return Ok(());
        }

        let agents_home = Self::canon_of(&self.home.join(".agents"));
        if let Ok(rel) = path.strip_prefix(&agents_home) {
            let comps = Self::normal_components(rel);
            if comps.first().map(|s| s.as_str()) == Some("skills") {
                return Ok(());
            }
        }

        let managed = Self::canon_of(&self.managed_dir);
        if path.starts_with(&managed) {
            return Ok(());
        }

        let cursor_managed = Self::canon_of(&self.cursor_managed_hooks);
        if path == cursor_managed {
            return Ok(());
        }
        if let Some(parent) = self.cursor_managed_hooks.parent() {
            let parent = Self::canon_of(parent);
            if let Ok(rel) = path.strip_prefix(&parent) {
                let first = rel.components().next().and_then(|c| match c {
                    Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                    _ => None,
                });
                if first.as_deref() == Some("hooks") || path == cursor_managed {
                    return Ok(());
                }
            }
        }

        for root in project_roots {
            let root = Self::canon_of(root);
            if let Ok(rel) = path.strip_prefix(&root) {
                let comps = Self::normal_components(rel);
                let first = comps.first().map(|s| s.as_str());
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let under_claude = first == Some(".claude");
                let under_cursor = first == Some(".cursor");
                let under_codex = first == Some(".codex");
                let under_agents_skills =
                    first == Some(".agents") && comps.get(1).map(|s| s.as_str()) == Some("skills");
                if under_claude
                    || under_cursor
                    || under_codex
                    || under_agents_skills
                    || PROJECT_FILE_NAMES.contains(&name.as_str())
                {
                    return Ok(());
                }
            }
        }
        Err(AppError::AccessDenied(format!(
            "{} is outside known Claude Code / Cursor / Codex locations",
            path.display()
        )))
    }

    /// Extra denials for write/rename/delete (subset of read).
    fn assert_writable_extras(&self, path: &Path) -> AppResult<()> {
        if path
            .file_name()
            .map(|n| n == ".credentials.json" || n == "auth.json")
            .unwrap_or(false)
        {
            return Err(AppError::AccessDenied("credentials are never written".into()));
        }
        if self.is_claude_json(path) {
            return Err(AppError::AccessDenied(
                ".claude.json is view-only (redacted); writing is not allowed".into(),
            ));
        }

        let managed = Self::canon_of(&self.managed_dir);
        if path.starts_with(&managed) {
            return Err(AppError::AccessDenied(
                "managed/system Claude Code paths are not writable".into(),
            ));
        }

        let cursor_managed = Self::canon_of(&self.cursor_managed_hooks);
        if path == cursor_managed {
            return Err(AppError::AccessDenied(
                "managed Cursor hooks are not writable".into(),
            ));
        }
        if let Some(parent) = self.cursor_managed_hooks.parent() {
            let parent = Self::canon_of(parent);
            if path.starts_with(&parent) && parent.exists() {
                let under_hooks = path
                    .strip_prefix(&parent)
                    .ok()
                    .and_then(|rel| {
                        rel.components().next().and_then(|c| match c {
                            Component::Normal(s) => Some(s.to_string_lossy() == "hooks"),
                            _ => None,
                        })
                    })
                    .unwrap_or(false);
                if under_hooks || path == cursor_managed {
                    return Err(AppError::AccessDenied(
                        "managed Cursor hooks are not writable".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validate a single path basename for rename (no separators / `.` / `..`).
    pub fn validate_new_basename(name: &str) -> AppResult<&str> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::InvalidPath("new name must not be empty".into()));
        }
        if name == "." || name == ".." {
            return Err(AppError::InvalidPath("invalid new name".into()));
        }
        if name.contains('/') || name.contains('\\') || Path::new(name).components().count() != 1
        {
            return Err(AppError::InvalidPath(
                "new name must be a single path component".into(),
            ));
        }
        if !matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        ) {
            return Err(AppError::InvalidPath("invalid new name".into()));
        }
        Ok(name)
    }

    /// Check that `requested` may be served to the UI.
    /// Returns the canonical path. `.claude.json` is allowed here but callers must
    /// serve a redacted view (see `is_claude_json`).
    /// Paths belonging to a disabled product are rejected.
    pub fn check_readable(
        &self,
        requested: &Path,
        project_roots: &[PathBuf],
        filter: &ToolingFilter,
    ) -> AppResult<PathBuf> {
        let canon = dunce::canonicalize(requested)
            .map_err(|_| AppError::NotFound(requested.display().to_string()))?;
        if !canon.is_file() {
            return Err(AppError::InvalidPath(format!("{} is not a file", canon.display())));
        }
        self.assert_in_read_allowlist(&canon, project_roots)?;
        self.assert_product_enabled(&canon, project_roots, filter)?;
        Ok(canon)
    }

    pub fn is_claude_json(&self, path: &Path) -> bool {
        if let Ok(c) = dunce::canonicalize(&self.claude_json) {
            if c == path {
                return true;
            }
        }
        // Missing dest / non-canonical compare (rename checks).
        Self::canon_of(path) == Self::canon_of(&self.claude_json)
            || path == self.claude_json
    }

    /// Writable subset of [`Self::check_readable`]: never credentials, never
    /// `~/.claude.json` (redacted view only), never managed/system paths.
    pub fn check_writable(
        &self,
        requested: &Path,
        project_roots: &[PathBuf],
        filter: &ToolingFilter,
    ) -> AppResult<PathBuf> {
        let canon = self.check_readable(requested, project_roots, filter)?;
        self.assert_writable_extras(&canon)?;
        Ok(canon)
    }

    /// Destination for rename: parent must exist; file must not; path must be writable-allowlisted.
    pub fn check_writable_destination(
        &self,
        parent: &Path,
        new_name: &str,
        project_roots: &[PathBuf],
        filter: &ToolingFilter,
    ) -> AppResult<PathBuf> {
        let name = Self::validate_new_basename(new_name)?;
        let parent_canon = dunce::canonicalize(parent).map_err(|_| {
            AppError::NotFound(format!("parent directory {}", parent.display()))
        })?;
        if !parent_canon.is_dir() {
            return Err(AppError::InvalidPath(format!(
                "{} is not a directory",
                parent_canon.display()
            )));
        }
        let dest = parent_canon.join(name);
        if dest.exists() {
            return Err(AppError::Conflict(format!(
                "{} already exists",
                dest.display()
            )));
        }
        self.assert_in_read_allowlist(&dest, project_roots)?;
        self.assert_product_enabled(&dest, project_roots, filter)?;
        self.assert_writable_extras(&dest)?;
        Ok(dest)
    }
}

pub fn language_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("json") => "json",
        Some("md") | Some("markdown") => "markdown",
        Some("yml") | Some("yaml") => "yaml",
        Some("js") | Some("mjs") | Some("cjs") => "javascript",
        Some("ts") => "typescript",
        Some("toml") => "toml",
        Some("sh") | Some("bash") | Some("zsh") => "shell",
        Some("py") => "python",
        _ => "plaintext",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn all() -> ToolingFilter {
        ToolingFilter::all()
    }

    #[test]
    fn allowlist_rules() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        fs::create_dir_all(paths.config_dir.join("rules")).unwrap();
        fs::create_dir_all(paths.config_dir.join("projects/x/memory")).unwrap();
        fs::write(paths.config_dir.join("settings.json"), "{}").unwrap();
        fs::write(paths.config_dir.join("rules/a.md"), "x").unwrap();
        fs::write(paths.config_dir.join(".credentials.json"), "secret").unwrap();
        fs::write(paths.config_dir.join("projects/x/transcript.jsonl"), "x").unwrap();
        fs::write(paths.config_dir.join("projects/x/memory/MEMORY.md"), "x").unwrap();
        fs::write(dir.path().join("outside.txt"), "x").unwrap();

        let proj = dir.path().join("proj");
        fs::create_dir_all(proj.join(".claude")).unwrap();
        fs::write(proj.join("CLAUDE.md"), "x").unwrap();
        fs::write(proj.join(".env"), "SECRET=1").unwrap();
        fs::write(proj.join(".claude/settings.json"), "{}").unwrap();
        let roots = vec![proj.clone()];
        let f = all();

        assert!(paths.check_readable(&paths.config_dir.join("settings.json"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.config_dir.join("rules/a.md"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.config_dir.join("projects/x/memory/MEMORY.md"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.config_dir.join("projects/x/transcript.jsonl"), &roots, &f).is_err());
        assert!(paths.check_readable(&paths.config_dir.join(".credentials.json"), &roots, &f).is_err());
        assert!(paths.check_readable(&dir.path().join("outside.txt"), &roots, &f).is_err());
        assert!(paths.check_readable(&proj.join("CLAUDE.md"), &roots, &f).is_ok());
        assert!(paths.check_readable(&proj.join(".claude/settings.json"), &roots, &f).is_ok());
        assert!(paths.check_readable(&proj.join(".env"), &roots, &f).is_err());
        assert!(paths.check_readable(&dir.path().join("nope"), &roots, &f).is_err());
    }

    #[test]
    fn allowlist_cursor_files() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        fs::create_dir_all(paths.cursor_dir.join("rules")).unwrap();
        fs::create_dir_all(paths.cursor_dir.join("skills")).unwrap();
        fs::create_dir_all(paths.cursor_dir.join("hooks")).unwrap();
        fs::write(paths.cursor_dir.join("mcp.json"), "{}").unwrap();
        fs::write(paths.cursor_dir.join("hooks.json"), "{}").unwrap();
        fs::write(paths.cursor_dir.join("rules/a.mdc"), "x").unwrap();
        fs::write(paths.cursor_dir.join("skills/s.md"), "x").unwrap();
        fs::write(paths.cursor_dir.join("hooks/pre.sh"), "echo").unwrap();
        fs::write(paths.cursor_dir.join(".credentials.json"), "secret").unwrap();
        fs::write(paths.cursor_dir.join("secrets.txt"), "nope").unwrap();

        let proj = dir.path().join("proj");
        fs::create_dir_all(proj.join(".cursor/rules")).unwrap();
        fs::create_dir_all(proj.join(".agents/skills/foo")).unwrap();
        fs::write(proj.join(".cursor/mcp.json"), "{}").unwrap();
        fs::write(proj.join(".cursor/rules/r.mdc"), "x").unwrap();
        fs::write(proj.join("AGENTS.md"), "x").unwrap();
        fs::write(proj.join(".env"), "SECRET=1").unwrap();
        fs::write(proj.join(".agents/skills/foo/SKILL.md"), "x").unwrap();
        fs::write(proj.join(".credentials.json"), "secret").unwrap();
        let roots = vec![proj.clone()];
        let f = all();

        assert!(paths.check_readable(&paths.cursor_dir.join("mcp.json"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("hooks.json"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("rules/a.mdc"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("skills/s.md"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("hooks/pre.sh"), &roots, &f).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join(".credentials.json"), &roots, &f).is_err());
        assert!(paths.check_readable(&paths.cursor_dir.join("secrets.txt"), &roots, &f).is_err());

        assert!(paths.check_readable(&proj.join(".cursor/mcp.json"), &roots, &f).is_ok());
        assert!(paths.check_readable(&proj.join(".cursor/rules/r.mdc"), &roots, &f).is_ok());
        assert!(paths.check_readable(&proj.join("AGENTS.md"), &roots, &f).is_ok());
        assert!(paths.check_readable(&proj.join(".agents/skills/foo/SKILL.md"), &roots, &f).is_ok());
        assert!(paths.check_readable(&proj.join(".env"), &roots, &f).is_err());
        assert!(paths.check_readable(&proj.join(".credentials.json"), &roots, &f).is_err());
    }

    #[test]
    fn disabled_product_blocks_read_and_write() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        fs::create_dir_all(paths.config_dir.join("rules")).unwrap();
        fs::create_dir_all(paths.cursor_dir.join("rules")).unwrap();
        fs::write(paths.config_dir.join("settings.json"), "{}").unwrap();
        fs::write(paths.cursor_dir.join("mcp.json"), "{}").unwrap();
        let proj = dir.path().join("proj");
        fs::create_dir_all(proj.join(".claude")).unwrap();
        fs::create_dir_all(proj.join(".cursor")).unwrap();
        fs::write(proj.join(".claude/settings.json"), "{}").unwrap();
        fs::write(proj.join(".cursor/mcp.json"), "{}").unwrap();
        let roots = vec![proj.clone()];

        let no_claude = ToolingFilter {
            claude: false,
            cursor: true,
            codex: true,
        };
        assert!(paths
            .check_readable(&paths.config_dir.join("settings.json"), &roots, &no_claude)
            .is_err());
        assert!(paths
            .check_readable(&proj.join(".claude/settings.json"), &roots, &no_claude)
            .is_err());
        assert!(paths
            .check_readable(&paths.cursor_dir.join("mcp.json"), &roots, &no_claude)
            .is_ok());

        let no_cursor = ToolingFilter {
            claude: true,
            cursor: false,
            codex: true,
        };
        assert!(paths
            .check_writable(&paths.cursor_dir.join("mcp.json"), &roots, &no_cursor)
            .is_err());
        assert!(paths
            .check_writable(&proj.join(".cursor/mcp.json"), &roots, &no_cursor)
            .is_err());
        assert!(paths
            .check_writable(&paths.config_dir.join("settings.json"), &roots, &no_cursor)
            .is_ok());
    }

    #[test]
    fn allowlist_codex_files() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        let cx = &paths.codex_dir;
        fs::create_dir_all(cx.join("skills/s")).unwrap();
        fs::create_dir_all(cx.join("hooks")).unwrap();
        fs::create_dir_all(cx.join("sessions")).unwrap();
        fs::create_dir_all(paths.codex_admin_skills.join("adm")).unwrap();
        fs::create_dir_all(paths.home.join(".agents/skills/u")).unwrap();
        fs::write(cx.join("config.toml"), "model = \"x\"").unwrap();
        fs::write(cx.join("AGENTS.md"), "x").unwrap();
        fs::write(cx.join("AGENTS.override.md"), "x").unwrap();
        fs::write(cx.join("hooks.json"), "{}").unwrap();
        fs::write(cx.join("hooks/pre.sh"), "echo").unwrap();
        fs::write(cx.join("skills/s/SKILL.md"), "x").unwrap();
        fs::write(cx.join("auth.json"), "secret").unwrap();
        fs::write(cx.join("sessions/a.jsonl"), "x").unwrap();
        fs::write(cx.join("history.jsonl"), "x").unwrap();
        fs::write(paths.codex_admin_skills.join("adm/SKILL.md"), "x").unwrap();
        fs::write(paths.home.join(".agents/skills/u/SKILL.md"), "x").unwrap();

        let proj = dir.path().join("proj");
        fs::create_dir_all(proj.join(".codex/skills/p")).unwrap();
        fs::write(proj.join(".codex/config.toml"), "x").unwrap();
        fs::write(proj.join(".codex/hooks.json"), "{}").unwrap();
        fs::write(proj.join(".codex/skills/p/SKILL.md"), "x").unwrap();
        fs::write(proj.join(".codex/auth.json"), "secret").unwrap();
        fs::write(proj.join("AGENTS.override.md"), "x").unwrap();
        fs::write(proj.join(".env"), "SECRET=1").unwrap();
        let roots = vec![proj.clone()];
        let f = all();

        for ok in [
            cx.join("config.toml"),
            cx.join("AGENTS.md"),
            cx.join("AGENTS.override.md"),
            cx.join("hooks.json"),
            cx.join("hooks/pre.sh"),
            cx.join("skills/s/SKILL.md"),
            paths.codex_admin_skills.join("adm/SKILL.md"),
            paths.home.join(".agents/skills/u/SKILL.md"),
            proj.join(".codex/config.toml"),
            proj.join(".codex/hooks.json"),
            proj.join(".codex/skills/p/SKILL.md"),
            proj.join("AGENTS.override.md"),
        ] {
            assert!(paths.check_readable(&ok, &roots, &f).is_ok(), "{}", ok.display());
        }
        for denied in [
            cx.join("auth.json"),
            cx.join("sessions/a.jsonl"),
            cx.join("history.jsonl"),
            proj.join(".codex/auth.json"),
            proj.join(".env"),
        ] {
            assert!(paths.check_readable(&denied, &roots, &f).is_err(), "{}", denied.display());
        }
        let err = paths.check_readable(&cx.join("auth.json"), &roots, &f).unwrap_err();
        assert!(err.to_string().contains("credentials are never displayed"));
        let err = paths.check_readable(&cx.join("history.jsonl"), &roots, &f).unwrap_err();
        assert!(err.to_string().contains("not a known Codex config file"));

        // Writable subset: config yes, auth never.
        assert!(paths.check_writable(&cx.join("config.toml"), &roots, &f).is_ok());
        assert!(paths.check_writable(&cx.join("hooks.json"), &roots, &f).is_ok());
        assert!(paths.check_writable(&proj.join(".codex/config.toml"), &roots, &f).is_ok());
        assert!(paths.check_writable(&cx.join("auth.json"), &roots, &f).is_err());
    }

    #[test]
    fn disabled_codex_blocks_codex_paths_only() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        fs::create_dir_all(paths.codex_dir.join("skills/s")).unwrap();
        fs::create_dir_all(paths.home.join(".agents/skills/u")).unwrap();
        fs::write(paths.codex_dir.join("config.toml"), "x").unwrap();
        fs::write(paths.codex_dir.join("skills/s/SKILL.md"), "x").unwrap();
        fs::write(paths.home.join(".agents/skills/u/SKILL.md"), "x").unwrap();
        fs::create_dir_all(paths.cursor_dir.join("rules")).unwrap();
        fs::write(paths.cursor_dir.join("mcp.json"), "{}").unwrap();
        let proj = dir.path().join("proj");
        fs::create_dir_all(proj.join(".codex")).unwrap();
        fs::write(proj.join(".codex/config.toml"), "x").unwrap();
        fs::write(proj.join("AGENTS.md"), "x").unwrap();
        let roots = vec![proj.clone()];

        let no_codex = ToolingFilter { claude: true, cursor: true, codex: false };
        let err = paths
            .check_readable(&paths.codex_dir.join("config.toml"), &roots, &no_codex)
            .unwrap_err();
        assert!(err.to_string().contains("Codex"));
        assert!(paths.check_readable(&paths.codex_dir.join("skills/s/SKILL.md"), &roots, &no_codex).is_err());
        assert!(paths.check_readable(&proj.join(".codex/config.toml"), &roots, &no_codex).is_err());
        assert!(paths
            .check_writable(&paths.codex_dir.join("config.toml"), &roots, &no_codex)
            .is_err());
        assert!(paths
            .check_writable_destination(&paths.codex_dir, "new.toml", &roots, &no_codex)
            .is_err());
        // Shared + other products unaffected.
        assert!(paths.check_readable(&proj.join("AGENTS.md"), &roots, &no_codex).is_ok());
        assert!(paths.check_readable(&paths.home.join(".agents/skills/u/SKILL.md"), &roots, &no_codex).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("mcp.json"), &roots, &no_codex).is_ok());

        // Other products disabled must not block Codex.
        let only_codex = ToolingFilter { claude: false, cursor: false, codex: true };
        assert!(paths.check_readable(&paths.codex_dir.join("config.toml"), &roots, &only_codex).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("mcp.json"), &roots, &only_codex).is_err());
    }

    #[test]
    fn shared_allowed_when_only_codex_tracked() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        fs::create_dir_all(paths.home.join(".agents/skills/u")).unwrap();
        fs::write(paths.home.join(".agents/skills/u/SKILL.md"), "x").unwrap();
        let proj = dir.path().join("proj");
        fs::create_dir_all(proj.join(".agents/skills/p")).unwrap();
        fs::write(proj.join(".agents/skills/p/SKILL.md"), "x").unwrap();
        fs::write(proj.join("AGENTS.md"), "x").unwrap();
        fs::write(proj.join("AGENTS.override.md"), "x").unwrap();
        fs::write(proj.join("CLAUDE.md"), "x").unwrap();
        let roots = vec![proj.clone()];

        let codex_only = ToolingFilter { claude: false, cursor: false, codex: true };
        assert!(codex_only.allows_shared());
        for p in [
            paths.home.join(".agents/skills/u/SKILL.md"),
            proj.join(".agents/skills/p/SKILL.md"),
            proj.join("AGENTS.md"),
            proj.join("AGENTS.override.md"),
        ] {
            assert!(paths.check_readable(&p, &roots, &codex_only).is_ok(), "{}", p.display());
        }
        // Claude-owned file stays gated.
        assert!(paths.check_readable(&proj.join("CLAUDE.md"), &roots, &codex_only).is_err());
    }

    #[test]
    fn for_root_isolates_codex_paths() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        assert_eq!(paths.codex_dir, dir.path().join(".codex"));
        assert_eq!(paths.codex_admin_skills, dir.path().join("etc-codex/skills"));
    }

    #[test]
    fn for_root_isolates_cursor_paths() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        assert_eq!(paths.cursor_dir, dir.path().join(".cursor"));
        assert_eq!(paths.cursor_mcp_json, dir.path().join(".cursor/mcp.json"));
        assert_eq!(paths.cursor_hooks_json, dir.path().join(".cursor/hooks.json"));
        assert_eq!(paths.cursor_managed_hooks, dir.path().join("managed-cursor/hooks.json"));
    }

    #[test]
    fn managed_defaults_match_host_os() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                managed_dir_default(),
                PathBuf::from("/Library/Application Support/ClaudeCode")
            );
            assert_eq!(
                cursor_managed_hooks_default(),
                PathBuf::from("/Library/Application Support/Cursor/hooks.json")
            );
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                managed_dir_default(),
                PathBuf::from(r"C:\Program Files\ClaudeCode")
            );
            assert_eq!(
                cursor_managed_hooks_default(),
                PathBuf::from(r"C:\ProgramData\Cursor\hooks.json")
            );
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            assert_eq!(managed_dir_default(), PathBuf::from("/etc/claude-code"));
            assert_eq!(
                cursor_managed_hooks_default(),
                PathBuf::from("/etc/cursor/hooks.json")
            );
        }
    }

    #[test]
    fn skip_dirs() {
        assert!(is_skipped_dir("node_modules"));
        assert!(!is_skipped_dir("src"));
    }

    #[test]
    fn writable_allowlist_subset() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        fs::create_dir_all(paths.config_dir.join("rules")).unwrap();
        fs::create_dir_all(paths.config_dir.join("skills/s")).unwrap();
        fs::create_dir_all(paths.cursor_dir.join("rules")).unwrap();
        fs::create_dir_all(&paths.managed_dir).unwrap();
        fs::create_dir_all(paths.cursor_managed_hooks.parent().unwrap().join("hooks")).unwrap();

        fs::write(paths.config_dir.join("settings.json"), "{}").unwrap();
        fs::write(paths.config_dir.join("rules/a.md"), "x").unwrap();
        fs::write(paths.config_dir.join("skills/s/SKILL.md"), "x").unwrap();
        fs::write(paths.config_dir.join(".credentials.json"), "secret").unwrap();
        fs::write(&paths.claude_json, "{}").unwrap();
        fs::write(paths.cursor_dir.join("mcp.json"), "{}").unwrap();
        fs::write(paths.cursor_dir.join("hooks.json"), "{}").unwrap();
        fs::write(paths.cursor_dir.join("rules/a.mdc"), "x").unwrap();
        fs::write(dir.path().join("outside.txt"), "x").unwrap();
        fs::write(paths.managed_dir.join("settings.json"), "{}").unwrap();
        fs::write(&paths.cursor_managed_hooks, "{}").unwrap();
        fs::write(
            paths.cursor_managed_hooks.parent().unwrap().join("hooks/pre.sh"),
            "echo",
        )
        .unwrap();

        let proj = dir.path().join("proj");
        fs::create_dir_all(proj.join(".claude")).unwrap();
        fs::write(proj.join(".claude/settings.json"), "{}").unwrap();
        fs::write(proj.join("CLAUDE.md"), "x").unwrap();
        let roots = vec![proj.clone()];
        let f = all();

        assert!(paths
            .check_writable(&paths.config_dir.join("settings.json"), &roots, &f)
            .is_ok());
        assert!(paths
            .check_writable(&paths.config_dir.join("rules/a.md"), &roots, &f)
            .is_ok());
        assert!(paths
            .check_writable(&paths.config_dir.join("skills/s/SKILL.md"), &roots, &f)
            .is_ok());
        assert!(paths
            .check_writable(&paths.cursor_dir.join("mcp.json"), &roots, &f)
            .is_ok());
        assert!(paths
            .check_writable(&paths.cursor_dir.join("hooks.json"), &roots, &f)
            .is_ok());
        assert!(paths
            .check_writable(&proj.join(".claude/settings.json"), &roots, &f)
            .is_ok());
        assert!(paths.check_writable(&proj.join("CLAUDE.md"), &roots, &f).is_ok());

        assert!(paths
            .check_writable(&paths.config_dir.join(".credentials.json"), &roots, &f)
            .is_err());
        assert!(paths.check_writable(&paths.claude_json, &roots, &f).is_err());
        assert!(paths
            .check_writable(&dir.path().join("outside.txt"), &roots, &f)
            .is_err());
        assert!(paths
            .check_writable(&paths.managed_dir.join("settings.json"), &roots, &f)
            .is_err());
        assert!(paths
            .check_writable(&paths.cursor_managed_hooks, &roots, &f)
            .is_err());
        assert!(paths
            .check_writable(
                &paths.cursor_managed_hooks.parent().unwrap().join("hooks/pre.sh"),
                &roots,
                &f
            )
            .is_err());
    }

    #[test]
    fn writable_destination_same_dir() {
        let dir = tempdir().unwrap();
        let paths = Paths::for_root(dir.path());
        fs::create_dir_all(paths.config_dir.join("rules")).unwrap();
        fs::write(paths.config_dir.join("rules/a.md"), "x").unwrap();
        fs::write(paths.config_dir.join("rules/exists.md"), "x").unwrap();

        let proj = dir.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("CLAUDE.md"), "x").unwrap();
        let roots = vec![proj.clone()];
        let f = all();

        let rules = paths.config_dir.join("rules");
        assert!(paths
            .check_writable_destination(&rules, "b.md", &roots, &f)
            .is_ok());
        assert!(paths
            .check_writable_destination(&rules, "exists.md", &roots, &f)
            .is_err());
        assert!(paths
            .check_writable_destination(&rules, "../outside.md", &roots, &f)
            .is_err());
        assert!(paths
            .check_writable_destination(&rules, "foo/bar.md", &roots, &f)
            .is_err());
        assert!(paths
            .check_writable_destination(&proj, "FOO.md", &roots, &f)
            .is_err());
        assert!(paths
            .check_writable_destination(&proj, "AGENTS.md", &roots, &f)
            .is_ok());
    }
}
