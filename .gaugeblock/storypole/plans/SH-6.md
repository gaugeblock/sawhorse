# GB-84: Support DeepSeek and Cline Thinking Suppression Payload Formats and /no-think Directive

## Context & Root Cause
When calling `cline-pass/deepseek-v4.1-flash` via `sawhorse-core` during changelog synthesis, the upstream gateway rejected requests with HTTP 500:
`{"error":"empty response content","success":false} (model: 'cline-pass/deepseek-v4.1-flash', max_tokens: 4096)`.
Even with `max_tokens: 4096`, the model consumed all tokens inside `<think>...</think>` without generating completion content.

OpenAI-compatible endpoints use top-level `reasoning_effort` (`low`, `medium`, `high`). However:
1. DeepSeek and Cline/OpenRouter gateways default reasoning models to high thinking effort unless explicitly disabled via gateway-specific payload structures:
   - `"thinking": { "type": "disabled" }`
   - `"reasoning": { "effort": "none" }`
2. DeepSeek's instruction parser also supports an explicit instruction directive `/no-think` in the prompt to immediately suppress reasoning trace generation.
3. Because `build_payload` only emitted top-level `"reasoning_effort": "none"`, the gateway ignored it and ran with full default thinking mode.

## Implementation Steps
1. In `sawhorse/crates/sawhorse-core/src/providers/openai_compat/payload.rs`:
   - Map `ReasoningEffort::None` to:
     - `"reasoning_effort": "none"`
     - `"thinking": { "type": "disabled" }`
     - `"reasoning": { "effort": "none" }`
   - Map `ReasoningEffort::Low`, `Medium`, `High` to corresponding `"thinking": { "type": "enabled" }` and `"reasoning": { "effort": "<level>" }`.
2. In `outfeed/crates/outfeed-core/src/synthesis.rs`:
   - Prefix the changelog synthesis prompt with `/no-think\n` when `ReasoningEffort::None` is dispatched.
3. Validate with unit tests and ensure file size remains under 250 LOC.

