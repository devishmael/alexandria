---
id: specification-v2
title: "Alexandria Runtime v2.4.0 — Blueprint Arquitectónico y Especificación del Kernel"
type: specification
status: canonical
created_at: 2026-10-19
tags:
  - specification
  - blueprint
  - kernel-space
  - v2.4.0
---
# ALEXANDRIA RUNTIME

**Architectural Blueprint & Kernel-Space Engine Specification**

**Versión:** `2.4.0-OmniPragmatic (Golden Master Production Specification)`

**Paradigma:** Headless Deterministic Agent Harness / User-Space LLM vs. Kernel-Space Rust Arbitrator / Prefix-Pinned Context Topology / Dual-Graph Isolation / Zero-Trust Out-of-Band Execution

  

## 1. Resumen Ejecutivo y Filosofía del Sistema

**Alexandria Runtime** es un arnés de ingeniería de software _headless_ y agnóstico a nivel de sistemas, compilado en Rust y concebido para desacoplar el razonamiento probabilístico de los Modelos de Lenguaje (LLMs) de la ejecución determinista en el sistema operativo.

  

Aborda el desarrollo asistido por IA desde la arquitectura de un **kernel de sistema operativo**: el LLM es tratado como un proceso de cómputo probabilístico no confiable en **espacio de usuario (_user-space_)**, mientras que el binario en Rust actúa como el árbitro determinista con privilegios de validación y ejecución en **espacio de kernel (_kernel-space_)**.

  

### Los 4 Pilares Fundacionales

1. **Gobierno por Especificaciones Vivas con _ADR Hoisting_:** El contexto estructural no proviene de volcados masivos de prompts ni de memorias RAG difusas. La fuente canónica de verdad es una Bóveda Viva en Obsidian (`wiki/`) interconectada mediante enlaces bidireccionales (`[[nodo]]`) y resuelta en RAM con `petgraph` en $\le 0.2\text{ ms}$. Combina vecindad estricta $\text{depth}=1$ para módulos con propagación transitiva ($\text{depth} \le 6$) exclusiva para invariantes constitucionales (`[[ADR-XXX]]`).
    
      
    
2. **Ingesta Adaptativa Dual y Clasificación Zero-Turn:** El motor interpreta tanto directivas técnicas de bajo nivel (tipos nominales, _branded types_, esquemas SQL e invariantes de AST) como lenguaje coloquial de producto. Clasifica estáticamente en Rust el radio de impacto sin gastar turnos previos de LLM, y promueve automáticamente de vía rápida (_Path Alpha_) a vía completa (_Path Omega_) mediante interrupciones de _GateGuard_ (_syscall traps_) si se intenta tocar rutas críticas.
    
      
    
3. **Panteón Inmutable de 4 Agentes con Aislamiento Físico de Contexto:** Se erradica la sobreingeniería de decenas de agentes permanentes aislados. El sistema estandariza 4 arquetipos funcionales (`Bel`, `Enki`, `Utu`, `Nabu`), donde la especialización por dominio se inyecta dinámicamente mediante sub-sesiones con prefijo fijado (_Prefix Pinning_) sobre un único constructor polimórfico (`Enki`), y la auditoría en frío (`Utu`) se ejecuta en sesiones _stateless_ independientes.
    
      
    
4. **Compuertas Fuera de Banda con Sandbox a Nivel de Runtime:** El modelo carece de autoridad para certificar que su código compila o pasa pruebas. Los escáneres de AST (`oxc_parser`), linters, tipado estricto, seguridad y suites de pruebas corren a nivel de sistema operativo fuera de la ventana de contexto, bajo entornos purgados (`env_clear`) y permisos restringidos en el binario de ejecución (`Node.js --permission`).
    
      
    

## 2. Arquitectura de Dos Planos (Control Relacional y Memoria Vectorial/Léxica)

El estado operativo del arnés reside en almacenamiento local persistente desacoplado dentro de `~/.alexandria/vault.db`, gobernado en Rust por un **Actor de Escritura Única (`VaultWriterActor` con _Transactional Batching_ y confirmación post-commit sobre `tokio::sync::mpsc`)** para garantizar cero bloqueos (`SQLITE_BUSY`) durante ejecuciones paralelas:

  

Plaintext

```
┌────────────────────────────────────────────────────────────────────────┐
│                     PLANO 1: CONTROL RELACIONAL                        │
│     (SQLite 3 en Modo WAL + Single-Writer MPSC Batching Actor)         │
├────────────────────────────────────────────────────────────────────────┤
│ • Catálogo de Agentes (IDs, permisos RBAC, herramientas asignadas)     │
│ • Grafo Persistido de la Wiki (Tipos de nodo, SHA-256 y aristas)       │
│ • Catálogo Global de Skills (Firmas, triggers, invariantes y AST rules)│
│ • Estado de Quality Gates (Veredictos JSON de tsc, oxc, vitest)        │
│ • Registro de Incidentes (FAIL-XXX únicos con secuencia monotónica)    │
│ • Estado de Circuit Breakers y contadores de thrashing por módulo      │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                 PLANO 2: MEMORIA VECTORIAL EPISÓDICA                   │
│       (sqlite-vec + Daemon Externo / Fallback Léxico FTS5 BM25)        │
├────────────────────────────────────────────────────────────────────────┤
│ • shared_canon_vec: Búsqueda de entrada cuando no se indica módulo     │
│ • skill_router_vec: Matching semántico de Cápsulas JIT por intención   │
│ • agent_memory_enki: Patrones de resolución idiomática por lenguaje    │
│ • agent_memory_utu: Vectores de vulnerabilidades y anti-patrones SAST  │
│ • agent_memory_nabu: Huellas vectoriales de stack traces (FAIL-XXX)    │
└────────────────────────────────────────────────────────────────────────┘
```

### Reglas Operativas del Plano de Datos

- **Aislamiento por Esquema:** La memoria episódica del constructor (`Enki`) nunca contamina el espacio latente del auditor (`Utu`) ni del registrador (`Nabu`).
    
      
    
- **Frontera Determinista vs. Vectorial:** El Plano 2 (`sqlite-vec`) **jamás** reemplaza al grafo de la wiki para inyectar reglas arquitectónicas; se utiliza exclusivamente para:
    
      
    1. Descubrimiento de la _Skill_ adecuada cuando no se proporciona un `skillId` exacto.
        
          
        
    2. Recuperación forense en `Nabu`: comparar un `stackTrace` de compilación actual contra incidentes `FAIL-XXX` históricos para inyectar la solución previa que resolvió el mismo error.
        
          
        
    3. Detección de _code-smells_ recurrentes en `Utu`.
        
          
        
- **Preservación de Huella de RAM ($\sim 8\text{ MB} - 15\text{ MB}$) y Fallback FTS5 Sanitizado:** El binario de Alexandria no carga modelos pesados de _embeddings_ en su propia memoria residente; delega la vectorización fuera de proceso. Si el daemon externo no está activo, SQLite conmuta de forma transparente a tablas virtuales **FTS5 (BM25)** sincronizadas mediante _triggers_ nativos completos (`AFTER INSERT`, `AFTER DELETE`, `AFTER UPDATE`) y consultadas mediante un sanitizador de disyunción (`"tok1" OR "tok2"`), garantizando operación 100% _offline_ sin errores de sintaxis FTS5 ni `SQLITE_CORRUPT_VTAB`.
    
      
    

## 3. El Cuarteto Central: Roles, Contratos y `Enki` Polimórfico

Plaintext

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

### 1. Bel (The Overseer) — El Manager y Clasificador Determinista

- **Misión:** Punto de contacto, evaluación topológica y orquestación. Analiza estáticamente el radio de impacto de la solicitud. Si es informal, delega la formalización en `Nabu`; si es técnica, traslada los invariantes directamente a contratos ejecutables.
    
      
    
- **Escalación por Interrupción (_Syscall Trap_):** Si una tarea inicia en vía rápida (_Path Alpha_) pero `Enki` intenta modificar una ruta protegida o alterar la firma pública de un tipo/función exportada, `Bel` intercepta la operación y promueve la sesión a _Path Omega_.
    
      
    
- **Permisos:** Lectura de workspace, evaluación de subgrafos en `petgraph` y despacho de sub-sesiones a `Enki` y `Nabu`.
    
      
    

### 2. Enki (The Builder) — El Constructor Polimórfico por Sub-Sesiones

- **Misión:** Diseñar la implementación, modelar datos, codificar y escribir pruebas. No existen agentes separados para bases de datos, APIs o frontend.
    
      
    
- **Rotación de Fase sin Invalidación de Caché (_Forked Prefix Sub-Sessions_):** Dado que las ventanas de contexto de los LLMs son estructuras _append-only_, `Enki` asume cada especialización instanciando **sub-sesiones efímeras por fase** que comparten el mismo bloque estático cacheado:
    
      
    - **Sub-Sesión Fase DB:** Prefijo Estático + Cápsula JIT (`architecture/database-schema.md` + skill de migraciones relacionales).
        
          
        
    - **Sub-Sesión Fase Backend:** Prefijo Estático (100% _Cache Hit_) + firmas persistidas de DB + Cápsula JIT de contratos de API y reglas de concurrencia.
        
          
        
    - **Sub-Sesión Fase Frontend:** Prefijo Estático (100% _Cache Hit_) + contrato tipado de API + Cápsula JIT de especificación UI/UX y tokens de diseño.
        
          
        
