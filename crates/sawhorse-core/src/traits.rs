//! Unified provider trait abstraction for `sawhorse`.

use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;

use crate::error::SawhorseError;
use crate::types::{ExecutionMode, InferenceRequest, InferenceResponse};

/// Boxed pinned stream yielding token text chunks or errors.
pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String, SawhorseError>> + Send>>;

/// Provider-agnostic inference engine trait.
///
/// Consumers like `chalkline` (Phase 3) and `joiner` (Phase 6) interact
/// strictly through this trait without caring whether the backing engine is
/// an embedded Candle model, a local llama.cpp server, or a remote OpenAI endpoint.
#[async_trait]
pub trait ModelProvider: Send + Sync + std::fmt::Debug {
    /// Executes a unary inference request and returns the completed text response.
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, SawhorseError>;

    /// Executes a streaming inference request yielding incremental token chunks.
    async fn complete_stream(&self, req: &InferenceRequest) -> Result<TokenStream, SawhorseError>;

    /// Performs a health check ping to ensure the provider backend is responsive.
    async fn health_check(&self) -> Result<bool, SawhorseError>;

    /// Returns the unique configured name of this provider.
    fn provider_name(&self) -> &str;

    /// Returns the execution mode and metadata for this provider.
    fn execution_mode(&self) -> &ExecutionMode;
}
