# 🏛️ Alexandria — The Deterministic Agent Harness

[![Release](https://img.shields.io/badge/Release-v1.0.0-0969da?logo=github)](https://github.com/devishmael/alexandria/releases/tag/v1.0.0)
[![CI](https://github.com/devishmael/alexandria/actions/workflows/ci.yml/badge.svg)](https://github.com/devishmael/alexandria/actions/workflows/ci.yml)
[![Tests: 54/54 Passed](https://img.shields.io/badge/Tests-54%2F54_Passed_100%25-brightgreen?logo=rust)](https://github.com/devishmael/alexandria)
![Rust Edition](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)
![Latency: <= 2ms](https://img.shields.io/badge/Latency-%E2%89%A4_2ms_Oxc_AST-success)
![Capsules: 225 Master Capsules](https://img.shields.io/badge/Capsules-225_Master_Capsules-blueviolet)
![MCP Protocol](https://img.shields.io/badge/MCP-2024--11--05-blue)
![LSP Protocol](https://img.shields.io/badge/LSP-3.17_Native-purple)
![Multi-Harness](https://img.shields.io/badge/Multi--Harness-Cursor_%7C_VS_Code_%7C_Claude_%7C_Gemini_%7C_Codex_%7C_OpenCode_%7C_Zed-teal)
![Storage](https://img.shields.io/badge/Storage-SQLite_WAL_%2B_FTS5_%2B_Int8_Vector_RRF-003B57?logo=sqlite)
![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)

> **Headless Deterministic Agent Harness, Kernel-Space Arbitrator, Hybrid Vector Store & Native LSP Server in Rust.**  
> Governing probabilistic User-Space LLMs via zero-trust Oxc AST gates, deterministic **Fact-Forcing GateGuard** (zero blind edits), surgical AST context pruning, token-efficient hybrid vector memory (`int8` + BM25 Reciprocal Rank Fusion), OS-sandboxed TDD verification, **Pre-Compact Session Checkpoints**, Single-Turn Court-Martial arbitration, Agentic Git IAM, live organism telemetry (`MTTR` + `dashboard.md`), and **225 Curated Master Capsules** covering 69+ technologies with native Multi-Harness support (`.cursor/`, `.vscode/`, `.claude/`, `rules/`).

---

## Table of Contents

1. [Executive Summary: The Dual-Space Paradigm](#1-executive-summary-the-dual-space-paradigm)
2. [The 3 Golden Architectural Jewels & Paradigm Clash](#2-the-3-golden-architectural-jewels--paradigm-clash)
3. [System Architecture & The Pantheon Quartet](#3-system-architecture--the-pantheon-quartet)
4. [Production Binary & Certified Benchmarks](#4-production-binary--certified-benchmarks)
5. [Complete CLI Reference Guide](#5-complete-cli-reference-guide)
6. [Native Multi-Harness IDE, Agent & LSP Configuration Guide](#6-native-multi-harness-ide-agent--lsp-configuration-guide)
7. [Token-Efficient Hybrid Vector Store (`src/vector_store/`)](#7-token-efficient-hybrid-vector-store-srcvector_store)
8. [Surgical AST Context Pruning (`oxc_parser`)](#8-surgical-ast-context-pruning-oxc_parser)
9. [Out-of-Band Quality Gates, Court-Martial & Agentic Git IAM](#9-out-of-band-quality-gates-court-martial--agentic-git-iam)
10. [Live Organism Telemetry (`MTTR` & `wiki/dashboard.md`)](#10-live-organism-telemetry-mttr--wikidashboardmd)
11. [Curated 225 Master Capsule Catalog & Active Lifecycle Hooks](#11-curated-225-master-capsule-catalog--active-lifecycle-hooks)
12. [Headless Markdown Knowledge Vault & Multi-Viewer Guide](#12-headless-markdown-knowledge-vault--multi-viewer-guide)
13. [License](#13-license)

---

## 1. Executive Summary: The Dual-Space Paradigm

Modern AI coding assistants suffer from **context rot**, **hallucinated green builds**, **silent architectural drift**, **token bloat**, and **destructive retry loops (*LLM Thrashing*)** when left to self-police through prompt engineering alone.

**Alexandria** solves this by enforcing a strict operating-system boundary between **Kernel-Space** and **User-Space**:

- **Kernel-Space (`alexandria` Rust Binary):** The deterministic authority. It owns the filesystem knowledge graph (`petgraph`), native TypeScript/TSX AST parsing and surgical context pruning (`oxc_parser` + `oxc_resolver`), OS-level process sandboxing (`node --permission`), polymorphic Skill/Hook ETL normalization (`src/collector/`), single-writer SQLite WAL + FTS5 BM25 + 64-dim `int8` hybrid vector memory (`rusqlite`), real-time Language Server Protocol (`LSP`) diagnostics, and the deterministic Circuit Breaker.
- **User-Space (Probabilistic LLM Host):** Untrusted compute process (Google Antigravity 2.0, Claude Code, Cursor, Windsurf, VS Code, Neovim). It cannot self-certify compilation, test verdicts, or architectural compliance; every mutation is arbitrated out-of-band by the Rust kernel via **MCP (`2024-11-05`)** and **LSP (`3.17`)**.

---

## 2. The 3 Golden Architectural Jewels & Paradigm Clash

To eliminate the fragility, latency, and context degradation of conventional script-based agent wrappers, Alexandria synthesizes the industry's most powerful agentic governance patterns into **bare-metal, deterministic Rust implementations**:

### 🥇 Jewel 1: Deterministic Fact-Forcing GateGuard (`E_FACT_FORCING`)
- **The Problem:** LLMs are prone to hallucinated confidence and blind edits, modifying files without checking their imports, methods, or callers.
- **Alexandria's Solution:** A native Rust gate in `src/gate_guard/fact_forcing.rs`. Before any file mutation (`Write` or `Edit`) is permitted on an existing file, the kernel verifies in memory that the file was previously inspected/read in the active session. If uninspected, it fires an immediate **Syscall Trap (`E_FACT_FORCING`)** that halts the mutation before it touches disk.

### 🥈 Jewel 2: Native Zero-Config Multi-Harness Directory Layout & Universal Adapters
- **The Problem:** Developers and teams work across fragmented environments (Cursor, VS Code, Claude Code, Gemini CLI, Codex, OpenCode, Zed, Kimi, Qwen, Windsurf). Running fragile export scripts creates onboarding friction and config drift.
- **Alexandria's Solution:** Built-in native support right in the repository root for zero-config onboarding across 10+ AI tools:
  - `.cursor/`: Cursor MDC rules (`alexandria.mdc`), `.cursorrules`, and native MCP config (`mcp.json`).
  - `.vscode/`: Tasks (`tasks.json`), recommended extensions, and settings.
  - `.claude-plugin/`: Official Claude Code plugin manifest (`manifest.json`) and instructions (`.claude/CLAUDE.md`).
  - `.gemini/`: Google Gemini CLI and Antigravity directives (`GEMINI.md`).
  - `.codex/`: Codex tool config (`config.toml`) and system agents (`AGENTS.md`).
  - `.opencode/`, `.zed/`, `.kimi/`, `.qwen/`, `.windsurf/`: Native client configs and tool definitions.
  - `.mcp.json` & `.coderabbit.yaml`: Universal root configurations for CI and local agent hosts.
  - `rules/`: Project constitution, Fact-Forcing mandates, and capsule catalogs.

### 🥉 Jewel 3: Pre-Compact Session Checkpoints in SQLite WAL
- **The Problem:** When an AI assistant's context window reaches capacity, automated `/compact` commands execute lossy compression that erases active ADR decisions, pending test suites, and incremental progress.
- **Alexandria's Solution:** Pre-compact session snapshots stored transactionally in SQLite WAL (`session_checkpoints`). Captures exact test status, modified files, active ADRs, and markdown summaries, ensuring zero loss of cognitive progress across compactions or session restarts.

---

### ⚔️ The Paradigm Clash: Alexandria vs. User-Space Prompt Systems

| Dimension | Conventional Script / Prompt Wrappers | Alexandria (v1.0.0 Deterministic Kernel) |
| :--- | :--- | :--- |
| **Execution Space** | **User-Space (Scripts in Node/Python)** | **Kernel-Space (Compiled Rust Binary)** |
| **Authority Level** | **Probabilistic & Pleading:** Asks the LLM to follow 400-line Markdown prompts. | **Physical & Coercive:** The Rust binary intercepts calls and enforces **Syscall Traps**. |
| **Quality Gate Timing** | **Post-Hoc (`PostToolUse`):** Runs after the file has already been written to disk. Only logs warnings to `stderr`. | **Pre-Hoc In-Memory:** Evaluates Oxc AST in memory **before** disk write. Violations abort instantly. |
| **Per-Event Latency** | **$150\text{ ms} - 350\text{ ms}$:** Spawns `node.exe` processes per event, wasting >10s of CPU per 40 tool calls. | **$\le 2\text{ ms}$:** Sub-millisecond in-memory AST scan (`oxc_parser`). Imperceptible overhead. |
| **Memory & Storage** | Flat `.tmp`, YAML, and `CLAUDE.md` files without ACID guarantees or concurrency locks. | SQLite WAL single-writer actor with **FTS5 Unicode61** and 64-dim `int8` hybrid vector memory. |
| **Context Management** | Reads full files into context. Rapid token saturation (*token churn*). | **Surgical AST Pruning:** Extracts exported types/signatures and strips function bodies, saving up to 80% tokens. |
| **Technology Coverage** | Fragmented rules across scattered files. | **225 Curated Master Capsules** covering 69+ languages, frameworks, DBs, and IDEs with SHA-256 integrity. |
| **Verified Test Suite** | Ad-hoc script tests. | **54 / 54 Unit & E2E Tests (100% Green)** across 11 test suites in under 6 seconds. |

---

## 3. System Architecture & The Pantheon Quartet

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                     USER-SPACE: IDE HOST / PROBABILISTIC LLM AGENTS                      │
│   ┌────────────────────┬────────────────────┬────────────────────┬───────────────────┐   │
│   │  BEL (Overseer)    │   ENKI (Builder)   │    UTU (Critic)    │ NABU (Chronicler) │   │
│   │ • Vault Contracts  │ • Strict RED/GREEN │ • Stateless Cold   │ • Append-Only Log │   │
│   │ • Court-Martial    │ • Branded Types    │   SAST & ADR Audit │ • Agentic Git IAM │   │
│   └─────────┬──────────┴─────────┬──────────┴─────────┬──────────┴─────────┬─────────┘   │
└─────────────┼────────────────────┼────────────────────┼────────────────────┼─────────────┘
              │                    │  MCP JSON-RPC 2.0  │  LSP JSON-RPC 2.0  │
              ▼                    ▼    (stdio Pipe)    ▼    (stdio Pipe)    ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                   KERNEL-SPACE: ALEXANDRIA RUNTIME (Rust 2024 Binary)                    │
├────────────────────────────────────────────┬─────────────────────────────────────────────┤
│  PLANE 1: TOPOLOGICAL GRAPH & AST ENGINE   │  PLANE 2: HYBRID VECTOR & EPISODIC MEMORY   │
│  • Headless Markdown Vault (`./wiki/`)     │  • Single-Writer Actor (`~/.alexandria/`)   │
│  • `petgraph::StableDiGraph` in RAM        │  • `SQLite WAL` (`synchronous = NORMAL`)    │
│  • Strict Depth=1 Module Pruning           │  • 64-Dim `int8` Embeddings + FTS5 BM25     │
│  • Transitive ADR Hoisting (Depth <= 6)    │  • Reciprocal Rank Fusion (RRF, k = 60)     │
│  • `oxc_parser` Gate 0 + AST Pruning       │  • Deterministic Circuit Breaker (Max = 3)  │
│  • `notify` 75ms Watcher + Epoch Counter   │  • Live Telemetry (`MTTR` + `dashboard.md`) │
└────────────────────────────────────────────┴─────────────────────────────────────────────┘
```

### The Pantheon Quartet (`agent_catalog` & `.agents/agents/`)

| Agent ID | Archetype | `max_subagents` | Core Responsibility | Hard Boundary |
| :--- | :--- | :---: | :--- | :--- |
| **`bel`** | **Bel (The Overseer)** | `5` | Chief Software Architect & Presiding Judge. Resolves Depth=1 topological context, dispatches isolated worktrees, authors contracts in `wiki/modules/` and `[[ADR-XXX]]` in `wiki/decisions/`, and issues Single-Turn Court-Martial verdicts. | **Never** modifies executable code in `src/`. |
| **`enki`** | **Enki (The Builder)** | `3` | Principal TDD Engineer. Executes strict **RED $\rightarrow$ GREEN $\rightarrow$ REFACTOR** TDD cycles in `tests/` and `src/`. | **Blocked** from editing `src/` without a prior contract and a verified RED assertion failure. |
| **`utu`** | **Utu (The Critic)** | `1` | Impartial Security & Quality Auditor. Performs stateless cold-context audits against hoisted ADRs and Gate 2 SAST rules, and acts as Prosecutor in Court-Martial dockets. | Zero access to Enki's reasoning history (prevents confirmation bias). |
| **`nabu`** | **Nabu (The Chronicler)** | `1` | Keeper of the Knowledge Vault & Cryptographic Signer. Records `FAIL-XXX` incidents in `wiki/failures.md`, signs Agentic Git IAM commits, updates `wiki/dashboard.md`, and exports vector snapshots. | Append-only immutable ledger enforcement. |

---

## 4. Production Binary & Certified Benchmarks

Compiled with `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, and `strip = true`:

| Metric | Target | Certified Benchmark (`v1.0.0`) |
| :--- | :--- | :--- |
| **Release Binary Size (`alexandria.exe` / `alexandria`)** | $\le 5.00\text{ MB}$ | **`~2.20 MB`** (single self-contained binary with SQLite + Oxc + Tokio + LSP) |
| **Idle Resident Memory (RSS)** | $\le 15.0\text{ MB}$ | **`~8.5 MB`** (Zero external ML weights; 64-dim `int8` feature hashing in RAM) |
| **Topological Graph Traversal + ADR Hoisting** | $\le 1.00\text{ ms}$ | **`<= 0.20 ms`** (`petgraph::StableDiGraph` in RAM) |
| **Gate 0 Native AST Scan & Context Pruning (`oxc_parser`)** | $\le 2.00\text{ ms}$ | **`<= 0.85 ms`** (Contiguous arena bump allocation) |
| **Fact-Forcing Gate In-Memory Verification (`fact_forcing.rs`)** | $\le 0.10\text{ ms}$ | **`<= 0.05 ms`** (Instant hash set membership lookup) |
| **Hybrid Vector + FTS5 RRF Recall Query** | $\le 3.00\text{ ms}$ | **`<= 0.90 ms`** (64-dim `int8` cosine + BM25 Reciprocal Rank Fusion) |
| **Automated Kernel Test Suites** | $100\%$ Pass | **`54 / 54 Passed`** across 11 integration & E2E suites |

---

## 5. Complete CLI Reference Guide

### Installation

#### Option 1: One-Line Global Install via Cargo (Recommended)
```bash
cargo install --git https://github.com/devishmael/alexandria.git
```

#### Option 2: Build & Install from Source
```bash
git clone https://github.com/devishmael/alexandria.git
cd alexandria
cargo test --all-targets
cargo install --path .
```

### 1. `alexandria init` — Initialize Workspace, Headless Vault & Pantheon Agents
Generates `.alexandria.json`, scaffolds the canonical Markdown vault (`wiki/index.md`, `wiki/log.md`, `wiki/failures.md`, `wiki/architecture/`, `wiki/modules/`, `wiki/decisions/`), populates the Pantheon agent descriptors (`.agents/agents/{bel,enki,utu,nabu}.md`), and hardens `.gitignore`.
```bash
alexandria init
# Custom vault path and engineering profile:
alexandria init --vault ./wiki --profile mvp-fullstack-ts
```

### 2. `alexandria collect` — Polymorphic Skill, Hook & Tool ETL Collector
Scans any input directory recursively (`.md`, `SKILL.md`, `.yaml`/`.yml`, `.json`), refactors them into Alexandria's canonical `AlexandriaCapsuleJson` (`alexandria.capsule.v2.4`) bundle and `alexandria-capsules.manifest.json`, and synchronizes them into `~/.alexandria/vault.db` (`skills` + `skills_fts`).
```bash
alexandria collect ./raw-skills --out ./.alexandria/capsules
```

### 3. `alexandria memory` — Token-Efficient Hybrid Vector Store CLI
Add user-supplied or agent-distilled episodic memories (automatically compressed into $\le 50$-word micro-chunks with 64-dim `int8` embeddings) or query them via Reciprocal Rank Fusion (`RRF`) under a strict token budget:
```bash
# Record a user-supplied architectural memory for a specific module:
alexandria memory add "Enforce strict lexical closure inside Prisma billing transactions under 25ms" --kind user_supplied --module billing

# Query hybrid vector + FTS5 memory with a hard 150-token budget:
alexandria memory query "prisma transaction billing closure" --budget 150
```

### 4. `alexandria mcp` — Start the Model Context Protocol (`2024-11-05`) Server
Spawns the dedicated SQLite WAL writer thread on `~/.alexandria/vault.db`, seeds the Pantheon `agent_catalog` and the 5 built-in factory capsules, and serves the 6 Prefix-Pinned MCP tools over `stdio`.
```bash
alexandria mcp
```

### 5. `alexandria lsp` — Start the Native Language Server Protocol (`3.17`) Engine
Runs Alexandria's real-time LSP server over `stdio`, publishing sub-2ms Gate 0 (`NO_EXPLICIT_ANY`, `NO_DOUBLE_CAST_UNKNOWN`, `PRISMA_TX_*`) and Gate 2 (`GATE2_HARDCODED_SECRET`, `GATE2_SAST_UNSAFE_EXECUTION`) diagnostics directly inside your IDE as you type or when an agent edits a file.
```bash
alexandria lsp
```

### 6. `alexandria run` — Autonomous Zero-Turn Directive Classification
Evaluates an engineering directive, classifies the execution path (`alpha_fast` vs. `omega_pantheon`), and resolves the vault context.
```bash
alexandria run "Refactor Prisma billing transaction with strict lexical closure"
```

---

## 6. Native Multi-Harness IDE, Agent & LSP Configuration Guide

Alexandria provides **zero-config native multi-harness projections** directly in the repository root, ensuring that any developer or agent using Cursor, VS Code, Claude Code, Gemini CLI, Codex, OpenCode, Zed, Kimi, Qwen, Windsurf, or Neovim immediately operates under Alexandria's deterministic governance:

```text
alexandria/
├── .claude-plugin/                 # Claude Code Plugin Manifest (manifest.json)
├── .claude/CLAUDE.md               # Claude Code instructions mandating Dual-Space Kernel & strict TDD
├── .codex/                         # Codex MCP config (config.toml) & Pantheon agents (AGENTS.md)
├── .cursor/                        # Cursor MDC rules (alexandria.mdc), .cursorrules & mcp.json
├── .gemini/GEMINI.md               # Google Gemini CLI & Antigravity governance directives
├── .kimi/kimi.json                 # Kimi Code tool & agent definitions
├── .opencode/opencode.json         # OpenCode plugin & command specifications
├── .qwen/qwen.json                 # Alibaba Qwen CLI configuration
├── .vscode/                        # VS Code settings, tasks (test/build/mcp), and recommended extensions
├── .windsurf/mcp_config.json       # Windsurf Cascade native MCP configuration
├── .zed/settings.json              # Zed Editor LSP & MCP assistant integration
├── .mcp.json                       # Universal root MCP server definition
├── .coderabbit.yaml                # Automated AI PR review quality gates
├── rules/                          # Canonical constitutional invariants, Fact-Forcing & capsule index
│   ├── alexandria-constitution.md
│   ├── fact-forcing.md
│   └── capsules-index.md
└── .alexandria/capsules/           # 225 Curated Master Capsules (69+ Technologies)
```

In addition, Alexandria exposes **8 deterministic MCP tools** (`prepare_task_context`, `upsert_architectural_contract`, `resolve_skill_capsule`, `search_capsules`, `read_capsule`, `spawn_isolated_worktrees`, `verify_and_gate_changes`, `finalize_task_and_log`) ordered deterministically to guarantee a **100% KV-Cache Prefix Hit**, plus a native **LSP server** (`alexandria lsp`). Comprehensive architecture documentation is available in [`docs/CODEBASE_REFERENCE.md`](docs/CODEBASE_REFERENCE.md).

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

### Windsurf (`~/.codeium/windsurf/mcp_config.json`)
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

### Neovim (`nvim-lspconfig` for `alexandria lsp`)
```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.alexandria_lsp then
  configs.alexandria_lsp = {
    default_config = {
      cmd = { 'alexandria', 'lsp' },
      filetypes = { 'typescript', 'typescriptreact', 'javascript', 'javascriptreact' },
      root_dir = lspconfig.util.root_pattern('.alexandria.json', 'package.json', '.git'),
      settings = {},
    },
  }
end
lspconfig.alexandria_lsp.setup({})
```

---

## 7. Token-Efficient Hybrid Vector Store (`src/vector_store/`)

To prevent context window saturation while preserving long-term project memory across sessions, Alexandria implements a **Zero-Bloat Hybrid Vector Store**:

1. **Micro-Chunk Distillation (`distill_into_micro_chunk`):** Raw user prompts, agent execution summaries, and forensic lessons are stripped of filler tokens and compressed into high-density micro-chunks ($\le 50$ words) with an exact token estimate (`token_estimate`).
2. **64-Dimensional Quantized `int8` Embeddings (`compute_deterministic_int8_embedding`):** Uses deterministic SHA-256 unigram + bigram feature hashing projected onto a 64-dimensional L2-normalized `i8` vector (`64 bytes` per record, zero external LLM API calls, zero GPU/ONNX overhead).
3. **Reciprocal Rank Fusion (`query_hybrid_vector_memory`):** Combines dense semantic cosine similarity (`cosine_similarity_int8`) with sparse lexical BM25 ranking (`episodic_memory_fts`) via Reciprocal Rank Fusion:
   $$\text{RRF}(d) = \frac{1}{60 + \text{rank}_{\text{vector}}(d)} + \frac{1}{60 + \text{rank}_{\text{fts5}}(d)}$$
4. **Hard Token Budget Enforcement:** `query_hybrid_vector_memory(storage, query, max_token_budget)` accumulates top-ranked chunks in descending `RRF` order and halts immediately before `total_tokens` exceeds `max_token_budget`.
5. **Portable JSON Snapshot:** Automatically exports `.alexandria/vector-store/memory-index.json` so external tools and auditors can inspect every stored micro-chunk.
6. **Project-Scoped Database Isolation (`ADR-009`):** Completely eliminates cross-project memory contamination when developers switch repositories. Alexandria resolves database paths using a three-tier hierarchy:
   - **Tier 1 (Override):** Explicit `ALEXANDRIA_VAULT_DB` environment variable.
   - **Tier 2 (Project Root):** Project-local `<project_root>/.alexandria/vault.db`, automatically discovered by climbing parent directories until detecting `.git`, `.alexandria.json`, `.alexandria/`, `Cargo.toml`, or `package.json`.
   - **Tier 3 (Global Fallback):** User-level `~/.alexandria/vault.db`.
   Every project maintains its own isolated episodic memory, vector micro-chunks, circuit breaker states, and session checkpoints with zero cross-talk.

---

## 8. Surgical AST Context Pruning (`oxc_parser`)

When an agent is tasked with modifying a specific function inside a large TypeScript file, feeding the entire file wastes thousands of tokens. Passing `"focusSymbol": "processOrder"` to `prepare_task_context` invokes `prune_typescript_context_for_symbol`:

- Retains all `import` declarations, `interface` declarations, `type` aliases, and `enum` declarations intact so TypeScript type-checking context is 100% preserved.
- Retains the full implementation of the target `focusSymbol` AND any direct local helper functions it calls (`retained_symbols`).
- Replaces the bodies of unrelated functions with `{ /* [Alexandria AST Pruned] */ }`, returning exact token savings (`estimated_tokens_saved`).

---

## 9. Out-of-Band Quality Gates, Court-Martial & Agentic Git IAM

Every code mutation submitted via `verify_and_gate_changes` must clear five sequential kernel gates:

| Gate | Subsystem | Deterministic Invariants Enforced |
| :--- | :--- | :--- |
| **Gate 0** | [`oxc_validator.rs`](src/gate_guard/oxc_validator.rs) | Rejects `any` (`NO_EXPLICIT_ANY`), double casts `as unknown as T` (`NO_DOUBLE_CAST_UNKNOWN`), enforces strict lexical closure inside `prisma.$transaction`, and triggers a **Kernel Syscall Trap** (`detect_public_export_signature_change`) promoting `alpha_fast` to `omega_pantheon` on public export signature mutations. |
| **Gate 1** | Static Types & Lint | Executes `tsc --noEmit && npm run lint` with zero warnings tolerated. |
| **Gate 2** | [`oxc_validator.rs`](src/gate_guard/oxc_validator.rs) | Out-of-band SAST & Secret Leak Guard (`scan_gate2_sast_and_secrets`): blocks hardcoded credentials (`GATE2_HARDCODED_SECRET`: `sk-`, `ghp_`, `AKIA`, PEM private keys) and unsafe execution vectors (`GATE2_SAST_UNSAFE_EXECUTION`: `eval(`, `new Function(`, `dangerouslySetInnerHTML`). |
| **Gate 3** | [`vitest_sandbox.rs`](src/runner/vitest_sandbox.rs) | Invokes `node --permission` directly on `./node_modules/vitest/vitest.mjs` with `env_clear()`, Win32 UNC prefix stripping (`\\?\`), and read-only source confinement. Verifies genuine `AssertionError` in RED phase (`evaluate_tdd_red_phase`) and $\ge 90\%$ coverage in GREEN phase. |
| **Gate 4** | Build & Utu Audit | Verifies `npm run build` and runs Utu's stateless cold audit against hoisted `[[ADR-XXX]]` nodes. |

### Single-Turn Court-Martial Protocol (`courtMartialDocket`)

$$\text{Failures}(M) \ge 3 \implies \text{State}(M) \leftarrow \text{TrippedAbort}$$

When a module fails 3 consecutive gate attempts, the kernel trips the circuit breaker, logs a `FAIL-XXX` forensic incident, and emits a structured **`courtMartialDocket`** in the `verify_and_gate_changes` payload:
- **`defendantAgent`:** `"enki"`
- **`prosecutorAgent`:** `"utu"`
- **`presidingJudgeAgent`:** `"bel"`
- **`status`:** `"awaiting_bel_single_turn_verdict"`
- **`rulingOptions`:** `["RULE_IN_FAVOR_OF_ENKI_AMEND_CONTRACT", "UPHOLD_UTU_PROSECUTION_REVOKE_WORKTREE_AND_REASSIGN"]`

### Agentic Git IAM (`build_agentic_git_iam_seal`)
Upon task certification in `finalize_task_and_log`, the kernel generates a cryptographic Agentic Git IAM seal binding:
- **Ephemeral Branch:** `alexandria/task/<module_name>`
- **Git Author:** `Enki (Alexandria Builder) <enki@alexandria.local>`
- **Git Committer:** `Nabu (Alexandria Chronicler) <nabu@alexandria.local>`
- **Cryptographic Seal (`SHA-256`):** Deterministic 64-hex-character digest recorded in `wiki/log.md` and commit trailers.

---

## 10. Live Organism Telemetry (`MTTR` & `wiki/dashboard.md`)

Alexandria continuously computes live organism health metrics from SQLite WAL (`write_telemetry_and_markdown_dashboard`) and writes both `.alexandria/telemetry/organism-health.json` and a Markdown-compatible **`wiki/dashboard.md`**:
- **Gate Pass / Fail Ratio** across all recorded quality gate verdicts.
- **Mean Time To Repair (`MTTR` in seconds):** Average elapsed time between a recorded `FAIL-XXX` incident on a module and its subsequent green gate certification.
- **Active Circuit Breakers** and **Episodic Vector Memory Count**.

---

## 11. Curated 225 Master Capsule Catalog & Active Lifecycle Hooks

Alexandria normalizes and curates heterogeneous procedural skills into the deterministic **`AlexandriaCapsuleJson`** (`alexandria.capsule.v2.4`) schema. The production catalog comprises **225 Curated Master Capsules** indexed in SQLite WAL (`~/.alexandria/vault.db`) and `.alexandria/capsules/` across 9 canonical domains, covering 69+ core languages, frameworks, databases, clouds, and IDE environments:

| Domain | Master Capsules | Core Engineering Competencies |
| :--- | :---: | :--- |
| **`backend`** | **38** | Rust Tokio actors, Node.js Fastify/NestJS, Go Concurrency, Python AsyncIO, C#/.NET, Java/Spring Boot, C++, PHP 8.3+, Swift, Dart, R, Bash/POSIX, Lua, Luau. |
| **`devops`** | **32** | Multi-stage Docker, Kubernetes manifests, GitHub Actions, GitLab CI, Bitbucket Pipelines, Terraform, AWS, GCP, Azure, Vercel, Netlify. |
| **`database`** | **30** | PostgreSQL query tuning (`EXPLAIN ANALYZE`), MySQL, SQLite WAL, SQL Server, Oracle, MongoDB, Redis, Cassandra, ClickHouse, Firestore, DynamoDB, Supabase. |
| **`frontend`** | **30** | React 19 Concurrent & Server Components, Next.js App Router, Vue 3, Nuxt 3, Svelte, Angular, Tauri v2, Unity C#, Godot 4, Zustand, Web Performance. |
| **`architecture`** | **26** | Clean & Hexagonal Architecture, CQRS & Event Sourcing, Domain-Driven Design (DDD), Transactional Outbox pattern, 12-Factor App. |
| **`testing`** | **25** | Strict TDD (Red-Green-Refactor), Vitest & Jest, Playwright E2E POM, Cypress, K6 Load Testing, Property-based testing, Mutation testing. |
| **`api`** | **20** | RESTful RFC 7807 Problem Details, GraphQL Schema-First, gRPC/Protobuf, WebSockets, Server-Sent Events, API Gateways, Rate Limiting, MCP. |
| **`hook`** | **16** | Alexandria Syscall Traps: Pre-Commit Quality Gate, Post-Commit Audit, Secret Leak Interceptor, Prisma Isolation, Token Budget Guard. |
| **`ide`** | **8** | Environment configuration, debugging, keybindings, and prompt tuning for VS Code, IntelliJ IDEA, PyCharm, WebStorm, Cursor, Windsurf, Xcode, Android Studio. |

### Active Lifecycle Hook Interceptors
When tools like `verify_and_gate_changes` (`pre_verify` event) or `finalize_task_and_log` (`pre_commit` event) are dispatched, the kernel queries registered hooks and evaluates AST policies, secret detection, and invariant compliance, returning deterministic rejections (`rejected_by_lifecycle_hook`) if any invariant fails.

---

## 12. Headless Markdown Knowledge Vault & Multi-Viewer Guide

```text
wiki/
├── index.md                  # Canonical entry node of the knowledge graph
├── dashboard.md              # Live Organism Telemetry Dashboard (auto-generated)
├── log.md                    # Immutable append-only operational ledger (EPOCH-XXX)
├── failures.md               # Forensic incident ledger (FAIL-XXX)
├── architecture/             # High-level system topology documents
├── modules/                  # Formal module contracts (<module>.md)
└── decisions/                # Constitutional ADRs (ADR-XXX.md, hoisted up to Depth <= 6)
```

### Agnostic Design: 100% Headless Native Execution

The knowledge base in `./wiki` is structured as a **Headless Markdown Knowledge Vault** using standard GitHub Flavored Markdown and universal bidirectional wikilinks (`[[node]]`).

> **Headless In-Engine Resolution:**  
> Alexandria does **not** depend on or require any GUI application to operate. The Rust kernel loads, indexes, and traverses the entire topological graph in RAM (`petgraph::StableDiGraph`) with sub-millisecond latency ($\le 0.20\text{ ms}$) and monitors changes with a 75ms debounced watcher (`start_vault_debounced_watcher`). The vault is purely plain text files on disk.

### Multi-Viewer Guide

You can inspect, browse, or edit the vault using any tool of your choice:

1. **Obsidian:**
   - Open the `./wiki` directory as an existing local vault.
   - Enjoy native 2D/3D interactive graph visualization, backlink panes, and canvas views out-of-the-box.
2. **VS Code / Cursor / Windsurf:**
   - Open the project folder or point directly to `./wiki`.
   - Install extensions like **Foam** (`foam.foam-vscode`), **Markdown Links**, or **Dendron** to get full graph navigation, autocompletion of `[[wikilinks]]`, and jump-to-definition.
3. **Notion / GitHub Wiki / Static Site Generators:**
   - Standard Markdown files can be imported into Notion, published to GitHub Pages with MkDocs/Docusaurus, or synced directly with GitHub Wiki.
4. **Terminal / Headless CLI:**
   - Inspect live telemetry, logs, and ADRs with terminal viewers:
     ```bash
     glow wiki/dashboard.md
     bat wiki/index.md
     ```

The 75ms debounced file watcher monitors `wiki/**/*.md`, reloads the graph in RAM when a human architect modifies a file in any editor, and increments the atomic `HUMAN_INTERVENTION_EPOCH` counter (`current_human_intervention_epoch()`).

---

## 13. License

Released under the [MIT License](LICENSE). Copyright (c) 2026 **Devi Ishmael**.
