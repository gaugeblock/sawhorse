//! Model discovery for OpenAI-compatible endpoints (`GET {base_url}/models`).
//!
//! The endpoint is the same one `health_check` already calls, which means the path the VS Code
//! extension uses is reachable from here without a new transport. The payload is OpenAI-shaped:
//! `{"object":"list","data":[{"id":"…","owned_by":"…"}, …]}`.

use reqwest::header::AUTHORIZATION;
use serde_json::Value;

use super::OpenAiCompatProvider;
use crate::error::SawhorseError;

/// Extracts model identifiers from a `/models` payload.
///
/// Tolerant on purpose: providers differ in whether they wrap the list (`data` vs `models`), and a
/// catalogue that cannot be parsed should degrade to "nothing advertised" rather than failing a
/// diagnostic command.
#[must_use]
pub fn parse_model_ids(body: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let list = value
        .get("data")
        .or_else(|| value.get("models"))
        .and_then(Value::as_array);

    let mut ids: Vec<String> = list
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| match entry {
                    Value::String(id) => Some(id.clone()),
                    other => other
                        .get("id")
                        .or_else(|| other.get("name"))
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();

    ids.sort();
    ids.dedup();
    ids
}

impl OpenAiCompatProvider {
    /// Asks the endpoint which models it serves.
    ///
    /// # Errors
    ///
    /// [`SawhorseError::MissingSecret`] when the key environment variable is unset, and any
    /// transport or status failure from the endpoint.
    pub async fn fetch_models(&self) -> Result<Vec<String>, SawhorseError> {
        let token = self.resolve_token()?;
        let url = format!("{}/models", self.base_url);
        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .timeout(std::time::Duration::from_secs(self.timeout_ms / 1000))
            .send()
            .await
            .map_err(|error| SawhorseError::ProviderUnavailable {
                provider: self.name.clone(),
                reason: format!("model discovery failed: {error}"),
            })?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(SawhorseError::ProviderUnavailable {
                provider: self.name.clone(),
                reason: format!("model discovery returned {status}"),
            });
        }

        Ok(parse_model_ids(&body))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::parse_model_ids;

    #[test]
    fn test_openai_shaped_payload_yields_sorted_ids() {
        let body = r#"{"object":"list","data":[
            {"id":"deepseek/deepseek-v4.1-flash-1017","object":"model","owned_by":"deepseek"},
            {"id":"openai/gpt-6.1-terra","object":"model","owned_by":"openai"}
        ]}"#;
        assert_eq!(
            parse_model_ids(body),
            vec![
                "deepseek/deepseek-v4.1-flash-1017".to_string(),
                "openai/gpt-6.1-terra".to_string()
            ]
        );
    }

    #[test]
    fn test_alternate_wrappers_and_string_entries_are_accepted() {
        let wrapped = r#"{"models":[{"name":"local/tiny"},{"id":"local/wide"}]}"#;
        assert_eq!(
            parse_model_ids(wrapped),
            vec!["local/tiny".to_string(), "local/wide".to_string()]
        );
        let bare = r#"{"data":["a/one","b/two","a/one"]}"#;
        assert_eq!(
            parse_model_ids(bare),
            vec!["a/one".to_string(), "b/two".to_string()],
            "duplicates collapse"
        );
    }

    #[test]
    fn test_unparsable_body_advertises_nothing() {
        assert!(parse_model_ids("<html>gateway error</html>").is_empty());
        assert!(parse_model_ids("").is_empty());
    }
}
