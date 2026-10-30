//! YAML frontmatter (between `---` markers) extraction.

use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct Frontmatter {
    /// A leading `---` block exists.
    pub present: bool,
    /// Parsed mapping (always an object; empty if absent/invalid).
    pub data: Value,
    pub raw: Option<String>,
    pub body: String,
    /// Set when the block is unterminated or is not valid YAML mapping.
    pub error: Option<String>,
}

pub fn parse_frontmatter(text: &str) -> Frontmatter {
    let text = text.trim_start_matches('\u{feff}');
    let lines: Vec<&str> = text.lines().collect();
    let empty = || Value::Object(Map::new());

    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return Frontmatter {
            present: false,
            data: empty(),
            raw: None,
            body: text.to_string(),
            error: None,
        };
    }
    let close = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| matches!(l.trim_end(), "---" | "..."))
        .map(|(i, _)| i);
    let Some(close) = close else {
        return Frontmatter {
            present: true,
            data: empty(),
            raw: None,
            body: String::new(),
            error: Some("frontmatter is not terminated by a closing `---`".into()),
        };
    };
    let raw = lines[1..close].join("\n");
    let body = lines[close + 1..].join("\n");

    if raw.trim().is_empty() {
        return Frontmatter { present: true, data: empty(), raw: Some(raw), body, error: None };
    }
    match serde_yaml::from_str::<Value>(&raw) {
        Ok(Value::Object(m)) => Frontmatter {
            present: true,
            data: Value::Object(m),
            raw: Some(raw),
            body,
            error: None,
        },
        Ok(_) => Frontmatter {
            present: true,
            data: lenient(&raw),
            raw: Some(raw),
            body,
            error: Some("frontmatter must be a YAML mapping".into()),
        },
        Err(e) => Frontmatter {
            present: true,
            data: lenient(&raw),
            raw: Some(raw),
            body,
            error: Some(e.to_string()),
        },
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Best-effort `key: value` / `key:\n  - item` reader used when strict YAML fails
/// (e.g. unquoted globs such as `- **/*.ts`, which YAML reads as an alias).
fn lenient(raw: &str) -> Value {
    let mut map = Map::new();
    let mut current_list: Option<(String, Vec<Value>)> = None;
    let flush = |map: &mut Map<String, Value>, cur: &mut Option<(String, Vec<Value>)>| {
        if let Some((k, items)) = cur.take() {
            map.insert(k, Value::Array(items));
        }
    };
    for line in raw.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        let trimmed = line.trim_start();
        if let Some(item) = trimmed.strip_prefix("- ") {
            if let Some((_, items)) = current_list.as_mut() {
                items.push(Value::String(unquote(item)));
            }
            continue;
        }
        if indented {
            continue;
        }
        flush(&mut map, &mut current_list);
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            if k.is_empty() {
                continue;
            }
            let v = v.trim();
            if v.is_empty() {
                current_list = Some((k.to_string(), Vec::new()));
            } else {
                map.insert(k.to_string(), Value::String(unquote(v)));
            }
        }
    }
    flush(&mut map, &mut current_list);
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic() {
        let fm = parse_frontmatter("---\nname: foo\ndescription: Bar baz\n---\n# Body\n");
        assert!(fm.present && fm.error.is_none());
        assert_eq!(fm.data["name"], "foo");
        assert_eq!(fm.data["description"], "Bar baz");
        assert_eq!(fm.body.trim(), "# Body");
    }

    #[test]
    fn absent_frontmatter() {
        let fm = parse_frontmatter("# Just markdown\n---\nnot fm\n");
        assert!(!fm.present && fm.error.is_none());
        assert!(fm.data.as_object().unwrap().is_empty());
    }

    #[test]
    fn unterminated_and_invalid() {
        assert!(parse_frontmatter("---\nname: x\n").error.is_some());
        let fm = parse_frontmatter("---\nname: [unclosed\n---\nbody");
        assert!(fm.error.is_some());
        assert_eq!(fm.body, "body");
    }

    #[test]
    fn crlf_and_bom() {
        let fm = parse_frontmatter("\u{feff}---\r\nname: a\r\n---\r\nx");
        assert_eq!(fm.data["name"], "a");
    }

    #[test]
    fn lenient_globs() {
        let fm = parse_frontmatter("---\npaths:\n  - **/*.ts\n  - \"src/**\"\n---\n");
        assert!(fm.error.is_some());
        assert_eq!(fm.data["paths"], serde_json::json!(["**/*.ts", "src/**"]));
    }
}
