# 🪝 Alexandria Lifecycle Hooks & Interceptors

Alexandria intercepts tool dispatches and Git events out-of-band to enforce deterministic quality gates before code touches disk or is committed.

## Lifecycle Event Map

```text
User Directive
      │
      ▼
[Pre-Tool Use]  ───► Fact-Forcing GateGuard (E_FACT_FORCING)
      │
      ▼
[Pre-Commit]    ───► Gate 0 (Oxc AST) + Gate 2 (SAST & Secrets)
      │
      ▼
[Post-Fail]     ───► FAIL-XXX Log + Court-Martial Docket Trigger
      │
      ▼
[Pre-Compact]   ───► Lossless Session Snapshot in SQLite WAL
```

## Hook Implementations

| Hook | Type | Purpose |
| :--- | :--- | :--- |
| **[`hooks.json`](hooks.json)** | Declaration | Official declarative hook schema for AI client harnesses. |
| **[`pre-commit.sh`](pre-commit.sh)** | Git Hook (POSIX) | Blocks commits containing `any`, double casts, or hardcoded secrets. |
| **[`pre-commit.ps1`](pre-commit.ps1)** | Git Hook (PowerShell) | Windows version of pre-commit AST and secret scanner. |
| **[`fact-forcing.md`](fact-forcing.md)** | Kernel Gate | Specification for intercepting blind edits before disk writes. |
| **[`pre-compact.md`](pre-compact.md)** | Kernel Gate | Specification for SQLite WAL snapshots prior to context compression. |
| **[`post-fail-audit.md`](post-fail-audit.md)** | Kernel Gate | Circuit breaker trigger after 3 consecutive gate failures. |
