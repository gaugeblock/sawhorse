# Contributor Guide — `sawhorse`

> Guidelines for engineering, testing, and contributing to the `sawhorse` codebase.

---

## 1. Toolchain & Environment Setup

`sawhorse` is built with the **Rust 2024 edition**:

```bash
# Verify rust toolchain
rustc --version # Rust 1.85+ (2024 edition compatible)

# Install cargo-nextest (preferred test runner)
cargo install cargo-nextest --locked
```

Clone the repository and compile member crates:

```bash
git clone https://github.com/gaugeblock/sawhorse.git
cd sawhorse
cargo check --workspace --all-targets
```

---

## 2. Workspace Organization

```text
sawhorse/
├── src/                       # Root facade re-exporting sawhorse-core
└── crates/
    ├── sawhorse-core/         # Core ModelProvider traits, RoleRouter, Candle/GGUF/OpenAI adapters
    └── sawhorse-cli/          # Clap CLI binary and command handlers
```

---

## 3. Tiered Verification Workflow

All code contributions must satisfy the tiered verification matrix before merge:

### Tier 1 (Inner Loop)
Rapid check during crate modification:
```bash
cargo check -p sawhorse-core
cargo nextest run -p sawhorse-core --lib
```

### Tier 2 (Commit Gate)
Run on affected crates prior to making an atomic commit:
```bash
cargo fmt --check -p sawhorse-core
cargo clippy -p sawhorse-core --all-targets -- -D warnings
cargo nextest run -p sawhorse-core
chalkline docs check -d .
```

### Tier 3 (Release Gate)
Full workspace regression check:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
chalkline docs check --workspace
```

---

## 4. Coding Standards & Invariants

1. **Hard 250-Line File Limit:** No `.rs`, `.ts`, or `.tsx` file may exceed 250 lines. Decompose proactively into submodules.
2. **Strict Error Handling:** Library crates (`sawhorse-core`) must use `thiserror`. Never use `.unwrap()` or `.expect()` in non-test library code (`[workspace.lints.clippy]` `unwrap_used = "deny"`).
3. **Structured Telemetry:** Use `tracing` for logging. Never print unstructured lines to `stdout` in JSON mode; stdout is strictly reserved for machine-readable JSON frames.
4. **Conventional Commits:** `<type>(<scope>): <summary>` (e.g. `feat(sawhorse-core): add support for Anthropic Messages API dialect`).
