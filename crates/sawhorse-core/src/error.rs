//! Strongly-typed errors for the `sawhorse-core` library.

use thiserror::Error;

/// Core error type representing all failure modes in `sawhorse`.
#[derive(Debug, Error)]
pub enum SawhorseError {
    /// Configuration parsing, validation, or file discovery error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Missing environment variable for authentication.
    #[error("Missing environment variable '{env_var}' for provider '{provider}'")]
    MissingSecret {
        /// Name of the required environment variable.
        env_var: String,
        /// Name of the provider requiring this secret.
        provider: String,
    },

    /// The requested provider is unavailable or unrecognized.
    #[error("Provider '{provider}' is unavailable: {reason}")]
    ProviderUnavailable {
        /// Provider identifier.
        provider: String,
        /// Root cause explanation.
        reason: String,
    },

    /// HTTP error returned by remote or local model server.
    #[error("HTTP error from '{endpoint}' (status {status}): {message}")]
    Http {
        /// Endpoint that returned the error.
        endpoint: String,
        /// HTTP status code.
        status: u16,
        /// Error body or message with sensitive headers redacted.
        message: String,
    },

    /// Network connectivity or transport failure.
    #[error("Network transport failure contacting '{endpoint}': {message}")]
    Network {
        /// Endpoint contacted.
        endpoint: String,
        /// Error message.
        message: String,
    },

    /// Subprocess execution failure for local CLI runner.
    #[error("Subprocess execution failed: {stderr} (exit code: {exit_code:?})")]
    Subprocess {
        /// Process exit code if available.
        exit_code: Option<i32>,
        /// Stderr output from child process.
        stderr: String,
    },

    /// Requested model is unknown to the backend provider.
    #[error("Model '{0}' was not found or is not loaded")]
    ModelNotFound(String),

    /// Grammar format is not supported by the designated provider dialect.
    #[error("Grammar format '{format}' is unsupported by provider '{provider}': {reason}")]
    GrammarUnsupported {
        /// Format requested (e.g. Lark, GBNF).
        format: String,
        /// Provider name.
        provider: String,
        /// Explanation.
        reason: String,
    },

    /// ModelRole has no configured provider in routing table.
    #[error("No provider configured for ModelRole '{0:?}'")]
    RoleNotConfigured(String),

    /// All providers in a role's fallback chain failed.
    #[error("All providers in fallback chain failed for role '{role}': {attempts:?}")]
    AllFallbacksExhausted {
        /// Target role attempted.
        role: String,
        /// List of failure messages per provider attempted.
        attempts: Vec<String>,
    },

    /// Request exceeded the configured timeout threshold.
    #[error("Inference request timed out after {timeout_ms}ms")]
    InferenceTimeout {
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Native Candle ML tensor or model loader error.
    #[error("Candle native inference error: {0}")]
    Candle(String),

    /// JSON serialization or deserialization failure.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

