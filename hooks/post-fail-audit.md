# ⚖️ Post-Fail Audit & Court-Martial Trigger

## Mechanism
Triggered whenever a Quality Gate check fails:
1. Records module name and failure reason into `failures.md`.
2. Increments atomic module failure counter.
3. If failure count reaches 3, trips the circuit breaker and emits a `courtMartialDocket` payload.
