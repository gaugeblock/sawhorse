//! Semantic role router with automatic fallback chain execution.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tracing::{debug, warn};

use crate::config::schema::{RoleConfig, SawhorseConfig, TimeoutConfig};
use crate::error::SawhorseError;
use crate::providers::create_provider;
use crate::traits::{ModelProvider, TokenStream};
use crate::types::{InferenceRequest, InferenceResponse, ModelRole};

/// Router mapping semantic `ModelRole` requests to providers with fallback resilience.
#[derive(Debug, Clone)]
pub struct RoleRouter {
    providers: HashMap<String, Arc<dyn ModelProvider>>,
    roles: HashMap<ModelRole, RoleConfig>,
    timeouts: TimeoutConfig,
}

impl RoleRouter {
    /// Builds a new router from a validated `SawhorseConfig`.
    pub fn from_config(config: SawhorseConfig) -> Result<Self, SawhorseError> {
        let mut providers = HashMap::new();
        for (name, p_cfg) in &config.providers {
            let provider = create_provider(name, p_cfg, &config.timeouts)?;
            providers.insert(name.clone(), provider);
        }

        Ok(Self {
            providers,
            roles: config.roles,
            timeouts: config.timeouts,
        })
    }

    /// Retrieves an instantiated provider by its configured name.
    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn ModelProvider>> {
        self.providers.get(name).cloned()
    }

    /// Returns the global timeout configuration.
    pub fn timeouts(&self) -> &TimeoutConfig {
        &self.timeouts
    }

    /// Resolves the primary provider assigned to a given `ModelRole`.
    pub fn provider_for_role(
        &self,
        role: ModelRole,
    ) -> Result<Arc<dyn ModelProvider>, SawhorseError> {
        let role_cfg = self
            .roles
            .get(&role)
            .ok_or_else(|| SawhorseError::RoleNotConfigured(format!("{role:?}")))?;

        self.get_provider(&role_cfg.provider)
            .ok_or_else(|| SawhorseError::ProviderUnavailable {
                provider: role_cfg.provider.clone(),
                reason: "Provider not found in registry".into(),
            })
    }

    /// Dispatches an inference request through the role's primary provider and fallback chain.
    pub async fn complete(
        &self,
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, SawhorseError> {
        let role_cfg = self
            .roles
            .get(&req.role)
            .ok_or_else(|| SawhorseError::RoleNotConfigured(format!("{:?}", req.role)))?;

        let mut attempts = Vec::new();
        let mut chain = vec![&role_cfg.provider];
        chain.extend(&role_cfg.fallback);

        for provider_name in chain {
            let provider = match self.get_provider(provider_name) {
                Some(p) => p,
                None => {
                    attempts.push(format!("{provider_name}: provider not found"));
                    continue;
                }
            };

            debug!(role = ?req.role, provider = %provider_name, "Attempting inference dispatch");
            match provider.complete(req).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    warn!(
                        role = ?req.role,
                        provider = %provider_name,
                        error = %err,
                        "Provider failed; falling back to next provider if available"
                    );
                    attempts.push(format!("{provider_name}: {err}"));
                }
            }
        }

        Err(SawhorseError::AllFallbacksExhausted {
            role: format!("{:?}", req.role),
            attempts,
        })
    }

    /// Dispatches a streaming inference request through the role's provider chain.
    pub async fn complete_stream(
        &self,
        req: &InferenceRequest,
    ) -> Result<TokenStream, SawhorseError> {
        let role_cfg = self
            .roles
            .get(&req.role)
            .ok_or_else(|| SawhorseError::RoleNotConfigured(format!("{:?}", req.role)))?;

        let mut attempts = Vec::new();
        let mut chain = vec![&role_cfg.provider];
        chain.extend(&role_cfg.fallback);

        for provider_name in chain {
            let provider = match self.get_provider(provider_name) {
                Some(p) => p,
                None => {
                    attempts.push(format!("{provider_name}: not found"));
                    continue;
                }
            };

            match provider.complete_stream(req).await {
                Ok(stream) => return Ok(stream),
                Err(err) => {
                    warn!(
                        role = ?req.role,
                        provider = %provider_name,
                        error = %err,
                        "Provider stream failed; falling back"
                    );
                    attempts.push(format!("{provider_name}: {err}"));
                }
            }
        }

        Err(SawhorseError::AllFallbacksExhausted {
            role: format!("{:?}", req.role),
            attempts,
        })
    }

    /// Performs health checks and latency benchmarks across all registered providers.
    pub async fn ping_all(&self) -> HashMap<String, Result<u64, SawhorseError>> {
        let mut results = HashMap::new();
        for (name, provider) in &self.providers {
            let start = Instant::now();
            let res = match provider.health_check().await {
                Ok(true) => Ok(start.elapsed().as_millis() as u64),
                Ok(false) => Err(SawhorseError::ProviderUnavailable {
                    provider: name.clone(),
                    reason: "Health check returned unhealthy".into(),
                }),
                Err(err) => Err(err),
            };
            results.insert(name.clone(), res);
        }
        results
    }
}
