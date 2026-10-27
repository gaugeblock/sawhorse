//! Tests for configuration loading and validation.

use sawhorse_core::config::*;
use sawhorse_core::types::ModelRole;

#[test]
fn test_default_config_validation() {
    let cfg = SawhorseConfig::default();
    assert!(cfg.validate().is_ok());

    assert!(cfg.providers.contains_key("local"));
    assert!(cfg.providers.contains_key("openai"));

    assert_eq!(
        cfg.roles.get(&ModelRole::Planner).unwrap().provider,
        "local"
    );
    assert_eq!(
        cfg.roles.get(&ModelRole::Planner).unwrap().fallback,
        vec!["openai"]
    );
}

#[test]
fn test_invalid_config_detection() {
    let mut cfg = SawhorseConfig::default();
    // Point to non-existent provider
    cfg.roles.get_mut(&ModelRole::Oracle).unwrap().provider = "ghost_model".to_string();
    assert!(cfg.validate().is_err());

    let mut cfg_fallback = SawhorseConfig::default();
    // Point fallback to non-existent provider
    cfg_fallback
        .roles
        .get_mut(&ModelRole::HoleFiller)
        .unwrap()
        .fallback = vec!["missing_fallback".to_string()];
    assert!(cfg_fallback.validate().is_err());
}

#[test]
fn test_toml_roundtrip() {
    let cfg = SawhorseConfig::default();
    let toml_str = toml::to_string_pretty(&cfg).unwrap();
    let parsed: SawhorseConfig = toml::from_str(&toml_str).unwrap();
    assert_eq!(cfg, parsed);
}
