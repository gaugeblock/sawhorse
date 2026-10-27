//! `sawhorse-core` — Core library, traits, wire contracts, and routing engine.
//!
//! Provides the primary [`ModelProvider`] async trait and [`RoleRouter`]
//! used by `chalkline` (Phase 3) and `joiner` (Phase 6) to perform 3-mode
//! inference across embedded Candle models, local GGUFs, and OpenAI endpoints.

pub mod config;
pub mod error;
pub mod providers;
pub mod router;
pub mod traits;
pub mod types;

pub use config::{OpenAiDialect, ProviderConfig, RoleConfig, SawhorseConfig, TimeoutConfig};
pub use error::SawhorseError;
pub use providers::{
    CandleNativeProvider, LocalGgufProvider, OpenAiCompatProvider, create_provider,
};
pub use router::RoleRouter;
pub use traits::{ModelProvider, TokenStream};
pub use types::{
    ExecutionMode, GrammarFormat, InferenceRequest, InferenceResponse, ModelRole, ReasoningEffort,
};
