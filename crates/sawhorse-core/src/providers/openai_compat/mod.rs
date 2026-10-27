//! Mode 3: OpenAI-compatible HTTP client supporting vLLM, jev, and cloud providers.

pub mod complete;
pub mod payload;

use std::collections::HashMap;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};

use crate::config::schema::OpenAiDialect;
use crate::error::SawhorseError;
use crate::types::ExecutionMode;

/// Provider implementation for OpenAI-compatible endpoints.
#[derive(Debug, Clone)]
pub struct OpenAiCompatProvider {
    pub(crate) name: String,
    pub(crate) mode: ExecutionMode,
    pub(crate) base_url: String,
    pub(crate) api_key_env: String,
    pub(crate) model_name: String,
    pub(crate) dialect: OpenAiDialect,
    pub(crate) client: reqwest::Client,
    pub(crate) timeout_ms: u64,
}

impl OpenAiCompatProvider {
    /// Creates a new OpenAI-compatible provider.
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key_env: impl Into<String>,
        model_name: impl Into<String>,
        dialect: OpenAiDialect,
        headers: Option<HashMap<String, String>>,
        timeout_ms: u64,
    ) -> Result<Self, SawhorseError> {
        let b_url = base_url.into().trim_end_matches('/').to_string();
        let key_env = api_key_env.into();
        let m_name = model_name.into();
        let mut header_map = HeaderMap::new();

        for (k, v) in headers.unwrap_or_default() {
            if let Ok(n) = HeaderName::from_bytes(k.as_bytes())
                && let Ok(val) = HeaderValue::from_str(&v)
            {
                header_map.insert(n, val);
            }
        }
        let client = reqwest::Client::builder()
            .default_headers(header_map)
            .build()
            .map_err(|e| SawhorseError::Network {
                endpoint: "client_init".into(),
                message: e.to_string(),
            })?;

        let mode = ExecutionMode::OpenAiCompatible {
            base_url: b_url.clone(),
            api_key_env: key_env.clone(),
            model_name: m_name.clone(),
        };

        Ok(Self {
            name: name.into(),
            mode,
            base_url: b_url,
            api_key_env: key_env,
            model_name: m_name,
            dialect,
            client,
            timeout_ms,
        })
    }

    pub(crate) fn resolve_token(&self) -> Result<String, SawhorseError> {
        if let Ok(key) = std::env::var(&self.api_key_env)
            && !key.trim().is_empty()
        {
            return Ok(key);
        }
        crate::config::resolve_secret_from_env_file(&self.api_key_env).ok_or_else(|| {
            SawhorseError::MissingSecret {
                env_var: self.api_key_env.clone(),
                provider: self.name.clone(),
            }
        })
    }

    pub(crate) async fn post_chat(
        &self,
        payload: &serde_json::Value,
    ) -> Result<reqwest::Response, SawhorseError> {
        let token = self.resolve_token()?;
        let url = format!("{}/chat/completions", self.base_url);
        self.client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(payload)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .send()
            .await
            .map_err(|e| SawhorseError::Network {
                endpoint: url,
                message: e.to_string(),
            })
    }
}
