# 🏛️ Architecture Decision Record (ADR) RFC Workflow

## Purpose
Establishes how architectural changes, new invariants, and design pivots are codified and hoisted through the knowledge vault.

## Process
1. **Draft ADR:** Author `wiki/decisions/ADR-XXX-<name>.md` following canonical format (Context, Decision, Invariants, Consequences).
2. **Topological Graph Linking:** Link module contracts using standard wikilinks: `[[ADR-XXX-<name>]]`.
3. **Hoisting Verification:** The Alexandria kernel automatically hoists ADRs up to 6 levels of depth in RAM during context resolution.
4. **Sign-off:** Nabu commits the approved ADR to `wiki/log.md`.
