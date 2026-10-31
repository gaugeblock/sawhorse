//! Tests for catalog-backed limits (`GB-217`), split from `catalog.rs` to stay inside the file-size law.
//!
//! The precedence tests use a stub source so they run without the `catalog` feature and without the
//! crate's snapshot: what is being pinned is the field-by-field merge rule, not the catalog contents.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{CatalogSource, NullSource, resolve_limits};
use crate::config::schema::ModelLimits;
use crate::types::ReasoningEffort;

/// A source that answers with whatever it was constructed with.
struct Stub(Option<ModelLimits>);

impl CatalogSource for Stub {
    fn limits(&self, _provider: &str, _model: &str) -> Option<ModelLimits> {
        self.0
    }
}

fn declared(
    context: Option<u32>,
    output: Option<u16>,
    effort: Option<ReasoningEffort>,
) -> ModelLimits {
    ModelLimits {
        context_window: context,
        max_output_tokens: output,
        reasoning_effort: effort,
    }
}

#[test]
fn test_config_wins_over_the_catalog_field_by_field() {
    let catalog = Stub(Some(declared(
        Some(1_000_000),
        Some(60_000),
        Some(ReasoningEffort::High),
    )));

    // Operator pins the window only; the catalog's other fields survive.
    let merged = resolve_limits(
        &declared(Some(200_000), None, None),
        "universal",
        "m",
        &catalog,
    );
    assert_eq!(merged.context_window, Some(200_000), "config wins");
    assert_eq!(merged.max_output_tokens, Some(60_000), "catalog fills");
    assert_eq!(merged.reasoning_effort, Some(ReasoningEffort::High));

    // Operator pins everything; the catalog contributes nothing.
    let all = resolve_limits(
        &declared(Some(1), Some(2), Some(ReasoningEffort::None)),
        "universal",
        "m",
        &catalog,
    );
    assert_eq!(all.context_window, Some(1));
    assert_eq!(all.max_output_tokens, Some(2));
    assert_eq!(all.reasoning_effort, Some(ReasoningEffort::None));
}

#[test]
fn test_the_catalog_fills_a_wholly_blank_declaration() {
    let catalog = Stub(Some(declared(
        Some(1_000_000),
        Some(64_000),
        Some(ReasoningEffort::Medium),
    )));
    let merged = resolve_limits(&ModelLimits::default(), "universal", "m", &catalog);

    assert_eq!(merged.context_window, Some(1_000_000));
    assert_eq!(merged.max_output_tokens, Some(64_000));
    assert_eq!(merged.reasoning_effort, Some(ReasoningEffort::Medium));
}

#[test]
fn test_an_unknown_model_keeps_the_declaration_rather_than_inventing_limits() {
    let merged = resolve_limits(
        &declared(Some(1_000_000), Some(8_192), None),
        "universal",
        "not-in-the-catalog",
        &Stub(None),
    );

    assert_eq!(merged.context_window, Some(1_000_000));
    assert_eq!(merged.max_output_tokens, Some(8_192));
    assert_eq!(merged.reasoning_effort, None, "silence, not a guess");
}

#[test]
fn test_the_null_source_knows_nothing() {
    assert_eq!(NullSource.limits("universal", "anything"), None);
    let merged = resolve_limits(
        &ModelLimits::default(),
        "universal",
        "anything",
        &NullSource,
    );
    assert_eq!(merged, ModelLimits::default());
}

#[cfg(feature = "catalog")]
#[test]
fn test_the_bundled_snapshot_resolves_catalog_models() {
    let source = super::BundledSource::load().expect("bundled snapshot decompresses");

    // The configured provider name (`universal`) is a local alias that the catalog does not know,
    // and the wire id carries its own `cline-pass/` prefix, so this exercises the alias fallbacks.
    let base = source.limits("universal", "cline-pass/deepseek-v4.1-flash");
    let suffixed = source.limits("universal", "cline-pass/deepseek-v4.1-flash-1017");
    println!("base id     -> {base:?}");
    println!("1017 id     -> {suffixed:?}");

    let base = base.expect("models.dev lists cline-pass/deepseek-v4.1-flash");
    assert_eq!(
        base.context_window,
        Some(1_000_000),
        "1M window from the catalog"
    );
    assert_eq!(
        base.max_output_tokens,
        Some(u16::MAX),
        "384k output saturates to the u16 wire maximum rather than wrapping"
    );
}

