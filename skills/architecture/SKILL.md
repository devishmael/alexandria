---
name: clean-architecture-engineer
domain: architecture
description: "Hexagonal and Clean Architecture structural boundaries"
---

# Clean & Hexagonal Architecture

## Invariants
1. **Dependency Rule:** Source code dependencies must only point inwards toward domain entities.
2. **Framework Isolation:** Domain business logic must have zero dependencies on transport layers, HTTP frameworks, or ORM libraries.
3. **Boundary Contracts:** Cross architectural boundaries exclusively through abstract interfaces (ports and adapters).
