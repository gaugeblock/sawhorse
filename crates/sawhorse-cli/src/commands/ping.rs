//! Health check and latency diagnostic command for providers.

use anyhow::Result;
use serde_json::json;
use tracing::info;

use sawhorse_core::config::SawhorseConfig;
use sawhorse_core::router::RoleRouter;

/// Runs `sawhorse ping` across configured providers.
pub async fn run_ping(
    config: SawhorseConfig,
    provider: Option<String>,
    as_json: bool,
) -> Result<()> {
    let router = RoleRouter::from_config(config)?;
    let ping_results = router.ping_all().await;

    if as_json {
        let mut map = serde_json::Map::new();
        for (name, res) in ping_results {
            if let Some(ref target) = provider
                && target != &name
            {
                continue;
            }
            match res {
                Ok(latency_ms) => {
                    map.insert(name, json!({ "status": "ok", "latency_ms": latency_ms }));
                }
                Err(err) => {
                    map.insert(name, json!({ "status": "error", "error": err.to_string() }));
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&map)?);
        return Ok(());
    }

    println!("{:<20} {:<10} {:<12}", "PROVIDER", "STATUS", "LATENCY");
    println!("{}", "-".repeat(45));

    for (name, res) in ping_results {
        if let Some(ref target) = provider
            && target != &name
        {
            continue;
        }
        match res {
            Ok(latency) => {
                println!("{:<20} {:<10} {:<12}", name, "OK", format!("{latency}ms"));
            }
            Err(err) => {
                println!("{:<20} {:<10} {:<12}", name, "OFFLINE", format!("({err})"));
            }
        }
    }

    info!("Ping diagnostics completed");
    Ok(())
}
