//! Payload shape for reasoning effort (GB-217 step 5).
//!
//! The request dump that cracked GB-214 showed three dialect spellings of one instruction in the
//! same body: `reasoning_effort`, `thinking` and `reasoning`. The gateway tolerated all three, which
//! is why it survived; what it could not do was guarantee they agreed. One canonical field is the fix,
//! and this test is what keeps a fourth spelling from creeping in.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::{GrammarFormat, InferenceRequest, ModelRole, ReasoningEffort};
use sawhorse_core::config::schema::OpenAiDialect;
use sawhorse_core::providers::openai_compat::payload::build_payload;

fn request(effort: Option<ReasoningEffort>) -> InferenceRequest {
    InferenceRequest {
        role: ModelRole::HoleFiller,
        prompt: "fill it".to_string(),
        system_prompt: None,
        grammar_constraint: None,
        grammar_format: Option::<GrammarFormat>::None,
        max_tokens: 4096,
        temperature: 0.2,
        reasoning_effort: effort,
    }
}

#[test]
fn test_exactly_one_reasoning_field_is_emitted() {
    for effort in [
        ReasoningEffort::None,
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::XHigh,
        ReasoningEffort::Max,
    ] {
        let body = build_payload(
            OpenAiDialect::StandardOpenAi,
            "test-model",
            &request(Some(effort)),
            false,
        );
        let object = body.as_object().expect("object");

        let present: Vec<&String> = object
            .keys()
            .filter(|key| matches!(key.as_str(), "reasoning_effort" | "reasoning" | "thinking"))
            .collect();
        assert_eq!(
            present,
            vec!["reasoning_effort"],
            "expected only the canonical field for {effort:?}, got {present:?}"
        );
        assert_eq!(
            object["reasoning_effort"],
            serde_json::json!(effort.wire_token()),
            "the field must carry the canonical token"
        );
    }
}

#[test]
fn test_no_reasoning_field_when_effort_is_absent() {
    let body = build_payload(
        OpenAiDialect::StandardOpenAi,
        "test-model",
        &request(None),
        false,
    );
    let object = body.as_object().expect("object");
    assert!(
        !object.contains_key("reasoning_effort") && !object.contains_key("thinking"),
        "an absent effort must mean an absent field, not a guess: {object:?}"
    );
}
