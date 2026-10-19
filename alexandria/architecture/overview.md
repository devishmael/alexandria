---
id: overview
title: "Blueprint Maestro: Topología de Dos Planos, Panteón de Agentes y Orquestación Adaptativa"
type: architecture
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - architecture
  - kernel-space
  - user-space
  - pantheon
  - prefix-pinning
  - dual-plane
---

# Blueprint Maestro: Topología de Dos Planos, Panteón de Agentes y Orquestación Adaptativa

> **Documento Canónico de Arquitectura:** `alexandria/architecture/overview.md`  
> **Especificación Primaria:** [[specification-v2]]  
> **Decisiones Constitucionales Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]], [[ADR-005-node-permission-sandboxing]], [[ADR-006-oxc-ast-engine]]

---

## 1. Filosofía del Sistema: Desacoplamiento Kernel-Space vs. User-Space

**Alexandria Runtime** (`v2.4.0-OmniPragmatic`) es un arnés de ingeniería de software *headless* compilado nativamente en Rust (Edición 2024). Su diseño aborda la programación asistida por IA desde la teoría de sistemas operativos: separa de forma infranqueable el **razonamiento probabilístico** en espacio de usuario (*User-Space*) de la **validación y ejecución determinista** en espacio de kernel (*Kernel-Space*).

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                           USER-SPACE (PROCESO PROBABILÍSTICO)                            │
│  • Modelos de Lenguaje (LLMs) operando como procesos de cómputo no confiables (Untrusted) │
│  • Cero autoridad de autocertificación sobre compilación, tipado, seguridad o pruebas    │
│  • Comunicación estrictamente restringida a llamadas de sistema vía protocolo MCP stdio  │
└────────────────────────────────────────────┬─────────────────────────────────────────────┘
                                             │
                        Syscalls JSON-RPC 2.0│(prepare_task_context, verify_and_gate_changes)
                                             ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                         KERNEL-SPACE (ÁRBITRO DETERMINISTA EN RUST)                      │
│  • Topología en RAM (petgraph): Resolución Depth=1 + ADR Hoisting (<=6) en <= 0.2 ms     │
│  • Escáner AST Nativo (oxc_parser/oxc_resolver): Gate 0 y Syscall Traps en <= 2.0 ms     │
│  • Sandbox de Subprocesos (Node --permission): Ejecución aislada y lectura JSON          │
│  • Persistencia Transaccional (rusqlite WAL): Actor MPSC Single-Writer Post-Commit       │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Garantías del Desacoplamiento ([[ADR-001-kernel-space-harness]])
1. **Erradicación de la Alucinación de Veredicto:** El LLM carece de permisos para declarar una tarea como completada si el binario en Rust no ha verificado previamente el código de salida `0` y el reporte estructurado de cada compuerta en [[sandbox-runner]] y [[gate-guard-ast]].
2. **Inmunidad al Desbordamiento de Contexto (*Anti Context-Rot*):** Los volcados crudos de compilación, trazas de subprocesos y árboles sintácticos se procesan fuera de banda en memoria de Rust. El modelo solo recibe diagnósticos quirúrgicos resumidos y pistas históricas recuperadas por ranking léxico BM25 o similitud vectorial.
3. **Interrupciones de Sistema (*Syscall Traps*):** Cuando una tarea iniciada como cambio local intenta mutar una ruta protegida o alterar la firma pública de un símbolo exportado (`export function`, `export type`, `export interface`), el kernel intercepta la operación antes de ejecutar pruebas y promueve la sesión al protocolo de auditoría total.

---

## 2. Arquitectura de Dos Planos (Control Relacional y Memoria Episódica)

