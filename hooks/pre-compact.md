# 💾 Pre-Compact Snapshot Hook

## Mechanism
Triggered prior to context window compression:
1. Gathers current task state, modified files, active test suite, and uncommitted changes.
2. Writes a structured JSON record into `session_checkpoints` table in SQLite WAL.
3. Allows seamless context reconstitution after `/compact` executes.
