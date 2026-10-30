//! Read-only parsers for Claude Code config files.

pub mod frontmatter;
pub mod mask;

pub use frontmatter::{parse_frontmatter, Frontmatter};
pub use mask::{is_secret_key, mask_json_text, mask_secrets, mask_text_lenient, MASK};

use crate::models::DiscoveryStatus;
use serde_json::Value;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

// ---------------------------------------------------------------- JSON

#[derive(Debug, Clone)]
pub struct JsonFile {
    pub status: DiscoveryStatus,
    pub value: Option<Value>,
    pub error: Option<String>,
}

/// Strict JSON parse (no comments / trailing commas) with error capture.
pub fn parse_json_str(text: &str) -> Result<Value, String> {
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())
}

pub fn read_json_file(path: &Path) -> JsonFile {
    match fs::read_to_string(path) {
        Ok(text) => match parse_json_str(&text) {
            Ok(v) => JsonFile { status: DiscoveryStatus::Present, value: Some(v), error: None },
            Err(e) => JsonFile { status: DiscoveryStatus::InvalidJson, value: None, error: Some(e) },
        },
        Err(e) if e.kind() == ErrorKind::NotFound => {
            JsonFile { status: DiscoveryStatus::Missing, value: None, error: None }
        }
        Err(e) => JsonFile {
            status: DiscoveryStatus::Unreadable,
            value: None,
            error: Some(e.to_string()),
        },
    }
}

// ---------------------------------------------------------------- helpers

fn str_field(fm: &Value, key: &str) -> Option<String> {
    match fm.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    }
}

fn string_list(v: Option<&Value>, split_commas: bool) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::String(s)) if split_commas => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => Vec::new(),
    }
}

/// First non-empty, non-heading line of a markdown body, truncated.
pub fn first_paragraph_line(body: &str) -> Option<String> {
    body.lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("```"))
        .map(|l| {
            let mut s: String = l.chars().take(240).collect();
            if l.chars().count() > 240 {
                s.push('…');
            }
            s
        })
}

// ---------------------------------------------------------------- skill

#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    pub frontmatter: Value,
    pub status: DiscoveryStatus,
    pub error: Option<String>,
}

/// Skills, commands and output styles: frontmatter is optional.
pub fn parse_skill(text: &str) -> SkillMeta {
    let fm = parse_frontmatter(text);
    let description = str_field(&fm.data, "description").or_else(|| first_paragraph_line(&fm.body));
    SkillMeta {
        name: str_field(&fm.data, "name"),
        description,
        status: if fm.error.is_some() {
            DiscoveryStatus::InvalidFrontmatter
        } else {
            DiscoveryStatus::Present
        },
        error: fm.error,
        frontmatter: fm.data,
    }
}

// ---------------------------------------------------------------- rule

#[derive(Debug, Clone)]
pub struct RuleMeta {
    pub paths: Vec<String>,
    pub status: DiscoveryStatus,
    pub error: Option<String>,
}

/// Rules: only the `paths` frontmatter field matters.
pub fn parse_rule(text: &str) -> RuleMeta {
    let fm = parse_frontmatter(text);
    let paths = string_list(fm.data.get("paths"), false);
    // A lenient recovery that still yielded `paths` is good enough for a rule.
    let status = if fm.error.is_some() && paths.is_empty() {
        DiscoveryStatus::InvalidFrontmatter
    } else {
        DiscoveryStatus::Present
    };
    RuleMeta { paths, status, error: if status == DiscoveryStatus::Present { None } else { fm.error } }
}

// ---------------------------------------------------------------- agent

#[derive(Debug, Clone)]
pub struct AgentMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tools: Vec<String>,
    pub model: Option<String>,
    /// name AND description present
    pub valid: bool,
    pub frontmatter: Value,
    pub status: DiscoveryStatus,
    pub error: Option<String>,
}

pub fn parse_agent(text: &str) -> AgentMeta {
    let fm = parse_frontmatter(text);
    let name = str_field(&fm.data, "name");
    let description = str_field(&fm.data, "description");
    let valid = name.is_some() && description.is_some();
    let (status, error) = if let Some(e) = fm.error.clone() {
        (DiscoveryStatus::InvalidFrontmatter, Some(e))
    } else if !valid {
        (
            DiscoveryStatus::Partial,
            Some(if !fm.present {
                "agent has no frontmatter; `name` and `description` are required".to_string()
            } else {
                "agent frontmatter needs both `name` and `description`".to_string()
            }),
        )
    } else {
        (DiscoveryStatus::Present, None)
    };
    AgentMeta {
        tools: string_list(fm.data.get("tools"), true),
        model: str_field(&fm.data, "model"),
        name,
        description,
        valid,
        frontmatter: fm.data,
        status,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn json_missing_invalid_valid() {
        let d = tempdir().unwrap();
        let missing = read_json_file(&d.path().join("nope.json"));
        assert_eq!(missing.status, DiscoveryStatus::Missing);

        let bad = d.path().join("bad.json");
        fs::write(&bad, "{ \"a\": 1,, }").unwrap();
        let r = read_json_file(&bad);
        assert_eq!(r.status, DiscoveryStatus::InvalidJson);
        assert!(r.error.is_some() && r.value.is_none());

        // strict: trailing comma and comments rejected
        assert!(parse_json_str("{\"a\":1,}").is_err());
        assert!(parse_json_str("// c\n{}").is_err());

        let ok = d.path().join("ok.json");
        fs::write(&ok, "\u{feff}{\"a\":1}").unwrap();
        assert_eq!(read_json_file(&ok).value.unwrap()["a"], 1);
    }

    #[test]
    fn skill_optional_frontmatter() {
        let s = parse_skill("# Title\n\nDoes a thing.\n");
        assert_eq!(s.status, DiscoveryStatus::Present);
        assert_eq!(s.description.as_deref(), Some("Does a thing."));
        assert!(s.name.is_none());

        let s = parse_skill("---\nname: pdf\ndescription: Make PDFs\nallowed-tools: Read\n---\nbody");
        assert_eq!(s.name.as_deref(), Some("pdf"));
        assert_eq!(s.description.as_deref(), Some("Make PDFs"));
        assert_eq!(s.frontmatter["allowed-tools"], "Read");

        let s = parse_skill("---\nname: [oops\n---\nbody");
        assert_eq!(s.status, DiscoveryStatus::InvalidFrontmatter);
    }

    #[test]
    fn rule_paths() {
        let r = parse_rule("---\npaths:\n  - \"src/**/*.ts\"\n  - lib/**\n---\nrules");
        assert_eq!(r.paths, vec!["src/**/*.ts", "lib/**"]);
        assert_eq!(parse_rule("no frontmatter").paths.len(), 0);
        let r = parse_rule("---\npaths: \"src/**\"\n---\n");
        assert_eq!(r.paths, vec!["src/**"]);
    }

    #[test]
    fn agent_validity() {
        let a = parse_agent("---\nname: rev\ndescription: Reviews\ntools: Read, Grep\nmodel: sonnet\n---\nprompt");
        assert!(a.valid);
        assert_eq!(a.tools, vec!["Read", "Grep"]);
        assert_eq!(a.model.as_deref(), Some("sonnet"));
        let a = parse_agent("---\nname: rev\n---\nprompt");
        assert!(!a.valid);
        assert_eq!(a.status, DiscoveryStatus::Partial);
        let a = parse_agent("just text");
        assert!(!a.valid);
    }
}
