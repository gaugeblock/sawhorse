//! Configuration data structures for `sawhorse.toml`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{ExecutionMode, ModelRole};

/// Dialect variations across OpenAI-compatible servers for structured outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OpenAiDialect {
    /// vLLM / outlines engine (guided_grammar, guided_regex, guided_choice).
    #[default]
    Vllm,
    /// llama.cpp server in OpenAI chat completion emulation mode (grammar).
    LlamaCpp,
    /// Standard OpenAI / cloud proxies (response_format / json_schema only).
    StandardOpenAi,
}

/// Configuration block for an individual model provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Backend execution mode and connection parameters.
    #[serde(flatten)]
    pub mode: ExecutionMode,

    /// Structured output dialect (applicable for Mode 3).
    pub dialect: Option<OpenAiDialect>,

    /// Optional custom HTTP headers (e.g. for enterprise gateways or CF Access).
    pub headers: Option<HashMap<String, String>>,

    /// Custom request timeout in milliseconds overriding defaults.
    pub timeout_ms: Option<u64>,
}

/// Routing policy for a specific semantic `ModelRole`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleConfig {
    /// Primary provider name (must match a key in `[providers.<name>]`).
    pub provider: String,

    /// Ordered fallback provider names if primary fails.
    #[serde(default)]
    pub fallback: Vec<String>,
}

/// Global timeout settings for inference and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Maximum duration allowed for inference requests in seconds. Default: 60.
    #[serde(default = "default_inference_timeout")]
    pub inference_timeout_secs: u64,

    /// Maximum duration allowed for health check pings in seconds. Default: 5.
    #[serde(default = "default_ping_timeout")]
    pub ping_timeout_secs: u64,
}

const fn default_inference_timeout() -> u64 {
    60
}

const fn default_ping_timeout() -> u64 {
    5
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            inference_timeout_secs: default_inference_timeout(),
            ping_timeout_secs: default_ping_timeout(),
        }
    }
}

/// Root schema for `sawhorse.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SawhorseConfig {
    /// Registered provider endpoints indexed by unique name.
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Role-to-provider mappings.
    #[serde(default)]
    pub roles: HashMap<ModelRole, RoleConfig>,

    /// Global timeout parameters.
    #[serde(default)]
    pub timeouts: TimeoutConfig,
}