#[test]
fn test_a_source_without_a_gateway_rule_falls_back_to_the_declared_arithmetic() {
    // The stub cannot compute a gateway budget, so the composition must use GB-215's formula:
    // min(output cap, window - prompt - 64), floored at 256.
    let source = Stub(Some(declared(Some(10_000), Some(32_768), None)));
    let budget = super::output_budget(
        &ModelLimits::default(),
        "universal",
        "m",
        9_000,
        None,
        &source,
    );
    assert_eq!(budget, Some(936), "window minus prompt minus headroom");
}

#[test]
fn test_nothing_known_means_no_budget_rather_than_a_guess() {
    let budget = super::output_budget(
        &ModelLimits::default(),
        "universal",
        "m",
        1_000,
        None,
        &NullSource,
    );
    assert_eq!(budget, None);
}

#[cfg(feature = "catalog")]
#[test]
fn test_the_catalog_owns_the_budget_when_it_can_compute_one() {
    use super::{BundledSource, CatalogSource};

    let source = BundledSource::load().expect("bundled snapshot");
    let window = 1_048_576u32;

    // A small prompt leaves the 32,000-token gateway cap as the binding constraint.
    let small = source
        .gateway_output_budget(
            "universal",
            "cline-pass/deepseek-v4.1-flash-1017",
            1_000,
            None,
        )
        .expect("a known model answers");
    assert_eq!(
        small, 32_000,
        "the gateway cap wins over a 384k model limit"
    );

    // An explicit request above the cap is honoured, still bounded by the model.
    let asked = source
        .gateway_output_budget(
            "universal",
            "cline-pass/deepseek-v4.1-flash-1017",
            1_000,
            Some(50_000),
        )
        .expect("a known model answers");
    assert_eq!(asked, 50_000, "an explicit request raises the cap");

    // Near the window, the remaining context wins over every cap.
    let tight = source
        .gateway_output_budget(
            "universal",
            "cline-pass/deepseek-v4.1-flash-1017",
            window - 2_024,
            None,
        )
        .expect("still fits");
    assert_eq!(tight, 1_000, "window - prompt - 1024 reserve");

    // A prompt that cannot fit reports overflow instead of inventing a budget.
    assert_eq!(
        source.gateway_output_budget(
            "universal",
            "cline-pass/deepseek-v4.1-flash-1017",
            window,
            None
        ),
        None,
        "overflow is an answer, not a zero budget"
    );
}

#[test]
fn test_a_source_without_catalog_data_flags_nothing_for_streaming() {
    assert!(!NullSource.requires_streaming("universal", "anything"));
    assert!(!Stub(None).requires_streaming("universal", "anything"));
}

#[cfg(feature = "catalog")]
#[test]
fn test_streaming_flags_report_what_the_catalog_says() {
    use super::{BundledSource, CatalogSource};

    let source = BundledSource::load().expect("bundled snapshot");
    for id in [
        "cline-pass/deepseek-v4.1-flash-1017",
        "cline-pass/deepseek-v4.1-flash",
        "cline-pass/glm-5.3",
        "cline-pass/kimi-k3",
        "cline-pass/qwen3.8-max",
        "cline-pass/deepseek-v4-pro",
        "cline-pass/minimax-m3",
    ] {
        println!(
            "requires_streaming {id} -> {}",
            source.requires_streaming("universal", id)
        );
    }
    // An unknown model is not evidence of a constraint.
    assert!(!source.requires_streaming("universal", "not-a-model"));
}
