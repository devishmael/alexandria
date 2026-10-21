---
name: database-acid-engineer
domain: database
description: "Invariants for transaction isolation, connection pools, and query optimization"
---

# Database Systems & ACID Engineering

## Invariants
1. **Lexical Closure:** Enclose all interactive transactions in a single scope without escaping clients.
2. **Timing Bounds:** Complete all interactive transactions under 25ms to prevent connection pool exhaustion.
3. **No External I/O:** Never perform HTTP calls, filesystem writes, or heavy hashing inside an active database transaction.
