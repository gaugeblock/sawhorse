//! ModelProvider implementation for OpenAI-compatible HTTP endpoints.

use std::time::Instant;

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::AUTHORIZATION;
use serde_json::Value;
use tracing::{debug, warn};

use super::OpenAiCompatProvider;
use super::payload::{build_payload, extract_chat_response, format_http_error};
use crate::error::SawhorseError;
use crate::traits::{ModelProvider, TokenStream};
use crate::types::{ExecutionMode, InferenceRequest, InferenceResponse};

#[async_trait]
impl ModelProvider for OpenAiCompatProvider {
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, SawhorseError> {
        let url = format!("{}/chat/completions", self.base_url);
        let payload = build_payload(self.dialect, &self.model_name, req, false);
        let start = Instant::now();

        debug!(
            provider = %self.name,
            url = %url,
            model = %self.model_name,
            prompt_chars = req.prompt.len(),
            max_tokens = req.max_tokens,
            temperature = req.temperature,
            role = ?req.role,
            "Mode 3 OpenAI-compatible completion request dispatched"
        );

        let resp = self.post_chat(&payload).await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let raw_message = resp.text().await.unwrap_or_default();
            return Err(format_http_error(
                &url,
                status,
                &self.model_name,
                req.max_tokens,
                &raw_message,
            ));
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
        let payload = build_payload(self.dialect, &self.model_name, req, true);
        debug!(
            provider = %self.name,
            model = %self.model_name,
            prompt_chars = req.prompt.len(),
            max_tokens = req.max_tokens,
            "Mode 3 OpenAI-compatible streaming completion dispatched"
        );

        let resp = self.post_chat(&payload).await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let raw_message = resp.text().await.unwrap_or_default();
            return Err(format_http_error(
                &format!("{}/chat/completions", self.base_url),
                status,
                &self.model_name,
                req.max_tokens,
                &raw_message,
            ));
        }

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
