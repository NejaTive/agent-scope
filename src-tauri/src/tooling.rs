//! AI tooling providers (Claude Code, Cursor, Codex) and preference-based filtering.
//!
//! Add a new product by extending `Tooling`, `TOOLING_PROVIDERS`, `ToolingFilter`,
//! prefs (`track_*`), presence probe, and a scanner module — avoid copy-pasting
//! filter/validate logic per product.

use crate::errors::{AppError, AppResult};
use crate::models::{Preferences, Tooling};
use crate::paths::Paths;
use serde::Serialize;

/// Stable id → display name for known AI tooling products.
/// Frontend mirror: `src/lib/tooling.ts` (`TOOLING_PROVIDERS`).
pub const TOOLING_PROVIDERS: &[(&str, &str)] = &[
    ("claude", "Claude Code"),
    ("cursor", "Cursor"),
    ("codex", "Codex"),
];

// Future stubs (no scanner yet): windsurf, continue, gemini-cli — see docs/spec.

/// Which products to scan, watch, and allow-list for I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolingFilter {
    pub claude: bool,
    pub cursor: bool,
    pub codex: bool,
}

impl Default for ToolingFilter {
    fn default() -> Self {
        Self::all()
    }
}

impl ToolingFilter {
    pub fn all() -> Self {
        Self {
            claude: true,
            cursor: true,
            codex: true,
        }
    }

    pub fn from_prefs(p: &Preferences) -> Self {
        Self {
            claude: p.track_claude,
            cursor: p.track_cursor,
            codex: p.track_codex,
        }
    }

    pub fn allows(&self, product: Tooling) -> bool {
        match product {
            Tooling::Claude => self.claude,
            Tooling::Cursor => self.cursor,
            Tooling::Codex => self.codex,
        }
    }

    /// Any tracked product that uses shared paths (`AGENTS.md`, `.agents/skills`).
    pub fn allows_shared(&self) -> bool {
        self.claude || self.cursor || self.codex
    }

    /// At least one AI tooling product must stay enabled.
    pub fn validate_prefs(p: &Preferences) -> AppResult<()> {
        if !p.track_claude && !p.track_cursor && !p.track_codex {
            return Err(AppError::Other(
                "Нужно оставить включённым хотя бы один AI-инструмент".into(),
            ));
        }
        Ok(())
    }
}

/// Whether typical on-disk roots for each product exist (for UI hints).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolingPresence {
    /// `~/.claude` directory or `~/.claude.json` exists.
    pub claude: bool,
    /// `~/.cursor` directory exists.
    pub cursor: bool,
    /// `CODEX_HOME` / `~/.codex` directory exists.
    pub codex: bool,
}

impl ToolingPresence {
    pub fn probe(paths: &Paths) -> Self {
        Self {
            claude: paths.config_dir.is_dir() || paths.claude_json.exists(),
            cursor: paths.cursor_dir.is_dir(),
            codex: paths.codex_dir.is_dir(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_one() {
        let mut p = Preferences::default();
        assert!(p.track_claude && p.track_cursor && p.track_codex);
        ToolingFilter::validate_prefs(&p).unwrap();
        p.track_claude = false;
        p.track_cursor = false;
        ToolingFilter::validate_prefs(&p).unwrap();
        p.track_codex = false;
        let err = ToolingFilter::validate_prefs(&p).unwrap_err();
        assert!(err.to_string().contains("хотя бы один"));
    }

    #[test]
    fn allows_matches_flags() {
        let f = ToolingFilter {
            claude: true,
            cursor: false,
            codex: true,
        };
        assert!(f.allows(Tooling::Claude));
        assert!(!f.allows(Tooling::Cursor));
        assert!(f.allows(Tooling::Codex));
        assert!(f.allows_shared());
    }
}
