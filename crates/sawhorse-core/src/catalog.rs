//! Catalog-backed model limits (`GB-217`).
//!
//! Two sources can answer "what is this model's window": the operator's config, and the published
//! catalog. Config wins, because it is the only lever an operator has when a catalog entry is wrong or
//! stale; the catalog fills the gaps that a hand-written config leaves blank, which is exactly the hole
//! `GB-215` had to paper over by declaring `context_window` by hand.
//!
//! The catalog is optional. With the `catalog` feature off nothing here links the crate, `NullSource`
//! answers "nothing known", and every existing test and offline build keeps its dependency graph. That
//! matters for the Tier 2/3 gates: they must never need the network.

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;

mod lookup;

pub use lookup::{knows, pass_eligibility};

use crate::config::schema::ModelLimits;
// Only the feature-gated mapping names this type.
#[cfg(feature = "catalog")]
use crate::types::ReasoningEffort;

/// Answers what the published catalog knows about a model.
///
/// Deliberately narrow: limits now, and whatever the callers demonstrably need next. A source that
/// cannot answer returns `None` rather than a guess, so the caller's fallback stays visible.
pub trait CatalogSource {
    /// Declared limits for `model` served by `provider`, when the catalog knows the pair.
    ///
    /// `provider` is the configured provider name (e.g. `universal`) and `model` the wire id
    /// (e.g. `cline-pass/deepseek-v4.1-flash-1017`). Implementations decide how to map the configured
    /// name onto the catalog's provider ids - the bundled source treats the configured name as an
    /// alias when it does not match a catalog provider.
    fn limits(&self, provider: &str, model: &str) -> Option<ModelLimits>;

    /// Gateway output budget for a prompt of `prompt_tokens`, when this source can compute one.
    ///
    /// The calculation is not a simple subtraction: it applies a 32,000-token default cap, a
    /// 1,024-token reserve, a reasoning floor when a thinking budget is set, and reports context
    /// overflow as an error rather than returning a zero budget. A source that cannot compute it
    /// returns `None`, which is how callers know to fall back to [`ModelLimits::output_budget`].
    fn gateway_output_budget(
        &self,
        _provider: &str,
        _model: &str,
        _prompt_tokens: u32,
        _requested: Option<u16>,
    ) -> Option<u16> {
        None
    }

    /// True when the provider requires this model to be streamed.
    ///
    /// Constrained models (some gateways route them through a streaming-only backend) answer a
    /// non-streamed request with an empty body. Knowing this up front means never sending that
    /// request: the caller streams first instead of failing and retrying (`GB-214`, `GB-217` step 4).
    fn requires_streaming(&self, _provider: &str, _model: &str) -> bool {
        false
    }
}

/// A source that knows nothing. The offline default, and what tests use when they are not exercising
/// catalog behaviour.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NullSource;

impl CatalogSource for NullSource {
    fn limits(&self, _provider: &str, _model: &str) -> Option<ModelLimits> {
        None
    }
}

/// Resolves effective limits: config override first, then the catalog, then nothing.
///
/// Field by field rather than all-or-nothing, so an operator who pins only `max_output_tokens` keeps
/// the catalog's context window instead of losing every field.
#[must_use]
pub fn resolve_limits(
    declared: &ModelLimits,
    provider: &str,
    model: &str,
    source: &dyn CatalogSource,
) -> ModelLimits {
    let catalog = source.limits(provider, model).unwrap_or_default();
    ModelLimits {
        context_window: declared.context_window.or(catalog.context_window),
        max_output_tokens: declared.max_output_tokens.or(catalog.max_output_tokens),
        reasoning_effort: declared.reasoning_effort.or(catalog.reasoning_effort),
    }
}

/// Output budget for one request: the catalog's gateway calculation when available, otherwise the
/// declared-limits approximation.
///
/// Order matters. The catalog owns the real rules - cap, reserve, reasoning floor and overflow - so it
/// answers first. [`ModelLimits::output_budget`] is the fallback for a model the catalog does not know,
/// and remains the arithmetic `GB-215` shipped: it knows nothing about a reasoning floor because it has
/// no way to see one.
#[must_use]
pub fn output_budget(
    declared: &ModelLimits,
    provider: &str,
    model: &str,
    prompt_tokens: u32,
    requested: Option<u16>,
    source: &dyn CatalogSource,
) -> Option<u16> {
    if let Some(budget) = source.gateway_output_budget(provider, model, prompt_tokens, requested) {
        return Some(budget);
    }
    resolve_limits(declared, provider, model, source).output_budget(prompt_tokens)
}

