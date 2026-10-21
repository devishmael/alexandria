# `/gate` — Manual Quality Gate Trigger

## Description
Triggers an immediate out-of-band audit of the current working tree against Alexandria's deterministic Quality Gates:
- **Gate 0:** In-memory Oxc AST scan (`NO_EXPLICIT_ANY`, `NO_DOUBLE_CAST_UNKNOWN`, Prisma lexical closure).
- **Gate 1:** Static compiler check (`cargo check` or `tsc --noEmit`).
- **Gate 2:** Secret leak interception and SAST scan.
- **Gate 3:** Subprocess test sandbox verification.

## CLI Equivalent
```bash
alexandria mcp  # Evaluated continuously during verify_and_gate_changes
```
