//! Resolution of Claude Code config locations and the read allowlist.

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

/// Top-level entries of the user config dir that may be read.
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

#[derive(Debug, Clone)]
pub struct Paths {
    pub home: PathBuf,
    /// `CLAUDE_CONFIG_DIR` or `~/.claude`
    pub config_dir: PathBuf,
    /// `~/.claude.json` (inside `CLAUDE_CONFIG_DIR` when that is set)
    pub claude_json: PathBuf,
    pub managed_dir: PathBuf,
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
        Self {
            home,
            config_dir,
            claude_json,
            managed_dir: managed_dir_default(),
        }
    }

    /// Fully sandboxed layout under `root` (used by tests; never touches real dirs).
    pub fn for_root(root: &Path) -> Self {
        Self {
            home: root.to_path_buf(),
            config_dir: root.join(".claude"),
            claude_json: root.join(".claude.json"),
            managed_dir: root.join("managed"),
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

        let managed = canon_of(&self.managed_dir);
        if canon.starts_with(&managed) {
            return Ok(canon);
        }

        for root in project_roots {
            let root = canon_of(root);
            if let Ok(rel) = canon.strip_prefix(&root) {
                let first = rel.components().next().and_then(|c| match c {
                    Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                    _ => None,
                });
                let name = canon.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let under_claude = first.as_deref() == Some(".claude");
                if under_claude || PROJECT_FILE_NAMES.contains(&name.as_str()) {
                    return Ok(canon);
                }
            }
        }
        Err(AppError::AccessDenied(format!(
            "{} is outside known Claude Code locations",
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
    fn skip_dirs() {
        assert!(is_skipped_dir("node_modules"));
        assert!(!is_skipped_dir("src"));
    }
}
