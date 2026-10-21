---
name: backend-systems-engineer
domain: backend
description: "Core invariants for building low-latency, resilient backend services"
---

# Backend Systems Engineering

## Invariants
1. **Concurrency Safety:** Avoid shared mutable state across threads without actor supervision or channel communication.
2. **Resource Management:** Rely on RAII and deterministic destructors for database connections, file handles, and sockets.
3. **Error Propagation:** Use typed error variants (`Result<T, E>`) over untyped panics or generic catch-all blocks.
