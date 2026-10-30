//! Resolution of Claude Code / Cursor config locations and the read allowlist.

use crate::errors::{AppError, AppResult};
use std::path::{Component, Path, PathBuf};

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
    /// Managed Cursor hooks (macOS `/Library/Application Support/Cursor/hooks.json`).
    pub cursor_managed_hooks: PathBuf,
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
        PathBuf::from("C:\\Program Files\\Cursor\\hooks.json")
    } else {
        PathBuf::from("/etc/cursor/hooks.json")
    }
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
        Self {
            home,
            config_dir,
            claude_json,
            managed_dir: managed_dir_default(),
            cursor_mcp_json: cursor_dir.join("mcp.json"),
            cursor_hooks_json: cursor_dir.join("hooks.json"),
            cursor_dir,
            cursor_managed_hooks: cursor_managed_hooks_default(),
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
        }
    }

    pub fn credentials_file(&self) -> PathBuf {
        self.config_dir.join(".credentials.json")
    }

    /// Check that `requested` may be served to the UI.
    /// Returns the canonical path. `.claude.json` is allowed here but callers must
    /// serve a redacted view (see `is_claude_json`).
    pub fn check_readable(&self, requested: &Path, project_roots: &[PathBuf]) -> AppResult<PathBuf> {
        let canon = dunce::canonicalize(requested)
            .map_err(|_| AppError::NotFound(requested.display().to_string()))?;
        if !canon.is_file() {
            return Err(AppError::InvalidPath(format!("{} is not a file", canon.display())));
        }
        if canon.file_name().map(|n| n == ".credentials.json").unwrap_or(false) {
            return Err(AppError::AccessDenied("credentials are never displayed".into()));
        }
        if self.is_claude_json(&canon) {
            return Ok(canon);
        }

        let canon_of = |p: &Path| dunce::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());

        let cfg = canon_of(&self.config_dir);
        if let Ok(rel) = canon.strip_prefix(&cfg) {
            let comps: Vec<String> = rel
                .components()
                .filter_map(|c| match c {
                    Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect();
            if let Some(first) = comps.first() {
                if CONFIG_DIR_ALLOWED.contains(&first.as_str()) {
                    if first == "projects" && !comps.iter().skip(1).any(|c| c == "memory") {
                        return Err(AppError::AccessDenied(
                            "only project memory may be read from projects/".into(),
                        ));
                    }
                    return Ok(canon);
                }
            }
            return Err(AppError::AccessDenied(format!(
                "{} is not a known Claude Code config file",
                canon.display()
            )));
        }

        let cursor = canon_of(&self.cursor_dir);
        if let Ok(rel) = canon.strip_prefix(&cursor) {
            let comps: Vec<String> = rel
                .components()
                .filter_map(|c| match c {
                    Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect();
            if let Some(first) = comps.first() {
                if CURSOR_DIR_ALLOWED.contains(&first.as_str()) {
                    return Ok(canon);
                }
            }
            return Err(AppError::AccessDenied(format!(
                "{} is not a known Cursor config file",
                canon.display()
            )));
        }

        let managed = canon_of(&self.managed_dir);
        if canon.starts_with(&managed) {
            return Ok(canon);
        }

        let cursor_managed = canon_of(&self.cursor_managed_hooks);
        if canon == cursor_managed {
            return Ok(canon);
        }
        // Allow adjacent managed Cursor hook scripts under the same parent dir.
        if let Some(parent) = self.cursor_managed_hooks.parent() {
            let parent = canon_of(parent);
            if let Ok(rel) = canon.strip_prefix(&parent) {
                let first = rel.components().next().and_then(|c| match c {
                    Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                    _ => None,
                });
                if first.as_deref() == Some("hooks") || canon == cursor_managed {
                    return Ok(canon);
                }
            }
        }

        for root in project_roots {
            let root = canon_of(root);
            if let Ok(rel) = canon.strip_prefix(&root) {
                let comps: Vec<String> = rel
                    .components()
                    .filter_map(|c| match c {
                        Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                        _ => None,
                    })
                    .collect();
                let first = comps.first().map(|s| s.as_str());
                let name = canon.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let under_claude = first == Some(".claude");
                let under_cursor = first == Some(".cursor");
                let under_agents_skills =
                    first == Some(".agents") && comps.get(1).map(|s| s.as_str()) == Some("skills");
                if under_claude
                    || under_cursor
                    || under_agents_skills
                    || PROJECT_FILE_NAMES.contains(&name.as_str())
                {
                    return Ok(canon);
                }
            }
        }
        Err(AppError::AccessDenied(format!(
            "{} is outside known Claude Code / Cursor locations",
            canon.display()
        )))
    }

    pub fn is_claude_json(&self, canon: &Path) -> bool {
        dunce::canonicalize(&self.claude_json)
            .map(|c| c == canon)
            .unwrap_or(false)
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

        assert!(paths.check_readable(&paths.config_dir.join("settings.json"), &roots).is_ok());
        assert!(paths.check_readable(&paths.config_dir.join("rules/a.md"), &roots).is_ok());
        assert!(paths.check_readable(&paths.config_dir.join("projects/x/memory/MEMORY.md"), &roots).is_ok());
        assert!(paths.check_readable(&paths.config_dir.join("projects/x/transcript.jsonl"), &roots).is_err());
        assert!(paths.check_readable(&paths.config_dir.join(".credentials.json"), &roots).is_err());
        assert!(paths.check_readable(&dir.path().join("outside.txt"), &roots).is_err());
        assert!(paths.check_readable(&proj.join("CLAUDE.md"), &roots).is_ok());
        assert!(paths.check_readable(&proj.join(".claude/settings.json"), &roots).is_ok());
        assert!(paths.check_readable(&proj.join(".env"), &roots).is_err());
        assert!(paths.check_readable(&dir.path().join("nope"), &roots).is_err());
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

        assert!(paths.check_readable(&paths.cursor_dir.join("mcp.json"), &roots).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("hooks.json"), &roots).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("rules/a.mdc"), &roots).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("skills/s.md"), &roots).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join("hooks/pre.sh"), &roots).is_ok());
        assert!(paths.check_readable(&paths.cursor_dir.join(".credentials.json"), &roots).is_err());
        assert!(paths.check_readable(&paths.cursor_dir.join("secrets.txt"), &roots).is_err());

        assert!(paths.check_readable(&proj.join(".cursor/mcp.json"), &roots).is_ok());
        assert!(paths.check_readable(&proj.join(".cursor/rules/r.mdc"), &roots).is_ok());
        assert!(paths.check_readable(&proj.join("AGENTS.md"), &roots).is_ok());
        assert!(paths.check_readable(&proj.join(".agents/skills/foo/SKILL.md"), &roots).is_ok());
        assert!(paths.check_readable(&proj.join(".env"), &roots).is_err());
        assert!(paths.check_readable(&proj.join(".credentials.json"), &roots).is_err());
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
    fn skip_dirs() {
        assert!(is_skipped_dir("node_modules"));
        assert!(!is_skipped_dir("src"));
    }
}
