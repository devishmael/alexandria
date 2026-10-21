# 🏛️ ALEXANDRIA — GOVERNANCE DIRECTIVES FOR GOOGLE GEMINI CLI / ANTIGRAVITY

Alexandria Runtime operates under a strict **Dual-Space Architectural Separation** between **Kernel-Space (Rust Arbitrator)** and **User-Space (LLM Intelligence)**:

1. **Kernel-Space (`alexandria-core`):** Absolute deterministic authority over filesystem mutations, Oxc AST lexical analysis, out-of-band quality gates, and SQLite WAL single-writer persistence.
2. **User-Space (Gemini / Antigravity):** Probabilistic and untrusted compute process. Lacks authority to self-certify test results, compilation verdicts, or architectural compliance.
3. **Fact-Forcing GateGuard (`E_FACT_FORCING`):** Strict prohibition on modifying, writing, or editing existing files without verified inspection/read in the active session.
4. **Strict TDD Cycle:** RED (business assertion failure in `tests/`) -> GREEN (minimal code in `src/`) -> REFACTOR (nominal types, zero `any`).
5. **Catalog of 225 Master Capsules:** Located in `.alexandria/capsules/` covering 69+ technologies with SHA-256 cryptographic integrity.
