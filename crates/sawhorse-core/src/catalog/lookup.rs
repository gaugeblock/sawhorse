//! What the catalog knows about an id, as opposed to what an endpoint will serve (`GB-217`).
//!
//! These answers are deliberately `Option`al or explicit: the hand-maintained list this replaced
//! answered `false` for every id it had not heard of, which reported eligible models as
//! ineligible and made a missing answer indistinguishable from a negative one.

#[cfg(feature = "catalog")]
use super::BundledSource;

/// Whether the catalog knows this model id at all.
///
/// `GET /models` lists what an endpoint will serve, which is not the same as what exists: the
/// configured wire id is absent from that list and present in the catalog, and reporting only the
/// endpoint produced a false negative for months (`GB-210`, `GB-217` step 6).
#[must_use]
pub fn knows(model_id: &str) -> bool {
    #[cfg(feature = "catalog")]
    {
        BundledSource::load()
            .ok()
            .and_then(|source| source.find("", model_id).map(|_| ()))
            .is_some()
    }
    #[cfg(not(feature = "catalog"))]
    {
        let _ = model_id;
        false
    }
}

/// Whether the curated feed places `model_id` in the subscription tier.
///
/// `None` means "the catalog does not know this id", which is different from `Some(false)` and is why
/// the return type is not a bool: the hand-maintained list this replaces answered `false` for every id
/// it had not heard of, so a model could be pass-eligible and reported as not (`GB-210`, `GB-217`).
#[must_use]
pub fn pass_eligibility(model_id: &str) -> Option<bool> {
    #[cfg(feature = "catalog")]
    {
        let source = BundledSource::load().ok()?;
        let model = source.find("", model_id)?;
        Some(matches!(
            model.billing_tier,
            Some(models_catalog::BillingTier::Subscription)
        ))
    }
    #[cfg(not(feature = "catalog"))]
    {
        let _ = model_id;
        None
    }
}

/// How to express `effort` for this model on the wire, when the catalog can say.
///
/// `None` means the catalog has no opinion - an unknown id, or the feature is off - and the caller
/// should fall back to the canonical `reasoning_effort` field.
#[must_use]
pub fn wire_reasoning(
    model_id: &str,
    effort: crate::types::ReasoningEffort,
    max_output_tokens: u16,
) -> Option<crate::catalog::WireReasoning> {
    // `None` is a caller policy decision - "do not spend budget on thinking" - not an effort to be
    // clamped into the nearest supported tier. The catalog clamps it upward when a model has no
    // declared options (it assumes Low/Medium/High), which would switch thinking ON for a caller who
    // asked for it off; that is how GB-206/GB-212 lost whole budgets.
    if effort == crate::types::ReasoningEffort::None {
        return Some(crate::catalog::WireReasoning::Disabled);
    }

    #[cfg(feature = "catalog")]
    {
        let source = BundledSource::load().ok()?;
        let model = source.find("", model_id)?;
        let requested = map_effort(effort);
        let effort = model.normalize_reasoning_effort(requested)?;
        let params = model.to_wire_reasoning_params(
            source.protocol_of(model),
            effort,
            u64::from(max_output_tokens),
        );
        Some(match params {
            models_catalog::WireReasoningParams::Disabled => {
                crate::catalog::WireReasoning::Disabled
            }
            models_catalog::WireReasoningParams::OpenAiEffort { reasoning_effort } => {
                crate::catalog::WireReasoning::Effort(match reasoning_effort.as_str() {
                    "low" => "low",
                    "medium" => "medium",
                    "high" => "high",
                    "xhigh" => "xhigh",
                    "max" => "max",
                    _ => "none",
                })
            }
            models_catalog::WireReasoningParams::ThinkingToggle { enabled } => {
                crate::catalog::WireReasoning::ThinkingToggle { enabled }
            }
            models_catalog::WireReasoningParams::AnthropicThinking { budget_tokens } => {
                crate::catalog::WireReasoning::ThinkingBudget { budget_tokens }
            }
            models_catalog::WireReasoningParams::GoogleThinking { thinking_budget } => {
                crate::catalog::WireReasoning::GoogleBudget { thinking_budget }
            }
        })
    }
    #[cfg(not(feature = "catalog"))]
    {
        let _ = (model_id, effort, max_output_tokens);
        None
    }
}

#[cfg(feature = "catalog")]
fn map_effort(effort: crate::types::ReasoningEffort) -> models_catalog::ReasoningEffort {
    use crate::types::ReasoningEffort as O;
    use models_catalog::ReasoningEffort as C;
    match effort {
        O::None => C::None,
        O::Low => C::Low,
        O::Medium => C::Medium,
        O::High => C::High,
        O::XHigh => C::XHigh,
        O::Max => C::Max,
    }
}
