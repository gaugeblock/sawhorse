//! Verification tests for Section 5 of `CONTRACTS.md`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sawhorse_core::types::*;

#[test]
fn test_execution_mode_serialization() {
    let mode_candle = ExecutionMode::InHouseCandle {
        model_id: "gaugeblock-micro-1".to_string(),
        weights_path: "/tmp/weights.safetensors".to_string(),
    };
    let json_candle = serde_json::to_string(&mode_candle).unwrap();
    assert!(json_candle.contains(r#""mode":"in_house_candle""#));

    let mode_local = ExecutionMode::LocalRuntime {
        model_id: "qwen2.5-coder-7b".to_string(),
        endpoint_or_path: "http://localhost:8080".to_string(),
    };
    let json_local = serde_json::to_string(&mode_local).unwrap();
    assert!(json_local.contains(r#""mode":"local_runtime""#));

    let mode_openai = ExecutionMode::OpenAiCompatible {
        base_url: "https://api.openai.com/v1".to_string(),
        api_key_env: "OPENAI_API_KEY".to_string(),
        model_name: "gpt-4o-mini".to_string(),
    };
    let json_openai = serde_json::to_string(&mode_openai).unwrap();
    assert!(json_openai.contains(r#""mode":"open_ai_compatible""#));
}

#[test]
fn test_model_roles_and_grammar_formats() {
    let roles = [
        (ModelRole::Planner, "\"planner\""),
        (ModelRole::System1Router, "\"system1_router\""),
        (ModelRole::HoleFiller, "\"hole_filler\""),
        (ModelRole::Oracle, "\"oracle\""),
    ];

    for (role, expected_json) in roles {
        assert_eq!(serde_json::to_string(&role).unwrap(), expected_json);
    }

    let grammars = [
        (GrammarFormat::Gbnf, "\"gbnf\""),
        (GrammarFormat::Lark, "\"lark\""),
        (GrammarFormat::Regex, "\"regex\""),
    ];

    for (format, expected_json) in grammars {
        assert_eq!(serde_json::to_string(&format).unwrap(), expected_json);
    }
}

#[test]
fn test_inference_request_and_response_serde() {
    let req = InferenceRequest::new(ModelRole::HoleFiller, "let active = ?;")
        .with_gbnf_grammar("root ::= [a-z]+")
        .with_system_prompt("You are a hole filler");

    let json_str = serde_json::to_string_pretty(&req).unwrap();
    let deserialized: InferenceRequest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(req, deserialized);

    let resp = InferenceResponse {
        text: "user.id()".to_string(),
        prompt_tokens: 15,
        completion_tokens: 3,
        latency_ms: 42,
        model_used: "gaugeblock-micro-1".to_string(),
        reasoning_trace: Some("analyzing user syntax".to_string()),
        reasoning_tokens: Some(10),
    };

    let resp_json = serde_json::to_string(&resp).unwrap();
    let resp_deserialized: InferenceResponse = serde_json::from_str(&resp_json).unwrap();
    assert_eq!(resp, resp_deserialized);
}
