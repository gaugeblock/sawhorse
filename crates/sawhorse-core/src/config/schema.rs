//! Configuration data structures for `sawhorse.toml`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{ExecutionMode, ModelRole, ReasoningEffort};

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

    /// Total context window of the configured model, in tokens, when known.
    ///
    /// Configured rather than discovered: the OpenAI-compatible catalogue returns ids only, there is
    /// no per-model endpoint, and a completion's `usage` reports what a prompt cost rather than what
    /// the ceiling is (`GB-215`). Used as the hard cap for a sliding output budget.
    pub context_window: Option<u32>,

    /// Maximum output tokens the model accepts in one response.
    pub max_output_tokens: Option<u16>,

    /// Reasoning effort to request when a caller does not choose one.
    ///
    /// A reasoning model spends budget on its thinking trace before emitting content, so this is the
    /// knob that decides how much of the window goes to thinking rather than to the answer.
    pub reasoning_effort: Option<ReasoningEffort>,
}

/// Declared limits for one provider's model, used to size a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModelLimits {
    /// Total context window, when configured.
    pub context_window: Option<u32>,
    /// Maximum output tokens per response, when configured.
    pub max_output_tokens: Option<u16>,
    /// Default reasoning effort, when configured.
    pub reasoning_effort: Option<ReasoningEffort>,
}

impl ProviderConfig {
    /// Returns the declared limits for this provider.
    #[must_use]
    pub fn limits(&self) -> ModelLimits {
        ModelLimits {
            context_window: self.context_window,
            max_output_tokens: self.max_output_tokens,
            reasoning_effort: self.reasoning_effort,
        }
    }
}

impl ModelLimits {
    /// Output budget for a prompt of `prompt_tokens`, never exceeding the declared maximum.
    ///
    /// Returns `None` when no window is declared, so a caller keeps its own default rather than
    /// inventing a ceiling. The result is at least 256 tokens so a tiny window never asks for zero.
    #[must_use]
    pub fn output_budget(&self, prompt_tokens: u32) -> Option<u16> {
        let window = self.context_window?;
        let available = window.saturating_sub(prompt_tokens).saturating_sub(64);
        let ceiling = u32::from(self.max_output_tokens.unwrap_or(u16::MAX));
        let budget = available.min(ceiling).min(u32::from(u16::MAX));
        Some(u16::try_from(budget.max(256).min(ceiling)).unwrap_or(u16::MAX))
    }
}

#[cfg(test)]
mod limits_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::ModelLimits;

    #[test]
    fn test_no_window_means_no_opinion() {
        assert_eq!(ModelLimits::default().output_budget(1000), None);
    }

    #[test]
    fn test_budget_is_the_window_minus_the_prompt_capped_by_the_output_limit() {
        let limits = ModelLimits {
            context_window: Some(1_000_000),
            max_output_tokens: Some(32_768),
            reasoning_effort: None,
        };
        assert_eq!(
            limits.output_budget(20_000),
            Some(32_768),
            "output cap wins"
        );
        let narrow = ModelLimits {
            context_window: Some(10_000),
            max_output_tokens: Some(32_768),
            reasoning_effort: None,
        };
        assert_eq!(
            narrow.output_budget(9_000),
            Some(936),
            "window minus prompt minus headroom"
        );
    }

    #[test]
    fn test_a_full_window_still_asks_for_something() {
        let limits = ModelLimits {
            context_window: Some(1_000),
            max_output_tokens: Some(32_768),
            reasoning_effort: None,
        };
        assert_eq!(limits.output_budget(2_000), Some(256));
    }
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