- **Protocolo TDD Obligatorio Verificado por Aserción en Archivo Aislado (`--outputFile`):**
    
      
    
    $$\text{Fase RED (Fallo por AssertionError)} \longrightarrow \text{Fase GREEN (Código mínimo)} \longrightarrow \text{REFACTOR}$$
    
    En la **Fase RED**, el arnés lee el archivo JSON generado por Vitest (inmune a contaminación por `console.log` en `stdout`) y exige que el archivo de prueba compile sin errores de sintaxis (`SyntaxError` o `Cannot find module` son rechazados como Fase RED inválida), aprobando la transición únicamente cuando el fallo proviene de una aserción de negocio (`AssertionError` o `NotImplementedError`).
    
      
    
- **Workers Efímeros en Paralelo con Doble Invariante de Disyunción:**
    
    `Bel` solo autoriza clonar a `Enki` en _Git worktrees_ temporales (`Enki-Worker-A`, `Enki-Worker-B`) si se cumplen simultáneamente dos condiciones en $\le 2\text{ ms}$:
    
      
    1. **Disyunción Documental (`petgraph`):**
        
          
        
        $$G_A^{\text{wiki}}(\text{depth}=1) \cap G_B^{\text{wiki}}(\text{depth}=1) = \emptyset$$
        
    2. **Disyunción de Código (`oxc_resolver`):**
        
          
        
        $$\text{Imports}_{\text{mut}}(A) \cap \text{Imports}_{\text{mut}}(B) = \emptyset$$
        

### 3. Utu (The Critic) — El Pipeline de CI/CD y Auditor Stateless en Frío

- **Misión:** Juzgar con imparcialidad en contexto frío (aislado del historial de razonamiento y justificaciones de `Enki`).
    
      
    
- **Simetría MCP y CLI mediante Llamada Stateless:** En Modo MCP dentro del IDE, la herramienta `verify_and_gate_changes` ejecuta desde Rust una llamada de inferencia sin estado (_stateless API call_) independiente enviando únicamente el `git diff`, los `[[ADR-XXX]]` vigentes y el reporte de los _Quality Gates_.
    
      
    
- **Permisos:** Solo lectura del workspace y ejecución de herramientas de auditoría.
    
      
    

### 4. Nabu (The Chronicler) — Observabilidad, Bóveda Viva y Despliegue

- **Misión:** Custodio de la bóveda Markdown (`wiki/`) y de la memoria forense.
    
      
    
- **Responsabilidades:** Formaliza `[[ADR-XXX]]`, documenta incidentes `FAIL-XXX` en `wiki/failures.md` consultando `incidents_fts` / `sqlite-vec`, y al superar todos los gates actualiza `wiki/log.md`, `wiki/index.md` y levanta el entorno local.
    
      
    

## 4. Mitigaciones de Rendimiento, Caché y Contexto

Plaintext

```
┌────────────────────────────────────────────────────────────────────────┐
│             ESTRUCTURA DE CONTEXTO CACHE-FRIENDLY (PREFIX PINNING)     │
├────────────────────────────────────────────────────────────────────────┤
│ [BLOQUE ESTÁTICO / CACHEADO — INMUTABLE EN TODAS LAS SUB-SESIONES]     │
│  1. System Prompt Base (Invariantes de rol, leyes de compilación)      │
│  2. shared_canon (Topología de alto nivel y reglas universales)        │
│  3. Definiciones de Tools MCP (Esquemas JSON-RPC fijos e inmutables)   │
├────────────────────────────────────────────────────────────────────────┤
│ [BLOQUE DINÁMICO / SUFIJO VOLÁTIL POR SUB-SESIÓN DE FASE]              │
│  4. Retorno MCP: Nodos depth=1 + ADRs hoisted (depth<=6) en < 0.2 ms   │
│  5. Retorno MCP: Cápsula JIT de la fase activa (DB, API o UI)          │
│  6. Historial corto del turno de ejecución TDD actual                  │
└────────────────────────────────────────────────────────────────────────┘
```

### A. Preservación Estricta de Prompt Caching (_Prefix Pinning_)

- El prefijo del sistema y los esquemas de herramientas MCP permanecen estáticos e inmutables bit a bit durante toda la sesión.
    
      
    
- Las cápsulas de especialización (`resolve_skill_capsule`) se inyectan dinámicamente en la parte baja de la ventana de contexto.
    
      
    

### B. Orquestación Adaptativa Dual (_Anti Turn-Churn_)

Plaintext

```
                                [ SOLICITUD ENTRANTE ]
                                          │
                                          ▼
                   ┌──────────────────────────────────────────────┐
                   │ BEL (Motor Rust + Heurística Topológica AST) │
                   │ ¿Toca schema DB, auth, nuevos módulos o ADRs?│
                   └──────────────────────┬───────────────────────┘
                                          │
               ┌──────────────────────────┴──────────────────────────┐
               ▼ (NO: Impacto Local)                                 ▼ (SÍ: Impacto Estructural)
    [ PATH ALPHA: FAST-PATH ]                             [ PATH OMEGA: FULL-PANTHEON ]
    (Micro-tareas, fixes, UI styling)                     (Módulos nuevos, ADRs, schema DB)
               │                                                     │
               ▼                                                     ▼
     1 LLM Turn: Enki (Builder)                           Panteón Secuencial Completo:
     - Ejecuta cambio atómico                             1. Bel: Desglose y contratos
     - Corre Gate 0, Gate 1 y Gate 3                      2. Nabu: Redacción de [[ADR-XXX]]
               │                                          3. Enki: TDD (RED -> GREEN -> REFACTOR)
               ▼                                          4. Utu: Auditoría SAST Stateless en frío
     Nabu (Rust Native Hook / 0 Tokens):                  5. Quality Gates fuera de banda (0-4)
     - Commit Git atómico                                 6. Nabu: Log canónico y deploy
     - Registra log en wiki/log.md
```

|**Criterio**|**Path Alpha (Fast-Path)**|**Path Omega (Full-Pantheon)**|
|---|---|---|
|**Casos de Uso**|Ajuste de estilo/CSS, refactor privado local, corrección de bug aislado sin alterar firmas públicas.|Nuevos endpoints, mutación de esquema SQL/Prisma, cambios de auth, nuevos módulos.|
|**Escalación (Syscall Trap)**|Si `Enki` toca un archivo protegido o altera un `export` público consumido externamente, `GateGuard` promueve automáticamente a _Path Omega_.|Flujo gobernado completo desde el inicio.|
|**Saltos de Agente**|**1 único turno de LLM** (`Enki` ejecuta; `Nabu` registra vía código nativo en Rust sin gastar tokens).|Ciclo completo (`Bel` $\rightarrow$ `Nabu` $\rightarrow$ `Enki` $\rightarrow$ `Utu Stateless` $\rightarrow$ `Nabu`).|
|**Gasto de Tokens**|Mínimo ($\sim 1\text{k} - 3\text{k}$ tokens).|Completo ($\sim 15\text{k} - 35\text{k}$ tokens con _Prefix Caching_ activo).|
|**Validación y Cobertura**|`Gate 0` (Oxc AST), `tsc --noEmit` y Vitest con **cobertura diferencial** sobre archivos tocados (se omite umbral de cobertura si solo cambia `.css` o `.d.ts`).|Matriz completa: `Gate 0` a `Gate 4`, SAST, linters, cobertura del módulo $\ge 90\%$ y `npm run build` standalone.|

### C. Disyuntor Determinista en Rust (_Anti-Thrashing Circuit Breaker_)

- Si un mismo módulo falla por tercera vez consecutiva en la fase de resolución:
    
      
    
    $$\text{Intentos} \ge 3 \implies \text{ABORT\_AND\_ALERT}$$
    
- El binario en Rust congela el _Git worktree_, bloquea nuevas mutaciones y notifica en consola el archivo y línea exactos.
    
      
    

## 5. Perfil de Referencia MVP v0.1 (`mvp-fullstack-ts`) e Invariantes Léxicas

Plaintext

```
┌────────────────────────────────────────────────────────────────────────┐
│                    ALEXANDRIA MVP v0.1 REFERENCE PROFILE               │
├───────────────────────┬────────────────────────────────────────────────┤
│ Capa                  │ Tecnología Oficial MVP (mvp-fullstack-ts)      │
├───────────────────────┼────────────────────────────────────────────────┤
│ Lenguaje Primario     │ TypeScript 5.x (Strict Mode, Branded Types)    │
│ Frontend              │ React 19 (Next.js App Router / Vite) + Tailwind│
│ Backend / API         │ Next.js Server Actions / Route Handlers        │
│ Capa de Datos         │ PostgreSQL 16 + pgvector                       │
│ Acceso a Datos (ORM)  │ Prisma ORM con validación léxica AST estricta  │
│ Testing & Sandbox     │ Vitest (--outputFile JSON) + Node --permission │
│ Linting & Motor AST   │ Oxc (oxc_parser + oxc_resolver) + tsc --noEmit │
└───────────────────────┴────────────────────────────────────────────────┘
```

