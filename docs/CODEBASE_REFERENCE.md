# 🏛️ Alexandria Codebase Technical Reference Manual

> **Product:** Alexandria (*The Deterministic Agent Harness*)  
> **Official Version:** `v1.2.0`  
> **Author & System Architect:** Devi Ishmael (`devishmael`)  
> **Language & Toolchain:** Rust (Edition 2024, Tokio Async Runtime)  
> **License:** MIT License  

---

## 1. Architectural Philosophy: The Dual-Space Paradigm

Alexandria operates under a fundamental operating system metaphor: the separation between **Kernel-Space** and **User-Space**:

```
+-------------------------------------------------------------------------+
|                  USER-SPACE (Probabilistic LLM Agents)                   |
|  - Reasoning & Code Generation (Bel, Enki, Utu, Nabu)                   |
|  - Cannot self-certify test results or bypass compile gates             |
+-------------------------------------------------------------------------+
                                    |
                    JSON-RPC 2.0 (MCP & LSP over Stdio)
                                    v
+-------------------------------------------------------------------------+
|                 KERNEL-SPACE (Deterministic Rust Arbitrator)            |
|  - Gate 0: Oxc AST Lexical Linter (< 2ms)                               |
|  - Gate 1: TypeScript & ESLint Verification                             |
|  - Gate 2: SAST & Secret Leak Detector                                  |
|  - Gate 3: Vitest Node.js Subprocess Sandbox (--permission)             |
|  - Gate 4: Stateless Cold Architectural Audit                           |
|  - Storage: Single-Writer MPSC SQLite WAL Actor & FTS5 BM25 Engine      |
|  - Memory: 64-Dim int8 Hybrid Vector Store (RRF Fusion)                 |
|  - Graph: Petgraph Topological Knowledge Graph (Depth=1 & ADR Hoisting)  |
+-------------------------------------------------------------------------+
```

---

## 2. Directory Structure & File Responsibilities

```
alexandria/
├── .github/
│   └── workflows/
│       └── ci.yml                 # Multi-platform CI pipeline (Ubuntu & Windows, Rust 2024)
├── Cargo.toml                     # Package manifest (alexandria v1.2.0, zero external weight)
├── docs/
│   └── CODEBASE_REFERENCE.md      # This comprehensive architectural reference
├── src/
│   ├── lib.rs                     # Library entrypoint, crate docs, and subsystem exports
│   ├── main.rs                    # Native CLI entrypoint and multi-threaded Tokio runtime
│   ├── cli.rs                     # CLI arguments parser, scaffolding, and subcommands
│   ├── collector/
│   │   ├── mod.rs                 # Collector module export
│   │   └── skill_etl.rs           # Skill/hook ETL engine, JSON normalization, canonical schema
│   ├── gate_guard/
│   │   ├── mod.rs                 # GateGuard module export
│   │   ├── fact_forcing.rs        # Fact-Forcing GateGuard (zero blind edits, E_FACT_FORCING)
│   │   └── oxc_validator.rs       # AST parsing via Oxc, TypeScript visitor, Gate 2 SAST scanner
│   ├── graph/
│   │   ├── mod.rs                 # Knowledge graph module export
│   │   └── resolver.rs            # Petgraph topological resolver, ADR hoisting, worktree check
│   ├── lsp/
│   │   ├── mod.rs                 # LSP module export
│   │   └── server.rs              # Language Server Protocol engine over stdio
│   ├── mcp/
│   │   ├── mod.rs                 # MCP module export
│   │   └── server.rs              # Model Context Protocol engine, tool dispatch, hook evaluator
│   ├── runner/
│   │   ├── mod.rs                 # Subprocess runner module export
│   │   └── vitest_sandbox.rs      # Hermetic Vitest sandbox evaluator for TDD RED-GREEN cycles
│   ├── storage/
│   │   ├── mod.rs                 # Storage module export
│   │   ├── actor.rs               # Transactional SQLite WAL actor, MPSC channel, FTS5 BM25
│   │   └── schema.sql             # Canonical SQLite relational & virtual tables schema
│   └── vector_store/
│       ├── mod.rs                 # Vector store module export
│       └── hybrid_memory.rs       # 64-dim int8 embedding generator, RRF fusion, token budgeting
└── tests/
    ├── cli_e2e_test.rs            # End-to-End CLI scaffolding, version banner, and custom vaults
    ├── fact_forcing_and_checkpoint_test.rs # Fact-Forcing E_FACT_FORCING and session checkpoints
    ├── graph_resolver_test.rs     # Depth=1, cycle prevention, ADR hoisting, worktree disjunction
    ├── kernel_spec_completeness_test.rs # Gate 2 SAST, watcher, syscall traps, SQLite persistence
    ├── mcp_integration_test.rs    # Real tool invocation, FTS fallback, AST rejections
    ├── mcp_server_test.rs         # Protocol handshakes, ping-pong, tools/list, hook evaluation
    ├── new_cli_commands_test.rs   # Lifecycle DX commands (doctor, gate, capsule, harness, vault)
    ├── oxc_validator_test.rs      # Explicit any, double casting, Prisma closures, unawaited promises
    ├── project_isolation_test.rs  # Project-Scoped DB Isolation (ADR-009, zero cross-project leak)
    ├── skill_collector_test.rs    # Directory ingestion, manifest generation, vault seeding
    ├── storage_actor_test.rs      # MPSC concurrency, SQLite WAL, FTS5 sanitization, monotonic IDs
    ├── v25_organism_and_vector_test.rs # Hybrid vector store, RRF, AST pruning, Court-Martial dockets
    └── vitest_sandbox_test.rs     # TDD RED phase assertion errors, syntax error rejections
```

