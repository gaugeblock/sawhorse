//! Comparative benchmarking command across configured providers.

use anyhow::Result;
use serde_json::json;
use tracing::info;

use sawhorse_core::config::SawhorseConfig;
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{InferenceRequest, ModelRole};

const DEFAULT_BENCH_PROMPT: &str =
    "Write a Rust function named `add_numbers` that takes two i64 arguments and returns their sum.";

/// Runs `sawhorse bench` to measure latency and token throughput.
pub async fn run_bench(
    config: SawhorseConfig,
    prompt: Option<String>,
    as_json: bool,
) -> Result<()> {
    let bench_prompt = prompt.unwrap_or_else(|| DEFAULT_BENCH_PROMPT.to_string());
    let router = RoleRouter::from_config(config.clone())?;

    let req = InferenceRequest::new(ModelRole::HoleFiller, &bench_prompt);

    if !as_json {
        println!(
            "{:<20} {:<12} {:<15} {:<10}",
            "PROVIDER", "LATENCY", "TOKENS (OUT)", "STATUS"
        );
        println!("{}", "-".repeat(60));
    }

    let mut json_results = Vec::new();

    for name in config.providers.keys() {
        if let Some(provider) = router.get_provider(name) {
            match provider.complete(&req).await {
                Ok(resp) => {
                    let tok_per_sec = if resp.latency_ms > 0 {
                        (resp.completion_tokens as f64) / (resp.latency_ms as f64 / 1000.0)
                    } else {
                        0.0
                    };

                    if as_json {
                        json_results.push(json!({
                            "provider": name,
                            "status": "ok",
                            "latency_ms": resp.latency_ms,
                            "tokens_out": resp.completion_tokens,
                            "tokens_per_sec": format!("{tok_per_sec:.1}"),
                            "model_used": resp.model_used,
                        }));
                    } else {
                        println!(
                            "{:<20} {:<12} {:<15} {:<10}",
                            name,
                            format!("{}ms", resp.latency_ms),
                            format!("{} ({:.1} t/s)", resp.completion_tokens, tok_per_sec),
                            "OK"
                        );
                    }
                }
                Err(err) => {
                    if as_json {
                        json_results.push(json!({
                            "provider": name,
                            "status": "error",
                            "error": err.to_string(),
                        }));
                    } else {
                        println!(
                            "{:<20} {:<12} {:<15} {:<10}",
                            name,
                            "-",
                            "-",
                            format!("ERR ({err})")
                        );
                    }
                }
            }
        }
    }

    if as_json {
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    }

    info!("Benchmark completed");
    Ok(())
}
