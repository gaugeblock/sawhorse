# `sawhorse` — Provider-Agnostic 3-Mode LLM Runtime

[![Version](https://img.shields.io/badge/version-0.1.0--beta.4-blue.svg)](https://gaugeblock.dev)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![gaugeblock](https://img.shields.io/badge/suite-gaugeblock-orange.svg)](https://github.com/gaugeblock)
[![Documentation](https://img.shields.io/badge/docs-9--doc%20topology-green.svg)](ARCHITECTURE.md)

Part of the **[`gaugeblock`](https://gaugeblock.dev)** neurosymbolic code synthesis ecosystem.

> In fine carpentry, a **sawhorse** is the rugged, reliable trestle that supports workpieces firmly at bench height while cuts and joinery take shape. In the `gaugeblock` suite, `sawhorse` provides the sturdy foundational runtime for neural generation: it decouples high-level spec planners ([`chalkline`](https://github.com/gaugeblock/chalkline)) and synthesis orchestrators ([`joiner`](https://github.com/gaugeblock/joiner)) from specific LLM providers, offering unified grammar-constrained decoding across embedded, local, and remote models.

---

## Why `sawhorse`?

Most AI coding harnesses tightly couple their synthesis logic to proprietary cloud APIs, creating vendor lock-in, latency bottlenecks, and broken grammar constraints.

| Without `sawhorse` | With `sawhorse` |
|---|---|
| **Vendor SDK Lock-in:** Code generation hardcodes OpenAI or Anthropic SDKs, requiring complete architectural rewrites to run on local hardware or private clouds. | **Unified `ModelProvider` Trait:** Single provider-agnostic trait abstracting Mode 1 (Candle embedded), Mode 2 (local GGUF), and Mode 3 (OpenAI-compatible) seamlessly. |
| **Grammar Dialect Mismatches:** Sending GBNF or Lark grammar masks to different servers fails because vLLM, llama.cpp, and outlines use incompatible JSON parameters. | **Automatic Dialect Translation:** Automatically translates grammar requests into server-specific fields (`guided_grammar`, `grammar`, `extra_body`) based on provider config. |
| **Fragile Single-Point Failures:** Cloud API rate limits, network outages, or token exhaustion kill long-running agent workflows mid-flight. | **Resilient Role Routing & Fallback:** Semantic role routing (`Planner`, `HoleFiller`, `Oracle`) with automatic cascading fallback to secondary local or cloud providers. |
| **High IPC Overhead for Small Holes:** Sending 5-token hole completions over HTTP adds 100–300ms of round-trip network latency per hole. | **In-Process Candle Execution:** Direct in-process execution using embedded `candle` micro-models (100M–1B params) filling holes in `<15ms` with zero network overhead. |

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

## 2. CLI Commands Reference

`sawhorse` provides diagnostic and testing commands for evaluating model backends, measuring latency, and validating grammar constraints. Pass `--json` to any command for structured output.

| Command | Description |
|---|---|
| `init` | Scaffold a starter `sawhorse.toml` or `.gaugeblock/config.toml` configuration. |
| `ping` | Ping configured providers and report round-trip network and generation latency. |
| `complete` | Execute prompt completion across configured semantic roles with optional grammar masks. |
| `bench` | Run comparative latency and token throughput benchmarks across all active providers. |

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
    println!("Completion: {}", resp.content);
    Ok(())
}
```

---

## 5. Canonical 9-Document Architecture

`sawhorse` adheres strictly to the canonical 9-Document Architecture:
* **[`README.md`](./README.md):** Public elevator pitch, CLI reference, and 3-mode overview.
* **[`CONTRIBUTING.md`](./CONTRIBUTING.md):** Developer environment setup, Rust 2024 toolchain, and test execution.
* **[`PRODUCT.md`](./PRODUCT.md):** User personas, JTBD, runtime flexibility metrics, and problem statements.
* **[`ARCHITECTURE.md`](./ARCHITECTURE.md):** Subsystem topologies, provider adapter pipelines, and crate boundaries.
* **[`SPEC.md`](./SPEC.md):** Formal inference invariants, fallback state machines, and type contracts.
* **[`DESIGN.md`](./DESIGN.md):** Architectural Decision Records (ADRs) and trade-off rationales.
* **[`ROADMAP.md`](./ROADMAP.md):** Strategic milestones, active beta capabilities, and horizons.
* **[`CHANGELOG.md`](./CHANGELOG.md):** Chronological historical release logs and unreleased entries.
* **[`AGENTS.md`](./AGENTS.md):** Non-negotiable engineering directives, 250-line file ceiling, and verification matrix.

---

## 6. License

Licensed under the [Apache License, Version 2.0](LICENSE).
