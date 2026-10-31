//! Check GitHub Releases for a newer app version.
//!
//! Uses `GET /repos/{owner}/{repo}/releases/latest`, which already returns the
//! latest non-draft, non-prerelease release — no client-side draft/prerelease
//! filtering needed.

use crate::models::UpdateCheckResult;
use serde::Deserialize;

/// Single source of truth for the public GitHub repo used for update checks.
pub const GITHUB_OWNER: &str = "NejaTive";
pub const GITHUB_REPO: &str = "agent-scope";

fn releases_latest_url() -> String {
    format!(
        "https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest"
    )
}

const MAX_NOTES_CHARS: usize = 2000;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

/// Parse `major.minor.patch` after stripping an optional leading `v`/`V`.
/// Extra pre-release/build suffixes after the patch (e.g. `-beta.1`) are ignored
/// for comparison; only the three numeric components are used.
pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let s = raw.trim();
    let s = s
        .strip_prefix('v')
        .or_else(|| s.strip_prefix('V'))
        .unwrap_or(s);
    let core = s.split(['-', '+']).next().unwrap_or(s);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Compare two semver strings numerically after normalizing optional `v` prefix.
/// Returns `Some(Ordering)` when both parse, else `None`.
pub fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let left = parse_semver(a)?;
    let right = parse_semver(b)?;
    Some(left.cmp(&right))
}

fn truncate_notes(body: Option<&str>) -> Option<String> {
    let text = body?.trim();
    if text.is_empty() {
        return None;
    }
    if text.chars().count() <= MAX_NOTES_CHARS {
        return Some(text.to_string());
    }
    let truncated: String = text.chars().take(MAX_NOTES_CHARS).collect();
    Some(format!("{truncated}…"))
}

fn soft_error(current_version: &str, message: impl Into<String>) -> UpdateCheckResult {
    UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version: None,
        update_available: false,
        release_url: None,
        release_notes: None,
        checked_at: chrono::Utc::now().to_rfc3339(),
        error: Some(message.into()),
    }
}

/// Query GitHub Releases and compare against `current_version`.
/// Soft failures (network, rate limit, bad JSON) return `Ok` with `error` set.
pub fn check_for_updates(current_version: &str) -> UpdateCheckResult {
    let checked_at = chrono::Utc::now().to_rfc3339();

    let client = match reqwest::blocking::Client::builder()
        .user_agent(format!("Agent-Scope/{current_version}"))
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => return soft_error(current_version, format!("failed to build HTTP client: {e}")),
    };

    let response = match client
        .get(releases_latest_url())
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            return soft_error(
                current_version,
                format!("network error checking for updates: {e}"),
            );
        }
    };

    let status = response.status();
    if status.as_u16() == 403 {
        return soft_error(
            current_version,
            "GitHub API returned 403 Forbidden (rate limited or blocked)",
        );
    }
    if status.as_u16() == 429 {
        return soft_error(
            current_version,
            "GitHub API rate limit exceeded (429); try again later",
        );
    }
    if status.as_u16() == 404 {
        return soft_error(
            current_version,
            "no GitHub releases found for this repository",
        );
    }
    if !status.is_success() {
        return soft_error(
            current_version,
            format!("GitHub API returned HTTP {status}"),
        );
    }

    let release: GithubRelease = match response.json() {
        Ok(r) => r,
        Err(e) => {
            return soft_error(
                current_version,
                format!("invalid GitHub release JSON: {e}"),
            );
        }
    };

    let latest_version = release.tag_name.trim().to_string();
    let update_available = match compare_semver(current_version, &latest_version) {
        Some(std::cmp::Ordering::Less) => true,
        Some(_) => false,
        None => {
            return UpdateCheckResult {
                current_version: current_version.to_string(),
                latest_version: Some(latest_version),
                update_available: false,
                release_url: Some(release.html_url),
                release_notes: truncate_notes(release.body.as_deref()),
                checked_at,
                error: Some("could not compare versions (invalid semver)".into()),
            };
        }
    };

    UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version: Some(latest_version),
        update_available,
        release_url: Some(release.html_url),
        release_notes: truncate_notes(release.body.as_deref()),
        checked_at,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_less_than() {
        assert_eq!(
            compare_semver("1.0.0", "1.0.1"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare_semver("0.3.0", "0.4.0"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare_semver("1.9.0", "1.10.0"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn semver_v_prefix() {
        assert_eq!(parse_semver("v1.0.1"), Some((1, 0, 1)));
        assert_eq!(parse_semver("V0.3.0"), Some((0, 3, 0)));
        assert_eq!(
            compare_semver("0.3.0", "v0.3.1"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare_semver("v1.0.0", "1.0.0"),
            Some(std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn semver_equal() {
        assert_eq!(
            compare_semver("1.0.0", "1.0.0"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_semver("v0.3.0", "0.3.0"),
            Some(std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn update_available_logic() {
        assert!(compare_semver("0.3.0", "0.3.1") == Some(std::cmp::Ordering::Less));
        assert!(compare_semver("0.3.0", "0.3.0") != Some(std::cmp::Ordering::Less));
        assert!(compare_semver("0.4.0", "0.3.0") != Some(std::cmp::Ordering::Less));
    }

    #[test]
    fn invalid_semver_returns_none() {
        assert!(parse_semver("").is_none());
        assert!(parse_semver("abc").is_none());
        assert!(parse_semver("1.0").is_none());
        assert!(compare_semver("1.0.0", "not-a-version").is_none());
    }

    #[test]
    fn truncate_notes_empty_and_long() {
        assert!(truncate_notes(None).is_none());
        assert!(truncate_notes(Some("   ")).is_none());
        assert_eq!(truncate_notes(Some("short")).as_deref(), Some("short"));
        let long: String = "x".repeat(MAX_NOTES_CHARS + 50);
        let out = truncate_notes(Some(&long)).unwrap();
        assert!(out.ends_with('…'));
        assert_eq!(out.chars().count(), MAX_NOTES_CHARS + 1);
    }

    #[test]
    fn repo_constants() {
        assert_eq!(GITHUB_OWNER, "NejaTive");
        assert_eq!(GITHUB_REPO, "agent-scope");
        let url = releases_latest_url();
        assert!(url.contains(GITHUB_OWNER));
        assert!(url.contains(GITHUB_REPO));
        assert!(url.ends_with("/releases/latest"));
    }
}
