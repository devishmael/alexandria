# 🛡️ FACT-FORCING GATEGUARD RULE (`E_FACT_FORCING`)

## Definition
The **Fact-Forcing Gate** prevents blind modifications and hallucinated edits.

## Invariant
Before modifying, writing, or editing any existing file in the project, the agent or harness **MUST** inspect the file first:
1. Read the target file's content or relevant symbol definitions.
2. Confirm the callers/consumers of exported functions.
3. Verify that the planned modification will not break consumer contracts.

## Kernel Reaction to Violations
If a mutation is attempted without prior recorded inspection:
- **Error Code:** `E_FACT_FORCING`
- **Action:** The operation is rejected deterministically before any write touches disk.
