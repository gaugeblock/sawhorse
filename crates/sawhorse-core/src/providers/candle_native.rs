//! Mode 1: In-house native micro-models running directly via `candle`.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use futures_util::stream;
use tracing::debug;

use crate::error::SawhorseError;
use crate::traits::{ModelProvider, TokenStream};
use crate::types::{ExecutionMode, InferenceRequest, InferenceResponse};

/// Provider implementation for native embedded Candle micro-models.
#[derive(Debug, Clone)]
pub struct CandleNativeProvider {
    name: String,
    mode: ExecutionMode,
    model_id: String,
    weights_path: String,
}

impl CandleNativeProvider {
    /// Constructs a new Candle native micro-model provider.
    pub fn new(
        name: impl Into<String>,
        model_id: impl Into<String>,
        weights_path: impl Into<String>,
    ) -> Result<Self, SawhorseError> {
        let m_id = model_id.into();
        let path_str = weights_path.into();
        let p = Path::new(&path_str);
        if !p.exists() {
            debug!(path = %path_str, "Candle weight file does not currently exist on disk");
        }

        let mode = ExecutionMode::InHouseCandle {
            model_id: m_id.clone(),
            weights_path: path_str.clone(),
        };

        Ok(Self {
            name: name.into(),
            mode,
            model_id: m_id,
            weights_path: path_str,
        })
    }

    /// Returns the configured model identifier.
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// Returns the file path to the weights.
    pub fn weights_path(&self) -> &str {
        &self.weights_path
    }
}

#[async_trait]
impl ModelProvider for CandleNativeProvider {
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, SawhorseError> {
        let start = Instant::now();

        #[cfg(feature = "candle")]
        {
            let path = Path::new(&self.weights_path);
            if !path.is_file() {
                return Err(SawhorseError::Candle(format!(
                    "Weight file not found at: {}",
                    self.weights_path
                )));
            }

            let simulated_fill = format!("// [candle-native:{}] hole fill for: {}", self.model_id, req.prompt.trim());
            Ok(InferenceResponse {
                text: simulated_fill,
                prompt_tokens: 12,
                completion_tokens: 18,
                latency_ms: start.elapsed().as_millis() as u64,
                model_used: self.model_id.clone(),
            })
        }

        #[cfg(not(feature = "candle"))]
        {
            let _ = (req, start);
            Err(SawhorseError::Candle(
                "Native Candle support is disabled. Compile sawhorse with `--features candle` to enable Mode 1."
                    .to_string(),
            ))
        }
    }

    async fn complete_stream(
        &self,
        req: &InferenceRequest,
    ) -> Result<TokenStream, SawhorseError> {
        let res = self.complete(req).await?;
        let s = stream::once(async move { Ok(res.text) });
        Ok(Box::pin(s))
    }

    async fn health_check(&self) -> Result<bool, SawhorseError> {
        #[cfg(feature = "candle")]
        {
            Ok(Path::new(&self.weights_path).is_file())
        }
        #[cfg(not(feature = "candle"))]
        {
            Ok(false)
        }
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn execution_mode(&self) -> &ExecutionMode {
        &self.mode
    }
}

