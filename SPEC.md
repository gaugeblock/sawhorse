# Behavioral Specifications & Invariants

## Formal Type Contracts

- **`candle_native::CandleNativeProvider`** (`struct`): Provider implementation for native embedded Candle micro-models.
- **`complete::CompleteArgs`** (`struct`): Arguments for `sawhorse complete`.
- **`types::ExecutionMode`** (`enum`): Execution backend modes supported by `sawhorse`.
- **`types::GrammarFormat`** (`enum`): Supported grammar constraint formats emitted by `batten`.
- **`types::InferenceRequest`** (`struct`): Unary inference request passed into `ModelProvider`.
- **`types::InferenceResponse`** (`struct`): Completed inference response returned by `ModelProvider`.
- **`local_gguf::LocalGgufProvider`** (`struct`): Provider implementation for local model runtimes.
- **`traits::ModelProvider:`** (`trait`): Provider-agnostic inference engine trait.  Consumers like `chalkline` (Phase 3) and `joiner` (Phase 6) interact strictly through this trait without caring whether the backing engine is an embedded Candle model, a local llama.cpp server, or a remote OpenAI endpoint.
- **`types::ModelRole`** (`enum`): Role assignments used by `gaugeblock` callers to route requests.
- **`mod::OpenAiCompatProvider`** (`struct`): Provider implementation for OpenAI-compatible endpoints.
- **`schema::OpenAiDialect`** (`enum`): Dialect variations across OpenAI-compatible servers for structured outputs.
- **`schema::ProviderConfig`** (`struct`): Configuration block for an individual model provider.
- **`types::ReasoningEffort`** (`enum`): Level of reasoning or thinking effort requested from the model.
- **`schema::RoleConfig`** (`struct`): Routing policy for a specific semantic `ModelRole`.
- **`router::RoleRouter`** (`struct`): Router mapping semantic `ModelRole` requests to providers with fallback resilience.
- **`schema::SawhorseConfig`** (`struct`): Root schema for `sawhorse.toml`.
- **`error::SawhorseError`** (`enum`): Core error type representing all failure modes in `sawhorse`.
- **`schema::TimeoutConfig`** (`struct`): Global timeout settings for inference and diagnostics.