---

## 3. Kernel Subsystems & Data Flow

### 3.1 Model Context Protocol (MCP) Server (`src/mcp/server.rs`)
The MCP server operates over standard I/O using JSON-RPC 2.0. It exposes 8 deterministic tools:
1. `prepare_task_context`: Analyzes target files and instructions to classify execution path (`alpha_fast` vs `omega_pantheon`), resolves the depth=1 topological knowledge graph with ADR hoisting up to 6 levels, prunes TypeScript AST contexts for focus symbols, and recalls episodic memory.
2. `upsert_architectural_contract`: Formalizes architectural decisions (ADRs) and module contracts in markdown within the knowledge vault before code modification.
3. `resolve_skill_capsule`: Injects procedural knowledge capsules into the prompt suffix or ingests external skill directories.
4. `search_capsules`: Searches the curated 225 master capsules via SQLite FTS5 BM25 ranking with optional domain filtering.
5. `read_capsule`: Retrieves complete specifications, invariants, instructions, and hooks for any capsule ID.
6. `spawn_isolated_worktrees`: Validates that parallel agent worktrees have disjoint dependency graphs using `petgraph` and `oxc_resolver`.
7. `verify_and_gate_changes`: Executes Gate 0 AST inspection, Gate 2 SAST secret scanning, and evaluates lifecycle hooks (`pre_verify`).
8. `finalize_task_and_log`: Evaluates `pre_commit` lifecycle hooks, generates cryptographic agentic git seals, records episodic memory, and appends immutable audit log entries to `wiki/log.md`.

### 3.2 GateGuard AST & Security Scanner (`src/gate_guard/oxc_validator.rs`)
Evaluated in under 2 milliseconds using native Rust `oxc_parser` and `oxc_ast_visit`:
- **Invariants Enforced:**
  - `NO_EXPLICIT_ANY`: Traps `TSAnyKeyword`.
  - `NO_DOUBLE_CAST_UNKNOWN`: Traps double type casts (`as unknown as T`).
  - `PRISMA_TX_STRICT_LEXICAL_CLOSURE`: Forbids aliasing or destructuring of `tx`, enforces direct `await` calls, and bans promise chaining (`.then()`).
  - `GATE_2_SAST`: Detects hardcoded API keys, private keys, AWS tokens, unparameterized SQL queries, `eval()`, and `dangerouslySetInnerHTML`.

### 3.3 Storage Actor & SQLite WAL (`src/storage/actor.rs`)
Operates as a dedicated background actor thread using an `mpsc` queue:
- **Persistence Mode:** SQLite Write-Ahead Logging (`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;`).
- **Tables Managed:**
  - `wiki_nodes` & `wiki_edges`: Full knowledge graph state synchronized with markdown vault.
  - `skills` & `skills_fts`: Curated master capsule repository with Porter Unicode61 full-text search.
  - `quality_gate_verdicts`: Audit trail of all gate evaluations.
  - `circuit_breaker_states`: Module retry counters tripping after 3 consecutive failures.
  - `forensic_failures` & `failures_fts`: Ledger of incidents for post-mortem analysis.
  - `episodic_vector_memory`: Micro-chunk memory entries storing 64-dim int8 embeddings.

### 3.4 Hybrid Vector Memory Store (`src/vector_store/hybrid_memory.rs`)
Zero external model dependencies:
- **Embedding Generation:** Computes deterministic 64-dimensional L2-normalized `int8` vectors (`[-127, 127]`) through multi-granular character and word n-gram feature hashing.
- **Hybrid Fusion:** Combines vector cosine similarity and SQLite FTS5 BM25 ranks using Reciprocal Rank Fusion:
  $$\text{RRF}(d) = \frac{1}{60 + \text{Rank}_{\text{vec}}(d)} + \frac{1}{60 + \text{Rank}_{\text{BM25}}(d)}$$
