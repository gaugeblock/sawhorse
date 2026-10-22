//! Provider implementations and factory dispatch for `sawhorse`.

pub mod candle_native;
pub mod local_gguf;
pub mod openai_compat;

use std::sync::Arc;

pub use candle_native::CandleNativeProvider;
pub use local_gguf::LocalGgufProvider;
pub use openai_compat::OpenAiCompatProvider;

use crate::config::schema::{OpenAiDialect, ProviderConfig, TimeoutConfig};
use crate::error::SawhorseError;
use crate::traits::ModelProvider;
use crate::types::ExecutionMode;

/// Instantiates an Arc-wrapped `ModelProvider` from configuration.
pub fn create_provider(
    name: &str,
    config: &ProviderConfig,
    timeouts: &TimeoutConfig,
) -> Result<Arc<dyn ModelProvider>, SawhorseError> {
    let timeout_ms = config
        .timeout_ms
        .unwrap_or(timeouts.inference_timeout_secs * 1000);

    match &config.mode {
        ExecutionMode::InHouseCandle {
            model_id,
            weights_path,
        } => {
            let provider = CandleNativeProvider::new(name, model_id, weights_path)?;
            Ok(Arc::new(provider))
        }
        ExecutionMode::LocalRuntime {
            model_id,
            endpoint_or_path,
        } => {
            let provider = LocalGgufProvider::new(name, model_id, endpoint_or_path, timeout_ms)?;
            Ok(Arc::new(provider))
        }
        ExecutionMode::OpenAiCompatible {
            base_url,
            api_key_env,
            model_name,
        } => {
            let dialect = config.dialect.unwrap_or(OpenAiDialect::Vllm);
            let provider = OpenAiCompatProvider::new(
                name,
                base_url,
                api_key_env,
                model_name,
                dialect,
                config.headers.clone(),
                timeout_ms,
            )?;
            Ok(Arc::new(provider))
        }
    }
}