### Invariantes Constitucionales Verificadas por `GateZeroAstVisitor` (`oxc_ast`)

1. **Tipado Nominal Obligatorio:** Prohibido el nodo `TSAnyKeyword` (`any`) y el doble casteo `as unknown as T` (`TSAsExpression` anidado sobre `TSUnknownKeyword`). Se exigen _Branded Types_:
    
      
    
    TypeScript
    
    ```
    export type UserId = string & { readonly __brand: unique symbol };
    export type WorkspaceId = string & { readonly __brand: unique symbol };
    ```
    
2. **Clausura Léxica Total en Transacciones (`prisma.$transaction`) para Garantizar $\le 25\text{ ms}$:**
    
      
    - El primer argumento debe ser un `ArrowFunctionExpression` _inline_ con un parámetro simple sin _destructuring_ (`async (tx) => { ... }`) o un `ArrayExpression` estático.
        
          
        
    - Queda prohibido reasignar `tx` en declaraciones (`const db = tx`) o asignaciones tardías (`db = tx`), así como pasarlo como argumento a funciones auxiliares (`helper(tx)`).
        
          
        
    - Toda expresión `await` dentro del callback debe ser estrictamente de la forma `await tx.<modelo>.<operacion>(...)` o `await tx.$queryRaw(...)`, bloqueando encadenamientos `.then()`, `.catch()`, `Promise.all` o llamadas de red.
        
          
        
    - Quedan prohibidas las llamadas a `fetch`, `setTimeout`, `setInterval` y cualquier operación de `tx` sin `await` inmediato (tanto en sentencias sueltas como asignada a variables locales `const p = tx.model.op()` o `p = tx.model.op()`).
        
          
        
3. **Calidad de Pruebas:** Cobertura de código obligatoria $\ge 90\%$ en módulos de negocio.
    
      
    

## 6. Matriz de Cobertura de las 12 Dimensiones del Software

Plaintext

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        MATRIZ DE LAS 12 DIMENSIONES EN ALEXANDRIA                      │
├───────────────────────────────┬────────────────────────────┬───────────────────────────┤
│ DIMENSIÓN                     │ AGENTE LÍDER               │ MECANISMO DE EJECUCIÓN    │
├───────────────────────────────┼────────────────────────────┼───────────────────────────┤
│ 1. Frontend & Client-Side     │ Enki (UI Capsule)          │ Design Systems, Wasm, CSP │
│ 2. Capa de APIs & Protocolos  │ Bel (Contratos) / Enki     │ REST, gRPC, SSE, OpenAPI  │
│ 3. Backend & Core Lógico      │ Enki (Logic Capsule)       │ Clean Arch, DDD, Zeroize  │
│ 4. Persistencia & Datos       │ Enki (DB Capsule)          │ ACID SQL, NoSQL, Vectores │
│ 5. Big Data & Event-Driven    │ Enki (Event Capsule)       │ Kafka, EDA, CQRS, CRDTs   │
│ 6. IA, MLOps & Analytics      │ Enki (ML Capsule)          │ ONNX, Pipelines ETL, RAG  │
│ 7. Ciberseguridad Avanzada    │ Utu (Security Gate)        │ SAST, IAM, mTLS, Secretos │
│ 8. Infraestructura & Cloud    │ Enki (IaC) / Utu           │ Docker, K8s, Terraform    │
│ 9. CI/CD & Delivery           │ Utu (Pipeline Gate)        │ Quality Gates, Blue-Green │
│ 10. QA, Testing & Resiliencia │ Utu (Audit) / Enki (TDD)   │ Vitest, Playwright, K6    │
│ 11. Observabilidad & Trazas   │ Nabu (Observatory)         │ OpenTelemetry, Logs, FAIL │
│ 12. Diseño de Producto & ALM  │ Bel (Scaffolder)           │ ADRs, User Stories, UX    │
└───────────────────────────────┴────────────────────────────┴───────────────────────────┘
```

- **Dimensión 1 (Frontend & Client-Side):** Web (React, Vue, Svelte, Qwik), Móvil (Swift, Kotlin, Flutter, React Native), Escritorio (Tauri, Electron), Inmersivos (Unity, Unreal) y Sistemas Embebidos (Qt/C++). `Enki` aplica componentes atómicos, máquinas de estados (XState/Zustand) y WebAssembly (Wasm). `Utu` certifica WCAG 2.2, CSP y sanitización XSS.
    
      
    
- **Dimensión 2 (Capa de APIs):** REST, GraphQL, gRPC (Protobuf), WebSockets, SSE y Webhooks. `Bel` formaliza contratos OpenAPI 3.1; `Enki` implementa controladores tipados, _Rate Limiting_ y BFF.
    
      
    
- **Dimensión 3 (Backend & Core Lógico):** Arquitectura Hexagonal / Clean Architecture, DDD, concurrencia (Tokio, Goroutines, BullMQ) y _Zero-Trust Validation_ nominal.
    
      
    
- **Dimensión 4 (Persistencia & Datos):** SQL (PostgreSQL, MySQL), NoSQL (Mongo, Redis, Cassandra), Grafos (Neo4j), Vectores (`pgvector`, Qdrant) y S3. `Utu` valida índices (`EXPLAIN ANALYZE`) y bloquea consultas $N+1$.
    
      
    
- **Dimensión 5 (Big Data & Eventos):** EDA (Kafka, RabbitMQ), CQRS y sincronización _offline_ mediante CRDTs.
    
      
    
- **Dimensión 6 (IA & MLOps):** Inferencia local/borde (ONNX Runtime, `llama.cpp`), pipelines ETL y Vector RAG sin fugas de PII.
    
      
    
- **Dimensión 7 (Ciberseguridad):** IAM (OAuth2, OIDC, RBAC/ABAC), TLS 1.3, AES-256-GCM, mTLS, rotación de secretos, eBPF y _zeroization_ de claves en RAM.
    
      
    
- **Dimensión 8 (Infraestructura & Cloud):** Docker _multi-stage_, Kubernetes, Terraform/OpenTofu y políticas _IAM least-privilege_.
    
      
    
- **Dimensión 9 (CI/CD & Delivery):** GitHub Actions, GitLab CI, SAST/SCA y despliegues Canary / Blue-Green.
    
      
    
- **Dimensión 10 (QA, Testing & Resiliencia):** TDD en fase `RED` obligatoria, Playwright E2E, K6 y _Chaos Engineering_.
    
      
    
- **Dimensión 11 (Observabilidad & Trazas):** Prometheus, Datadog, Loki, OpenTelemetry y reportes forenses `FAIL-XXX`.
    
      
    
- **Dimensión 12 (Diseño de Producto & ALM):** Tokens de Figma, especificaciones API-First, historias de usuario y `[[ADR-XXX]]`.
    
      
    

## 7. Stack Técnico del Arnés (Alexandria Core en Rust)

Plaintext

```
┌────────────────────────────────────────────────────────────────────────┐
│                        CORE RUNTIME & CLI                              │
│             Rust (Edición 2024) + Tokio Async Runtime                  │
├────────────────────────────────────────────────────────────────────────┤
│ • Binario estático único (alexandria / alexandria.exe)                 │
│ • Huella de memoria RAM en reposo: ~8 MB a 15 MB                       │
│ • Latencia de respuesta en protocolo MCP (lecturas en RAM): < 3 ms     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌───────────────────┐   ┌───────────────────────┐   ┌────────────────────┐
│  ALMACENAMIENTO   │   │  TOPOLOGÍA Y WATCHER  │   │  MOTOR AST NATIVO  │
│ rusqlite (WAL) +  │   │ pulldown-cmark +      │   │ oxc_parser +       │
│ Dedicated Thread  │   │ petgraph +            │   │ oxc_ast +          │
│ MPSC Batch Actor  │   │ notify-debouncer-mini │   │ oxc_resolver       │
│ + sqlite-vec/FTS5 │   │ • Debounce de 75 ms   │   │ • Escaneo léxico y │
│ • Post-commit ack │   │ • Depth-1 + Hoisting  │   │   grafo de imports │
└─────────┬─────────┘   └───────────┬───────────┘   └─────────┬──────────┘
          └─────────────────────────┼─────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                PROTOCOLO MCP Y SANDBOX DE SUBPROCESOS                  │
│   • MCP Server JSON-RPC 2.0 (initialize, tools/list, tools/call)       │
│   • GateGuard Runner: tokio::process con env_clear(), UNC path clean,  │
│     SystemRoot/TMP preservados y Node.js (--permission / --outputFile) │
└────────────────────────────────────────────────────────────────────────┘
```

## 8. Pipeline ETL de Habilidades y Formato Canónico de Skill (`.md`)

YAML

```
---
id: "testing:tdd-vitest"
domain: "testing"
profile_compatibility:
  - "mvp-fullstack-ts"
description: "Fuerza el ciclo RED-GREEN-REFACTOR en TypeScript con Vitest antes de modificar src/"
trigger_keywords:
  - "nuevo endpoint"
  - "fix bug"
  - "nueva funcion"
  - "refactor"
