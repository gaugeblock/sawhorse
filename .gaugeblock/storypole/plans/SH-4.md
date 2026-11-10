# GB-80: Enrich Sawhorse Provider Observability and Expand Synthesis Token Headroom for Reasoning Models

## Problem
1. **Empty Response Content from Reasoning Models:**
   `cline-pass/deepseek-v4.1-flash` is a reasoning model. During inference, reasoning models emit an extensive thinking trace (`reasoning_tokens`) before outputting Markdown `content`. In empirical tests on changelog synthesis, the model used 1,950 reasoning tokens. Because `outfeed-core::synthesis` hardcoded `max_tokens = 1024`, the token limit was exhausted mid-thought, leaving `message.content` empty (`""`). The Cline API gateway rejected the empty completion with HTTP 500 `{"error":"empty response content","success":false}`.
2. **Zero Visibility into Sent Requests & Errors:**
   `sawhorse-core` lacked structured logging of model names, prompt sizes, and token limits. On HTTP errors, `Http` error messages omitted the target model and request parameters, leaving operators blind to what request payload was dispatched or why the provider failed.
3. **Timeout Insufficient for Thought Chains:**
   `outfeed-core` had a 10s synthesis timeout, which is insufficient for reasoning models that take 12-18s to finish complex thought chains.

## Solution
1. **Enrich `sawhorse-core` Observability & Error Diagnostics:**
   - In `OpenAiCompatProvider::complete` and `complete_stream`, emit structured `tracing::debug!` logs capturing `model`, `url`, `prompt_len`, `max_tokens`, `temperature`.
   - On HTTP errors, format the error to include the model name and `max_tokens`, and append an actionable diagnostic hint if `"empty response content"` is encountered.
2. **Expand `outfeed-core` Token Headroom & Timeout:**
   - Increase `req.max_tokens` from 1024 to 4096 in `outfeed-core::synthesis`.
   - Increase LLM synthesis timeout from 10s to 30s.
   - Add structured `tracing::debug!` logs before dispatching synthesis requests.
3. **Verification:**
   - Pass Tier 2 verification in `sawhorse` and `outfeed`.
   - Test live changelog synthesis with `outfeed release --all --dry-run` to confirm successful LLM synthesis without timeouts or empty response failures.

