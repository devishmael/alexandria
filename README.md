# 🏛️ Alexandria Runtime (`v2.4.0-OmniPragmatic`)

[![Alexandria CI](https://github.com/devishmael/alexandria-runtime/actions/workflows/ci.yml/badge.svg)](https://github.com/devishmael/alexandria-runtime/actions/workflows/ci.yml)
![Rust Edition](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)
![MCP Protocol](https://img.shields.io/badge/MCP-2024--11--05-blue)
![SQLite WAL + FTS5](https://img.shields.io/badge/Storage-SQLite_WAL_%2B_FTS5-003B57?logo=sqlite)
![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)

> **Headless Deterministic Agent Harness & Kernel-Space Arbitrator in Rust.**  
> Governing probabilistic User-Space LLMs via zero-trust AST gates, OS-sandboxed TDD verification, single-writer SQLite WAL persistence, and a live Obsidian knowledge graph.

---

## 1. Executive Summary: The Dual-Space Paradigm

Modern AI coding assistants suffer from **context rot**, **hallucinated green builds**, **silent architectural drift**, and **destructive retry loops (*LLM Thrashing*)** when left to self-police through prompt engineering alone.

**Alexandria Runtime** solves this by enforcing a strict operating-system boundary between **Kernel-Space** and **User-Space**:

- **Kernel-Space (`alexandria` Rust Binary):** The deterministic authority. It owns the filesystem graph (`petgraph`), native TypeScript/TSX AST parsing (`oxc_parser`), OS-level process sandboxing (`node --permission`), single-writer SQLite WAL + FTS5 BM25 memory (`rusqlite`), and the deterministic Circuit Breaker.
- **User-Space (Probabilistic LLM Host):** Untrusted compute process (Antigravity 2.0, Claude Code, Cursor, Windsurf). It cannot self-certify compilation, test verdicts, or architectural compliance; every mutation is arbitrated out-of-band by the Rust kernel via the **Model Context Protocol (MCP `2024-11-05`)**.

---

## 2. System Architecture & Dual-Plane Knowledge Graph

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                     USER-SPACE: IDE HOST / PROBABILISTIC LLM AGENTS                      │
│   ┌────────────────────┬────────────────────┬────────────────────┬───────────────────┐   │
│   │  BEL (Overseer)    │   ENKI (Builder)   │    UTU (Critic)    │ NABU (Chronicler) │   │
│   │ • Vault Contracts  │ • Strict RED/GREEN │ • Stateless Cold   │ • Append-Only Log │   │
│   │ • ADR Governance   │ • Branded Types    │   SAST & ADR Audit │ • FAIL-XXX Ledger │   │
│   └─────────┬──────────┴─────────┬──────────┴─────────┬──────────┴─────────┬─────────┘   │
└─────────────┼────────────────────┼────────────────────┼────────────────────┼─────────────┘
              │                    │  MCP JSON-RPC 2.0  │                    │
              ▼                    ▼    (stdio Pipe)    ▼                    ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                   KERNEL-SPACE: ALEXANDRIA RUNTIME (Rust 2024 Binary)                    │
├────────────────────────────────────────────┬─────────────────────────────────────────────┤
│  PLANE 1: TOPOLOGICAL GRAPH & AST ENGINE   │  PLANE 2: EPISODIC & PROCEDURAL MEMORY      │
│  • Obsidian Vault (`./wiki/`)              │  • Single-Writer Actor (`~/.alexandria/`)   │
│  • `petgraph::StableDiGraph` in RAM        │  • `SQLite WAL` (`synchronous = NORMAL`)    │
│  • Strict Depth=1 Module Pruning           │  • `skills_fts` (BM25 + Porter Stemmer)     │
│  • Transitive ADR Hoisting (Depth <= 6)    │  • `incidents_fts` (Forensic `FAIL-XXX`)    │
│  • `oxc_parser` Gate 0 AST Visitor         │  • Deterministic Circuit Breaker (Max = 3)  │
└────────────────────────────────────────────┴─────────────────────────────────────────────┘
```

### The Pantheon Quartet (`agent_catalog` & `.agents/agents/`)
| Agent ID | Archetype | `max_subagents` | Core Responsibility | Hard Boundary |
| :--- | :--- | :---: | :--- | :--- |
| **`bel`** | **Bel (The Overseer)** | `5` | Chief Software Architect. Authors contracts in `wiki/modules/` and `[[ADR-XXX]]` in `wiki/decisions/`. | **Never** modifies executable code in `src/`. |
| **`enki`** | **Enki (The Builder)** | `3` | Principal TDD Engineer. Executes strict **RED $\rightarrow$ GREEN $\rightarrow$ REFACTOR** TDD cycles in `tests/` and `src/`. | **Blocked** from editing `src/` without a prior contract and a verified RED assertion failure. |
| **`utu`** | **Utu (The Critic)** | `1` | Impartial Security & Quality Auditor. Performs stateless cold-context audits against hoisted ADRs and SAST rules. | Zero access to Enki's reasoning history (prevents confirmation bias). |
| **`nabu`** | **Nabu (The Chronicler)** | `1` | Keeper of the Obsidian Vault. Records `FAIL-XXX` forensic incidents in `wiki/failures.md` and `EPOCH-XXX` entries in `wiki/log.md`. | Append-only immutable ledger enforcement. |

---

## 3. Production Binary & Performance Metrics

Compiled with `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, and `strip = true`:

| Metric | Target | Certified Benchmark (`v2.4.0`) |
| :--- | :--- | :--- |
| **Release Binary Size (`alexandria.exe` / `alexandria`)** | $\le 5.00\text{ MB}$ | **`2.17 MB` (`2,278,400 bytes`)** (includes bundled SQLite + Oxc + Tokio) |
| **Idle Resident Memory (RSS)** | $\le 15.0\text{ MB}$ | **`~8.4 MB`** (Zero ML weights in process address space) |
| **Topological Graph Traversal + ADR Hoisting** | $\le 1.00\text{ ms}$ | **`<= 0.20 ms`** (`petgraph::StableDiGraph` in RAM) |
| **Gate 0 Native AST Scan (`oxc_parser`)** | $\le 2.00\text{ ms}$ | **`<= 0.80 ms`** (Contiguous arena bump allocation) |
| **Automated Kernel Test Suites** | $100\%$ Pass | **`32 / 32 Passed`** across 7 integration & E2E suites |

---

## 4. Quickstart Guide

### Build from Source
```bash
git clone https://github.com/devishmael/alexandria-runtime.git
cd alexandria-runtime
cargo test --all-targets
cargo build --release
```

### CLI Commands

1. **Initialize a Workspace (`init`):**
   Generates `.alexandria.json`, scaffolds the canonical Obsidian vault (`wiki/index.md`, `wiki/log.md`, `wiki/failures.md`, `wiki/architecture/`, `wiki/modules/`, `wiki/decisions/`), populates the Pantheon agent descriptors (`.agents/agents/{bel,enki,utu,nabu}.md`), and hardens `.gitignore`.
   ```bash
   ./target/release/alexandria init
   # Optional custom Obsidian vault path and profile:
   ./target/release/alexandria init --vault ./wiki --profile mvp-fullstack-ts
   ```

2. **Start the MCP JSON-RPC 2.0 Server (`mcp`):**
   Spawns the dedicated SQLite WAL writer thread on `~/.alexandria/vault.db`, seeds the Pantheon `agent_catalog`, and serves the 6 Prefix-Pinned MCP tools over `stdio`.
   ```bash
   ./target/release/alexandria mcp
   ```

3. **Autonomous Zero-Turn Directive Classification (`run`):**
   Evaluates an engineering directive, classifies the execution path (`alpha_fast` vs. `omega_pantheon`), and resolves the vault context.
   ```bash
   ./target/release/alexandria run "Refactor Prisma billing transaction with strict lexical closure"
   ```

---

## 5. Multi-Host IDE Integration (`MCP 2024-11-05`)

Alexandria Runtime exposes **6 immutable tools** (`prepare_task_context`, `upsert_architectural_contract`, `resolve_skill_capsule`, `spawn_isolated_worktrees`, `verify_and_gate_changes`, `finalize_task_and_log`) ordered deterministically to guarantee a **100% KV-Cache Prefix Hit**.

### Google Antigravity 2.0 (`mcp_config.json`)
```json
{
  "mcpServers": {
    "alexandria-engine": {
      "command": "alexandria",
      "args": ["mcp"],
      "env": {
        "ALEXANDRIA_LOG_LEVEL": "info",
        "ALEXANDRIA_STRICT_TDD": "true"
      }
    }
  }
}
```

### Claude Code (`.mcp.json` or CLI)
```bash
claude mcp add alexandria-engine -- alexandria mcp
```

### Cursor (`.cursor/mcp.json`)
```json
{
  "mcpServers": {
    "alexandria-engine": {
      "command": "/absolute/path/to/alexandria",
      "args": ["mcp"]
    }
  }
}
```

---

## 6. Out-of-Band Quality Gates & Deterministic TDD

Every code mutation submitted via `verify_and_gate_changes` must clear five sequential kernel gates:

| Gate | Subsystem | Deterministic Invariants Enforced |
| :--- | :--- | :--- |
| **Gate 0** | [`oxc_validator.rs`](src/gate_guard/oxc_validator.rs) | Rejects `any` (`NO_EXPLICIT_ANY`), double casts `as unknown as T` (`NO_DOUBLE_CAST_UNKNOWN`), and enforces strict lexical closure inside `prisma.$transaction` (no `tx` aliasing/destructuring, no `.then()` chaining, no unawaited promises, no external `fetch`/`setTimeout`). |
| **Gate 1** | Static Types & Lint | Executes `tsc --noEmit && npm run lint` with zero warnings tolerated. |
| **Gate 2** | SAST & Secret Guard | Scans for credential leaks and unsafe patterns out-of-band. |
| **Gate 3** | [`vitest_sandbox.rs`](src/runner/vitest_sandbox.rs) | Invokes `node --permission` directly on `./node_modules/vitest/vitest.mjs` with `env_clear()`, Win32 UNC prefix stripping (`\\?\`), and read-only source confinement. Verifies genuine `AssertionError` in RED phase (`evaluate_tdd_red_phase`) and $\ge 90\%$ coverage in GREEN phase. |
| **Gate 4** | Build & Utu Audit | Verifies `npm run build` and runs Utu's stateless cold audit against hoisted `[[ADR-XXX]]` nodes. |

### Deterministic Circuit Breaker
$$\text{consecutive\_failures}(M) \ge 3 \implies \text{State}(M) \leftarrow \texttt{tripped\_abort}$$
When a module fails 3 consecutive gate attempts, the kernel trips the circuit breaker, freezes automated retries, logs a `FAIL-XXX` forensic incident in SQLite (`forensic_incidents` + `incidents_fts`), and requires architectural intervention.

---

## 7. License

Released under the [MIT License](LICENSE). Copyright (c) 2026 **Dev Ishmael**.