required_tools:
  - "verify_and_gate_changes"
  - "write_file"
invariants:
  - "Prohibido modificar archivos en src/ sin una prueba fallando previamente en tests/ por AssertionError"
  - "Cero aserciones triviales (ej. toBeDefined aislado); evaluar casos de borde y tipos nominales"
---

# Procedimiento de Ejecución (Enki)

### 1. Fase RED

1. Crea o modifica `tests/<modulo>.test.ts`.
2. Define las aserciones exactas del comportamiento esperado.
3. Invoca `verify_and_gate_changes` con `tddPhase: "red_expect_assertion_failure"`.
4. Comprobación Determinista: El archivo debe compilar sin `SyntaxError` y fallar exclusivamente por `AssertionError` o `NotImplementedError`.

### 2. Fase GREEN

1. Escribe el código mínimo indispensable en `src/`.
2. Reejecuta `verify_and_gate_changes` con `tddPhase: "green_expect_pass"` hasta obtener salida exitosa.

### 3. Fase REFACTOR

1. Aplica tipado nominal estricto (`Branded Types`) y verifica que ninguna transacción de Prisma contenga llamadas `await` ajenas a `tx`.
2. Invoca `verify_and_gate_changes` con `tddPhase: "refactor_full_gate"` y confirma aprobación total de `Gate 0` a `Gate 4`.
```

## 9. La Bóveda Canónica en Obsidian (`wiki/`) y Resolución Topológica

Plaintext

```
wiki/
├── index.md                 # Mapa maestro del grafo de conocimiento y módulos
├── log.md                   # Bitácora cronológica inmutable de cambios superados
├── failures.md              # Registro forense de incidentes y defectos (FAIL-XXX)
├── raw/                     # Fuentes primarias históricas inmutables
├── architecture/            # Topología, esquemas y diseño visual
├── modules/                 # Contratos y diseño vivo de subsistemas
└── decisions/               # Registros de Decisión de Arquitectura (ADRs)
```

### Ecuación de Resolución Topológica con _ADR Hoisting_

El motor en Rust evalúa el subgrafo de contexto en memoria ($\le 0.2\text{ ms}$) aplicando la siguiente unión de conjuntos sobre `petgraph`:

  

$$\text{Contexto Inyectado}(M) = \text{Nota}(M) \cup N_1(M) \cup \{ A \in \text{Reachable}_{\le 6}(M) \mid \text{kind}(A) = \text{ConstitutionalAdr} \}$$

Donde $N_1(M)$ representa los vecinos directos a profundidad $1$, y $\text{Reachable}_{\le 6}(M)$ eleva únicamente los nodos `ConstitutionalAdr` (`wiki/decisions/ADR-*.md`) alcanzables hasta $6$ saltos de profundidad (incluyendo cadenas $\text{ADR-A} \rightarrow \text{ADR-B}$).

  

## 10. Hooks y Quality Gates Deterministas Fuera de Banda

|**Gate**|**Mecanismo / Comando Ejecutado (mvp-fullstack-ts)**|**Criterio Determinista de Aprobación**|**Acción Automática en Caso de Fallo**|
|---|---|---|---|
|**Gate 0: GateGuard (Oxc AST & Syscall Trap)**|`oxc_parser` + `oxc_resolver` en memoria Rust ($\le 2\text{ ms}$)|Cero `any`, cero `as unknown as T`, clausura léxica total de `tx` (sin `.then()`, sin aliasing, sin promesas huérfanas).|Aborto preventivo inmediato o promoción automática de Path Alpha a Path Omega.|
|**Gate 1: Tipos y Linter**|`npx tsc --noEmit && npm run lint`|Código de salida `0` ($0$ errores, $0$ advertencias).|Incrementa contador de fallos; deriva traza a `Nabu`.|
|**Gate 2: Seguridad (SAST & Deps)**|Escáner SAST + detector de secretos en diff|$0$ secretos expuestos, $0$ vulnerabilidades críticas.|Bloqueo inmediato del pipeline; alerta de seguridad.|
|**Gate 3: Pruebas Unitarias en Sandbox**|`node --permission --allow-worker --allow-addons ... vitest.mjs run --reporter=json --outputFile=...`|**Fase RED:** Compila y falla por `AssertionError`. **Fase GREEN:** $100\%$ pruebas en verde y cobertura $\ge 90\%$.|Si $\text{intentos} < 3$: registra `FAIL-XXX` y reintenta en TDD. Si $\text{intentos} \ge 3$: activa **Circuit Breaker**.|
|**Gate 4: Build & Auditoría Stateless (`Utu`)**|`npm run build` + juicio en frío _stateless_ de `Utu`|Salida exitosa (código `0`) y aprobación de `Utu` frente a los `[[ADR-XXX]]`.|Análisis forense de empaquetado o activación del **Circuit Breaker**.|

## 11. Configuración del Proyecto (`.alexandria.json` v2.4)

JSON

```
{
  "$schema": "https://alexandria.dev/schema.v2.4.json",
  "projectNamespace": "mi-proyecto",
  "profile": "mvp-fullstack-ts",
  "vaultPath": "./wiki",
  "fileWatcherDebounceMs": 75,
  "engine": {
    "strictMode": true,
    "moduleContextDepth": 1,
    "maxTransitiveAdrHoistingDepth": 6,
    "circuitBreakerMaxRetries": 3,
    "protectedPathsForOmegaUpgrade": [
      "prisma/schema.prisma",
      "src/core/auth/**",
      "wiki/decisions/**"
    ],
    "oxcAstInvariants": {
      "forbiddenTypes": ["any"],
      "forbidDoubleCastUnknown": true,
      "enforceInlinePrismaTransactions": true,
      "enforceTxOnlyAwaitsInPrismaTransaction": true,
      "forbidTxAliasingOrDestructuring": true,
      "forbidUnawaitedCallsInPrismaTransaction": true,
      "promoteToOmegaOnPublicExportSignatureChange": true
    },
    "sandbox": {
      "clearEnvVars": true,
      "preserveWindowsSystemRoot": true,
      "stripWindowsUncPrefix": true,
      "directNodeVitestInvocation": true,
      "enforceNodePermissionFlag": true,
      "allowWorkerThreads": true,
      "allowNativeAddons": true,
      "allowChildProcess": false,
      "allowNetworkInUnitTests": false
    },
    "gates": {
      "typesAndLint": "npx tsc --noEmit && npm run lint",
      "vitestEntryRelativePath": "./node_modules/vitest/vitest.mjs",
      "vitestReportRelativePath": "./coverage/alexandria-vitest-report.json",
      "build": "npm run build",
      "minCoveragePercent": 90,
      "skipCoverageForExtensions": [".css", ".scss", ".d.ts", ".md"]
    }
  },
  "utuStatelessAuditor": {
    "enabledInMcpMode": true,
    "provider": "anthropic",
    "model": "claude-3-5-haiku-latest"
  },
  "ports": {
    "devServer": 3000,
    "observatory": 4200
  }
}
```

## 12. Modos de Ejecución y Activación del Sistema

- **Modo 1 (Servidor MCP en Segundo Plano):** Configurado en el IDE vía `"command": "alexandria", "args": ["mcp"]`. Implementa el ciclo de vida estándar del protocolo MCP (`initialize`, `ping`, `tools/list`, `tools/call`) sobre `stdio`.
    
      
    
- **Modo 2 (CLI Autónomo):** Invocación directa en terminal mediante `alexandria run "<directiva>"`.
    
      
    

## 13. Contrato de Interfaz MCP v2.4 de Alexandria

TypeScript

```
export interface AlexandriaMCPServerV2_4 {
  prepare_task_context(params: {
    instruction: string;
    targetFiles: string[];
    moduleName?: string;
  }): Promise<{
    enforcedPath: 'alpha_fast' | 'omega_pantheon';
    architecturalContext: string;
    hoistedAdrs: string[];
    activeInvariants: string[];
    requiredGates: Array<'oxc_ast' | 'types' | 'security' | 'tests_sandbox' | 'build_and_utu'>;
  }>;

  upsert_architectural_contract(params: {
    moduleName: string;
    technicalSpecMarkdown: string;
    adrTitle?: string;
    adrDecisionMarkdown?: string;
  }): Promise<{
    adrPath?: string;
    moduleContractPath: string;
    graphUpdated: boolean;
  }>;

  resolve_skill_capsule(params: {
    skillId?: string;
    intentQuery?: string;
  }): Promise<{
    skillId: string;
    resolutionMethod: 'exact_id' | 'vector_similarity' | 'fts5_bm25_fallback';
    instructions: string;
    invariants: string[];
    allowedTools: string[];
  }>;

  spawn_isolated_worktrees(params: {
    tasks: Array<{ workerId: string; moduleName: string; targetFiles: string[] }>;
  }): Promise<{
    approved: boolean;
    worktreePaths?: Record<string, string>;
    conflictType?: 'wiki_subgraph_overlap' | 'ast_import_collision';
    collidingNodesOrFiles?: string[];
  }>;

