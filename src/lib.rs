//! # Alexandria
//!
//! **The Deterministic Agent Harness & Kernel-Space Arbitrator**
//!
//! Alexandria provides deterministic quality gates, full-text and hybrid vector retrieval,
//! topological knowledge graph resolution, isolated Git worktree management, and native
//! MCP / LSP protocol servers for AI-assisted engineering.
//!
//! ## Architectural Architecture
//!
//! Alexandria operates under a strict **Dual-Space Paradigm**:
//! - **Kernel-Space (Rust Arbitrator):** Exclusive authority over filesystem operations, AST static analysis,
//!   sub-process sandboxing, quality gate arbitration, and SQLite WAL transactional persistence.
//! - **User-Space (LLM Intelligence):** Probabilistic computation without authority to self-certify
//!   compilation or test verdicts.
//!
//! ## Subsystems
//!
//! - [`cli`]: Command-line interface, argument parsing, and command dispatchers (`init`, `mcp`, `lsp`, `run`, `collect`, `memory`).
//! - [`collector`]: Canonical ETL engine transforming skills, hooks, and tools into `alexandria.capsule.v2.4` JSON capsules.
//! - [`gate_guard`]: High-performance lexical AST validation and security scanning via `oxc`.
//! - [`graph`]: Topological knowledge graph resolver with Depth=1 constraint and transitive ADR hoisting.
//! - [`lsp`]: Language Server Protocol implementation providing real-time AST and SAST diagnostics.
//! - [`mcp`]: Model Context Protocol implementation exposing kernel tools over standard I/O JSON-RPC.
//! - [`runner`]: Hermetic sub-process runner and Vitest report evaluation for strict TDD RED-GREEN cycles.
//! - [`storage`]: SQLite WAL transactional actor with FTS5 BM25 search, circuit breaker, and audit logging.
//! - [`vector_store`]: High-density hybrid vector store combining Reciprocal Rank Fusion (RRF) and int8 quantization.

pub mod cli;
pub mod collector;
pub mod gate_guard;
pub mod graph;
pub mod lsp;
pub mod mcp;
pub mod runner;
pub mod storage;
pub mod vector_store;
