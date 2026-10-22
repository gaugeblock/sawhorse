//! Scaffolds a starter `sawhorse.toml` template.

use std::path::Path;
use anyhow::{bail, Context, Result};

const STARTER_TOML: &str = r#"# sawhorse.toml — Provider-Agnostic 3-Mode LLM Runtime Configuration
# Reference: https://github.com/gaugeblock/sawhorse

[timeouts]
inference_timeout_secs = 60
ping_timeout_secs = 5

# --- Provider Definitions ---

# Mode 2: Local GGUF / llama.cpp or mistral.rs HTTP server
[providers.local_server]
mode = "local_runtime"
model_id = "qwen2.5-coder-7b"
endpoint_or_path = "http://127.0.0.1:8080"
dialect = "llama_cpp"

# Mode 3: OpenAI-compatible endpoint (jev, vLLM, OpenRouter, cloud)
[providers.cloud_openai]
mode = "open_ai_compatible"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
model_name = "gpt-4o-mini"
dialect = "standard_open_ai"

# Mode 1: Specialized in-house gaugeblock micro-weights via Candle
[providers.candle_micro]
mode = "in_house_candle"
model_id = "gaugeblock-micro-v1"
weights_path = "./models/micro.safetensors"

# --- Semantic Role Routing & Fallback Chains ---

[roles.planner]
provider = "cloud_openai"
fallback = ["local_server"]

[roles.system1_router]
provider = "local_server"
fallback = ["cloud_openai"]

[roles.hole_filler]
provider = "local_server"
fallback = ["cloud_openai"]

[roles.oracle]
provider = "cloud_openai"
fallback = ["local_server"]
"#;

/// Runs `sawhorse init` to create a starter `sawhorse.toml`.
pub fn run_init(path: &Path, force: bool) -> Result<()> {
    let target = if path.is_dir() {
        path.join("sawhorse.toml")
    } else {
        path.to_path_buf()
    };

    if target.exists() && !force {
        bail!(
            "Configuration file '{}' already exists. Use --force to overwrite.",
            target.display()
        );
    }

    std::fs::write(&target, STARTER_TOML)
        .with_context(|| format!("Failed to write configuration to '{}'", target.display()))?;

    eprintln!("Initialized starter sawhorse configuration at '{}'", target.display());
    Ok(())
}

