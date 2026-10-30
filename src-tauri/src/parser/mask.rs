//! Secret masking helpers for JSON shown in the UI.

use serde_json::{Map, Value};

pub const MASK: &str = "••••••";

const SECRET_MARKERS: &[&str] = &[
    "TOKEN",
    "KEY",
    "SECRET",
    "PASSWORD",
    "CREDENTIAL",
    "API_KEY",
    "AUTHORIZATION",
];

/// Object keys whose children are treated as env-like maps.
const ENV_LIKE_KEYS: &[&str] = &["env", "headers", "environment"];

pub fn is_secret_key(key: &str) -> bool {
    let up = key.to_ascii_uppercase();
    SECRET_MARKERS.iter().any(|m| up.contains(m))
}

fn mask_scalar(v: &Value) -> Value {
    match v {
        Value::Null => Value::Null,
        Value::String(s) if s.is_empty() => Value::String(String::new()),
        _ => Value::String(MASK.to_string()),
    }
}

fn mask_env_map(map: &Map<String, Value>) -> Map<String, Value> {
    map.iter()
        .map(|(k, v)| {
            let nv = if is_secret_key(k) && !v.is_object() && !v.is_array() {
                mask_scalar(v)
            } else {
                mask_secrets(v)
            };
            (k.clone(), nv)
        })
        .collect()
}

/// Return a deep copy of `value` with secret-looking entries inside `env` /
/// `headers` objects replaced by `"••••••"`.
pub fn mask_secrets(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let is_env_like = ENV_LIKE_KEYS.contains(&k.as_str());
                    let nv = match (is_env_like, v) {
                        (true, Value::Object(inner)) => Value::Object(mask_env_map(inner)),
                        _ => mask_secrets(v),
                    };
                    (k.clone(), nv)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(mask_secrets).collect()),
        other => other.clone(),
    }
}

/// Mask and pretty-print JSON text. Returns `None` if the text is not valid JSON.
pub fn mask_json_text(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    serde_json::to_string_pretty(&mask_secrets(&v)).ok()
}

/// Regex-based masking for text that is not valid JSON: any `"…secret-ish…": "value"` pair.
pub fn mask_text_lenient(text: &str) -> String {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"(?i)("[^"\n]*(token|key|secret|password|credential|authorization)[^"\n]*"\s*:\s*)"[^"\n]*""#)
            .expect("valid regex")
    });
    re.replace_all(text, format!("${{1}}\"{MASK}\"").as_str()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn masks_secret_env_values() {
        let v = json!({
            "mcpServers": {"s": {"command": "x", "env": {
                "GITHUB_TOKEN": "ghp_abc",
                "MY_API_KEY": "k",
                "DB_PASSWORD": "p",
                "AWS_SECRET_ACCESS_KEY": "s",
                "SERVICE_CREDENTIALS": "c",
                "DEBUG": "1"
            }, "headers": {"Authorization": "Bearer zzz", "X-Trace": "t"}}}
        });
        let m = mask_secrets(&v);
        let env = &m["mcpServers"]["s"]["env"];
        for k in ["GITHUB_TOKEN", "MY_API_KEY", "DB_PASSWORD", "AWS_SECRET_ACCESS_KEY", "SERVICE_CREDENTIALS"] {
            assert_eq!(env[k], MASK, "{k}");
        }
        assert_eq!(env["DEBUG"], "1");
        assert_eq!(m["mcpServers"]["s"]["headers"]["Authorization"], MASK);
        assert_eq!(m["mcpServers"]["s"]["headers"]["X-Trace"], "t");
        assert!(!serde_json::to_string(&m).unwrap().contains("ghp_abc"));
        // original untouched
        assert_eq!(v["mcpServers"]["s"]["env"]["GITHUB_TOKEN"], "ghp_abc");
    }

    #[test]
    fn lenient_masks_broken_json() {
        let t = mask_text_lenient("{ \"env\": { \"MY_TOKEN\": \"abc\", \"A\": \"b\" },, }");
        assert!(t.contains(MASK) && !t.contains("abc") && t.contains("\"b\""));
    }

    #[test]
    fn mask_text_roundtrip() {
        let t = mask_json_text(r#"{"env":{"A_TOKEN":"x"}}"#).unwrap();
        assert!(t.contains(MASK));
        assert!(!t.contains("\"x\""));
        assert!(mask_json_text("{bad").is_none());
    }
}