/// Converts a catalog `u64` window into the contract's `u32`, saturating rather than wrapping.
///
/// A window above `u32::MAX` is beyond any request the suite can build, so saturation is the honest
/// answer; a silent wrap would produce a tiny budget and truncate every answer without saying why.
#[cfg(feature = "catalog")]
fn clamp_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Maps the catalog's effort enum onto ours. Both are the same closed six-tier scale (`GB-221`), so
/// this is a plain translation rather than a policy decision.
#[cfg(feature = "catalog")]
fn map_effort(effort: models_catalog::ReasoningEffort) -> ReasoningEffort {
    match effort {
        models_catalog::ReasoningEffort::None => ReasoningEffort::None,
        models_catalog::ReasoningEffort::Low => ReasoningEffort::Low,
        models_catalog::ReasoningEffort::Medium => ReasoningEffort::Medium,
        models_catalog::ReasoningEffort::High => ReasoningEffort::High,
        models_catalog::ReasoningEffort::XHigh => ReasoningEffort::XHigh,
        models_catalog::ReasoningEffort::Max => ReasoningEffort::Max,
    }
}

/// The catalog snapshot compiled into the binary.
#[cfg(feature = "catalog")]
#[derive(Debug)]
pub struct BundledSource {
    snapshot: models_catalog::CatalogSnapshot,
}

#[cfg(feature = "catalog")]
impl BundledSource {
    /// Loads the bundled snapshot. No network, no disk, no runtime.
    ///
    /// # Errors
    ///
    /// Propagates the crate's [`models_catalog::CatalogError`] when the embedded snapshot cannot be
    /// decompressed - a build defect rather than a runtime condition, which is why it is surfaced
    /// instead of degraded.
    pub fn load() -> Result<Self, models_catalog::CatalogError> {
        Ok(Self {
            snapshot: models_catalog::CatalogSnapshot::bundled()?,
        })
    }
}

#[cfg(feature = "catalog")]
impl BundledSource {
    /// Finds a model, tolerating the three id shapes the suite actually produces.
    ///
    /// Tries the configured pairing first, then the qualified wire id, then the id re-qualified under
    /// the configured provider. The configured provider name is a local alias (`universal`) while the
    /// catalog keys on the upstream provider, and a wire id already carries its own prefix, so neither
    /// shape matches on its own.
    fn find(&self, provider: &str, model: &str) -> Option<&models_catalog::ModelInfo> {
        self.snapshot
            .get_model(provider, model)
            .or_else(|| self.snapshot.find_model(model))
            .or_else(|| {
                let (_, tail) = model.split_once('/')?;
                self.snapshot.find_model(&format!("{provider}/{tail}"))
            })
    }
}

#[cfg(feature = "catalog")]
impl CatalogSource for BundledSource {
    fn limits(&self, provider: &str, model: &str) -> Option<ModelLimits> {
        let found = self.find(provider, model)?;

        Some(ModelLimits {
            context_window: found.context_window.map(clamp_u32),
            // Saturates at `u16::MAX` on purpose: a catalog can advertise 384k output tokens, but
            // `InferenceRequest::max_tokens` is a `u16`, so anything above 65,535 could never be
            // requested. Clamping here keeps the declared limit inside what the wire can express
            // instead of storing a number the request builder would silently truncate later.
            max_output_tokens: Some(u16::try_from(found.max_tokens).unwrap_or(u16::MAX)),
            reasoning_effort: found.default_reasoning_effort.map(map_effort),
        })
    }

    fn gateway_output_budget(
        &self,
        provider: &str,
        model: &str,
        prompt_tokens: u32,
        requested: Option<u16>,
    ) -> Option<u16> {
        let found = self.find(provider, model)?;
        // Computed from the crate's own `u64` limit, never from our clamped `u16`: a model advertising
        // 384k output would otherwise be budgeted as if it capped at 65,535 before the cap was applied.
        // No reasoning budget yet - coupling effort to the budget is step 5 of the plan.
        match found.resolve_gateway_max_output_tokens(
            u64::from(prompt_tokens),
            requested.map(u64::from),
            None,
        ) {
            Ok(budget) => Some(u16::try_from(budget).unwrap_or(u16::MAX)),
            Err(overflow) => {
                // A prompt that cannot fit is a real answer, not a silent zero budget: report it and
                // let the caller fall back so the failure surfaces upstream.
                tracing::warn!(
                    model = %model,
                    prompt_tokens = prompt_tokens,
                    context_window = overflow.context_window,
                    "prompt does not fit the model's context window"
                );
                None
            }
        }
    }

    fn requires_streaming(&self, provider: &str, model: &str) -> bool {
        self.find(provider, model)
            .is_some_and(|found| found.requires_streaming)
    }
}
