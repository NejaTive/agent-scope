//! AI tooling providers (Claude Code, Cursor) and preference-based filtering.

use crate::errors::{AppError, AppResult};
use crate::models::{Preferences, Tooling};
use crate::paths::Paths;
use serde::Serialize;

/// Stable id → display name for known AI tooling products.
pub const TOOLING_PROVIDERS: &[(&str, &str)] = &[("claude", "Claude Code"), ("cursor", "Cursor")];

/// Which products to scan, watch, and allow-list for I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolingFilter {
    pub claude: bool,
    pub cursor: bool,
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
        }
    }

    pub fn from_prefs(p: &Preferences) -> Self {
        Self {
            claude: p.track_claude,
            cursor: p.track_cursor,
        }
    }

    pub fn allows(&self, product: Tooling) -> bool {
        match product {
            Tooling::Claude => self.claude,
            Tooling::Cursor => self.cursor,
        }
    }

    /// At least one AI tooling product must stay enabled.
    pub fn validate_prefs(p: &Preferences) -> AppResult<()> {
        if !p.track_claude && !p.track_cursor {
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
}

impl ToolingPresence {
    pub fn probe(paths: &Paths) -> Self {
        Self {
            claude: paths.config_dir.is_dir() || paths.claude_json.exists(),
            cursor: paths.cursor_dir.is_dir(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_one() {
        let mut p = Preferences::default();
        assert!(p.track_claude && p.track_cursor);
        ToolingFilter::validate_prefs(&p).unwrap();
        p.track_claude = false;
        ToolingFilter::validate_prefs(&p).unwrap();
        p.track_cursor = false;
        let err = ToolingFilter::validate_prefs(&p).unwrap_err();
        assert!(err.to_string().contains("хотя бы один"));
    }

    #[test]
    fn allows_matches_flags() {
        let f = ToolingFilter {
            claude: true,
            cursor: false,
        };
        assert!(f.allows(Tooling::Claude));
        assert!(!f.allows(Tooling::Cursor));
    }
}
