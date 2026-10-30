//! Model discovery command: asks each provider what it actually serves.
//!
//! The catalogue is the one the VS Code extension uses (`GET {base_url}/models`), so the answer is
//! authoritative rather than a maintained list - which is why `cline-pass/...` aliases do not appear
//! in it while the underlying `deepseek/...` ids do (`GB-210`).

use anyhow::Result;
use serde_json::json;

use sawhorse_core::config::SawhorseConfig;
use sawhorse_core::providers::{
    CLINE_PASS_CAPTURED, CLINE_PASS_DOCS, CLINE_PASS_MODELS, is_pass_model,
};
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::ExecutionMode;

/// Runs `sawhorse models`.
///
/// Prints every advertised id per OpenAI-compatible provider, marks the configured model, and warns
/// when the configured id is absent from the catalogue, because that is the case where a typo and a
/// routing alias look identical from here.
pub async fn run_models(
    config: SawhorseConfig,
    provider: Option<String>,
    filter: Option<String>,
    as_json: bool,
) -> Result<()> {
    let mut configured = std::collections::BTreeMap::new();
    for (name, entry) in &config.providers {
        if let ExecutionMode::OpenAiCompatible {
            base_url,
            model_name,
            ..
        } = &entry.mode
        {
            configured.insert(name.clone(), (model_name.clone(), base_url.clone()));
        }
    }

    let router = RoleRouter::from_config(config)?;
    let mut providers: Vec<&String> = configured.keys().collect();
    providers.sort();

    let mut reports = Vec::new();
    for name in providers {
        if let Some(target) = &provider
            && target != name
        {
            continue;
        }
        let (model_name, base_url) = &configured[name];
        let discovered = match router.get_provider(name) {
            Some(source) => source.list_models().await,
            None => Err(sawhorse_core::error::SawhorseError::ProviderUnavailable {
                provider: name.clone(),
                reason: "provider is not registered with the router".to_string(),
            }),
        };

        let models = match discovered {
            Ok(list) => list,
            Err(error) => {
                if as_json {
                    reports.push(json!({
                        "provider": name,
                        "base_url": base_url,
                        "configured": model_name,
                        "error": error.to_string(),
                    }));
                } else {
                    println!("{name}: discovery failed ({error})");
                }
                continue;
            }
        };

        let advertised_configured = models.iter().any(|id| id == model_name);
        let shown: Vec<&String> = models
            .iter()
            .filter(|id| match &filter {
                Some(needle) => id.contains(needle.as_str()),
                None => true,
            })
            .collect();

        if as_json {
            reports.push(json!({
                "provider": name,
                "base_url": base_url,
                "configured": model_name,
                "configured_is_advertised": advertised_configured,
                "configured_is_pass_eligible": is_pass_model(model_name),
                "pass_model_count": CLINE_PASS_MODELS.len(),
                "pass_captured": CLINE_PASS_CAPTURED,
                "advertised": models.len(),
                "models": shown,
            }));
            continue;
        }

        println!("{name}  ({base_url})");
        let eligibility = if is_pass_model(model_name) {
            "pass-eligible (documented)".to_string()
        } else if advertised_configured {
            "catalogue only - billed as credits, not covered by ClinePass".to_string()
        } else {
            "not in the catalogue and not a documented pass id".to_string()
        };
        println!("  configured: {model_name}   [{eligibility}]");
        println!("  advertised: {} model(s)", models.len());
        for id in &shown {
            let marker = if id.as_str() == model_name.as_str() {
                " *"
            } else {
                ""
            };
            let pass = if is_pass_model(id) { "  [pass]" } else { "" };
            println!("    {id}{marker}{pass}");
        }
        println!(
            "  [pass] = curated ClinePass set ({} ids, captured {CLINE_PASS_CAPTURED}); the catalogue
         lists what exists, not what the subscription covers: {CLINE_PASS_DOCS}",
            CLINE_PASS_MODELS.len()
        );
        println!();
    }

    if as_json {
        println!("{}", serde_json::to_string_pretty(&json!(reports))?);
    } else if reports.is_empty() && filter.is_none() {
        println!("No OpenAI-compatible providers are configured.");
    }

    Ok(())
}
