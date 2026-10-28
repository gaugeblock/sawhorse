# Product Vision & Specifications — `sawhorse`

> **Domain:** Provider-Agnostic 3-Mode LLM Runtime  
> **Ecosystem:** `gaugeblock` Neurosymbolic Meta-Harness  
> **License:** Apache-2.0

---

## 1. Problem Statement & Market Context

Current AI coding tools face four critical inference runtime challenges:

1. **Proprietary Vendor Lock-In:** Systems hardcode OpenAI, Anthropic, or Gemini APIs, leaving teams vulnerable to pricing shifts, privacy concerns, and cloud dependency.
2. **Network Latency Bottlenecks:** In neurosymbolic code synthesis, models are queried repeatedly for tiny 5–25 token hole fills. Sending every request over HTTP to remote clouds adds hundreds of milliseconds of latency per hole, grinding multi-hole synthesis loops to a halt.
3. **Incompatible Grammar Standards:** Constrained decoding engines require completely different JSON fields across runtimes (`guided_grammar` for vLLM, `grammar` for llama.cpp), making portable grammar constraints impossible without custom translation logic.
4. **Fragile Single-Provider Pipelines:** A single cloud rate limit or timeout aborts hours of multi-step agent coding workflows.

`sawhorse` solves this by providing a unified, provider-agnostic runtime across embedded, local, and cloud models with resilient role-based routing.

---

## 2. Target Personas & User Journeys

### Persona A: Synthesis Orchestrator (`joiner`)
- **Needs:** Provider-agnostic inference with sub-15ms latency for hole filling, automatic grammar translation, and resilient fallback to frontier Oracle models.
- **Workflow:** Dispatches `HoleFiller` queries to fast local models and `Oracle` queries to cloud frontier models through a single programmatic router.

### Persona B: Systems Engineer & DevOps Architect
- **Needs:** Total control over model hosting (100% air-gapped local execution vs private cloud vLLM clusters) without modifying downstream applications.
- **Workflow:** Configures `sawhorse.toml` to route all code generation to internal endpoints with zero external telemetry leakage.

### Persona C: Open-Weight Model Evaluator
- **Needs:** Standardized benchmarking across local GGUF models, embedded weights, and commercial APIs.
- **Workflow:** Runs `sawhorse bench` to measure token throughput, first-token latency, and grammar compliance across multiple backends.

---

## 3. Jobs to Be Done (JTBD)

| When I... | I want to... | So that I can... |
|---|---|---|
| Need to fill small 5–25 token code holes | Run inference directly in-process via embedded `candle` weights | Complete hole fills in `<15ms` without incurring network HTTP overhead. |
| Execute inference against different backends (vLLM, llama.cpp, cloud) | Pass a single GBNF or Lark grammar constraint | Have `sawhorse` automatically translate the grammar into the target server's specific wire dialect. |
| Experience an API rate limit or cloud outage | Automatically failover to a configured secondary provider | Complete long-running agent coding tasks without human intervention. |
| Run coding agents in an air-gapped environment | Route all semantic roles to local GGUF models or embedded Candle weights | Ensure zero source code or proprietary identifiers ever leave the local machine. |

---

## 4. Key Metrics & Success Criteria

* **In-Process Generation Latency:** `<15ms` for 10-token hole fills using Candle micro-models.
* **Grammar Translation Accuracy:** `100%` wire format accuracy across vLLM, llama.cpp, and OpenAI dialects.
* **Fallback Resilience:** `0%` workflow termination when primary providers fail with configured fallbacks.
* **Provider Switch Overhead:** `0` code changes required in high-level callers when swapping backends.
