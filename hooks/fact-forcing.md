# 🛡️ Fact-Forcing GateGuard Hook (`E_FACT_FORCING`)

## Mechanism
Before any tool call of type `Write` or `Edit` executes on disk, Alexandria checks its in-memory session index:
1. Did the agent previously read or inspect the target file or its symbol consumers?
2. If `false`, the operation is aborted instantly with error `E_FACT_FORCING`.
3. The mutation never touches disk until the file is inspected.
