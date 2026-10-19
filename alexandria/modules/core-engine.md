---
id: core-engine
title: "Contrato de Subsistema: Core Engine, Runtime Rust 2024, Perfil de Compilación y Circuit Breaker"
type: module
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - module
  - rust
  - tokio
  - cli
  - circuit-breaker
  - cargo
---

# Contrato de Subsistema: Core Engine, Runtime Rust 2024, Perfil de Compilación y Circuit Breaker

> **Documento Canónico de Módulo:** `alexandria/modules/core-engine.md`  
> **Archivos de Destino en Kernel:** `Cargo.toml`, `src/main.rs`, `src/config.rs`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]]

---

## 1. Arquitectura de Ejecución y Modelo de Asincronía (Tokio)

El subsistema **Core Engine** compila como un binario nativo estático y monolítico en **Rust (Edición 2024)**. Su arquitectura de hilos desacopla estrictamente tres cargas de trabajo con perfiles de latencia incompatibles:

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                        PROCESO NATIVO: alexandria / alexandria.exe                       │
│                         (Huella de Memoria Residente RSS <= 15 MB)                       │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│  1. REACTOR TOKIO (Multi-Thread Acotado: 2 a 4 Workers Asíncronos)                       │
│     • Loop de lectura/escritura no bloqueante JSON-RPC 2.0 sobre stdin/stdout            │
│     • Consultas de lectura concurrente en RAM (petgraph) y SQLite Read-Only (mmap)       │
│     • Orquestación asíncrona de subprocesos de Quality Gates (tokio::process::Command)   │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│  2. HILO DEDICADO DE OS: "alexandria-sqlite-writer" (std::thread::Builder)               │
│     • Bloqueo síncrono en canal MPSC (writer_rx.blocking_recv())                         │
│     • Drenaje oportunista de lotes (try_recv hasta 64 comandos) en transacción única     │
│     • Cero inanición (starvation) de los workers asíncronos de Tokio por fsync de disco  │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│  3. HILO DEDICADO DE WATCHER: "alexandria-fs-watcher" (notify-debouncer-mini)            │
│     • Monitoreo de eventos de sistema de archivos (inotify / ReadDirectoryChangesW)      │
│     • Ventana de colapso (Debounce) de 75 ms y recarga incremental de nodos SHA-256      │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Justificación del Modelo Híbrido (Tokio Multi-Thread Acotado + OS Thread para SQLite)
- Si se utilizara un runtime `current_thread` puro, cualquier parseo AST de múltiples archivos o compresión de lotes bloquearía el latido (`ping`) del protocolo MCP sobre `stdio`.
- Si se ejecutaran transacciones de escritura SQLite dentro de `tokio::spawn` convencionales, las llamadas de sistema `fsync` bloquearían los hilos del planificador cooperativo de Tokio. Al aislar `rusqlite` de escritura en su propio hilo nativo de OS comunicado por `tokio::sync::mpsc`, el reactor asíncrono mantiene una latencia de despacho inferior a $0.5\text{ ms}$.

---

## 2. Presupuesto de Memoria RAM ($\le 15\text{ MB}$) y Optimización de Binario

Para garantizar que Alexandria Runtime pueda ejecutarse simultáneamente en múltiples terminales e IDEs sin degradar la máquina del desarrollador, se aplican las siguientes invariantes de memoria:

1. **Cero Pesos de ML en Espacio de Direcciones:** El binario jamás carga modelos ONNX/GGUF en memoria residente. La indexación opera vía tablas virtuales FTS5 BM25 o delega a un daemon externo fuera de proceso.
2. **Asignación en Arena Contigua (`oxc_allocator::Allocator`):** Durante la ejecución de **Gate 0** ([[gate-guard-ast]]), los nodos del AST de TypeScript se alojan en un *bump allocator* lineal que se libera en bloque ($O(1)$) al concluir cada archivo, evitando fragmentación en el heap global y eliminando llamadas individuales a `free()`.
3. **Reutilización de Buffers en `stdio`:** El bucle de [[mcp-server]] reutiliza el buffer de lectura de línea (`BufReader::lines`) evitando realocaciones innecesarias en tramas JSON-RPC grandes.
4. **Perfil de Compilación `release` Hiperoptimizado:** Se aplica *Fat Link-Time Optimization* (`lto = "fat"`), unidad de generación de código única (`codegen-units = 1`) para maximizar el *inlining* entre crates (especialmente entre `oxc_parser` y `GateZeroAstVisitor`), `panic = "abort"` para eliminar las tablas de desenrollado de pila (*unwinding tables* DWARF/SEH) y `strip = true`.

---

## 3. Manifiesto Canónico `Cargo.toml` (Edición 2024)

