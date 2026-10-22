//! Integration tests for role routing and fallback logic.

use sawhorse_core::config::*;
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::*;

#[tokio::test]
async fn test_router_construction_and_role_resolution() {
    let cfg = SawhorseConfig::default();
    let router = RoleRouter::from_config(cfg).expect("Failed to build router");

    let planner_provider = router.provider_for_role(ModelRole::Planner);
    assert!(planner_provider.is_ok());
    assert_eq!(planner_provider.unwrap().provider_name(), "local");

    let oracle_provider = router.provider_for_role(ModelRole::Oracle);
    assert!(oracle_provider.is_ok());
    assert_eq!(oracle_provider.unwrap().provider_name(), "local");
}

#[tokio::test]
async fn test_router_fallback_exhaustion_on_unreachable() {
    let mut cfg = SawhorseConfig::default();
    // Update local provider to non-listening port
    if let Some(p) = cfg.providers.get_mut("local") {
        p.mode = ExecutionMode::LocalRuntime {
            model_id: "test".to_string(),
            endpoint_or_path: "http://127.0.0.1:59999".to_string(),
        };
    }
    // Update openai fallback to invalid token env
    if let Some(p) = cfg.providers.get_mut("openai") {
        p.mode = ExecutionMode::OpenAiCompatible {
            base_url: "http://127.0.0.1:59998".to_string(),
            api_key_env: "DEFINITELY_NON_EXISTENT_KEY_VAR_12345".to_string(),
            model_name: "test".to_string(),
        };
    }

    let router = RoleRouter::from_config(cfg).expect("Failed to build router");
    let req = InferenceRequest::new(ModelRole::HoleFiller, "test prompt");

    let res = router.complete(&req).await;
    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(err_str.contains("All providers in fallback chain failed"));
}