El estado operativo del runtime persiste de forma desacoplada en `~/.alexandria/vault.db`, administrado por el subsistema [[core-engine]] bajo el esquema detallado en [[database-schema]] y gobernado por [[ADR-004-single-writer-mpsc-storage]]:

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                            PLANO 1: CONTROL RELACIONAL DETERMINISTA                      │
│              (SQLite 3 en Modo WAL + Single-Writer MPSC Transactional Actor)             │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│ • Catálogo de Agentes: Identificadores, permisos RBAC y herramientas MCP autorizadas     │
│ • Grafo Persistido de la Bóveda: Nodos canónicos, hashes SHA-256 y aristas dirigidas     │
│ • Catálogo Global de Skills: Firmas YAML, palabras clave de disparo e invariantes AST    │
│ • Estado de Quality Gates: Veredictos estructurados de oxc, tsc, SAST y Vitest           │
│ • Registro Forense (FAIL-XXX): Secuencia monotónica AtomicU64 y trazas de error          │
│ • Estado de Circuit Breakers: Contadores de fallos consecutivos por módulo (Umbral = 3)  │
└────────────────────────────────────────────┬─────────────────────────────────────────────┘
                                             │
                                             ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                         PLANO 2: MEMORIA VECTORIAL Y LÉXICA EPISÓDICA                    │
│               (sqlite-vec + Daemon Externo / Fallback Nativo FTS5 BM25 Porter)           │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│ • shared_canon_vec: Punto de entrada semántico cuando se omite el módulo objetivo        │
│ • skill_router_vec / skills_fts: Enrutamiento de Cápsulas JIT por intención técnica      │
│ • agent_memory_enki: Patrones de resolución idiomática y tipado nominal por lenguaje     │
│ • agent_memory_utu: Vectores de vulnerabilidades, regresiones y anti-patrones SAST       │
│ • agent_memory_nabu / incidents_fts: Huellas de stack traces históricos (FAIL-XXX)       │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Invariantes del Plano de Datos
- **Aislamiento Estricto por Esquema:** La memoria episódica del constructor (`Enki`) jamás contamina el espacio de evaluación del auditor en frío (`Utu`) ni del cronista forense (`Nabu`).
- **Primacía del Grafo sobre RAG:** El Plano 2 **nunca** sustituye al grafo topológico de `./alexandria/` para inyectar reglas arquitectónicas. Las leyes del sistema se resuelven deterministamente mediante [[topological-graph]]; la búsqueda léxica/vectorial se restringe al descubrimiento de *Skills* JIT y a la correlación de stack traces con incidentes `FAIL-XXX` previos.
- **Huella Residente $\le 15\text{ MB}$ y Operación 100% Offline:** El binario de Alexandria no enlaza modelos pesados de ML en su propio espacio de direcciones. Si el daemon externo de *embeddings* no está disponible, el motor conmuta instantáneamente a consultas FTS5 BM25 sanitizadas (`"tok1" OR "tok2"`), garantizando latencias sub-milisegundo sin riesgo de `SQLITE_CORRUPT_VTAB`.

---

## 3. El Cuarteto Central de Agentes y Polimorfismo por Sub-Sesiones

Alexandria Runtime reemplaza los enjambres caóticos de decenas de agentes por un **Panteón Inmutable de 4 Arquetipos Funcionales** con aislamiento físico de contexto:

```
                         [ INGENIERO / PRODUCT OWNER ]
                         (Directiva Técnica o Coloquial)
                                       │
                                       ▼
                         ┌───────────────────────────┐
                         │    BEL (The Overseer)     │
                         │ Clasificador AST/Topología│
                         └─────────────┬─────────────┘
                                       │
                   (Sub-Sesión Limpia + Prefix Pinning + JIT)
                                       ▼
                         ┌───────────────────────────┐
                         │    ENKI (The Builder)     │
                         │   Constructor Polimórfico │
                         └─────────────┬─────────────┘
                                       │
                    (Git Diff + JSON Test Report + Oxc AST)
                                       ▼
                         ┌───────────────────────────┐
                         │    UTU (The Critic)       │
                         │ Auditor Stateless en Frío │
                         └─────────────┬─────────────┘
                                       │
                           (Aprobación / Quality Gates)
                                       ▼
                         ┌───────────────────────────┐
                         │    NABU (The Chronicler)  │
                         │ Docs / Incidentes / Deploy│
                         └───────────────────────────┘
```