```toml
[package]
name = "alexandria"
version = "2.4.0"
edition = "2024"
description = "Headless Deterministic Agent Harness & Kernel-Space Arbitrator"
license = "MIT"

[dependencies]
tokio = { version = "1.43", features = ["rt-multi-thread", "macros", "process", "io-std", "io-util", "sync", "time", "fs"] }
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
oxc_ast_visit = "0.56"
oxc_parser = "0.56"
oxc_span = "0.56"
oxc_resolver = "4.0"
sha2 = "0.10"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

---

## 4. Configuración Estructural del Proyecto (`.alexandria.json` v2.4)

```json
{
  "$schema": "https://alexandria.dev/schema.v2.4.json",
  "projectNamespace": "alexandria-runtime",
  "profile": "mvp-fullstack-ts",
  "vaultPath": "./alexandria",
  "fileWatcherDebounceMs": 75,
  "engine": {
    "strictMode": true,
    "moduleContextDepth": 1,
    "maxTransitiveAdrHoistingDepth": 6,
    "circuitBreakerMaxRetries": 3,
    "protectedPathsForOmegaUpgrade": [
      "prisma/schema.prisma",
      "src/core/auth/**",
      "alexandria/decisions/**"
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
  }
}
```

---

## 5. Máquina de Estados del Disyuntor Determinista (*Circuit Breaker*)

Para impedir el fenómeno de *LLM Thrashing* (bucles destructivos donde el modelo intenta reparar un fallo rompiendo invariantes adyacentes), el kernel mantiene una máquina de estados determinista por cada módulo en la tabla `circuit_breakers`:

```
                     ┌────────────────────────────────────────┐
                     │              closed_ok                 │
                     │       (consecutive_failures = 0)       │
                     └───────────────────┬────────────────────┘
                                         │
                        Fallo en Gate 0..4 (Intento 1 o 2)
                                         ▼
                     ┌────────────────────────────────────────┐
      Pass en Gates  │             retry_allowed              │
   ◄─────────────────┤     (consecutive_failures in 1..2)     │
   (Reset a 0 +      │  • Genera FAIL-XXX en forensic_incidents│
    Resolution Hint) │  • Inyecta pistas BM25 de incidents_fts│
                     └───────────────────┬────────────────────┘
                                         │
                        Fallo en Gate 0..4 (Intento >= 3)
                                         ▼
                     ┌────────────────────────────────────────┐
                     │             tripped_abort              │
                     │       (consecutive_failures >= 3)      │
                     │  • Emite señal ABORT_AND_ALERT         │
                     │  • Congela Git Worktree contra edición │
                     │  • Exige intervención de [ARQUITECTO]  │
                     └────────────────────────────────────────┘
```

### Regla Matemática de Disparo:
$$\text{consecutive\_failures}(M) \ge 3 \implies \text{State}(M) \leftarrow \texttt{tripped\_abort}$$

Cuando un módulo entra en estado `tripped_abort`, toda invocación subsiguiente de escritura o ejecución de `Enki` sobre dicho módulo es rechazada de inmediato por el kernel hasta que el `[ARQUITECTO]` audite el incidente en [[failures]] y ejecute el comando de reinicio con su pista de resolución.

---

## 6. Implementación del Punto de Entrada y Despacho CLI (`src/cli.rs` y `src/main.rs`)

El parser de línea de comandos utiliza exclusivamente `std::env::args` sin dependencias pesadas de terceros para preservar la huella de memoria y velocidad de arranque:

- `alexandria --version` / `alexandria -v`: Emite `alexandria-runtime v2.4.0-OmniPragmatic` y finaliza con código `0`.
- `alexandria --help` / `alexandria -h`: Emite el catálogo de subcomandos y banderas y finaliza con código `0`.
- `alexandria init [--vault <ruta>] [--profile <perfil>]`: Genera `.alexandria.json`, crea el árbol documental base (`wiki/index.md`, `wiki/log.md`, `wiki/failures.md`, `wiki/architecture/`, `wiki/modules/`, `wiki/decisions/`) y blinda `.gitignore`.
- `alexandria mcp`: Inicializa `VaultStorageHandle::spawn` en `~/.alexandria/vault.db` y arranca `run_mcp_stdio_loop()`.
- `alexandria run "<directiva>"`: Clasifica la directiva en *Zero-Turn* (`alpha_fast` vs. `omega_pantheon`) y ejecuta el flujo autónomo.

---

## 7. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Esquema de Base de Datos y Actor:** [[database-schema]]
- **Servidor MCP:** [[mcp-server]]
- **Motor de Grafo Topológico:** [[topological-graph]]
- **Escáner AST Gate 0:** [[gate-guard-ast]]
- **Sandbox Runner Gate 3:** [[sandbox-runner]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]]
- **Registro de Fallos:** [[failures]]
