# `/telemetry` — Live Organism Health & MTTR Dump

## Description
Generates a real-time markdown dump of the organism health telemetry recorded in SQLite WAL:
- Pass / Fail quality gate verdict ratios.
- Mean Time To Repair (MTTR in seconds) across modules.
- Active circuit breaker trip status.
- Total episodic memory micro-chunks.

## Output Destination
- Formatted markdown in chat.
- Synchronized with `wiki/dashboard.md` and `.alexandria/telemetry/organism-health.json`.
