# 🏛️ Alexandria — The Deterministic Agent Harness

[![Release](https://img.shields.io/badge/Release-v1.0.0-0969da?logo=github)](https://github.com/devishmael/alexandria-runtime/releases/tag/v1.0.0)
![Rust Edition](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)
![MCP Protocol](https://img.shields.io/badge/MCP-2024--11--05-blue)
![LSP Protocol](https://img.shields.io/badge/LSP-3.17_Native-purple)
![SQLite WAL + FTS5 + Int8 RRF](https://img.shields.io/badge/Storage-SQLite_WAL_%2B_FTS5_%2B_Int8_Vector_RRF-003B57?logo=sqlite)
![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)

> **Headless Deterministic Agent Harness, Kernel-Space Arbitrator, Hybrid Vector Store & Native LSP Server in Rust.**  
> Governing probabilistic User-Space LLMs via zero-trust Oxc AST gates, surgical AST context pruning, token-efficient hybrid vector memory (`int8` + BM25 Reciprocal Rank Fusion), OS-sandboxed TDD verification, Single-Turn Court-Martial arbitration, Agentic Git IAM, live organism telemetry (`MTTR` + `dashboard.md`), and an agnostic Headless Markdown Knowledge Vault.

---

## Table of Contents

1. [Executive Summary: The Dual-Space Paradigm](#1-executive-summary-the-dual-space-paradigm)
2. [System Architecture & The Pantheon Quartet](#2-system-architecture--the-pantheon-quartet)
3. [Production Binary & Certified Benchmarks](#3-production-binary--certified-benchmarks)
4. [Complete CLI Reference Guide](#4-complete-cli-reference-guide)
5. [Multi-Host IDE, Agent & LSP Configuration Guide](#5-multi-host-ide-agent--lsp-configuration-guide)
6. [Token-Efficient Hybrid Vector Store (`src/vector_store/`)](#6-token-efficient-hybrid-vector-store-srcvector_store)
7. [Surgical AST Context Pruning (`oxc_parser`)](#7-surgical-ast-context-pruning-oxc_parser)
8. [Out-of-Band Quality Gates, Court-Martial & Agentic Git IAM](#8-out-of-band-quality-gates-court-martial--agentic-git-iam)
9. [Live Organism Telemetry (`MTTR` & `wiki/dashboard.md`)](#9-live-organism-telemetry-mttr--wikidashboardmd)
10. [Polymorphic Skill & Hook ETL Collector & 5-Skill Starter Pack](#10-polymorphic-skill--hook-etl-collector--5-skill-starter-pack)
11. [Headless Markdown Knowledge Vault & Multi-Viewer Guide](#11-headless-markdown-knowledge-vault--multi-viewer-guide)
12. [License](#12-license)

---

## 1. Executive Summary: The Dual-Space Paradigm

Modern AI coding assistants suffer from **context rot**, **hallucinated green builds**, **silent architectural drift**, **token bloat**, and **destructive retry loops (*LLM Thrashing*)** when left to self-police through prompt engineering alone.

**Alexandria** solves this by enforcing a strict operating-system boundary between **Kernel-Space** and **User-Space**:

- **Kernel-Space (`alexandria` Rust Binary):** The deterministic authority. It owns the filesystem knowledge graph (`petgraph`), native TypeScript/TSX AST parsing and surgical context pruning (`oxc_parser` + `oxc_resolver`), OS-level process sandboxing (`node --permission`), polymorphic Skill/Hook ETL normalization (`src/collector/`), single-writer SQLite WAL + FTS5 BM25 + 64-dim `int8` hybrid vector memory (`rusqlite`), real-time Language Server Protocol (`LSP`) diagnostics, and the deterministic Circuit Breaker.
- **User-Space (Probabilistic LLM Host):** Untrusted compute process (Google Antigravity 2.0, Claude Code, Cursor, Windsurf, VS Code, Neovim). It cannot self-certify compilation, test verdicts, or architectural compliance; every mutation is arbitrated out-of-band by the Rust kernel via **MCP (`2024-11-05`)** and **LSP (`3.17`)**.

---

## 2. System Architecture & The Pantheon Quartet

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

## 3. Production Binary & Certified Benchmarks

Compiled with `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, and `strip = true`:

| Metric | Target | Certified Benchmark (`v1.0.0`) |
| :--- | :--- | :--- |
| **Release Binary Size (`alexandria.exe` / `alexandria`)** | $\le 5.00\text{ MB}$ | **`~2.20 MB`** (single self-contained binary with SQLite + Oxc + Tokio + LSP) |
| **Idle Resident Memory (RSS)** | $\le 15.0\text{ MB}$ | **`~8.5 MB`** (Zero external ML weights; 64-dim `int8` feature hashing in RAM) |
| **Topological Graph Traversal + ADR Hoisting** | $\le 1.00\text{ ms}$ | **`<= 0.20 ms`** (`petgraph::StableDiGraph` in RAM) |
| **Gate 0 Native AST Scan & Context Pruning (`oxc_parser`)** | $\le 2.00\text{ ms}$ | **`<= 0.85 ms`** (Contiguous arena bump allocation) |
| **Hybrid Vector + FTS5 RRF Recall Query** | $\le 3.00\text{ ms}$ | **`<= 0.90 ms`** (64-dim `int8` cosine + BM25 Reciprocal Rank Fusion) |
| **Automated Kernel Test Suites** | $100\%$ Pass | **`48 / 48 Passed`** across 10 integration & E2E suites |

---

## 4. Complete CLI Reference Guide

### Build & Install from Source
```bash
git clone https://github.com/devishmael/alexandria-runtime.git
cd alexandria-runtime
cargo test --all-targets
cargo build --release
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

## 5. Multi-Host IDE, Agent & LSP Configuration Guide

Alexandria exposes **8 deterministic MCP tools** (`prepare_task_context`, `upsert_architectural_contract`, `resolve_skill_capsule`, `search_capsules`, `read_capsule`, `spawn_isolated_worktrees`, `verify_and_gate_changes`, `finalize_task_and_log`) ordered deterministically to guarantee a **100% KV-Cache Prefix Hit**, plus a native **LSP server** (`alexandria lsp`). Comprehensive architecture documentation is available in [`docs/CODEBASE_REFERENCE.md`](docs/CODEBASE_REFERENCE.md).

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

## 6. Token-Efficient Hybrid Vector Store (`src/vector_store/`)

To prevent context window saturation while preserving long-term project memory across sessions, Alexandria implements a **Zero-Bloat Hybrid Vector Store**:

1. **Micro-Chunk Distillation (`distill_into_micro_chunk`):** Raw user prompts, agent execution summaries, and forensic lessons are stripped of filler tokens and compressed into high-density micro-chunks ($\le 50$ words) with an exact token estimate (`token_estimate`).
2. **64-Dimensional Quantized `int8` Embeddings (`compute_deterministic_int8_embedding`):** Uses deterministic SHA-256 unigram + bigram feature hashing projected onto a 64-dimensional L2-normalized `i8` vector (`64 bytes` per record, zero external LLM API calls, zero GPU/ONNX overhead).
3. **Reciprocal Rank Fusion (`query_hybrid_vector_memory`):** Combines dense semantic cosine similarity (`cosine_similarity_int8`) with sparse lexical BM25 ranking (`episodic_memory_fts`) via Reciprocal Rank Fusion:
   $$\text{RRF}(d) = \frac{1}{60 + \text{rank}_{\text{vector}}(d)} + \frac{1}{60 + \text{rank}_{\text{fts5}}(d)}$$
4. **Hard Token Budget Enforcement:** `query_hybrid_vector_memory(storage, query, max_token_budget)` accumulates top-ranked chunks in descending `RRF` order and halts immediately before `total_tokens` exceeds `max_token_budget`.
5. **Portable JSON Snapshot:** Automatically exports `.alexandria/vector-store/memory-index.json` so external tools and auditors can inspect every stored micro-chunk.

---

## 7. Surgical AST Context Pruning (`oxc_parser`)

When an agent is tasked with modifying a specific function inside a large TypeScript file, feeding the entire file wastes thousands of tokens. Passing `"focusSymbol": "processOrder"` to `prepare_task_context` invokes `prune_typescript_context_for_symbol`:

- Retains all `import` declarations, `interface` declarations, `type` aliases, and `enum` declarations intact so TypeScript type-checking context is 100% preserved.
- Retains the full implementation of the target `focusSymbol` AND any direct local helper functions it calls (`retained_symbols`).
- Replaces the bodies of unrelated functions with `{ /* [Alexandria AST Pruned] */ }`, returning exact token savings (`estimated_tokens_saved`).

---

## 8. Out-of-Band Quality Gates, Court-Martial & Agentic Git IAM

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

## 9. Live Organism Telemetry (`MTTR` & `wiki/dashboard.md`)

Alexandria continuously computes live organism health metrics from SQLite WAL (`write_telemetry_and_markdown_dashboard`) and writes both `.alexandria/telemetry/organism-health.json` and a Markdown-compatible **`wiki/dashboard.md`**:
- **Gate Pass / Fail Ratio** across all recorded quality gate verdicts.
- **Mean Time To Repair (`MTTR` in seconds):** Average elapsed time between a recorded `FAIL-XXX` incident on a module and its subsequent green gate certification.
- **Active Circuit Breakers** and **Episodic Vector Memory Count**.

---

## 10. Curated Master Capsule Catalog & Active Lifecycle Hooks

Alexandria normalizes and curates heterogeneous procedural skills into the deterministic **`AlexandriaCapsuleJson`** (`alexandria.capsule.v2.4`) schema. The production catalog comprises **189 Curated Master Capsules** indexed in SQLite WAL (`~/.alexandria/vault.db`) and `.alexandria/capsules/` across 8 canonical domains:

| Domain | Master Capsules | Core Engineering Competencies |
| :--- | :---: | :--- |
| **`backend`** | **28** | Rust Tokio actors, Node.js Fastify/NestJS, Go Concurrency, Python AsyncIO, RabbitMQ, Zero-Copy serialization. |
| **`architecture`** | **26** | Clean & Hexagonal Architecture, CQRS & Event Sourcing, Domain-Driven Design (DDD), Transactional Outbox pattern. |
| **`devops`** | **26** | Multi-stage Docker, Kubernetes manifests, GitHub Actions CI/CD pipelines, Terraform IaC, Prometheus telemetry. |
| **`frontend`** | **26** | React 19 Concurrent & Server Components, Next.js App Router, Vue 3 Composition, Zustand, Web Performance. |
| **`testing`** | **25** | Strict TDD (Red-Green-Refactor), Vitest & Jest, Playwright E2E POM, Property-based testing, Mutation testing. |
| **`database`** | **22** | PostgreSQL query tuning (`EXPLAIN ANALYZE`), Prisma/Drizzle transactions, SQLite WAL concurrency, Redis sharding. |
| **`api`** | **20** | RESTful RFC 7807 Problem Details, GraphQL Schema-First, gRPC/Protobuf, WebSockets, API Gateways, OAuth2/OIDC. |
| **`hook`** | **16** | Alexandria Syscall Traps: Pre-Commit Quality Gate, Post-Commit Audit, Secret Interceptor, Prisma Isolation. |

### Active Lifecycle Hook Interceptors
When tools like `verify_and_gate_changes` (`pre_verify` event) or `finalize_task_and_log` (`pre_commit` event) are dispatched, the kernel queries registered hooks and evaluates AST policies, secret detection, and invariant compliance, returning deterministic rejections (`rejected_by_lifecycle_hook`) if any invariant fails.

---

## 11. Headless Markdown Knowledge Vault & Multi-Viewer Guide

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

## 12. License

Released under the [MIT License](LICENSE). Copyright (c) 2026 **Devi Ishmael**.
