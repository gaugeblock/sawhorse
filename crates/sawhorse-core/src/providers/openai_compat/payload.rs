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
            ReasoningEffort::None => {
                body["reasoning_effort"] = json!("none");
                body["thinking"] = json!({ "type": "disabled" });
                body["reasoning"] = json!({ "effort": "none" });
            }
            ReasoningEffort::Low => {
                body["reasoning_effort"] = json!("low");
                body["thinking"] = json!({ "type": "enabled" });
                body["reasoning"] = json!({ "effort": "low" });
            }
            ReasoningEffort::Medium => {
                body["reasoning_effort"] = json!("medium");
                body["thinking"] = json!({ "type": "enabled" });
                body["reasoning"] = json!({ "effort": "medium" });
            }
            ReasoningEffort::High => {
                body["reasoning_effort"] = json!("high");
                body["thinking"] = json!({ "type": "enabled" });
                body["reasoning"] = json!({ "effort": "high" });
            }
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
        // Report what was observed and how to tell the causes apart. This used to assert that
        // max_tokens was exhausted, which is one of at least three explanations and was wrong for the
        // case that provoked it: a repeated call showed 2 successes and 3 failures for the same
        // prompt, so the fault was transient (`GB-214`, `GB-215`).
        message.push_str(&format!(
            " [diagnostic: the provider returned status {status} with an empty `content`. That is an              observation, not a cause. Three explanations fit and they are distinguishable: (1) a              transient provider or gateway fault - retry the identical request before changing              anything; (2) the model spent its budget on a thinking trace - check              `usage.completion_tokens_details.reasoning_tokens` against max_tokens={max_tokens} on a              call that did return, and lower the reasoning effort or raise the budget; (3) the payload              was rejected or filtered by the gateway - reduce the prompt. Retrying first is the only              one of these that costs nothing.]"
        ));
    }

    SawhorseError::Http {
        endpoint: endpoint.to_string(),
        status,
        message,
    }
}
