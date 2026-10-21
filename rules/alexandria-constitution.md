# 🏛️ ALEXANDRIA CONSTITUTION & CORE INVARIANTS

Alexandria is a deterministic agent harness written in Rust. All agents, contributors, and harnesses operating on this repository must adhere to these invariants:

## 1. Dual-Space Architectural Separation
- **Kernel-Space (Rust Arbitrator):** Sole authority over the filesystem, AST lexing, subprocessing, and SQLite persistence.
- **User-Space (LLM Reasoning):** Probabilistic and untrusted. Cannot self-certify tests, compilations, or architectural validity.

## 2. Inviolable Quality Gates
Every code mutation must pass through Alexandria's deterministic gates:
1. **Gate 0 (AST GateGuard):** Oxc AST inspection in memory ($\le 2\text{ ms}$). Rejects `any` (`TSAnyKeyword`), double casts (`as unknown as T`), and uncontained transactional clients.
2. **Gate 1 (Typecheck):** Strict compiler verification with 0 errors.
3. **Gate 2 (SAST & Secrets):** Static analysis and credential leak interception.
4. **Gate 3 (Sandbox Execution):** Isolated subprocess test execution with JSON reporting.
5. **Gate 4 (Build & State Integrity):** Clean compilation and constitutional ADR compliance.

## 3. Fact-Forcing Verification
No file mutation may take place without prior verified inspection of the file's contents in the current session.
