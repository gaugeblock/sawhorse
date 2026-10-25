//! File-based credential resolution fallback for `.gaugeblock/.env` and `.env`.

use std::fs;
use std::path::Path;

/// Resolves an environment secret from `.gaugeblock/.env` or `.env` in the current or ancestor directories.
pub fn resolve_secret_from_env_file(key: &str) -> Option<String> {
    let mut cur = std::env::current_dir().ok()?;
    for _ in 0..5 {
        if let Some(val) = check_dir_for_secret(&cur, key) {
            return Some(val);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

/// Checks a specific directory for `.gaugeblock/.env` or `.env`.
pub fn check_dir_for_secret(dir: &Path, key: &str) -> Option<String> {
    let gb_env = dir.join(".gaugeblock").join(".env");
    if let Ok(content) = fs::read_to_string(&gb_env)
        && let Some(val) = parse_env_key(&content, key)
    {
        return Some(val);
    }

    let dot_env = dir.join(".env");
    if let Ok(content) = fs::read_to_string(&dot_env)
        && let Some(val) = parse_env_key(&content, key)
    {
        return Some(val);
    }

    None
}

/// Extracts a key's value from dotenv-style file content.
pub fn parse_env_key(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=')
            && k.trim() == key
        {
            let clean = v.trim().trim_matches('"').trim_matches('\'');
            if !clean.is_empty() {
                return Some(clean.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_key() {
        let content = r#"
# Comment
CLINE_API_KEY="sk-cline-12345"
ANOTHER_KEY=unquoted_value
EMPTY_KEY=""
"#;
        assert_eq!(
            parse_env_key(content, "CLINE_API_KEY"),
            Some("sk-cline-12345".to_string())
        );
        assert_eq!(
            parse_env_key(content, "ANOTHER_KEY"),
            Some("unquoted_value".to_string())
        );
        assert_eq!(parse_env_key(content, "EMPTY_KEY"), None);
        assert_eq!(parse_env_key(content, "MISSING_KEY"), None);
    }
}