### 3.1. `Bel` (The Overseer) — Orquestador y Clasificador Zero-Turn
- **Función:** Punto de entrada único. Evalúa en Rust el radio de impacto de la directiva mediante [[topological-graph]] y [[gate-guard-ast]] sin consumir turnos previos de LLM.
- **Autoridad:** Clasifica la solicitud en *Path Alpha* o *Path Omega*, verifica la doble disyunción antes de autorizar *Git worktrees* paralelos y procesa las interrupciones de sistema (*Syscall Traps*).

### 3.2. `Enki` (The Builder) — Constructor Polimórfico con Rotación de Fase
- **Función:** Diseño de implementación, modelado de datos, codificación en `src/` y escritura de pruebas en `tests/`.
- **Rotación mediante *Forked Prefix Sub-Sessions* ([[ADR-002-prefix-pinned-caching]]):** Dado que las ventanas de contexto de los LLMs son estructuras *append-only*, no se instancian agentes separados para base de datos, backend y frontend. `Enki` asume cada especialidad instanciando sub-sesiones efímeras que comparten el mismo bloque estático cacheado al 100%:
  - **Sub-Sesión Fase DB:** Prefijo Estático Inmutable + `architecture/database-schema.md` + Cápsula JIT de migraciones relacionales.
  - **Sub-Sesión Fase Backend:** Prefijo Estático Inmutable + firmas generadas de DB + Cápsula JIT de contratos API y clausura transaccional.
  - **Sub-Sesión Fase Frontend:** Prefijo Estático Inmutable + contrato tipado de API + Cápsula JIT de componentes UI y estado.
- **Paralelismo Seguro en Worktrees:** `Bel` solo autoriza clonar a `Enki` en árboles de trabajo paralelos (`Enki-Worker-A`, `Enki-Worker-B`) si se satisfacen en $\le 2\text{ ms}$ dos invariantes matemáticos:
  1. **Disyunción Documental (`petgraph`):**
     $$G_A^{\text{wiki}}(\text{depth}=1) \cap G_B^{\text{wiki}}(\text{depth}=1) = \emptyset$$
  2. **Disyunción de Código (`oxc_resolver`):**
     $$\text{Imports}_{\text{mut}}(A) \cap \text{Imports}_{\text{mut}}(B) = \emptyset$$

### 3.3. `Utu` (The Critic) — Auditor Stateless en Frío
- **Función:** Juez imparcial de calidad y seguridad en **Gate 4**.
- **Aislamiento Epistémico:** Se invoca mediante una llamada de inferencia *stateless* completamente separada de la sesión de `Enki`. `Utu` jamás lee las justificaciones ni los intentos fallidos del constructor; recibe exclusivamente el `git diff` limpio, la lista de ADRs elevados (`hoistedAdrs`) y los reportes JSON de `oxc`, `tsc` y `vitest`.

### 3.4. `Nabu` (The Chronicler) — Custodio de la Bóveda y Memoria Forense
- **Función:** Mantenimiento del grafo en `./alexandria/`, formalización de contratos en `modules/`, redacción de `[[ADR-XXX]]` en `decisions/`, registro de incidentes `FAIL-XXX` en [[failures]] y asentamiento de cambios aprobados en [[log]].

---

## 4. Topología de Contexto Cache-Friendly (*Prefix Pinning*)

