//! Mode 2: Local GGUF runtime adapter for llama.cpp, mistral.rs, or subprocess CLI.

use std::path::Path;
use std::process::Command;
use std::time::Instant;

use async_trait::async_trait;
use futures_util::stream;
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::error::SawhorseError;
use crate::traits::{ModelProvider, TokenStream};
use crate::types::{ExecutionMode, GrammarFormat, InferenceRequest, InferenceResponse};

/// Provider implementation for local model runtimes.
#[derive(Debug, Clone)]
pub struct LocalGgufProvider {
    name: String,
    mode: ExecutionMode,
    model_id: String,
    endpoint_or_path: String,
    is_url: bool,
    client: reqwest::Client,
    timeout_ms: u64,
}

impl LocalGgufProvider {
    /// Constructs a new local GGUF provider.
    pub fn new(
        name: impl Into<String>,
        model_id: impl Into<String>,
        endpoint_or_path: impl Into<String>,
        timeout_ms: u64,
    ) -> Result<Self, SawhorseError> {
        let ep = endpoint_or_path.into();
        let m_id = model_id.into();
        let is_url = ep.starts_with("http://") || ep.starts_with("https://");
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| SawhorseError::Network {
                endpoint: "local_client_init".into(),
                message: e.to_string(),
            })?;

        let mode = ExecutionMode::LocalRuntime {
            model_id: m_id.clone(),
            endpoint_or_path: ep.clone(),
        };

        Ok(Self {
            name: name.into(),
            mode,
            model_id: m_id,
            endpoint_or_path: ep,
            is_url,
            client,
            timeout_ms,
        })
    }

    async fn complete_http(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, SawhorseError> {
        let url = format!("{}/completion", self.endpoint_or_path.trim_end_matches('/'));
        let mut body = json!({
            "prompt": &req.prompt,
            "n_predict": req.max_tokens,
            "temperature": req.temperature,
            "stream": false,
        });

        if let Some(ref grammar) = req.grammar_constraint
            && (req.grammar_format == Some(GrammarFormat::Gbnf) || req.grammar_format.is_none())
        {
            body["grammar"] = json!(grammar);
        }

        let start = Instant::now();
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .send()
            .await
            .map_err(|e| SawhorseError::Network {
                endpoint: url.clone(),
                message: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(SawhorseError::Http {
                endpoint: url,
                status: status.as_u16(),
                message: msg,
            });
        }

        let res: Value = resp.json().await.map_err(|e| SawhorseError::Network {
            endpoint: url,
            message: format!("Malformed JSON: {e}"),
        })?;

        let text = res["content"].as_str().unwrap_or_default().to_string();
        let prompt_tokens = res["tokens_evaluated"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = res["tokens_predicted"].as_u64().unwrap_or(0) as u32;

        Ok(InferenceResponse {
            text,
            prompt_tokens,
            completion_tokens,
            latency_ms: start.elapsed().as_millis() as u64,
            model_used: self.model_id.clone(),
            reasoning_trace: None,
            reasoning_tokens: None,
        })
    }

    fn complete_subprocess(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, SawhorseError> {
        let binary = Path::new(&self.endpoint_or_path);
        let start = Instant::now();

        let mut cmd = Command::new(binary);
        cmd.arg("-m")
            .arg(&self.model_id)
            .arg("-p")
            .arg(&req.prompt)
            .arg("-n")
            .arg(req.max_tokens.to_string())
            .arg("--temp")
            .arg(req.temperature.to_string());

        if let Some(ref grammar) = req.grammar_constraint {
            cmd.arg("--grammar").arg(grammar);
        }

        let output = cmd.output().map_err(|e| SawhorseError::Subprocess {
            exit_code: None,
            stderr: format!("Failed to spawn local runtime binary: {e}"),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SawhorseError::Subprocess {
                exit_code: output.status.code(),
                stderr,
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(InferenceResponse {
            text,
            prompt_tokens: 0,
            completion_tokens: 0,
            latency_ms: start.elapsed().as_millis() as u64,
            model_used: self.model_id.clone(),
            reasoning_trace: None,
            reasoning_tokens: None,
        })
    }
}

#[async_trait]
impl ModelProvider for LocalGgufProvider {
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, SawhorseError> {
        if self.is_url {
            self.complete_http(req).await
        } else {
            self.complete_subprocess(req)
        }
    }

    async fn complete_stream(&self, req: &InferenceRequest) -> Result<TokenStream, SawhorseError> {
        let res = self.complete(req).await?;
        let s = stream::once(async move { Ok(res.text) });
        Ok(Box::pin(s))
    }

    async fn health_check(&self) -> Result<bool, SawhorseError> {
        if self.is_url {
            let url = format!("{}/health", self.endpoint_or_path.trim_end_matches('/'));
            match self.client.get(&url).send().await {
                Ok(r) if r.status().is_success() => Ok(true),
                Ok(r) => {
                    warn!(status = %r.status(), "Local runtime health check non-200");
                    Ok(false)
                }
                Err(e) => {
                    debug!(error = %e, "Local runtime offline");
                    Ok(false)
                }
            }
        } else {
            let p = Path::new(&self.endpoint_or_path);
            Ok(p.is_file())
        }
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn execution_mode(&self) -> &ExecutionMode {
        &self.mode
    }
}
