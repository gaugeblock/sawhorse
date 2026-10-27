//! Configuration loading, hierarchical discovery, and validation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod env_loader;
pub mod schema;
pub use env_loader::resolve_secret_from_env_file;
pub use schema::*;

use crate::error::SawhorseError;
use crate::types::{ExecutionMode, ModelRole};

impl SawhorseConfig {
    /// Loads and parses configuration from a specific file path.
    pub fn load_from_path(path: &Path) -> Result<Self, SawhorseError> {
        let content = std::fs::read_to_string(path).map_err(|err| {
            SawhorseError::Config(format!("Failed to read {}: {err}", path.display()))
        })?;
        let val: toml::Value = toml::from_str(&content).map_err(|err| {
            SawhorseError::Config(format!("Failed to parse {}: {err}", path.display()))
        })?;

        let config: Self = if let Some(sub) = val.get("sawhorse") {
            sub.clone().try_into().map_err(|err| {
                SawhorseError::Config(format!(
                    "Failed to parse [sawhorse] in {}: {err}",
                    path.display()
                ))
            })?
        } else {
            val.try_into().map_err(|err| {
                SawhorseError::Config(format!("Failed to parse {}: {err}", path.display()))
            })?
        };

        config.validate()?;
        Ok(config)
    }

    /// Discovers configuration using hierarchical search paths.
    ///
    /// Searches:
    /// 1. `.gaugeblock/config.toml` in the current working directory.
    /// 2. `./sawhorse.toml` in the current working directory.
    /// 3. `~/.config/sawhorse/sawhorse.toml` (or platform equivalent).
    /// 4. Falls back to default configuration.
    pub fn discover() -> Result<Self, SawhorseError> {
        let gb_config = PathBuf::from(".gaugeblock").join("config.toml");
        if gb_config.is_file()
            && let Ok(cfg) = Self::load_from_path(&gb_config)
        {
            return Ok(cfg);
        }

        let local_path = PathBuf::from("sawhorse.toml");
        if local_path.is_file() {
            return Self::load_from_path(&local_path);
        }

        if let Ok(home) = std::env::var("HOME") {
            let user_config = PathBuf::from(home)
                .join(".config")
                .join("sawhorse")
                .join("sawhorse.toml");
            if user_config.is_file() {
                return Self::load_from_path(&user_config);
            }
        }

        Ok(Self::default())
    }

    /// Validates that all roles point to valid providers and fallbacks are consistent.
    pub fn validate(&self) -> Result<(), SawhorseError> {
        for (role, role_cfg) in &self.roles {
            if !self.providers.contains_key(&role_cfg.provider) {
                return Err(SawhorseError::Config(format!(
                    "Role '{role:?}' references unknown primary provider '{}'",
                    role_cfg.provider
                )));
            }
            for fallback in &role_cfg.fallback {
                if !self.providers.contains_key(fallback) {
                    return Err(SawhorseError::Config(format!(
                        "Role '{role:?}' references unknown fallback provider '{fallback}'"
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Default for SawhorseConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();
        let mut roles = HashMap::new();

        // Default Local Provider
        providers.insert(
            "local".to_string(),
            ProviderConfig {
                mode: ExecutionMode::LocalRuntime {
                    model_id: "default-local".to_string(),
                    endpoint_or_path: "http://127.0.0.1:8080".to_string(),
                },
                dialect: Some(OpenAiDialect::LlamaCpp),
                headers: None,
                timeout_ms: None,
            },
        );

        // Default OpenAI-compatible Cloud Provider
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                mode: ExecutionMode::OpenAiCompatible {
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key_env: "OPENAI_API_KEY".to_string(),
                    model_name: "gpt-4o-mini".to_string(),
                },
                dialect: Some(OpenAiDialect::StandardOpenAi),
                headers: None,
                timeout_ms: None,
            },
        );

        // Map roles with local-first, cloud-fallback policy
        let default_roles = [
            ModelRole::Planner,
            ModelRole::System1Router,
            ModelRole::HoleFiller,
            ModelRole::Oracle,
        ];
        for role in default_roles {
            roles.insert(
                role,
                RoleConfig {
                    provider: "local".to_string(),
                    fallback: vec!["openai".to_string()],
                },
            );
        }

        Self {
            providers,
            roles,
            timeouts: TimeoutConfig::default(),
        }
    }
}
