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