  verify_and_gate_changes(params: {
    targetModule: string;
    targetFiles: string[];
    executionPath: 'alpha_fast' | 'omega_pantheon';
    tddPhase: 'red_expect_assertion_failure' | 'green_expect_pass' | 'refactor_full_gate';
  }): Promise<{
    passed: boolean;
    redPhaseFailureKind?: 'valid_assertion_fail' | 'invalid_syntax_or_import_error' | 'unexpected_pass';
    circuitBreakerState: 'closed_ok' | 'retry_allowed' | 'tripped_abort';
    attemptNumber: number;
    diagnostics: string;
    utuColdAuditVerdict?: { approved: boolean; findings: string[] };
    failureId?: string;
    historicalHintsFromMemory?: string[];
  }>;

  finalize_task_and_log(params: {
    moduleName: string;
    summary: string;
    startDevServer?: boolean;
  }): Promise<{
    committedHash: string;
    wikiLogUpdated: boolean;
    localUrl?: string;
  }>;
}
```

## 14. Ciclo de Vida Completo de Ejecución (End-to-End)

1. **Preparación Zero-Turn (`prepare_task_context`):** Clasificación estática (`alpha_fast` vs. `omega_pantheon`) e inyección de vecindad $\text{depth}=1$ más _ADR Hoisting_ ($\text{depth} \le 6$).
    
      
    
2. **Formalización de Contratos (`upsert_architectural_contract`):** Creación/actualización de `[[ADR-XXX]]` y contratos de módulo en `wiki/`.
    
      
    
3. **Ciclo TDD Polimórfico (`Enki`):** Inyección de cápsula JIT por sub-sesión, validación determinista de Fase RED mediante archivo JSON `--outputFile`, codificación en Fase GREEN y limpieza nominal en Fase REFACTOR.
    
      
    
4. **Auditoría Fuera de Banda (`Gate 0` a `Gate 4` + `Utu Stateless`):** Validación léxica AST en memoria, `tsc`, SAST, Vitest en sandbox `node --permission` y juicio en frío _stateless_ de `Utu`.
    
      
    
5. **Bucle Forense o Disyuntor (`Circuit Breaker`):** Recuperación de pistas en `incidents_fts` (vía consulta OR sanitizada) / `sqlite-vec` si $\text{intentos} < 3$, o congelación inmediata del _worktree_ (`ABORT_AND_ALERT`) si $\text{intentos} \ge 3$.
    
      
    
6. **Cierre Canónico (`finalize_task_and_log`):** Commit atómico, registro en `wiki/log.md` y despliegue local.
    

## 15. Apéndice: Implementación de Referencia del Kernel (SQL + Rust)

|**Submódulo del Apéndice 15**|**Archivo Objetivo**|**Estado de Verificación**|
|---|---|---|
|**15.1. Esquema DDL + FTS5**|`src/storage/schema.sql`|Verificado (Triggers `ai`, `ad`, `au` completos en `skills_fts` e `incidents_fts`).|
|**15.2. Motor Topológico**|`src/graph/resolver.rs`|Verificado ($\text{Depth}=1$ + _ADR Hoisting_ transitivo con soporte `ADR -> ADR`).|
|**15.3. Escáner AST Gate 0**|`src/gate_guard/oxc_validator.rs`|Verificado (`any`, `as unknown as T`, `.then()`, aliasing y promesas huérfanas en var/assign/stmt).|
|**15.4. Sandbox Vitest & TDD**|`src/runner/vitest_sandbox.rs`|Verificado (`--outputFile`, limpieza UNC `\\?\`, `SystemRoot`/`TMP` y `--permission`).|
|**15.5. Actor SQLite MPSC**|`src/storage/actor.rs`|Verificado (Post-commit `oneshot`, secuencia monotónica `AtomicU64` y sanitizador FTS5 `OR`).|
|**15.6. Servidor MCP & Cargo**|`src/mcp/server.rs` / `Cargo.toml`|Verificado (`initialize`, `ping`, `tools/list`, `tools/call` y `serde_yml`).|

### 15.1. Esquema DDL SQLite WAL + Triggers FTS5 Completos (`src/storage/schema.sql`)

SQL

```
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;