Para eliminar la penalización de latencia y el costo de re-procesar tokens estáticos en cada turno, el servidor [[mcp-server]] estructura la ventana de contexto dividiendo el estado en dos bloques estrictos ([[ADR-002-prefix-pinned-caching]]):

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                 ESTRUCTURA DE CONTEXTO CACHE-FRIENDLY (PREFIX PINNING)                   │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│ [BLOQUE ESTÁTICO / PREFIJO FIJADO BIT A BIT — 100% KV-CACHE HIT EN TODA SUB-SESIÓN]      │
│  1. System Prompt Constitucional (AGENTS.md / GEMINI.md: Leyes e Invariantes de Rol)     │
│  2. Canon Compartido (Topología de alto nivel y directivas universales de compilación)   │
│  3. Catálogo de Herramientas MCP (Esquemas JSON-RPC ordenados deterministamente)         │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│ [BLOQUE DINÁMICO / SUFIJO VOLÁTIL POR SUB-SESIÓN DE FASE]                                │
│  4. Retorno MCP (prepare_task_context): Vecinos Depth=1 + ADRs Hoisted (Depth<=6)        │
│  5. Retorno MCP (resolve_skill_capsule): Cápsula JIT de la fase activa (DB, API o UI)    │
│  6. Historial Corto de Ejecución: Veredictos JSON de la fase TDD en curso (RED/GREEN)    │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Orquestación Adaptativa Dual: Path Alpha vs. Path Omega

```
                                   [ SOLICITUD ENTRANTE ]
                                             │
                                             ▼
                      ┌──────────────────────────────────────────────┐
                      │ BEL (Motor Rust + Heurística Topológica AST) │
                      │ ¿Toca rutas críticas, firmas públicas o ADRs?│
                      └──────────────────────┬───────────────────────┘
                                             │
                  ┌──────────────────────────┴──────────────────────────┐
                  ▼ (NO: Impacto Local Acotado)                         ▼ (SÍ: Impacto Estructural)
       [ PATH ALPHA: FAST-PATH ]                             [ PATH OMEGA: FULL-PANTHEON ]
       (Micro-fixes, estilos, refactors internos)            (Nuevos módulos, schema DB, auth, ADRs)
                  │                                                     │
                  ▼                                                     ▼
        1 Turno LLM: Enki (Builder)                           Panteón Secuencial Completo:
        - Ejecuta cambio local                                1. Bel: Análisis y plan topológico
        - Corre Gate 0 (Oxc), Gate 1 y Gate 3                 2. Nabu: Contrato en modules/ y [[ADR-XXX]]
        - Si muta export público -> SYSCALL TRAP ───────────► 3. Enki: Ciclo TDD (RED -> GREEN -> REFACTOR)
                  │                                           4. GateGuard: Gates 0, 1, 2 y 3 en Sandbox
                  ▼                                           5. Utu: Auditoría Stateless en frío (Gate 4)
        Nabu (Hook Nativo en Rust / 0 Tokens):                6. Nabu: Commit atómico y registro en log.md
        - Commit Git atómico y actualización de log.md
```

### Matriz Comparativa de Rutas de Ejecución

| Dimensión Operativa | Path Alpha (`alpha_fast`) | Path Omega (`omega_pantheon`) |
|---|---|---|
| **Casos de Uso** | Ajustes de UI/CSS, corrección de bugs internos, refactorización de funciones privadas sin alterar contratos. | Creación de módulos, mutación de esquemas SQL/Prisma, cambios en autenticación, alteración de firmas públicas. |
| **Mecanismo de Escalación** | **Syscall Trap Automático:** Si `GateGuard` detecta edición en `protectedPathsForOmegaUpgrade` o cambio en un `export` público, promueve a Path Omega. | Ejecución gobernada de extremo a extremo desde el turno cero. |
| **Intervención de Agentes** | 1 único turno de LLM (`Enki`); cierre documental ejecutado nativamente en Rust por `Nabu` (0 tokens). | Ciclo completo (`Bel` $\rightarrow$ `Nabu` $\rightarrow$ `Enki` $\rightarrow$ `Utu Stateless` $\rightarrow$ `Nabu`). |
| **Presupuesto de Tokens** | Mínimo ($\sim 1\text{k} - 3\text{k}$ tokens). | Estructurado ($\sim 15\text{k} - 35\text{k}$ tokens con $>85\%$ de ahorro por *Prefix Pinning*). |
| **Compuertas Exigidas** | `Gate 0` (Oxc AST), `Gate 1` (`tsc --noEmit`) y `Gate 3` (Vitest diferencial sobre archivos modificados). | Matriz completa `Gate 0` a `Gate 4`, cobertura $\ge 90\%$, escaneo SAST y aprobación en frío de `Utu`. |

