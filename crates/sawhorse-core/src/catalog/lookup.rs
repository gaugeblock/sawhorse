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
