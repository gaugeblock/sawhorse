//! Interactive and headless prompt completion command.

use std::io::Write;
use std::path::Path;

use anyhow::{Result, bail};
use futures_util::StreamExt;
use tracing::info;

use sawhorse_core::config::SawhorseConfig;
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{GrammarFormat, InferenceRequest, ModelRole};

/// Arguments for `sawhorse complete`.
#[derive(Debug, Clone)]
pub struct CompleteArgs {
    pub prompt: String,
    pub role: ModelRole,
    pub provider: Option<String>,
    pub grammar: Option<String>,
    pub grammar_format: Option<GrammarFormat>,
    pub system_prompt: Option<String>,
    pub max_tokens: u16,
    pub temperature: f32,
    pub stream: bool,
    pub as_json: bool,
}

/// Runs `sawhorse complete`.
pub async fn run_complete(config: SawhorseConfig, mut args: CompleteArgs) -> Result<()> {
    // Resolve grammar file if path exists
    if let Some(ref g) = args.grammar {
        let p = Path::new(g);
        if p.is_file() {
            args.grammar = Some(std::fs::read_to_string(p)?);
        }
    }

    let mut req = InferenceRequest::new(args.role, args.prompt);
    req.max_tokens = args.max_tokens;
    req.temperature = args.temperature;
    req.system_prompt = args.system_prompt;
    req.grammar_constraint = args.grammar;
    req.grammar_format = args.grammar_format;

    let router = RoleRouter::from_config(config)?;

    if args.stream && !args.as_json {
        let mut stream = if let Some(ref prov_name) = args.provider {
            let prov = router
                .get_provider(prov_name)
                .ok_or_else(|| anyhow::anyhow!("Provider '{prov_name}' not found"))?;
            prov.complete_stream(&req).await?
        } else {
            router.complete_stream(&req).await?
        };

        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        while let Some(chunk_res) = stream.next().await {
            match chunk_res {
                Ok(chunk) => {
                    write!(lock, "{chunk}")?;
                    lock.flush()?;
                }
                Err(err) => {
                    bail!("Stream error: {err}");
                }
            }
        }
        writeln!(lock)?;
        return Ok(());
    }

    let resp = if let Some(ref prov_name) = args.provider {
        let prov = router
            .get_provider(prov_name)
            .ok_or_else(|| anyhow::anyhow!("Provider '{prov_name}' not found"))?;
        prov.complete(&req).await?
    } else {
        router.complete(&req).await?
    };

    if args.as_json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("{}", resp.text);
    }

    info!(model = %resp.model_used, latency = %resp.latency_ms, "Completion finished");
    Ok(())
}
