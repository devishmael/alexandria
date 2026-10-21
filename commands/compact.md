# `/compact` — Pre-Compact Session Checkpoint

## Description
Executes a lossless session snapshot in SQLite WAL before the host AI assistant triggers automated context compression.

## Execution Flow
1. Gathers active module contracts, pending test suites, and modified files in the active session.
2. Writes an atomic snapshot into `session_checkpoints` table with monotonic epoch ordering.
3. Provides a distilled summary for prompt re-injection post-compaction.

## CLI Equivalent
```bash
alexandria memory add "Session checkpoint pre-compaction" --kind checkpoint
```
