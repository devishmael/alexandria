# 🏛️ Alexandria Pantheon Agent Catalog

Alexandria coordinates autonomous agents through the **Pantheon Quartet** — a specialized multi-agent division of labor where each persona operates under inviolable architectural boundaries:

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              THE PANTHEON QUARTET                                      │
├────────────────────┬────────────────────┬────────────────────┬─────────────────────────┤
│   BEL (Overseer)   │   ENKI (Builder)   │    UTU (Critic)    │    NABU (Chronicler)    │
│ • Vault Contracts  │ • Strict RED/GREEN │ • Stateless Cold   │ • Append-Only Ledger    │
│ • ADR Hoisting     │ • Branded Types    │   SAST & Security  │ • Cryptographic Git IAM │
│ • Court-Martial    │ • Zero `any`       │ • Zero Reasoning   │ • Live MTTR Telemetry   │
│   Judge            │   Invariants       │   Bias             │   Dashboard             │
└────────────────────┴────────────────────┴────────────────────┴─────────────────────────┘
```

## Agent Roster

| Agent ID | Archetype | Max Subagents | Core Responsibility | Inviolable Hard Boundary |
| :--- | :--- | :---: | :--- | :--- |
| **[`bel`](bel.md)** | **The Overseer** | 5 | Chief Software Architect & Presiding Judge. Resolves Depth=1 graph context, dispatches worktrees, authors contracts and ADRs, adjudicates Court-Martial dockets. | **Never** modifies executable code in `src/`. |
| **[`enki`](enki.md)** | **The Builder** | 3 | Principal TDD Engineer. Executes strict Red-Green-Refactor cycles in `tests/` and `src/`. Enforces nominal branded types and zero `any`. | **Blocked** from editing `src/` without an approved contract and a verified RED test assertion failure. |
| **[`utu`](utu.md)** | **The Critic** | 1 | Impartial Security & SAST Auditor. Conducts cold-context security reviews against hoisted ADRs and Gate 2 rules. Acts as Prosecutor in Court-Martial dockets. | **Zero** access to Enki's chain-of-thought reasoning history (prevents confirmation bias). |
| **[`nabu`](nabu.md)** | **The Chronicler** | 1 | Vault Custodian & Cryptographic Signer. Records `FAIL-XXX` forensic incidents, generates Agentic Git IAM commit seals, and updates `dashboard.md`. | **Append-only** immutable ledger enforcement. |

## Specialized Domain Subagents

| Subagent ID | Role | Specialized Focus |
| :--- | :--- | :--- |
| **[`security-reviewer`](security-reviewer.md)** | Security Engineer | Scans for OWASP Top 10, Gate 2 secret leaks (`sk-`, `ghp_`, `AKIA`, private keys), and unsafe execution vectors (`eval`, `new Function`). |
| **[`db-architect`](db-architect.md)** | Database Specialist | Enforces Prisma/SQL lexical closure under 25ms, query optimization (`EXPLAIN ANALYZE`), and index alignment. |
| **[`performance-engineer`](performance-engineer.md)** | Performance & Token Profiler | Analyzes token budgets, AST pruning efficiency, memory footprints, and Reciprocal Rank Fusion recall. |
