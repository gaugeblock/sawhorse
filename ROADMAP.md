# Product Roadmap & Strategic Horizons — `sawhorse`

> **Active Suite Lifecycle:** `0.1.0-beta.X`  
> **Target:** Provider-Agnostic 3-Mode LLM Runtime

---

## 1. Active Milestone: `v0.1.0-beta.5` (Current Focus)

- [x] **3-Mode Execution Pipeline:** Mode 1 (Candle in-process), Mode 2 (Local GGUF), and Mode 3 (OpenAI-compatible).
- [x] **Semantic RoleRouter:** Flexible mapping of `Planner`, `System1Router`, `HoleFiller`, and `Oracle` roles.
- [x] **Resilient Cascading Fallback:** Ordered fallback provider chains with global timeout budgets.
- [x] **Grammar Dialect Translation:** Automatic mapping of GBNF, Lark, and Regex across server dialects.
- [x] **Benchmark & Diagnostic CLI:** `sawhorse ping` and `sawhorse bench` commands.
- [x] **9-Document Architecture Compliance:** Certified clean under `chalkline docs check`.

---

## 2. Horizon 1: Hardware Acceleration & Vendor Dialects (`v0.2.0`)

- [ ] **Metal & CUDA Acceleration:** Native Apple Silicon Metal and NVIDIA CUDA kernel support in `CandleNativeProvider`.
- [ ] **Native Anthropic Messages Dialect:** First-class adapter for Anthropic Claude models with thinking effort tokens.
- [ ] **Streaming Token Callbacks:** Low-latency streaming token emitter for live UI feedback in `benchwork`.
- [ ] **Token Usage & Cost Accounting:** Per-role token spend and API cost tracking across providers.

---

## 3. Horizon 2: Speculative Decoding & Latency Optimization (`v0.3.0`)

- [ ] **Speculative Decoding Engine:** Use embedded Candle micro-models as draft predictors for large local 70B models.
- [ ] **Dynamic Latency-Aware Routing:** Automatically route queries to the fastest responding provider based on rolling latency windows.
- [ ] **In-Memory Embedding Provider:** Native embedding generation for semantic search in `storypole` and `chalkline`.

---

## 4. Horizon 3: Zero-Copy Synthesis Fabric (`v1.0.0`)

- [ ] **Zero-Copy In-Memory IPC:** Shared-memory ring buffer connecting `sawhorse` token streams directly to `joiner`.
- [ ] **Enterprise Compliance Proxy:** Data loss prevention (DLP) filtering and compliance logging before dispatching to external clouds.
- [ ] **Long-Term Configuration Stability:** Freeze `sawhorse.toml` schema for multi-year backward compatibility.
