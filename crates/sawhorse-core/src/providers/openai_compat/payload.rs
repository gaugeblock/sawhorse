//! Request payload building, response parsing, and error enrichment for OpenAI compatibility.

use serde_json::{Value, json};

use crate::config::schema::OpenAiDialect;
use crate::error::SawhorseError;
use crate::types::{GrammarFormat, InferenceRequest};

/// Builds a payload without catalog guidance.
#[must_use]
pub fn build_payload(
    dialect: OpenAiDialect,
    model_name: &str,
    req: &InferenceRequest,
    stream: bool,
) -> Value {
    build_payload_with(dialect, model_name, req, stream, None)
}

/// Constructs an OpenAI chat completion JSON payload from an `InferenceRequest`.
///
/// `wire` is the catalog's answer for how this model's protocol expresses reasoning. Without it the
/// payload falls back to the one canonical `reasoning_effort` field (`GB-217` step 5/5b).
#[must_use]
pub fn build_payload_with(
    dialect: OpenAiDialect,
    model_name: &str,
    req: &InferenceRequest,
    stream: bool,
    wire: Option<crate::catalog::WireReasoning>,
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

    if let Some(wire) = wire {
        // The catalog named the shape this protocol speaks - an effort string, a thinking toggle, an
        // Anthropic budget or a Gemini budget. Sending the right one is the whole point of step 5b.
        match wire {
            crate::catalog::WireReasoning::Disabled => {}
            crate::catalog::WireReasoning::Effort(token) => {
                body["reasoning_effort"] = json!(token);
            }
            crate::catalog::WireReasoning::ThinkingToggle { enabled } => {
                body["thinking"] = json!({ "type": if enabled { "enabled" } else { "disabled" } });
            }
            crate::catalog::WireReasoning::ThinkingBudget { budget_tokens } => {
                body["thinking"] = json!({ "type": "enabled", "budget_tokens": budget_tokens });
            }
            crate::catalog::WireReasoning::GoogleBudget { thinking_budget } => {
                body["thinkingConfig"] = json!({ "thinkingBudget": thinking_budget });
            }
        }
    } else if let Some(effort) = req.reasoning_effort {
        // No catalog opinion: one canonical field, which every OpenAI-compatible endpoint reads.
        body["reasoning_effort"] = json!(effort.wire_token());
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
        message.push_str(&format!(concat!(
            " [diagnostic: the provider returned status {status} with an empty `content`.",
            " That is an observation, not a cause. Retry the identical request first: that costs",
            " nothing and settles the cheapest explanation. If it recurs, compare",
            " `usage.completion_tokens_details.reasoning_tokens` with max_tokens={max_tokens} on a",
            " call that did return, and if the trace is small treat the payload as rejected. A 500",
            " that repeats at 4096 and at 32768 tokens alike is not budget exhaustion (GB-214).]"
        ), status = status, max_tokens = max_tokens));
    }

    SawhorseError::Http {
        endpoint: endpoint.to_string(),
        status,
        message,
    }
}
