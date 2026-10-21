# 🏛️ ALEXANDRIA — GOVERNANCE DIRECTIVES FOR CLAUDE CODE

Welcome to the **Alexandria (v1.0.0)** repository. This system operates under a strict **Dual-Space Architectural Separation**:
- **Kernel-Space (Rust Arbitrator):** The compiled Rust binary (`alexandria-core`) arbitrates all mutations, quality gates, and filesystem operations.
- **User-Space (LLM):** Probabilistic, untrusted compute process governed out-of-band by Alexandria's deterministic Quality Gates.

## Mandatory Invariants for Claude Code

1. **Fact-Forcing GateGuard (`E_FACT_FORCING`):** Strictly forbidden from modifying or creating files without first reading or inspecting their contents and consumer contracts in the active session.
2. **Strict TDD Cycle:** Every new feature or bug fix in `src/` requires:
   - A unit test in `tests/` failing deterministically due to a business assertion (`AssertionError` / `assert_eq!`) in **RED Phase**.
   - Minimal implementation to transition to **GREEN Phase**.
   - **REFACTOR Phase** enforcing nominal types (Branded Types), zero explicit `any`, and zero double casting (`as unknown as T`).
3. **225 Master Capsules Catalog:** Consult technology capsules in `.alexandria/capsules/` to adhere to best practices and invariants across 69+ technology stacks.
4. **Deterministic Quality Gate Commands:**
   - Full test suite: `cargo test --all-targets`
   - Static type and syntax check: `cargo check --all-targets`
   - Run MCP Server: `cargo run --bin alexandria -- mcp`
