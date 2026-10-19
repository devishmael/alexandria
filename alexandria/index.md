---
id: index
title: "Mapa Maestro del Grafo de Conocimiento, Contratos de Subsistema y ADRs Constitucionales"
type: index
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - index
  - graph
  - architecture
  - contracts
  - adrs
---

# Mapa Maestro del Grafo de Conocimiento, Contratos de Subsistema y ADRs Constitucionales

> **Sistema:** Alexandria Runtime (`v2.4.0-OmniPragmatic`)  
> **Árbitro Determinista:** Kernel-Space Rust Arbitrator (`alexandria`)  
> **Bóveda de Verdad Canónica:** `./alexandria/`  
> **Estado del Grafo:** 18 Nodos Canónicos Activos | 0 Enlaces Rotos | Resolución $\le 0.2\text{ ms}$

---

## 1. Topología Dirigida del Grafo en Memoria (`petgraph::StableDiGraph`)

El siguiente grafo representa las aristas dirigidas cargadas en RAM por el subsistema [[topological-graph]]. Toda consulta a `prepare_task_context` sobre un módulo $M$ inyecta su vecindad inmediata $\text{Depth}=1$ y eleva transitivamente (*ADR Hoisting*, $\text{Depth} \le 6$) todos los nodos `ConstitutionalAdr`:

```
                             ┌───────────────────────────────────────┐
                             │               [[index]]               │
                             │      Mapa Maestro de la Bóveda        │
                             └───────────────────┬───────────────────┘
                                                 │
         ┌───────────────────┬───────────────────┼───────────────────┬───────────────────┐
         ▼                   ▼                   ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│[[specification- │ │  [[overview]]   │ │  [[database-    │ │    [[log]]      │ │  [[failures]]   │
│      v2]]       │ │  Topología &    │ │    schema]]     │ │ Bitácora Ledger │ │ Registro FAIL   │
│ Blueprint Crudo │ │   Cuarteto      │ │ SQLite WAL/FTS5 │ │   Inmutable     │ │ Forense & BM25  │
└─────────────────┘ └────────┬────────┘ └────────┬────────┘ └─────────────────┘ └────────┬────────┘
                             │                   │                                       │
         ┌───────────────────┼───────────────────┼───────────────────────────────────────┘
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│ [[core-engine]] │ │ [[mcp-server]]  │ │ [[topological-  │ │[[gate-guard-    │ │   [[sandbox-    │ │[[host-adapters]]│
│ Runtime Rust,   │ │ JSON-RPC 2.0    │ │    graph]]      │ │     ast]]       │ │    runner]]     │ │ .agents, MCP &  │
│ CLI & Breaker   │ │ sobre stdio     │ │ petgraph & BFS  │ │ Escáner Oxc 0.56│ │ Node Permission │ │   working.md    │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘ └────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │                   │                   │                   │
         ▼                   ▼                   ▼                   ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                           CAPA CONSTITUCIONAL DE ADRs (HOISTING TRANSITIVO DEPTH <= 6)                              │
├──────────────────────────────────────────────────────────┬──────────────────────────────────────────────────────────┤
│ • [[ADR-001-kernel-space-harness]]                       │ • [[ADR-005-node-permission-sandboxing]]                 │
│ • [[ADR-002-prefix-pinned-caching]]                      │ • [[ADR-006-oxc-ast-engine]]                             │
│ • [[ADR-003-zero-trust-quality-gates]]                   │ • [[ADR-007-multi-host-adapter-and-persistence]]         │
│ • [[ADR-004-single-writer-mpsc-storage]]                 │                                                          │
└──────────────────────────────────────────────────────────┴──────────────────────────────────────────────────────────┘
```

---

## 2. Catálogo Maestro por Planos y Subsistemas

### 2.1. Plano de Arquitectura Base (`alexandria/architecture/`)
| Nodo Canónico | Ruta en Bóveda | Destino en Kernel (`src/`) | Responsabilidad y Alcance |
|---|---|---|---|
| [[overview]] | `alexandria/architecture/overview.md` | N/A (Gobierno Global) | Separación Kernel-Space vs. User-Space, Arquitectura de 2 Planos, Cuarteto Central (`Bel`, `Enki`, `Utu`, `Nabu`), *Prefix Pinning*, *Path Alpha/Omega* y Matriz de 12 Dimensiones. |
| [[database-schema]] | `alexandria/architecture/database-schema.md` | `src/storage/schema.sql`, `src/storage/actor.rs` | Esquema DDL SQLite 3 en modo WAL, tuning por PRAGMAs (`mmap_size`, `cache_size`), triggers FTS5 simétricos (`_ai`, `_ad`, `_au`), `VaultWriterActor` MPSC post-commit y sanitizador `bm25()`. |

