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
impl CatalogSource for BundledSource {
    fn limits(&self, provider: &str, model: &str) -> Option<ModelLimits> {
        // Try the configured pairing first, then the qualified wire id, then the id alone. The
        // configured provider name is a local alias (`universal`), while the catalog keys on the
        // upstream provider, and the wire id already carries its provider prefix.
        let found = self
            .snapshot
            .get_model(provider, model)
            .or_else(|| self.snapshot.find_model(model))
            .or_else(|| {
                let (_, tail) = model.split_once('/')?;
                self.snapshot.find_model(&format!("{provider}/{tail}"))
            })?;

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
}