---

## 6. Matriz de Cobertura de las 12 Dimensiones de Ingeniería de Software

| # | Dimensión | Subsistema Responsable | Mecanismo Determinista de Verificación |
|---|---|---|---|
| **1** | **Arquitectura y Contratos** | [[topological-graph]] / `Nabu` | Resolución `petgraph` ($\text{Depth}=1$) + *ADR Hoisting* transitivo ($\text{Depth} \le 6$). Prohibido tocar `src/` sin contrato en `modules/`. |
| **2** | **Calidad Léxica y Tipado** | [[gate-guard-ast]] | Escaneo en RAM con `oxc_parser` ($\le 2\text{ ms}$). Prohibición absoluta de `TSAnyKeyword` (`any`) y doble casteo `as unknown as T`. |
| **3** | **Integridad Transaccional** | [[gate-guard-ast]] / [[database-schema]] | Clausura léxica total en `prisma.$transaction` (cero aliasing, cero `.then()`, `await` directo obligatorio) y actor MPSC post-commit en SQLite. |
| **4** | **Verificación Funcional (TDD)** | [[sandbox-runner]] | Ciclo `RED -> GREEN -> REFACTOR` validado sobre archivo `--outputFile` JSON. Rechazo de `SyntaxError` en Fase RED; exigencia de `AssertionError`. |
| **5** | **Aislamiento y Seguridad de Runtime** | [[sandbox-runner]] | Ejecución de pruebas bajo Node.js Permission Model (`--permission`, `--allow-fs-read`, `--allow-fs-write`), `env_clear()` y bloqueo de red. |
| **6** | **Seguridad Estática (SAST)** | [[core-engine]] / `Utu` | Escaneo fuera de banda de secretos en `git diff`, auditoría de dependencias y juicio en frío de `Utu` en Gate 4. |
| **7** | **Concurrencia y Paralelismo** | [[topological-graph]] / [[gate-guard-ast]] | Doble invariante de disyunción (subgrafos de wiki disjuntos + grafos de `import` disjuntos vía `oxc_resolver`) antes de crear *Git worktrees*. |
| **8** | **Eficiencia de Caché y Latencia** | [[mcp-server]] | *Prefix Pinning* inmutable bit a bit y entrega de cápsulas de habilidad JIT mediante retornos de herramientas MCP. |
| **9** | **Prevención de Bucles (Anti-Thrashing)** | [[core-engine]] | Disyuntor determinista (*Circuit Breaker*) en SQLite. Si $\text{intentos consecutivos} \ge 3 \implies \text{ABORT\_AND\_ALERT}$ y congelación del worktree. |
| **10** | **Memoria Forense y Autocuración** | [[database-schema]] / [[failures]] | Indexación automática de stack traces en `incidents_fts` con triggers completos y recuperación BM25 mediante consultas `OR` sanitizadas. |
| **11** | **Portabilidad Multiplataforma** | [[core-engine]] / [[sandbox-runner]] | Normalización de rutas extendidas UNC en Windows (`\\?\` y `\\?\UNC\`), preservación de `SystemRoot` para `libuv` y redirección explícita de `TMP`. |
| **12** | **Trazabilidad y Observabilidad** | [[log]] / [[mcp-server]] | Bitácora canónica inmutable en `log.md`, commits atómicos vinculados a contratos y telemetría estructurada por `stderr`. |

---

## 7. Grafo de Enlaces Canónicos

- **Raíz y Mapa Maestro:** [[index]]
- **Especificación Primaria:** [[specification-v2]]
- **Esquema de Persistencia:** [[database-schema]]
- **Módulos del Kernel:** [[core-engine]], [[mcp-server]], [[topological-graph]], [[gate-guard-ast]], [[sandbox-runner]]
- **Registros de Decisión (ADRs):** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]], [[ADR-005-node-permission-sandboxing]], [[ADR-006-oxc-ast-engine]]
- **Bitácoras Operativas:** [[log]], [[failures]]
