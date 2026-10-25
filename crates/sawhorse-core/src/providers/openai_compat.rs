//! Mode 3: OpenAI-compatible HTTP client supporting vLLM, jev, and cloud providers.

use std::collections::HashMap;
use std::time::Instant;

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::config::schema::OpenAiDialect;
use crate::error::SawhorseError;
use crate::traits::{ModelProvider, TokenStream};
use crate::types::{ExecutionMode, GrammarFormat, InferenceRequest, InferenceResponse};

/// Provider implementation for OpenAI-compatible endpoints.
#[derive(Debug, Clone)]
pub struct OpenAiCompatProvider {
    name: String,
    mode: ExecutionMode,
    base_url: String,
    api_key_env: String,
    model_name: String,
    dialect: OpenAiDialect,
    client: reqwest::Client,
    timeout_ms: u64,
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

    fn resolve_token(&self) -> Result<String, SawhorseError> {
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

    fn build_payload(&self, req: &InferenceRequest, stream: bool) -> Value {
        let mut messages = Vec::new();
        if let Some(ref sys) = req.system_prompt {
            messages.push(json!({"role": "system", "content": sys}));
        }
        messages.push(json!({"role": "user", "content": &req.prompt}));

        let mut body = json!({
            "model": &self.model_name,
            "messages": messages,
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "stream": stream,
        });

        if let Some(ref grammar) = req.grammar_constraint {
            match (self.dialect, req.grammar_format) {
                (OpenAiDialect::Vllm, Some(GrammarFormat::Regex)) => {
                    body["guided_regex"] = json!(grammar)
                }
                (OpenAiDialect::Vllm, _) => body["guided_grammar"] = json!(grammar),
                (OpenAiDialect::LlamaCpp, _) => body["grammar"] = json!(grammar),
                (OpenAiDialect::StandardOpenAi, _) => {
                    body["extra_body"] = json!({ "grammar": grammar })
                }
            }
        }
        body
    }

    async fn post_chat(&self, payload: &Value) -> Result<reqwest::Response, SawhorseError> {
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

fn extract_chat_response(body: &Value) -> (String, u32, u32) {
    let d = &body["data"];
    let text = body["choices"][0]["message"]["content"]
        .as_str()
        .or_else(|| d["choices"][0]["message"]["content"].as_str())
        .unwrap_or_default()
        .to_string();
    let get_u32 = |k: &str| {
        body["usage"][k]
            .as_u64()
            .or_else(|| d["usage"][k].as_u64())
            .unwrap_or(0) as u32
    };
    (text, get_u32("prompt_tokens"), get_u32("completion_tokens"))
}

#[async_trait]
impl ModelProvider for OpenAiCompatProvider {
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, SawhorseError> {
        let url = format!("{}/chat/completions", self.base_url);
        let payload = self.build_payload(req, false);
        let start = Instant::now();
        debug!(provider = %self.name, url = %url, "Mode 3 completion");

        let resp = self.post_chat(&payload).await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(SawhorseError::Http {
                endpoint: url,
                status,
                message,
            });
        }

        let body: Value = resp.json().await.map_err(|e| SawhorseError::Network {
            endpoint: url,
            message: format!("Malformed JSON: {e}"),
        })?;

        let (text, prompt_tokens, completion_tokens) = extract_chat_response(&body);
        Ok(InferenceResponse {
            text,
            prompt_tokens,
            completion_tokens,
            latency_ms: start.elapsed().as_millis() as u64,
            model_used: self.model_name.clone(),
        })
    }

    async fn complete_stream(&self, req: &InferenceRequest) -> Result<TokenStream, SawhorseError> {
        let payload = self.build_payload(req, true);
        let resp = self.post_chat(&payload).await?;
        let byte_stream = resp.bytes_stream();
        let token_stream = byte_stream.map(|chunk_res| {
            let chunk = chunk_res.map_err(|e| SawhorseError::Network {
                endpoint: "stream".into(),
                message: e.to_string(),
            })?;
            let text = String::from_utf8_lossy(&chunk);
            let mut extracted = String::new();
            for line in text.lines() {
                if let Some(data) = line.trim().strip_prefix("data: ")
                    && data != "[DONE]"
                    && let Ok(v) = serde_json::from_str::<Value>(data)
                    && let Some(d) = v["choices"][0]["delta"]["content"]
                        .as_str()
                        .or_else(|| v["data"]["choices"][0]["delta"]["content"].as_str())
                {
                    extracted.push_str(d);
                }
            }
            Ok(extracted)
        });

        Ok(Box::pin(token_stream))
    }

    async fn health_check(&self) -> Result<bool, SawhorseError> {
        let token = self.resolve_token()?;
        let url = format!("{}/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => Ok(true),
            Ok(r) => {
                warn!(status = %r.status(), "Health check returned non-200");
                Ok(false)
            }
            Err(e) => {
                debug!(error = %e, "Health check failed to connect");
                Ok(false)
            }
        }
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn execution_mode(&self) -> &ExecutionMode {
        &self.mode
    }
}
