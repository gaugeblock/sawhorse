//! Request payload building, response parsing, and error enrichment for OpenAI compatibility.

use serde_json::{Value, json};

use crate::config::schema::OpenAiDialect;
use crate::error::SawhorseError;
use crate::types::{GrammarFormat, InferenceRequest, ReasoningEffort};

/// Constructs an OpenAI chat completion JSON payload from an `InferenceRequest`.
pub fn build_payload(
    dialect: OpenAiDialect,
    model_name: &str,
    req: &InferenceRequest,
    stream: bool,
) -> Value {
    let mut messages = Vec::new();
    if let Some(ref sys) = req.system_prompt {
        messages.push(json!({"role": "system", "content": sys}));
    }
    messages.push(json!({"role": "user", "content": &req.prompt}));

    let mut body = json!({
        "model": model_name,
        "messages": messages,
        "max_tokens": req.max_tokens,
        "temperature": req.temperature,
        "stream": stream,
    });

    if let Some(effort) = req.reasoning_effort {
        match effort {
            ReasoningEffort::None => body["reasoning_effort"] = json!("none"),
            ReasoningEffort::Low => body["reasoning_effort"] = json!("low"),
            ReasoningEffort::Medium => body["reasoning_effort"] = json!("medium"),
            ReasoningEffort::High => body["reasoning_effort"] = json!("high"),
        }
    }

    if let Some(ref grammar) = req.grammar_constraint {
        match (dialect, req.grammar_format) {
            (OpenAiDialect::Vllm, Some(GrammarFormat::Regex)) => {
                body["guided_regex"] = json!(grammar)
            }
            (OpenAiDialect::Vllm, _) => body["guided_grammar"] = json!(grammar),
            (OpenAiDialect::LlamaCpp, _) => body["grammar"] = json!(grammar),
            (OpenAiDialect::StandardOpenAi, _) => {
                body["extra_body"] = json!({ "grammar": grammar })
            }
        }
    }
    body
}

/// Extracts text content, usage, and reasoning metadata from a chat completion response body.
pub fn extract_chat_response(body: &Value) -> (String, u32, u32, Option<String>, Option<u32>) {
    let d = &body["data"];
    let text = body["choices"][0]["message"]["content"]
        .as_str()
        .or_else(|| d["choices"][0]["message"]["content"].as_str())
        .unwrap_or_default()
        .to_string();

    let reasoning_trace = body["choices"][0]["message"]["reasoning"]
        .as_str()
        .or_else(|| d["choices"][0]["message"]["reasoning"].as_str())
        .or_else(|| body["choices"][0]["message"]["reasoning_content"].as_str())
        .or_else(|| d["choices"][0]["message"]["reasoning_content"].as_str())
        .map(|s| s.to_string());

    let get_u32 = |k: &str| {
        body["usage"][k]
            .as_u64()
            .or_else(|| d["usage"][k].as_u64())
            .unwrap_or(0) as u32
    };

    let reasoning_tokens = body["usage"]["completion_tokens_details"]["reasoning_tokens"]
        .as_u64()
        .or_else(|| d["usage"]["completion_tokens_details"]["reasoning_tokens"].as_u64())
        .map(|n| n as u32);

    (
        text,
        get_u32("prompt_tokens"),
        get_u32("completion_tokens"),
        reasoning_trace,
        reasoning_tokens,
    )
}

/// Enriches HTTP error responses with model identity and diagnostic guidance.
pub fn format_http_error(
    endpoint: &str,
    status: u16,
    model_name: &str,
    max_tokens: u16,
    body_text: &str,
) -> SawhorseError {
    let mut message = format!("(model: '{model_name}', max_tokens: {max_tokens}) {body_text}");
    if body_text.contains("empty response content")
        || (status == 500 && (body_text.is_empty() || body_text == "{}"))
    {
        message.push_str(&format!(
            " [diagnostic: reasoning models (such as '{model_name}') consume tokens during the thinking trace; max_tokens={max_tokens} was likely exhausted before completion content could be generated. Increase max_tokens or shorten prompt context]"
        ));
    }

    SawhorseError::Http {
        endpoint: endpoint.to_string(),
        status,
        message,
    }
}
