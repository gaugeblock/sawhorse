//! The documented ClinePass model set.
//!
//! ClinePass is a *curated* subscription set, not a namespace: prefixing an arbitrary catalogue id
//! with `cline-pass/` does not make it pass-eligible, and the set changes as models are added and
//! deprecated (`GB-210`). Discovery alone therefore cannot answer "may I use this" - the catalogue
//! lists 439 ids, of which the pass covers the ones below.
//!
//! Source: <https://docs.cline.bot/getting-started/clinepass#models>, captured 2026-10-29.
//! Refresh with `scripts/refresh-cline-pass.sh`, which re-reads the page's markdown form. Do not
//! extend this list by inference; an id is here only because the documentation says so.

/// Documentation URL the list was captured from.
pub const CLINE_PASS_DOCS: &str = "https://docs.cline.bot/getting-started/clinepass#models";

/// Date the list below was captured, so a stale list is recognisable rather than authoritative.
pub const CLINE_PASS_CAPTURED: &str = "2026-10-29";

/// Model ids the subscription covers, sorted.
pub const CLINE_PASS_MODELS: &[&str] = &[
    "cline-pass/deepseek-v4-pro",
    "cline-pass/deepseek-v4.1-flash",
    "cline-pass/deepseek-v4.1-flash-1017",
    "cline-pass/glm-5.3",
    "cline-pass/glm-5.3-flash",
    "cline-pass/kimi-k3",
    "cline-pass/kimi-k3.1-pro",
    "cline-pass/mimo-v2.6-flash",
    "cline-pass/mimo-v2.6-pro",
    "cline-pass/minimax-m3",
    "cline-pass/minimax-m3.1-flash",
    "cline-pass/muse-spark-1.3-contributor",
    "cline-pass/qwen3.7-max",
    "cline-pass/qwen3.7-plus",
    "cline-pass/qwen3.8-max",
];

/// True when `model_id` is in the documented pass set for the captured date.
#[must_use]
pub fn is_pass_model(model_id: &str) -> bool {
    CLINE_PASS_MODELS.contains(&model_id)
}

/// Models that were removed from the pass, kept so a stale config can be explained rather than
/// looking like a typo.
pub const CLINE_PASS_DEPRECATED: &[&str] = &[
    "cline-pass/deepseek-v4-flash",
    "cline-pass/glm-5.2",
    "cline-pass/kimi-k2.6",
    "cline-pass/kimi-k2.7-code",
];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{CLINE_PASS_MODELS, is_pass_model};

    #[test]
    fn test_the_requested_model_is_in_the_documented_set() {
        assert!(is_pass_model("cline-pass/deepseek-v4.1-flash-1017"));
    }

    #[test]
    fn test_a_catalogue_id_is_not_pass_eligible_by_prefixing() {
        assert!(
            !is_pass_model("cline-pass/openai/gpt-6.1-terra"),
            "the pass is a curated set, not a namespace (GB-210)"
        );
        assert!(!is_pass_model("deepseek/deepseek-v4.1-flash-1017"));
    }

    #[test]
    fn test_the_list_is_sorted_and_unique() {
        let mut sorted = CLINE_PASS_MODELS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, CLINE_PASS_MODELS.to_vec());
    }
}
