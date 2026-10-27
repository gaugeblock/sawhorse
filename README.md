# `sawhorse` — Provider-Agnostic 3-Mode LLM Runtime

[![Version](https://img.shields.io/badge/version-0.1.0--beta.4-blue.svg)](https://gaugeblock.dev)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![gaugeblock](https://img.shields.io/badge/suite-gaugeblock-orange.svg)](https://github.com/gaugeblock)
[![Documentation](https://img.shields.io/badge/docs-9--doc%20topology-green.svg)](ARCHITECTURE.md)

Part of the **[`gaugeblock`](https://gaugeblock.dev)** neurosymbolic code synthesis ecosystem.

> In fine carpentry, a **sawhorse** is the rugged, reliable trestle that supports workpieces firmly at bench height while cuts and joinery take shape. In the `gaugeblock` suite, `sawhorse` provides the sturdy foundational runtime for neural generation: it decouples high-level spec planners ([`chalkline`](https://github.com/gaugeblock/chalkline)) and synthesis orchestrators ([`joiner`](https://github.com/gaugeblock/joiner)) from specific LLM providers, offering unified grammar-constrained decoding across embedded, local, and remote models.

---

## 1. The 3 Execution Modes

```text
┌────────────────────────────────────────────────────────┐
│               sawhorse Unified Trait                   │
└───────┬───────────────────┬────────────────────┬───────┘
        │ Mode 1            │ Mode 2             │ Mode 3
        ▼                   ▼                    ▼
┌──────────────┐    ┌──────────────┐     ┌──────────────┐
│ InHouseCandle│    │ LocalRuntime │     │OpenAiCompat  │
│ (Embedded ML)│    │(llama.cpp/GGUF)    │(vLLM/jev/API)│
└──────────────┘    └──────────────┘     └──────────────┘
```

1. **Mode 1 (`InHouseCandle`):** Direct in-process execution using Hugging Face's pure-Rust `candle` framework. Optimized for specialized, fine-tuned `gaugeblock` micro-models (100M–1B params) where network/IPC overhead would dominate 5-to-25 token hole filling.
2. **Mode 2 (`LocalRuntime`):** URL-first local inference adapter. Connects to `llama.cpp` server or `mistral.rs` via native HTTP `/completion` APIs with GBNF grammar constraints, with automatic fallback to safe local subprocess CLI execution.
3. **Mode 3 (`OpenAiCompatible`):** Flexible HTTP client supporting `vLLM`, `jev`, OpenRouter, and standard cloud APIs. Automatically translates grammar formats (`GBNF`, `Lark`, `Regex`) into backend-specific dialect fields (`guided_grammar`, `grammar`, or `extra_body`).

---

## 2. Quick Start CLI

```bash
# Initialize a starter configuration
sawhorse init

# Check health and measure latency across all configured providers
sawhorse ping

# Test prompt completion through a semantic role
sawhorse complete "let active_user = ?;" --role hole-filler

# Constrain completion with a GBNF grammar
sawhorse complete "let status = " --grammar "root ::= \"Active\" | \"Inactive\""

# Benchmark latency and throughput across providers
sawhorse bench
```

---

## 3. Configuration (`sawhorse.toml`)

```toml
[timeouts]
inference_timeout_secs = 60
ping_timeout_secs = 5

[providers.local_server]
mode = "local_runtime"
model_id = "qwen2.5-coder-7b"
endpoint_or_path = "http://127.0.0.1:8080"
dialect = "llama_cpp"

[providers.cloud_openai]
mode = "open_ai_compatible"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
model_name = "gpt-4o-mini"
dialect = "standard_open_ai"

# Semantic Role Routing with Automatic Fallback
[roles.planner]
provider = "cloud_openai"
fallback = ["local_server"]

[roles.hole_filler]
provider = "local_server"
fallback = ["cloud_openai"]
```

---

## 4. Programmatic Rust Usage

```rust
use sawhorse_core::config::SawhorseConfig;
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{InferenceRequest, ModelRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SawhorseConfig::discover()?;
    let router = RoleRouter::from_config(config)?;

    let req = InferenceRequest::new(ModelRole::HoleFiller, "let id = ?;");
    let resp = router.complete(&req).await?;

    println!("Generated text: {}", resp.text);
    println!("Model used: {} (latency: {}ms)", resp.model_used, resp.latency_ms);
    Ok(())
}
```

---

## 5. Workspace Crate Layout

* **`sawhorse`**: Facade crate re-exporting `sawhorse-core`.
* **`crates/sawhorse-core`**: Core runtime traits, provider clients (OpenAI-compatible, Candle, LocalRuntime), grammar adapters, reasoning suppression, and role routing.
* **`crates/sawhorse-cli`**: Clap binary (`sawhorse`) for provider ping, benchmarking, role completion, and interactive testing.

---

## 6. Repository Documentation

`sawhorse` adheres strictly to the canonical **9-Document Architecture**:
* **[`CONTRIBUTING.md`](./CONTRIBUTING.md):** Local toolchain prerequisites, credentials setup (`.gaugeblock/.env`), and test execution.
* **[`ARCHITECTURE.md`](./ARCHITECTURE.md):** Subsystem execution modes, provider abstractions, and SSE streaming pipeline.
* **[`SPEC.md`](./SPEC.md):** Request/Response wire contracts, reasoning effort schemas, and error types.
* **[`AGENTS.md`](./AGENTS.md):** Autonomous systems directives, <=250 LOC constraints, and zero-unwrap rules.
* **[`CHANGELOG.md`](./CHANGELOG.md):** Historical release entries and unreleased changes.

---

## 7. Verification & Development Directives

* **File Size Ceiling:** All `.rs` source files must strictly obey the <= 250 LOC limit.
* **Tiered Verification Matrix:**
  * Tier 1: `cargo check -p sawhorse-core` (<2s).
  * Tier 2: `cargo fmt --check -p sawhorse-core && cargo clippy -p sawhorse-core && cargo test -p sawhorse-core`.
  * Tier 3: `chalkline docs check -d .` before task completion.

---

## License

Licensed under the Apache License, Version 2.0.
Part of the [`gaugeblock`](https://gaugeblock.dev) suite.

