//! Core domain types and wire contracts for `sawhorse`.
//!
//! These structures correspond directly to Section 5 of `CONTRACTS.md`.

use serde::{Deserialize, Serialize};

/// Supported grammar constraint formats emitted by `batten`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrammarFormat {
    /// Context-free grammar in llama.cpp GBNF notation.
    Gbnf,
    /// Context-free grammar in Lark notation (used by llguidance/outlines).
    Lark,
    /// Regular expression pattern constraint.
    Regex,
}

/// Execution backend modes supported by `sawhorse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Mode 1: Specialized `gaugeblock` micro-weights running natively via `candle`.
    InHouseCandle {
        /// Identifier or architecture tag of the micro-model.
        model_id: String,
        /// Path to safetensors or gguf weight file on disk.
        weights_path: String,
    },
    /// Mode 2: Local GGUF / quantized open weights via `mistral.rs` or `llama.cpp`.
    LocalRuntime {
        /// Model identifier or alias.
        model_id: String,
        /// Local HTTP endpoint (e.g. `http://localhost:8080`) or local CLI executable path.
        endpoint_or_path: String,
    },
    /// Mode 3: Any OpenAI-compatible HTTP endpoint (`jev`, `vLLM`, OpenRouter, standard cloud).
    OpenAiCompatible {
        /// Base URL (e.g. `https://api.openai.com/v1` or `http://localhost:8000/v1`).
        base_url: String,
        /// Name of the environment variable storing the API key.
        api_key_env: String,
        /// Canonical model name (e.g. `gpt-4o-mini`, `qwen2.5-coder-32b`).
        model_name: String,
    },
}

/// Role assignments used by `gaugeblock` callers to route requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    /// Spec and DAG planning (used by `chalkline`).
    Planner,
    /// Structural AST pattern & mixin selection (used by `joiner`).
    System1Router,
    /// Grammar-masked hole filling inside `batten` constraints.
    HoleFiller,
    /// Unconstrained or mildly constrained escalation for novel algorithms.
    Oracle,
}

/// Unary inference request passed into `ModelProvider`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// Target semantic role for routing and default parameters.
    pub role: ModelRole,
    /// User prompt or hole-fill context.
    pub prompt: String,
    /// Optional system instruction.
    pub system_prompt: Option<String>,
    /// Raw grammar string passed from `batten` or caller.
    pub grammar_constraint: Option<String>,
    /// Grammar syntax notation (GBNF, Lark, Regex).
    pub grammar_format: Option<GrammarFormat>,
    /// Maximum tokens to generate.
    pub max_tokens: u16,
    /// Sampling temperature (0.0 for deterministic output).
    pub temperature: f32,
}

impl InferenceRequest {
    /// Constructs a basic deterministic prompt request for a role.
    pub fn new(role: ModelRole, prompt: impl Into<String>) -> Self {
        Self {
            role,
            prompt: prompt.into(),
            system_prompt: None,
            grammar_constraint: None,
            grammar_format: None,
            max_tokens: 1024,
            temperature: 0.0,
        }
    }

    /// Attaches a GBNF grammar constraint to this request.
    pub fn with_gbnf_grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar_constraint = Some(grammar.into());
        self.grammar_format = Some(GrammarFormat::Gbnf);
        self
    }

    /// Attaches a Lark grammar constraint to this request.
    pub fn with_lark_grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar_constraint = Some(grammar.into());
        self.grammar_format = Some(GrammarFormat::Lark);
        self
    }

    /// Attaches an optional system prompt.
    pub fn with_system_prompt(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }
}

/// Completed inference response returned by `ModelProvider`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceResponse {
    /// Generated output text.
    pub text: String,
    /// Prompt / input token count.
    pub prompt_tokens: u32,
    /// Generated completion token count.
    pub completion_tokens: u32,
    /// Execution latency in milliseconds.
    pub latency_ms: u64,
    /// Identifier or model name that served the response.
    pub model_used: String,
}

