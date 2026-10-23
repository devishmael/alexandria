# Alexandria

> A lightweight, deterministic control layer and memory engine in Rust that keeps AI coding agents focused, verified, and aligned with project architecture.

---

## Table of Contents

1. [What is Alexandria?](#what-is-alexandria)
2. [Why Alexandria?](#why-alexandria)
3. [Architecture: Control Layer vs. AI Agents](#architecture-control-layer-vs-ai-agents)
4. [Core Features](#core-features)
5. [Language Support & Technology Roadmap](#language-support--technology-roadmap)
6. [Agent Roles & Workflow](#agent-roles--workflow)
7. [Installation & Quick Start](#installation--quick-start)
8. [CLI Reference](#cli-reference)
9. [IDE & Agent Configuration](#ide--agent-configuration)
10. [Local Hybrid Memory System](#local-hybrid-memory-system)
11. [Knowledge Vault & Documentation Structure](#knowledge-vault--documentation-structure)
12. [License](#license)

---

## What is Alexandria?

Alexandria is a single, self-contained native binary (~2.2 MB, written in Rust) that connects to AI coding assistants (like Cursor, Claude Code, Google Antigravity, Windsurf, or VS Code) via the **Model Context Protocol (MCP)** and **Language Server Protocol (LSP)**.

Instead of relying solely on long prompt files that LLMs often ignore as conversations get long, Alexandria runs locally as a deterministic supervisor:

- **Enforces pre-commit and in-memory quality checks:** Blocks bad syntax, forbidden patterns (like `any`), and leaked credentials before files are written or committed.
- **Provides fast local memory:** Stores episodic project memories and technical capsules using SQLite WAL with hybrid vector + full-text search, with zero external API calls and zero heavy GPU models.
- **Reduces token usage:** Dynamically trims large source files using AST pruning so agents only read what they need to edit, saving up to 80% on context tokens.
- **Maintains architectural alignment:** Resolves documentation contracts and Architectural Decision Records (ADRs) on demand, keeping project structure consistent over time.

---

## Why Alexandria?

When working on non-trivial codebases with LLM-based agents, several recurring problems appear:

1. **Context Drift & Hallucinated Tests:** As conversation history grows, models lose track of initial rules, skip unit tests, or report that tests passed when they were never run.
2. **Blind Edits:** Agents often modify files without first reading the related module definitions or checking imports, causing unintended regressions.
3. **Prompt Fatigue:** Trying to solve governance by adding more markdown instructions (`.cursorrules`, `CLAUDE.md`) leads to token bloat and slower responses.
4. **Tool Bypass:** In many editors, agents can use native file-writing tools to bypass guidelines entirely.

Alexandria addresses these issues by moving enforcement out of prompts and into a native Rust process that acts as an objective, out-of-band validator.

---

## Architecture: Control Layer vs. AI Agents

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                           DEVELOPER ENVIRONMENT & AI ASSISTANTS                          │
│                     (Cursor, Claude Code, Antigravity, VS Code, Zed)                     │
│   ┌────────────────────┬────────────────────┬────────────────────┬───────────────────┐   │
│   │     ARCHITECT      │      BUILDER       │      REVIEWER      │    CHRONICLER     │   │
│   │ • Designs specs    │ • Strict TDD       │ • Independent code │ • Change logging  │   │
│   │ • Reviews impact   │ • Implements code  │   and safety audit │ • Session records │   │
│   └─────────┬──────────┴─────────┬──────────┴─────────┬──────────┴─────────┬─────────┘   │
└─────────────┼────────────────────┼────────────────────┼────────────────────┼─────────────┘
              │                    │  MCP JSON-RPC 2.0  │  LSP JSON-RPC 2.0  │
              ▼                    ▼    (stdio pipe)    ▼    (stdio pipe)    ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                         ALEXANDRIA RUNTIME (Rust 2024 Binary)                            │
├────────────────────────────────────────────┬─────────────────────────────────────────────┤
│      GRAPH ENGINE & STATIC ANALYSIS        │      LOCAL PERSISTENCE & MEMORY ENGINE      │
│  • Markdown Knowledge Vault (`./wiki/`)    │  • Single-Writer SQLite WAL Actor           │
│  • In-Memory Dependency Graph (`petgraph`) │  • 128-dim int8 Vector + FTS5 BM25 Search   │
│  • AST Analysis & Token Pruning (`oxc`)    │  • Reciprocal Rank Fusion (RRF, k = 60)     │
│  • Git Pre-Commit Quality Guard            │  • Execution Circuit Breaker (Max Retries)  │
│  • File Watcher (75ms debounce)            │  • Project Health & Session Checkpoints     │
└────────────────────────────────────────────┴─────────────────────────────────────────────┘
```

Alexandria divides responsibilities cleanly:

- **AI Agents (User-Space):** Generate code, suggest designs, and run tasks within their IDE.
- **Alexandria (Control Layer):** Intercepts tool calls, checks AST rules in memory in under 2ms, runs security filters, manages long-term memory, and verifies test suites in an isolated sandbox.

---

## Core Features

### 1. In-Memory AST Scanning (Gate 0)
Using the high-performance `oxc_parser` engine in Rust, Alexandria scans TypeScript and JavaScript code in memory in under 2ms. It blocks explicit `any` types, double casts (`as unknown as T`), and unsafe transaction escapes before code ever reaches disk.

### 2. Physical Git Pre-Commit Hook (Vector 1)
To prevent agents from bypassing MCP tools using native editor commands, `alexandria init` and `alexandria harness add git` install an automated pre-commit hook (`.git/hooks/pre-commit`). If staged code contains AST violations or leaked secrets, the commit is automatically blocked in the terminal.

### 3. Surgical Context Pruning
When an agent only needs to edit a specific function in a 1,000-line file, passing that symbol name to Alexandria extracts the target function and its type dependencies while replacing unrelated function bodies with lightweight stubs. This preserves full type-checking context while reducing token usage by up to 80%.

### 4. Zero-Bloat Local Vector Store (Vector 2)
Alexandria implements a 128-dimensional quantized `int8` feature-hashing embedding model paired with SQLite FTS5 BM25. It performs hybrid semantic search in under 1ms with zero cloud dependencies and zero GPU requirements, running completely inside ~8.5 MB of RAM.

### 5. Multi-Harness Scaffolding
A single command (`alexandria harness add all`) scaffolds project configurations for Cursor (`.cursor/`), Claude Code (`.claude/`), Antigravity / Gemini (`GEMINI.md`), Codex (`AGENTS.md`), VS Code (`.vscode/`), and Git (`.git/hooks/`).

### 6. Pre-Compact Checkpoints
When an LLM context window fills up and triggers a `/compact` command, critical architectural decisions and test state are often erased. Alexandria automatically saves transactional state snapshots in SQLite WAL before compaction so agents can resume cleanly.

---

## Language Support & Technology Roadmap

We believe in being transparent about what works across all stacks today versus where language-specific engines are currently focused.

### What Works for Any Technology Today (v1.1.0)
If you work with **Python, Go, Rust, Java, C#, C++, Docker, or Kubernetes**, the following capabilities are 100% stack-agnostic:

- **Architectural Contracts & Graph:** Documenting module contracts, ADRs, and dependencies in Markdown.
- **MCP Protocol & Tools:** All 8 tools (`prepare_task_context`, `verify_and_gate_changes`, `search_capsules`, etc.) work identically.
- **Git Pre-Commit Quality Guard:** Terminal-level commit verification works on any git repository.
- **Local Hybrid Memory:** Episodic memory, token-budgeted queries, and RRF search work across any domain.
- **Security & Secret Leak Guard (Gate 2):** Scans for API keys (`sk-`, `ghp_`, `AKIA`, PEM private keys) and dangerous execution calls across all file types.
- **225 Curated Master Capsules:** Ready-to-use best practices covering 69+ technologies (Postgres, Redis, Docker, FastAPI, Tokio, Spring Boot, etc.).

### Specialized Engine Focus (TypeScript / Node.js)
In version 1.1.0, two subsystems have deep language-specific implementations:
- **Gate 0 AST Scanning & Pruning:** Built on `oxc_parser`, specialized for TypeScript, TSX, JavaScript, and JSX.
- **Gate 3 Sandboxed Runner:** Built on Node.js `--permission` and Vitest for hermetic test execution with JSON reporting.

### Polyglot Roadmap (Sprint v1.2.0)
We are actively expanding native AST parsing and sandboxing to other major languages:

| Language | AST Parsing & Pruning | Test Sandbox Runner | Status |
| :--- | :--- | :--- | :---: |
| **TypeScript / JavaScript** | `oxc_parser` ($\le 2\text{ ms}$) | Vitest + `node --permission` | **Shipped (v1.1.0)** |
| **Rust** | `syn` + `quote` | `cargo test --message-format=json` | **In Development (v1.2.0)** |
| **Python** | `tree-sitter-python` | `pytest --json-report` | **In Development (v1.2.0)** |
| **Go** | `tree-sitter-go` | `go test -json` | **Planned (v1.2.0)** |

---

## Agent Roles & Workflow

Rather than treating AI agents as generic chatbots, Alexandria organizes work into four distinct operational roles:

| Role | Archetype | Responsibilities | Boundary |
| :--- | :--- | :--- | :--- |
| **Architect** | Planner & Governor | Resolves dependency graphs, authors architectural contracts (`modules/`), documents ADRs, and handles escalation if retries fail. | Never modifies implementation code in `src/`. |
| **Builder** | Engineer | Implements features and bug fixes following strict Test-Driven Development (Red $\rightarrow$ Green $\rightarrow$ Refactor). | Cannot edit `src/` without an existing contract and failing test. |
| **Reviewer** | Security & Auditor | Performs independent audits in a clean, stateless context against active ADRs and security rules without bias. | Has no access to the Builder's previous reasoning chain. |
| **Chronicler** | Recorder & Signer | Maintains the operational changelog (`wiki/log.md`), records incident details, and saves memory checkpoints. | Append-only ledger updates. |

### Circuit Breaker & Retry Limits
If an agent attempts to fix a failing quality gate and fails 3 consecutive times, Alexandria triggers an automated circuit breaker. This freezes changes on that module, stops token consumption, logs an incident report (`wiki/failures.md`), and requests human review to decide whether to update the contract or revise the approach.

---

## Installation & Quick Start

### Option 1: Global Install via Cargo (Recommended)
```bash
cargo install --git https://github.com/devishmael/alexandria.git
```

### Option 2: Build from Source
```bash
git clone https://github.com/devishmael/alexandria.git
cd alexandria
cargo test
cargo install --path .
```

### Initialize a New or Existing Repository
```bash
cd your-project
alexandria init
```
This automatically:
1. Creates `.alexandria.json` and `.alexandria/` local storage.
2. Scaffolds the documentation vault in `./wiki/` (or `./alexandria/`).
3. Installs configurations for Cursor, Claude Code, Antigravity, and Git pre-commit hooks.

### Run System Health Check
```bash
alexandria doctor
```

---

## CLI Reference

### `alexandria doctor` (or `status`)
Runs an instant 5-point system check: validates binary installation, SQLite WAL database, vault nodes, multi-harness setup, Node.js runtime version, and active circuit breakers.
```bash
alexandria doctor
```

### `alexandria gate <path>` (or `check <path>`)
Runs fast in-memory AST checks and security scans without needing an LLM:
```bash
# Check a single file:
alexandria gate src/index.ts

# Check an entire source folder:
alexandria gate src/
```

### `alexandria capsule [list | search | show]`
Browses the 225 curated best-practice engineering capsules:
```bash
# List all capsules or filter by domain:
alexandria capsule list --domain backend

# Search capsules with instant full-text search:
alexandria capsule search postgresql

# Display full capsule markdown and examples:
alexandria capsule show database:postgres-pro
```

### `alexandria harness add <target>`
Scaffolds configuration files for specific environments on demand:
```bash
# Available targets: cursor, antigravity, claude, codex, git, all
alexandria harness add git
alexandria harness add cursor
alexandria harness add all
```

### `alexandria vault [status | validate | log | sync]`
Manages the documentation knowledge vault:
```bash
# Check graph metrics and module relationships:
alexandria vault status

# Verify zero broken wikilinks:
alexandria vault validate

# View certified operational logs:
alexandria vault log

# Sync markdown files into SQLite WAL:
alexandria vault sync
```

### `alexandria memory [add | query]`
Adds or queries episodic vector memories with a strict token budget:
```bash
# Add a key design rule or learned lesson:
alexandria memory add "Always use connection pool timeouts under 5 seconds for read replicas" --module database

# Query memory within a 150-token budget:
alexandria memory query "connection pool timeouts" --budget 150
```

### `alexandria mcp`
Starts the Model Context Protocol stdio server for editor integrations:
```bash
alexandria mcp
```

### `alexandria lsp`
Starts the real-time Language Server Protocol engine to surface diagnostics directly in VS Code, Zed, or Neovim:
```bash
alexandria lsp
```

---

## IDE & Agent Configuration

### Google Antigravity (`mcp_config.json`)
```json
{
  "mcpServers": {
    "alexandria": {
      "command": "alexandria",
      "args": ["mcp"],
      "env": {
        "ALEXANDRIA_LOG_LEVEL": "info"
      }
    }
  }
}
```

### Claude Code (`.mcp.json` or terminal)
```bash
claude mcp add alexandria -- alexandria mcp
```

### Cursor (`.cursor/mcp.json`)
```json
{
  "mcpServers": {
    "alexandria": {
      "command": "alexandria",
      "args": ["mcp"]
    }
  }
}
```

### Neovim (`nvim-lspconfig` for real-time diagnostics)
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

## Local Hybrid Memory System

To avoid context window exhaustion while keeping track of project lessons, Alexandria uses a **hybrid memory pipeline**:

1. **Micro-Chunk Compression:** Prompts and execution outcomes are condensed into high-density micro-chunks ($\le 50$ words).
2. **128-Dimensional Quantized `int8` Embeddings:** Fast bitwise n-gram feature hashing projects text into an L2-normalized 128-byte vector. Zero cloud latency, zero GPU requirements.
3. **Reciprocal Rank Fusion (RRF):** Combines dense semantic cosine similarity with SQLite FTS5 BM25 lexical ranking:
   $$\text{RRF}(d) = \frac{1}{60 + \text{rank}_{\text{vector}}(d)} + \frac{1}{60 + \text{rank}_{\text{bm25}}(d)}$$
4. **Hard Token Budget Enforcement:** The memory search accumulates top-ranked results in descending RRF order and stops immediately before crossing your specified `max_token_budget`.
5. **Project-Scoped Isolation:** Databases are resolved per project root (`.alexandria/vault.db`), preventing memory from one repository leaking into another.

---

## Knowledge Vault & Documentation Structure

Alexandria keeps documentation close to the code using standard Markdown files with universal Obsidian-compatible wikilinks (`[[node]]`):

```text
wiki/ (or alexandria/)
├── index.md                  # Master entry point and index of all modules
├── dashboard.md              # Auto-generated project health and telemetry metrics
├── log.md                    # Append-only ledger of certified changes
├── failures.md               # Incident records and recovery notes
├── architecture/             # High-level system blueprints
├── modules/                  # Subsystem contracts (one per major component)
└── decisions/                # Architectural Decision Records (ADR-XXX.md)
```

- **Depth-1 Resolution:** Regular modules only load their immediate neighbors into context, preventing prompt saturation.
- **ADR Hoisting:** Constitutional decision records (`ADR-XXX`) are transitively elevated up to 6 levels deep, ensuring critical invariants are never forgotten.
- **Editor Agnostic:** You can view and edit documentation with Obsidian, VS Code (using Foam or Markdown Links), GitHub web view, or terminal tools like `glow` and `bat`.

---

## Performance Summary

| Metric | Target | Verified Benchmark (v1.1.0) |
| :--- | :--- | :--- |
| **Release Binary Size** | $\le 5.0\text{ MB}$ | **`~2.20 MB`** (self-contained with SQLite, Oxc, Tokio, LSP) |
| **Idle Memory (RSS)** | $\le 15.0\text{ MB}$ | **`~8.5 MB`** (zero external ML models in RAM) |
| **Graph Resolution & ADR Hoisting** | $\le 1.0\text{ ms}$ | **`<= 0.20 ms`** (`petgraph::StableDiGraph`) |
| **Gate 0 In-Memory AST Scan** | $\le 2.0\text{ ms}$ | **`<= 0.85 ms`** (`oxc_parser` arena allocation) |
| **Fact-Forcing Verification** | $\le 0.1\text{ ms}$ | **`<= 0.05 ms`** (instant hash set lookup) |
| **Hybrid Memory Query (RRF)** | $\le 3.0\text{ ms}$ | **`<= 0.90 ms`** (128-dim cosine + SQLite FTS5) |
| **Automated Test Suite** | 100% Pass | **`66 / 66 Passed`** across 14 test suites in < 6s |

---

## License

Released under the [MIT License](LICENSE).  
Copyright (c) 2026 **Devi Ishmael**.
