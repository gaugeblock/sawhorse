# Architectural Design & Decision Records — `sawhorse`

> **Crates:** `sawhorse`, `sawhorse-core`, `sawhorse-cli`  
> **Status:** Active Architectural Decision Records (ADRs)

---

## Architectural Decision Records (ADRs)

### ADR-001: Provider-Agnostic `ModelProvider` Trait Over Vendor SDKs
- **Status:** Accepted
- **Context:** Hardcoding official vendor SDKs (e.g. `async-openai`) limits flexibility and forces high-level callers to maintain separate code paths for local GGUF models or embedded Candle runtimes.
- **Decision:** Introduce a single unified `ModelProvider` async trait with `complete` and `ping` methods. Implement separate adapters for Candle, local GGUF HTTP/subprocess, and OpenAI-compatible endpoints behind this trait.
- **Consequences:** Callers (`joiner`, `chalkline`) execute inference without knowing whether the model runs on a remote cloud cluster or an embedded in-memory tensor.

### ADR-002: Semantic Role-Based Routing with Cascading Fallback Chains
- **Status:** Accepted
- **Context:** Different synthesis tasks have wildly different requirements: architectural planning needs a large frontier model (`Planner`), while 5-token hole fills need an ultra-fast local model (`HoleFiller`). Moreover, cloud providers suffer from rate limits and outages.
- **Decision:** Define semantic `ModelRole` assignments (`Planner`, `System1Router`, `HoleFiller`, `Oracle`). In `sawhorse.toml`, configure each role with a primary provider and an ordered fallback chain (`fallback = ["local_server", "cloud_backup"]`).
- **Consequences:** Autonomous resilience against provider downtime and optimal matching of task complexity to model size.

### ADR-003: In-Process Candle Execution for Sub-15ms Hole Filling
- **Status:** Accepted
- **Context:** In neurosymbolic synthesis, holes are frequently small (5–25 tokens). Sending a request over loopback HTTP or spawning a subprocess adds 50–200ms of overhead, dominating synthesis loop time.
- **Decision:** Implement Mode 1 (`InHouseCandle`) using Hugging Face's pure-Rust `candle` framework. Load specialized open-weight micro-models (100M–1B params) directly into memory and sample in-process.
- **Consequences:** True sub-15ms hole completions with zero network round-trip overhead.

### ADR-004: Automatic Grammar Dialect Translation Layer
- **Status:** Accepted
- **Context:** Different inference engines expect grammar constraints in different JSON request fields: `vLLM` expects `guided_grammar`, `llama.cpp` expects `grammar`, and OpenAI expects structured `response_format`.
- **Decision:** Introduce `OpenAiDialect` configurations. When `sawhorse` sends an inference request with a grammar constraint, it automatically formats the JSON body to match the detected server dialect.
- **Consequences:** Callers pass a single neutral grammar constraint without worrying about target server JSON schemas.

### ADR-005: URL-First Local Inference with Graceful Subprocess Fallback
- **Status:** Accepted
- **Context:** Local developers may run a persistent `llama-server` background daemon for fast responses, but fallback to one-off CLI invocations if the server is stopped.
- **Decision:** In `LocalGgufProvider`, attempt HTTP communication with the configured local endpoint first. If the endpoint is unreachable and a local binary path is specified, fall back gracefully to spawning the local binary as a subprocess.
- **Consequences:** Maximum developer convenience with zero configuration friction.
