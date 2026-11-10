# GB-81: Add Reasoning Effort Controls and Capture Reasoning Traces in sawhorse-core

## Problem
1. **Inefficient Reasoning in Routine Utility Tasks:**
   For formatted utility prompts such as changelog synthesis in `outfeed`, frontier reasoning models (like `deepseek-v4.1-flash`) execute a 1,500–2,000 token thinking trace to produce simple Markdown bullets. This imposes 15–25s latency and high token consumption for deterministic output.
2. **Discarded High-Value Training Data:**
   `sawhorse-core::providers::openai_compat::payload::extract_chat_response` parses only `choices[0].message.content` and discards `choices[0].message.reasoning` / `reasoning_details`. In neurosymbolic architectures where outputs are compiler-verified, captured reasoning traces represent ground-truth process-supervised data for local SLM distillation.

## Solution
1. **Add `ReasoningEffort` to `sawhorse-core::types`:**
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "snake_case")]
   pub enum ReasoningEffort {
       None,
       Low,
       Medium,
       High,
   }
   ```
2. **Expose in `InferenceRequest`:**
   Add `pub reasoning_effort: Option<ReasoningEffort>` to `InferenceRequest`.
3. **Dialect-Specific Payload Mapping:**
   In `openai_compat::payload::build_payload`:
   - Map `ReasoningEffort` to OpenAI standard `reasoning_effort`.
   - For models requiring `/no-think` or `thinking: { type: "disabled" }`, serialize appropriately based on dialect.
4. **Preserve Reasoning Traces in `InferenceResponse`:**
   Extend `InferenceResponse` with:
   - `pub reasoning_trace: Option<String>`
   - `pub reasoning_tokens: Option<u32>`
5. **Update `extract_chat_response`:**
   Extract `body["choices"][0]["message"]["reasoning"]` or `body["data"]["choices"][0]["message"]["reasoning"]` into `InferenceResponse::reasoning_trace`.

## Step Breakdown
- Step 1: Add `ReasoningEffort` enum and update `InferenceRequest` and `InferenceResponse` in `sawhorse-core::types` (Gate: `cargo check -p sawhorse-core`)
- Step 2: Update `build_payload` and `extract_chat_response` in `sawhorse-core::providers::openai_compat::payload` (Gate: `cargo test -p sawhorse-core`)
- Step 3: Run Tier 2 verification across `sawhorse` workspace (Gate: `cargo clippy -p sawhorse-core && cargo nextest run -p sawhorse-core`)