### 2.2. Plano de Contratos de Módulos Ejecutables (`alexandria/modules/`)
| Nodo Canónico | Ruta en Bóveda | Destino en Kernel (`src/` y Raíz) | Vecinos Directos ($\text{Depth}=1$) | ADRs Elevados (*Hoisted*) |
|---|---|---|---|---|
| [[core-engine]] | `alexandria/modules/core-engine.md` | `Cargo.toml`, `src/main.rs`, `src/lib.rs` | [[overview]], [[database-schema]], [[mcp-server]], [[topological-graph]], [[gate-guard-ast]], [[sandbox-runner]], [[failures]] | [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]] |
| [[mcp-server]] | `alexandria/modules/mcp-server.md` | `src/mcp/server.rs` | [[overview]], [[core-engine]], [[topological-graph]], [[gate-guard-ast]], [[sandbox-runner]], [[log]] | [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-003-zero-trust-quality-gates]] |
| [[topological-graph]] | `alexandria/modules/topological-graph.md` | `src/graph/resolver.rs` | [[overview]], [[core-engine]], [[mcp-server]], [[gate-guard-ast]] | [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-006-oxc-ast-engine]] |
| [[gate-guard-ast]] | `alexandria/modules/gate-guard-ast.md` | `src/gate_guard/oxc_validator.rs` | [[overview]], [[core-engine]], [[mcp-server]], [[topological-graph]], [[sandbox-runner]], [[failures]] | [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-006-oxc-ast-engine]] |
| [[sandbox-runner]] | `alexandria/modules/sandbox-runner.md` | `src/runner/vitest_sandbox.rs` | [[overview]], [[core-engine]], [[mcp-server]], [[gate-guard-ast]], [[failures]] | [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-005-node-permission-sandboxing]] |
| [[host-adapters]] | `alexandria/modules/host-adapters.md` | `.agents/**`, `working.md`, `mcp_config.json` | [[overview]], [[core-engine]], [[mcp-server]], [[topological-graph]], [[database-schema]], [[log]] | [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-004-single-writer-mpsc-storage]], [[ADR-007-multi-host-adapter-and-persistence]] |

### 2.3. Plano Constitucional de Decisiones de Arquitectura (`alexandria/decisions/`)
| Registro ADR | Estado | Decisión Técnica Inmutable | Módulos Gobernados |
|---|---|---|---|
| [[ADR-001-kernel-space-harness]] | **Aceptado** | Desacoplamiento entre proceso probabilístico no confiable (LLM en User-Space) y árbitro determinista en Rust (Kernel-Space). | [[core-engine]], [[mcp-server]], [[overview]] |
| [[ADR-002-prefix-pinned-caching]] | **Aceptado** | Preservación del KV-Cache mediante prefijo estático inmutable, sub-sesiones efímeras de `Enki` y *ADR Hoisting* transitivo ($\le 6$). | [[mcp-server]], [[topological-graph]], [[overview]] |
| [[ADR-003-zero-trust-quality-gates]] | **Aceptado** | Ejecución fuera de banda de las 5 compuertas de calidad (`Gate 0` a `Gate 4`), juicio en frío de `Utu` y validación semántica de Fase RED. | [[gate-guard-ast]], [[sandbox-runner]], [[core-engine]] |
| [[ADR-004-single-writer-mpsc-storage]] | **Aceptado** | Persistencia SQLite WAL mediante hilo dedicado de OS (`VaultWriterActor`), *batching* MPSC hasta 64 comandos y confirmación *post-commit*. | [[database-schema]], [[core-engine]], [[failures]] |
| [[ADR-005-node-permission-sandboxing]] | **Aceptado** | Confinamiento nativo de pruebas mediante `node --permission`, normalización de rutas extendidas UNC (`\\?\`) y redirección de `TMP`. | [[sandbox-runner]], [[core-engine]] |
| [[ADR-006-oxc-ast-engine]] | **Aceptado** | Adopción de la suite `oxc` (`0.56`) sobre arena contigua (`oxc_allocator`) frente a Tree-sitter/SWC para validación AST en $\le 2\text{ ms}$. | [[gate-guard-ast]], [[topological-graph]] |
| [[ADR-007-multi-host-adapter-and-persistence]] | **Aceptado** | Desacoplamiento multi-host (`.agents/`, `mcp_config.json`) y proyección de estado persistente en disco (`working.md`). | [[host-adapters]], [[mcp-server]], [[database-schema]] |

### 2.4. Plano Operativo, Histórico y Fuentes Primarias
| Nodo Canónico | Ruta en Bóveda | Rol en el Ciclo de Vida |
|---|---|---|
| [[specification-v2]] | `alexandria/raw/specification-v2.md` | Especificación primaria *Golden Master* (`v2.4.0-OmniPragmatic`) de referencia inmutable. |
| [[log]] | `alexandria/log.md` | Bitácora *append-only* de tareas y auditorías aprobadas tras superar todos los *Quality Gates*. |
| [[failures]] | `alexandria/failures.md` | Registro forense estructurado de incidentes `FAIL-XXX`, análisis de causa raíz y semillas de autocuración BM25. |

---

## 3. Protocolo de Compilación para `[CONSTRUCTOR]`

Antes de escribir o modificar cualquier archivo dentro de `src/` o `tests/`, el `[CONSTRUCTOR]` está constitucionalmente obligado a:
1. Resolver el contexto del módulo objetivo desde este [[index]] y leer su contrato específico en `alexandria/modules/`.
2. Verificar que todos los `[[ADR-XXX]]` elevados en dicho contrato se cumplan sin excepciones.
3. Ejecutar el ciclo TDD estricto (`RED -> GREEN -> REFACTOR`) validado por [[sandbox-runner]] y [[gate-guard-ast]].
