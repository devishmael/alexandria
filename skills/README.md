# 🧠 Alexandria Modular Skills Catalog

This directory provides human-readable procedural engineering skills corresponding to the **225 Curated Master Capsules** indexed in `.alexandria/capsules/`.

## Domain Directories

| Directory | Focus Area | Sample Technologies |
| :--- | :--- | :--- |
| **[`backend/`](backend/)** | Server & Systems | Rust Tokio, Node.js NestJS, Go Concurrency, Python AsyncIO |
| **[`database/`](database/)** | Data Persistence | SQLite WAL, PostgreSQL query tuning, Prisma transaction isolation |
| **[`testing/`](testing/)** | Quality Assurance | Strict TDD (Red-Green-Refactor), Vitest sandbox, Property tests |
| **[`architecture/`](architecture/)** | System Design | Hexagonal Architecture, Clean Architecture, CQRS, Event Sourcing |
| **[`security/`](security/)** | Application Security | Gate 2 SAST, Secret Leak Interception, OWASP defenses |

## Compilation to Production Capsules
Run `alexandria collect` to ingest and validate raw Markdown skills into canonical `alexandria.capsule.v2.4` JSON bundles:
```bash
alexandria collect ./skills --out ./.alexandria/capsules
```