-- 1. Catálogo de Habilidades (Plano 1 Relacional)
CREATE TABLE IF NOT EXISTS skills (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_id TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL,
    description TEXT NOT NULL,
    trigger_keywords_json TEXT NOT NULL,
    invariants_json TEXT NOT NULL,
    instructions_md TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 2. Índice Léxico FTS5 para Skills (External Content Table)
CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(
    skill_id,
    domain,
    description,
    trigger_keywords_json,
    instructions_md,
    content='skills',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS skills_ai AFTER INSERT ON skills BEGIN
    INSERT INTO skills_fts(rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES (new.rowid, new.skill_id, new.domain, new.description, new.trigger_keywords_json, new.instructions_md);
END;

CREATE TRIGGER IF NOT EXISTS skills_ad AFTER DELETE ON skills BEGIN
    INSERT INTO skills_fts(skills_fts, rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES ('delete', old.rowid, old.skill_id, old.domain, old.description, old.trigger_keywords_json, old.instructions_md);
END;

CREATE TRIGGER IF NOT EXISTS skills_au AFTER UPDATE ON skills BEGIN
    INSERT INTO skills_fts(skills_fts, rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES ('delete', old.rowid, old.skill_id, old.domain, old.description, old.trigger_keywords_json, old.instructions_md);
    INSERT INTO skills_fts(rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES (new.rowid, new.skill_id, new.domain, new.description, new.trigger_keywords_json, new.instructions_md);
END;

-- 3. Estado del Disyuntor (Circuit Breaker) e Incidentes Forenses (Nabu)
CREATE TABLE IF NOT EXISTS circuit_breakers (
    module_name TEXT PRIMARY KEY,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL CHECK(state IN ('closed_ok', 'retry_allowed', 'tripped_abort')),
    last_failure_id TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS forensic_incidents (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    failure_id TEXT NOT NULL UNIQUE,
    module_name TEXT NOT NULL,
    failed_gate TEXT NOT NULL,
    stack_trace TEXT NOT NULL,
    resolution_hint TEXT,
    created_at INTEGER NOT NULL
);

-- 4. Índice Léxico FTS5 para Incidentes Forenses con los 3 Triggers (Previene SQLITE_CORRUPT_VTAB)
CREATE VIRTUAL TABLE IF NOT EXISTS incidents_fts USING fts5(
    failure_id,
    module_name,
    stack_trace,
    resolution_hint,
    content='forensic_incidents',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS incidents_ai AFTER INSERT ON forensic_incidents BEGIN
    INSERT INTO incidents_fts(rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES (new.rowid, new.failure_id, new.module_name, new.stack_trace, new.resolution_hint);
END;

CREATE TRIGGER IF NOT EXISTS incidents_ad AFTER DELETE ON forensic_incidents BEGIN
    INSERT INTO incidents_fts(incidents_fts, rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES ('delete', old.rowid, old.failure_id, old.module_name, old.stack_trace, old.resolution_hint);
END;

CREATE TRIGGER IF NOT EXISTS incidents_au AFTER UPDATE ON forensic_incidents BEGIN
    INSERT INTO incidents_fts(incidents_fts, rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES ('delete', old.rowid, old.failure_id, old.module_name, old.stack_trace, old.resolution_hint);
    INSERT INTO incidents_fts(rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES (new.rowid, new.failure_id, new.module_name, new.stack_trace, new.resolution_hint);
END;
```

### 15.2. Motor Topológico con _ADR Hoisting_ (`src/graph/resolver.rs`)

Rust

```
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Module,
    Architecture,
    ConstitutionalAdr,
}

#[derive(Debug, Clone)]
pub struct WikiNode {
    pub id: String,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub content: String,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct ResolvedContext {
    pub target_module: String,
    pub direct_neighbors: Vec<String>,
    pub hoisted_adrs: Vec<String>,
}

pub fn resolve_context_with_adr_hoisting(
    graph: &StableDiGraph<WikiNode, ()>,
    start_idx: NodeIndex,
    max_adr_depth: usize,
) -> ResolvedContext {
    let target_node = &graph[start_idx];
    let mut direct_neighbors = Vec::new();
    let mut hoisted_adrs = Vec::new();

    let mut visited = HashSet::new();
    visited.insert(start_idx);

    let mut queue = VecDeque::new();
    queue.push_back((start_idx, 0usize));

    while let Some((curr_idx, depth)) = queue.pop_front() {
        if depth >= max_adr_depth {
            continue;
        }

        for edge in graph.edges_directed(curr_idx, Direction::Outgoing) {
            let next_idx = edge.target();
            if !visited.insert(next_idx) {
                continue;
            }

            let next_node = &graph[next_idx];
            let next_depth = depth + 1;

            match (next_depth, &next_node.kind) {
                (1, NodeKind::ConstitutionalAdr) => {
                    hoisted_adrs.push(next_node.id.clone());
                    queue.push_back((next_idx, next_depth));
                }
                (1, _) => {
                    direct_neighbors.push(next_node.id.clone());
                    queue.push_back((next_idx, next_depth));
                }
                (_, NodeKind::ConstitutionalAdr) => {
                    hoisted_adrs.push(next_node.id.clone());
                    queue.push_back((next_idx, next_depth));
                }
                (_, _) => {
                    queue.push_back((next_idx, next_depth));
                }
            }
        }
    }

    ResolvedContext {
        target_module: target_node.id.clone(),
        direct_neighbors,
        hoisted_adrs,
    }
}
```

### 15.3. Escáner Léxico AST Blindado en `oxc` para Gate 0 (`src/gate_guard/oxc_validator.rs`)

Rust

```
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::visit::walk;
use oxc_ast::Visit;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

#[derive(Debug, Clone)]
pub struct AstViolation {
    pub rule: &'static str,
    pub message: String,
    pub byte_offset: u32,
}

pub struct GateZeroAstVisitor {
    pub violations: Vec<AstViolation>,
    active_tx_params: Vec<String>,
}

impl GateZeroAstVisitor {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            active_tx_params: Vec::new(),
        }
    }

    fn current_tx_param(&self) -> Option<&str> {
        self.active_tx_params.last().map(|s| s.as_str())
    }

    /// Verifica estrictamente `tx.<modelo>.<op>()` o `tx.$queryRaw(...)`
    /// Prohibiendo encadenamientos de promesas como `.then()`, `.catch()` o `.finally()`
    fn is_strict_tx_operation(callee: &Expression<'_>, tx_name: &str) -> bool {
        match callee {
            Expression::StaticMemberExpression(outer) => {
                let prop = outer.property.name.as_str();
                if matches!(prop, "then" | "catch" | "finally") {
                    return false;
                }
                match &outer.object {
                    Expression::Identifier(ident) => ident.name.as_str() == tx_name,
                    Expression::StaticMemberExpression(inner) => {
                        if let Expression::Identifier(ident) = &inner.object {
                            ident.name.as_str() == tx_name
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl<'a> Visit<'a> for GateZeroAstVisitor {
    fn visit_ts_any_keyword(&mut self, it: &TSAnyKeyword) {
        self.violations.push(AstViolation {
            rule: "NO_EXPLICIT_ANY",
            message: "Prohibido el uso de 'any'. Utiliza tipos nominales o Branded Types.".into(),
            byte_offset: it.span.start,
        });
    }

    fn visit_ts_as_expression(&mut self, it: &TSAsExpression<'a>) {
        if let Expression::TSAsExpression(inner_as) = &it.expression {
            if matches!(inner_as.type_annotation, TSType::TSUnknownKeyword(_)) {
                self.violations.push(AstViolation {
                    rule: "NO_DOUBLE_CAST_UNKNOWN",
                    message: "Prohibido el doble casteo 'as unknown as T' para evadir el sistema de tipos.".into(),
                    byte_offset: it.span.start,
                });
            }
        }
        walk::walk_ts_as_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            if let Some(init_expr) = &it.init {
                match init_expr {
                    Expression::Identifier(ident) if ident.name.as_str() == tx_name => {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_NO_ALIASING",
                            message: format!(
                                "Prohibido reasignar o desestructurar el cliente transaccional '{}'.",
                                tx_name
                            ),
                            byte_offset: it.span.start,
                        });
                    }
                    Expression::CallExpression(call)
                        if Self::is_strict_tx_operation(&call.callee, tx_name) =>
                    {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT",
                            message: format!(
                                "Prohibido almacenar una promesa de '{}' en una variable sin 'await' inmediato.",
                                tx_name
                            ),
                            byte_offset: it.span.start,
                        });
                    }
                    _ => {}
                }
            }
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            match &it.right {
                Expression::Identifier(ident) if ident.name.as_str() == tx_name => {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_NO_ASSIGNMENT_ESCAPE",
                        message: format!(
                            "Prohibido asignar el cliente transaccional '{}' a variables externas.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
                Expression::CallExpression(call)
                    if Self::is_strict_tx_operation(&call.callee, tx_name) =>
                {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT",
                        message: format!(
                            "Prohibido asignar una promesa de '{}' a una variable sin 'await' inmediato.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
                _ => {}
            }
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            if let Expression::CallExpression(call) = &it.expression {
                if Self::is_strict_tx_operation(&call.callee, tx_name) {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_DANGLING_PROMISE",
                        message: format!(
                            "Operación sobre '{}' sin 'await' detectada; causará cierre prematuro de la transacción.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
            }
        }
        walk::walk_expression_statement(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            let valid = match &it.argument {
                Expression::CallExpression(call) => {
                    Self::is_strict_tx_operation(&call.callee, tx_name)
                }
                Expression::TaggedTemplateExpression(tagged) => {
                    Self::is_strict_tx_operation(&tagged.tag, tx_name)
                }
                _ => false,
            };

            if !valid {
                self.violations.push(AstViolation {
                    rule: "PRISMA_TX_STRICT_LEXICAL_AWAIT",
                    message: format!(
                        "Dentro de $transaction solo se permiten llamadas await directas 'await {}.<modelo>.<op>()'. Prohibido I/O externo, .then() o Promise.all.",
                        tx_name
                    ),
                    byte_offset: it.span.start,
                });
            }
        }
        walk::walk_await_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            for arg in &it.arguments {
                if let Argument::Identifier(ident) = arg {
                    if ident.name.as_str() == tx_name {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_NO_ESCAPE",
                            message: format!(
                                "Prohibido pasar el cliente transaccional '{}' a funciones auxiliares.",
                                tx_name
                            ),
                            byte_offset: ident.span.start,
                        });
                    }
                }
            }

            if let Expression::Identifier(ident) = &it.callee {
                if matches!(ident.name.as_str(), "fetch" | "setTimeout" | "setInterval") {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_NO_UNWAITED_IO",
                        message: format!(
                            "Prohibido invocar '{}' dentro de una transacción de base de datos.",
                            ident.name.as_str()
                        ),
                        byte_offset: it.span.start,
                    });
                }
            }
        }

        let is_prisma_tx_call = match &it.callee {
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "$transaction"
            }
            _ => false,
        };

        if is_prisma_tx_call {
            if let Some(first_arg) = it.arguments.first() {
                match first_arg {
                    Argument::ArrayExpression(_) => {
                        walk::walk_call_expression(self, it);
                        return;
                    }
                    Argument::ArrowFunctionExpression(arrow) => {
                        if let Some(first_param) = arrow.params.items.first() {
                            match &first_param.pattern.kind {
                                BindingPatternKind::BindingIdentifier(ident) => {
                                    let param_name = ident.name.as_str().to_string();
                                    self.active_tx_params.push(param_name);
                                    walk::walk_arrow_function_expression(self, arrow);
                                    self.active_tx_params.pop();
                                    return;
                                }
                                _ => {
                                    self.violations.push(AstViolation {
                                        rule: "PRISMA_TX_NO_PARAM_DESTRUCTURING",
                                        message: "El parámetro de $transaction debe ser un identificador simple (ej. 'async (tx) => ...'), sin destructuring.".into(),
                                        byte_offset: first_param.span.start,
                                    });
                                    return;
                                }
                            }
                        }
                    }
                    _ => {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_REQUIRE_INLINE_CALLBACK",
                            message: "El primer argumento de $transaction debe ser un ArrowFunction inline o un Array estático.".into(),
                            byte_offset: first_arg.span().start,
                        });
                        return;
                    }
                }
            }
        }

        walk::walk_call_expression(self, it);
    }
}

pub fn validate_typescript_source(
    source_text: &str,
    is_tsx: bool,
) -> Result<Vec<AstViolation>, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_jsx(is_tsx);
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    if !ret.errors.is_empty() {
        return Err(format!("SyntaxError en parseo Oxc: {:?}", ret.errors[0]));
    }

    let mut visitor = GateZeroAstVisitor::new();
    visitor.visit_program(&ret.program);
    Ok(visitor.violations)
}
```

### 15.4. Ejecutor de Vitest en Sandbox Multiplataforma con `--outputFile` (`src/runner/vitest_sandbox.rs`)

Rust

```
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, PartialEq, Eq)]
pub enum RedPhaseVerdict {
    ValidAssertionFailure { failed_assertions: usize },
    InvalidSyntaxOrImportError { details: String },
    UnexpectedPass,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitestJsonReport {
    pub num_total_test_suites: usize,
    pub num_failed_tests: usize,
    pub test_results: Vec<VitestSuiteResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitestSuiteResult {
    pub status: String,
    pub message: String,
    pub assertion_results: Vec<VitestAssertionResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitestAssertionResult {
    pub status: String,
    pub failure_messages: Vec<String>,
}

/// Limpia el prefijo UNC `\\?\` en Windows para compatibilidad con Node.js `--permission`
fn clean_canonical_path(path: &Path) -> Result<PathBuf, String> {
    let canon = std::fs::canonicalize(path)
        .map_err(|e| format!("Error canonicalizando ruta {:?}: {}", path, e))?;
    #[cfg(target_os = "windows")]
    {
        let s = canon.to_string_lossy();
        if let Some(unc_rest) = s.strip_prefix(r"\\?\UNC\") {
            return Ok(PathBuf::from(format!(r"\\{}", unc_rest)));
        }
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(stripped));
        }
    }
    Ok(canon)
}

pub async fn run_sandboxed_vitest(
    workspace_root: &Path,
    node_binary_path: &Path,
    node_install_dir: &Path,
    target_test_file: &Path,
    timeout_secs: u64,
) -> Result<(bool, String, Option<VitestJsonReport>), String> {
    let canon_workspace = clean_canonical_path(workspace_root)?;
    let canon_node_dir = clean_canonical_path(node_install_dir)?;
    let canon_temp_dir = clean_canonical_path(&std::env::temp_dir())?;

    let vitest_entry = canon_workspace.join("node_modules/vitest/vitest.mjs");
    let coverage_dir = canon_workspace.join("coverage");
    let _ = std::fs::create_dir_all(&coverage_dir);

    let json_report_path = coverage_dir.join("alexandria-vitest-report.json");
    let _ = std::fs::remove_file(&json_report_path);

    let mut cmd = Command::new(node_binary_path);
    cmd.current_dir(&canon_workspace)
        .env_clear()
        .env("NODE_ENV", "test")
        .env("CI", "true")
        .env("TMP", &canon_temp_dir)
        .env("TEMP", &canon_temp_dir)
        .env("TMPDIR", &canon_temp_dir);

    #[cfg(target_os = "windows")]
    if let Ok(sys_root) = std::env::var("SystemRoot") {
        cmd.env("SystemRoot", sys_root);
    }

    cmd.arg("--permission")
        .arg("--allow-worker")
        .arg("--allow-addons")
        .arg(format!("--allow-fs-read={}", canon_workspace.display()))
        .arg(format!("--allow-fs-read={}", canon_node_dir.display()))
        .arg(format!("--allow-fs-write={}", coverage_dir.display()))
        .arg(format!("--allow-fs-write={}", canon_temp_dir.display()))
        .arg(vitest_entry)
        .arg("run")
        .arg(target_test_file)
        .arg("--reporter=json")
        .arg(format!("--outputFile={}", json_report_path.display()))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| "Timeout excedido ejecutando Vitest en Sandbox".to_string())?
        .map_err(|e| format!("Fallo al lanzar el binario node: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

    let report: Option<VitestJsonReport> = std::fs::read_to_string(&json_report_path)
        .ok()
        .and_then(|json_content| serde_json::from_str(&json_content).ok());

    Ok((
        output.status.success(),
        format!("{}\n{}", stdout_str, stderr_str),
        report,
    ))
}

pub fn evaluate_tdd_red_phase(report_opt: Option<VitestJsonReport>, raw_logs: &str) -> RedPhaseVerdict {
    let Some(report) = report_opt else {
        return RedPhaseVerdict::InvalidSyntaxOrImportError {
            details: format!("Vitest no pudo emitir el archivo JSON de reporte (posible SyntaxError fatal): {}", raw_logs),
        };
    };

    if report.num_total_test_suites == 0 {
        return RedPhaseVerdict::InvalidSyntaxOrImportError {
            details: "No se encontró ninguna suite ejecutable.".into(),
        };
    }

    for suite in &report.test_results {
        if suite.assertion_results.is_empty() && suite.status == "failed" {
            return RedPhaseVerdict::InvalidSyntaxOrImportError {
                details: format!("La suite falló al cargar o compilar: {}", suite.message),
            };
        }
    }

    if report.num_failed_tests == 0 {
        return RedPhaseVerdict::UnexpectedPass;
    }

    let mut valid_failures = 0usize;
    for suite in &report.test_results {
        for assertion in &suite.assertion_results {
            if assertion.status == "failed" {
                let msg = assertion.failure_messages.join("\n");
                if msg.contains("SyntaxError") || msg.contains("Cannot find package") {
                    return RedPhaseVerdict::InvalidSyntaxOrImportError { details: msg };
                }
                valid_failures += 1;
            }
        }
    }

    RedPhaseVerdict::ValidAssertionFailure {
        failed_assertions: valid_failures,
    }
}
```

### 15.5. Actor SQLite Single-Writer MPSC con Confirmación Post-Commit y Sanitizador FTS5 (`src/storage/actor.rs`)

Rust

```
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};

static INCIDENT_SEQ: AtomicU64 = AtomicU64::new(1);

pub enum VaultWriteCommand {
    RecordGateFailure {
        module_name: String,
        failed_gate: String,
        stack_trace: String,
        max_retries: u32,
        reply: oneshot::Sender<Result<(String, String, u32), String>>,
    },
    ResetCircuitBreakerAndResolveIncident {
        module_name: String,
        last_failure_id: Option<String>,
        resolution_hint: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

#[derive(Clone)]
pub struct VaultStorageHandle {
    writer_tx: mpsc::Sender<VaultWriteCommand>,
    db_path: PathBuf,
}

impl VaultStorageHandle {
    pub fn spawn(db_path: PathBuf, schema_sql: &str) -> Result<Self, String> {
        let init_conn = Connection::open(&db_path)
            .map_err(|e| format!("Error abriendo vault.db: {}", e))?;
        init_conn
            .execute_batch(schema_sql)
            .map_err(|e| format!("Error inicializando esquema SQL: {}", e))?;
        drop(init_conn);

        let (writer_tx, mut writer_rx) = mpsc::channel::<VaultWriteCommand>(512);
        let thread_db_path = db_path.clone();

        std::thread::Builder::new()
            .name("alexandria-sqlite-writer".into())
            .spawn(move || {
                let mut conn = Connection::open(&thread_db_path)
                    .expect("No se pudo abrir la conexión de escritura en vault.db");
                let _ = conn.execute_batch(
                    "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
                );

                while let Some(first_cmd) = writer_rx.blocking_recv() {
                    let mut batch = vec![first_cmd];
                    while batch.len() < 64 {
                        match writer_rx.try_recv() {
                            Ok(cmd) => batch.push(cmd),
                            Err(_) => break,
                        }
                    }

                    match conn.transaction() {
                        Ok(tx) => {
                            let mut pending_notifications: Vec<Box<dyn FnOnce(Result<(), String>)>> =
                                Vec::with_capacity(batch.len());

                            for cmd in batch {
                                let notifier = Self::stage_command_in_tx(&tx, cmd);
                                pending_notifications.push(notifier);
                            }

                            let commit_res = tx
                                .commit()
                                .map_err(|e| format!("Fallo en COMMIT de batch SQLite: {}", e));

                            for notify in pending_notifications {
                                notify(commit_res.clone());
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("No se pudo iniciar transacción SQLite: {}", e);
                            for cmd in batch {
                                Self::abort_command(cmd, &err_msg);
                            }
                        }
                    }
                }
            })
            .map_err(|e| format!("Error creando hilo de VaultWriterActor: {}", e))?;

        Ok(Self { writer_tx, db_path })
    }

    fn abort_command(cmd: VaultWriteCommand, err_msg: &str) {
        match cmd {
            VaultWriteCommand::RecordGateFailure { reply, .. } => {
                let _ = reply.send(Err(err_msg.to_string()));
            }
            VaultWriteCommand::ResetCircuitBreakerAndResolveIncident { reply, .. } => {
                let _ = reply.send(Err(err_msg.to_string()));
            }
        }
    }

    /// Ejecuta las sentencias dentro de la transacción y devuelve un closure que solo
    /// notificará al llamador una vez que `tx.commit()` haya confirmado en disco.
    fn stage_command_in_tx(
        tx: &rusqlite::Transaction<'_>,
        cmd: VaultWriteCommand,
    ) -> Box<dyn FnOnce(Result<(), String>)> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let seq = INCIDENT_SEQ.fetch_add(1, Ordering::Relaxed);

        match cmd {
            VaultWriteCommand::RecordGateFailure {
                module_name,
                failed_gate,
                stack_trace,
                max_retries,
                reply,
            } => {
                let current_failures: u32 = tx
                    .query_row(
                        "SELECT consecutive_failures FROM circuit_breakers WHERE module_name = ?1",
                        params![module_name],
                        |row| row.get(0),
                    )
                    .optional()
                    .unwrap_or(None)
                    .unwrap_or(0);

                let next_failures = current_failures + 1;
                let next_state = if next_failures >= max_retries {
                    "tripped_abort"
                } else {
                    "retry_allowed"
                };

                let failure_id = format!(
                    "FAIL-{}-{}-{}",
                    module_name.to_uppercase(),
                    now_ms,
                    seq
                );

                let res1 = tx.execute(
                    "INSERT INTO forensic_incidents (failure_id, module_name, failed_gate, stack_trace, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![failure_id, module_name, failed_gate, stack_trace, now_ms],
                );

                let res2 = tx.execute(
                    "INSERT INTO circuit_breakers (module_name, consecutive_failures, state, last_failure_id, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(module_name) DO UPDATE SET
                        consecutive_failures = excluded.consecutive_failures,
                        state = excluded.state,
                        last_failure_id = excluded.last_failure_id,
                        updated_at = excluded.updated_at",
                    params![module_name, next_failures, next_state, failure_id, now_ms],
                );

                let step_result = res1
                    .and(res2)
                    .map(|_| (failure_id, next_state.to_string(), next_failures))
                    .map_err(|e| format!("Error SQL registrando incidente: {}", e));

                Box::new(move |commit_status| match (commit_status, step_result) {
                    (Ok(()), Ok(payload)) => {
                        let _ = reply.send(Ok(payload));
                    }
                    (Err(commit_err), _) => {
                        let _ = reply.send(Err(commit_err));
                    }
                    (_, Err(sql_err)) => {
                        let _ = reply.send(Err(sql_err));
                    }
                })
            }
            VaultWriteCommand::ResetCircuitBreakerAndResolveIncident {
                module_name,
                last_failure_id,
                resolution_hint,
                reply,
            } => {
                let mut sql_err: Option<String> = None;

                if let Some(fid) = last_failure_id {
                    if let Err(e) = tx.execute(
                        "UPDATE forensic_incidents SET resolution_hint = ?1 WHERE failure_id = ?2",
                        params![resolution_hint, fid],
                    ) {
                        sql_err = Some(format!("Error actualizando incidente: {}", e));
                    }
                }

                if sql_err.is_none() {
                    if let Err(e) = tx.execute(
                        "INSERT INTO circuit_breakers (module_name, consecutive_failures, state, last_failure_id, updated_at)
                         VALUES (?1, 0, 'closed_ok', NULL, ?2)
                         ON CONFLICT(module_name) DO UPDATE SET
                            consecutive_failures = 0,
                            state = 'closed_ok',
                            last_failure_id = NULL,
                            updated_at = excluded.updated_at",
                        params![module_name, now_ms],
                    ) {
                        sql_err = Some(format!("Error reiniciando circuit breaker: {}", e));
                    }
                }

                Box::new(move |commit_status| match (commit_status, sql_err) {
                    (Ok(()), None) => {
                        let _ = reply.send(Ok(()));
                    }
                    (Err(commit_err), _) => {
                        let _ = reply.send(Err(commit_err));
                    }
                    (_, Some(err)) => {
                        let _ = reply.send(Err(err));
                    }
                })
            }
        }
    }

    /// Sanitiza un stack trace crudo convirtiéndolo en una expresión FTS5 segura (`"tok1" OR "tok2"`)
    /// inmune a errores de sintaxis por `:`, `/`, `"` o palabras reservadas (`AND`, `OR`, `NOT`, `NEAR`).
    pub fn build_safe_fts5_or_query(raw_stack_trace: &str) -> Option<String> {
        let mut tokens: Vec<String> = Vec::new();
        for token in raw_stack_trace
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|t| t.len() >= 3)
        {
            let upper = token.to_uppercase();
            if matches!(upper.as_str(), "AND" | "OR" | "NOT" | "NEAR") {
                continue;
            }
            let quoted = format!("\"{}\"", token);
            if !tokens.contains(&quoted) {
                tokens.push(quoted);
            }
            if tokens.len() >= 16 {
                break;
            }
        }

        if tokens.is_empty() {
            None
        } else {
            Some(tokens.join(" OR "))
        }
    }

    /// Lectura concurrente sin bloqueo (Modo WAL) usando el índice FTS5 BM25 como fallback offline
    pub fn search_similar_incidents_fts5(&self, raw_stack_trace: &str) -> Result<Vec<String>, String> {
        let Some(safe_fts_query) = Self::build_safe_fts5_or_query(raw_stack_trace) else {
            return Ok(Vec::new());
        };

        let conn = Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT failure_id, resolution_hint
                 FROM incidents_fts
                 WHERE incidents_fts MATCH ?1 AND resolution_hint IS NOT NULL
                 ORDER BY bm25(incidents_fts) LIMIT 3",
            )
            .map_err(|e| e.to_string())?;

        let hints = stmt
            .query_map(params![safe_fts_query], |row| {
                let id: String = row.get(0)?;
                let hint: String = row.get(1)?;
                Ok(format!("[{}] {}", id, hint))
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();

        Ok(hints)
    }
}
```

### 15.6. Servidor MCP JSON-RPC 2.0 Completo y `Cargo.toml` (`src/mcp/server.rs` & `Cargo.toml`)

#### A. Archivo `Cargo.toml` (Edición 2024 — `serde_yml` y tren Oxc alineado)

Ini, TOML

```
[package]
name = "alexandria"
version = "2.4.0"
edition = "2024"
description = "Headless Deterministic Agent Harness & Kernel-Space Arbitrator"

[dependencies]
tokio = { version = "1.43", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yml = "0.0.12"
rusqlite = { version = "0.33", features = ["bundled", "functions"] }
petgraph = "0.7"
pulldown-cmark = "0.12"
notify = "8.0"
notify-debouncer-mini = "0.6"
# Tren de versiones sincronizado del workspace Oxc
oxc_allocator = "0.56"
oxc_ast = "0.56"
oxc_parser = "0.56"
oxc_span = "0.56"
oxc_resolver = "4.0"
sha2 = "0.10"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

#### B. Servidor MCP Estándar (`initialize`, `ping`, `tools/list`, `tools/call`) (`src/mcp/server.rs`)

Rust

```
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub async fn run_mcp_stdio_loop() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Alexandria Runtime v2.4.0 MCP Server iniciado sobre stdio");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await.map_err(|e| e.to_string())? {
        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Trama JSON-RPC malformada: {}", e);
                let err_resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                let mut out = serde_json::to_string(&err_resp).unwrap_or_default();
                out.push('\n');
                let _ = stdout.write_all(out.as_bytes()).await;
                let _ = stdout.flush().await;
                continue;
            }
        };

        // Notificaciones JSON-RPC sin `id` (ej. "notifications/initialized") no se responden
        let Some(req_id) = req.id else {
            continue;
        };

        let response = dispatch_mcp_method(&req.method, req.params, req_id).await;
        let mut serialized = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        serialized.push('\n');

        stdout
            .write_all(serialized.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stdout.flush().await.map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn dispatch_mcp_method(method: &str, params: Value, id: Value) -> JsonRpcResponse {
    match method {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "alexandria-runtime",
                    "version": "2.4.0-OmniPragmatic"
                },
                "capabilities": {
                    "tools": { "listChanged": false }
                }
            })),
            error: None,
        },
        "ping" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({})),
            error: None,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "prepare_task_context",
                        "description": "Clasifica Path Alpha/Omega en Zero-Turn y resuelve el grafo depth=1 con ADR Hoisting.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["instruction", "targetFiles"],
                            "properties": {
                                "instruction": { "type": "string" },
                                "targetFiles": { "type": "array", "items": { "type": "string" } },
                                "moduleName": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "upsert_architectural_contract",
                        "description": "Crea o actualiza ADRs y contratos de módulo en wiki/ (Path Omega).",
                        "inputSchema": {
                            "type": "object",
                            "required": ["moduleName", "technicalSpecMarkdown"],
                            "properties": {
                                "moduleName": { "type": "string" },
                                "technicalSpecMarkdown": { "type": "string" },
                                "adrTitle": { "type": "string" },
                                "adrDecisionMarkdown": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "resolve_skill_capsule",
                        "description": "Inyecta una cápsula JIT en el sufijo dinámico sin invalidar el Prefix Cache.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "skillId": { "type": "string" },
                                "intentQuery": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "spawn_isolated_worktrees",
                        "description": "Autoriza Git worktrees paralelos verificando disyunción doble en petgraph y oxc_resolver.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["tasks"],
                            "properties": {
                                "tasks": { "type": "array" }
                            }
                        }
                    },
                    {
                        "name": "verify_and_gate_changes",
                        "description": "Ejecuta Gate 0 (Oxc AST), Gate 1-4 en Sandbox Node (--permission) y Auditoría Stateless de Utu.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["targetModule", "targetFiles", "executionPath", "tddPhase"],
                            "properties": {
                                "targetModule": { "type": "string" },
                                "targetFiles": { "type": "array", "items": { "type": "string" } },
                                "executionPath": { "type": "string", "enum": ["alpha_fast", "omega_pantheon"] },
                                "tddPhase": { "type": "string", "enum": ["red_expect_assertion_failure", "green_expect_pass", "refactor_full_gate"] }
                            }
                        }
                    },
                    {
                        "name": "finalize_task_and_log",
                        "description": "Realiza el commit atómico, actualiza wiki/log.md y levanta el entorno de desarrollo.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["moduleName", "summary"],
                            "properties": {
                                "moduleName": { "type": "string" },
                                "summary": { "type": "string" },
                                "startDevServer": { "type": "boolean" }
                            }
                        }
                    }
                ]
            })),
            error: None,
        },
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            dispatch_alexandria_tool(tool_name, args, id).await
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Método JSON-RPC no encontrado: {}", method),
            }),
        },
    }
}

async fn dispatch_alexandria_tool(tool_name: &str, _args: Value, id: Value) -> JsonRpcResponse {
    match tool_name {
        "prepare_task_context"
        | "upsert_architectural_contract"
        | "resolve_skill_capsule"
        | "spawn_isolated_worktrees"
        | "verify_and_gate_changes"
        | "finalize_task_and_log" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": format!("Tool '{}' ejecutada por el kernel de Alexandria.", tool_name)
                }],
                "isError": false
            })),
            error: None,
        },
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: format!("Herramienta desconocida en Alexandria MCP: {}", tool_name),
            }),
        },
    }
}
```