- **Token Budgeting:** Dynamically packs recalled chunks within a specified token budget (default: 350 tokens) to minimize context rot.

---

### 3.5 Lifecycle Developer Experience (DX) CLI Engine (`src/cli.rs`)
Formalized under [[ADR-010-lifecycle-dx-cli-suite]] and subsystem contract [[cli-dx-lifecycle]], providing bare-metal sub-15ms terminal workflows:
- **`alexandria doctor` (or `status`):** Instant 5-point environment diagnostic validating binary in PATH, SQLite WAL journal mode and indexed capsules count, project scoping and vault graph node count, multi-harness configurations, and live organism telemetry (MTTR and circuit breakers).
- **`alexandria gate <path>` (or `check`):** Evaluates in-memory Oxc AST quality gates directly from the terminal without launching an LLM. Enforces Gate 0 (`NO_EXPLICIT_ANY`, `NO_DOUBLE_CAST_UNKNOWN`, `PRISMA_TX_*`) and Gate 2 SAST in $\le 2\text{ ms}$. Exits with code `0` on pass or `1` on violation (ideal for git pre-commit hooks and CI pipelines).
- **`alexandria capsule [list | search | show]`:** Direct terminal access to the 225 curated master capsules: list by domain, search with SQLite FTS5 BM25, and view full markdown specifications with invariants and code examples.
- **`alexandria harness add <target>`:** Scaffolds configuration adapters for Cursor, Antigravity, Claude Code, Codex, or all 10+ environments. Automatically invoked during `alexandria init`.
- **`alexandria vault [status | validate | log | sync]`:** Inspects vault graph topology, validates wikilink acyclicity and broken links, inspects certified epochs with SHA-256 IAM seals, and synchronizes manually edited markdown files into SQLite WAL.

---

## 4. The 8 Canonical Capsule Domains

Alexandria categorizes all engineering capabilities into 8 domains:

| Domain | Scope & Responsibilities | Key Capabilities |
| :--- | :--- | :--- |
| **`backend`** | Server-side services, async concurrency, and high-performance runtimes. | Rust Tokio actors, Node.js Fastify/NestJS, Go goroutines, microservice caching, RabbitMQ message brokers. |
| **`architecture`** | Structural patterns, boundary separation, and enterprise contracts. | Hexagonal / Clean Architecture, CQRS & Event Sourcing, Domain-Driven Design (DDD), Transactional Outbox. |
| **`devops`** | Infrastructure as Code, containerization, and deployment automation. | Multi-stage Dockerfiles, Kubernetes manifests, GitHub Actions pipelines, Terraform modules, Prometheus telemetry. |
| **`frontend`** | Modern reactive user interfaces and client-side performance. | React 19 Server Components, Next.js App Router, Vue 3 Composition, Zustand state stores, Web Vitals optimization. |
| **`testing`** | Hermetic test suites and automated quality assurance. | Strict TDD (Red-Green-Refactor), Vitest sandbox, Playwright E2E with POM, property-based testing (proptest). |
| **`database`** | Relational schemas, indexing strategies, and transactional integrity. | PostgreSQL query tuning (`EXPLAIN ANALYZE`), Prisma ACID closures, SQLite WAL concurrency, Redis clusters. |
| **`api`** | Interface design, transport protocols, and communication contracts. | RESTful APIs (RFC 7807), GraphQL schema-first, gRPC / Protocol Buffers, WebSocket streaming, API rate limiting. |
| **`hook`** | Deterministic kernel syscall traps and lifecycle event interceptors. | Pre-commit quality gates, AST policy validators, credential leak traps, CI verification hooks. |

---

## 5. Lifecycle Hook Execution Mechanics

Hooks allow capsules to register active interceptors on critical kernel events:
- **`pre_verify` Event:** Evaluated during `verify_and_gate_changes`.
  - Queries active hooks registered for `pre_verify`, `pre_gate_0`, `pre_gate_2`, and `pre_execution`.
  - Enforces mandatory rules (e.g. `validate_prisma_tx_closure`, `enforce_nominal_types`, `scan_gate2_sast_and_secrets`, `enforce_test_file_existence`).
  - If any check fails, the quality gate returns a deterministic failure (`LIFECYCLE_HOOK_REJECTION`) and increments the Circuit Breaker attempt counter.
- **`pre_commit` Event:** Evaluated during `finalize_task_and_log`.
  - Queries active hooks registered for `pre_commit` and `post_gate_3`.
  - If any interceptor detects a violation (e.g. hardcoded secrets or uncommitted unvalidated source code), the commit is blocked deterministically with status `rejected_by_lifecycle_hook`.
