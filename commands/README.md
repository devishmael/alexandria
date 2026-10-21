# ⚡ Alexandria Slash Commands Reference

Alexandria integrates with developer chat environments (Cursor, Claude Code, Gemini Antigravity, OpenCode, Zed) by exposing deterministic slash commands that trigger bare-metal kernel actions.

## Command Index

| Command | Description | Kernel Subsystem Invoked |
| :--- | :--- | :--- |
| **[`/compact`](compact.md)** | Takes a pre-compact session snapshot in SQLite WAL before context is compressed. | `session_checkpoints` actor |
| **[`/gate`](gate.md)** | Manually runs Gate 0 Oxc AST scans and Gate 2 SAST security checks on active files. | `oxc_validator` / `GateGuard` |
| **[`/court-martial`](court-martial.md)** | Convenes an emergency Single-Turn Court-Martial between Bel, Enki, and Utu. | `courtMartialDocket` state |
| **[`/memory`](memory.md)** | Queries or stores episodic micro-chunks using 64-dim `int8` + BM25 RRF under token budget. | `hybrid_memory` engine |
| **[`/telemetry`](telemetry.md)** | Computes MTTR, pass/fail ratios, and outputs the live `dashboard.md` report. | `organism_health` telemetry |
| **[`/vault`](vault.md)** | Traverses the Depth=1 knowledge graph and hoists constitutional ADR records. | `graph::resolver` in RAM |
