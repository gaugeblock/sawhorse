//! Optional capture of outbound chat payloads, for diagnosing provider rejections.
//!
//! Point `SAWHORSE_DUMP_REQUEST` at a file or directory and every request body is written
//! before it is sent. Replaying a captured body verbatim is the only way to separate a
//! payload problem from a sizing problem: every sizing hypothesis behind `GB-214`'s
//! empty-content 500 has been refuted by experiment (32768 and 4096 fail alike, all six
//! reasoning efforts answer 200, structured output answers 200), so the next step is to
//! hold the exact bytes rather than guess again.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Environment variable naming the file or directory to capture payloads into.
pub const DUMP_ENV: &str = "SAWHORSE_DUMP_REQUEST";

/// Writes `payload` to the location named by [`DUMP_ENV`], returning the path written.
#[must_use]
pub fn maybe_dump(provider: &str, payload: &serde_json::Value) -> Option<PathBuf> {
    let target = std::env::var_os(DUMP_ENV)?;
    dump_to(std::path::Path::new(&target), provider, payload)
}

/// Writes `payload` under `target`, returning the path written.
///
/// A directory target gets one timestamped file per request; a file target is overwritten.
/// Every failure is swallowed: capturing is a diagnostic aid and must never turn a working
/// request into a failed one.
#[must_use]
pub fn dump_to(
    target: &std::path::Path,
    provider: &str,
    payload: &serde_json::Value,
) -> Option<PathBuf> {
    let mut path = target.to_path_buf();
    if path.is_dir() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        path.push(format!("{provider}-{stamp}.json"));
    }
    let body = serde_json::to_string_pretty(payload).ok()?;
    std::fs::write(&path, body).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::dump_to;

    #[test]
    fn test_absent_variable_captures_nothing() {
        // `SAWHORSE_DUMP_REQUEST` is unset in the test environment; the writer itself is
        // exercised through `dump_to` so no process-wide variable is touched here.
        assert!(super::maybe_dump("p", &serde_json::json!({"a": 1})).is_none());
    }

    #[test]
    fn test_directory_target_gets_one_file_per_request() {
        let dir = std::env::temp_dir().join(format!("sh-dump-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let written = dump_to(&dir, "universal", &serde_json::json!({"model": "m"})).unwrap();
        let body = std::fs::read_to_string(&written).unwrap();
        assert!(body.contains("\"model\": \"m\""), "captured body: {body}");
        assert_eq!(written.parent().unwrap(), dir);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_missing_parent_is_ignored_rather_than_fatal() {
        let path = std::path::Path::new("/nonexistent-sawhorse-dump/x.json");
        assert!(dump_to(path, "p", &serde_json::json!({})).is_none());
    }
}
