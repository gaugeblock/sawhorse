//! Where the ClinePass model set comes from.
//!
//! ClinePass is a *curated* subscription set, not a namespace: prefixing an arbitrary catalogue id
//! with `cline-pass/` does not make it pass-eligible, and the set moves as models are added and
//! deprecated. This file used to hold a hand-copied list of fifteen ids with a capture date and a
//! scraper to refresh it (`GB-210`), and the list was stale within days of being written.
//!
//! The set now comes from the catalog (`GB-217` step 7): `catalog::pass_eligibility` reads the
//! curated overlay's billing tier, and answers `None` when it has not heard of an id rather than
//! guessing `false`. The ids themselves are no longer duplicated here at all.

/// Documentation for the ClinePass subscription, kept as the human reference.
pub const CLINE_PASS_DOCS: &str = "https://docs.cline.bot/getting-started/clinepass#models";

#[cfg(test)]
mod tests {
    use crate::catalog::pass_eligibility;

    #[test]
    fn test_without_catalog_data_eligibility_is_unknown_not_false() {
        // The distinction that matters: the deleted list answered `false` for every id it had not
        // heard of, which reported eligible models as ineligible.
        let answer = pass_eligibility("cline-pass/deepseek-v4.1-flash-1017");
        #[cfg(not(feature = "catalog"))]
        assert_eq!(answer, None);
        #[cfg(feature = "catalog")]
        assert_eq!(
            answer,
            Some(true),
            "the curated feed marks this id subscription-tier"
        );
    }
}
