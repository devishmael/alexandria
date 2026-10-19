---
id: failures
title: "Registro Forense de Incidentes, Trazas Reales y Heurísticas de Resolución (FAIL-XXX)"
type: operational-log
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - forensic
  - failures
  - circuit-breaker
  - fts5
  - post-mortem
---

# Registro Forense de Incidentes, Trazas Reales y Heurísticas de Resolución (`FAIL-XXX`)

> **Documento Canónico Forense:** `alexandria/failures.md`  
> **Tabla Relacional Sincronizada:** `forensic_incidents` e `incidents_fts` en [[database-schema]]  
> **Custodio Operativo:** `Nabu (The Chronicler)` / `[ARQUITECTO]`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]]

---

## 1. Arquitectura de Memoria Forense y Correlación Automática

Cuando una mutación de código falla en cualquiera de las cinco compuertas de calidad (`Gate 0` a `Gate 4`), el `VaultWriterActor` ([[database-schema]]) genera un identificador único monotónico con formato:

$$\texttt{FAIL-<MODULO>-<UNIX\_MS>-<ATOMIC\_SEQ>}$$

Simultáneamente:
1. Se incrementa `consecutive_failures` en la tabla `circuit_breakers` ([[core-engine]]).
2. El trigger `incidents_ai` indexa la traza en la tabla virtual FTS5 `incidents_fts`.
3. El motor sanitiza la nueva traza mediante `build_safe_fts5_or_query` (`"tok1" OR "tok2" ...`) y ejecuta una búsqueda ponderada `bm25(incidents_fts)` sobre incidentes históricos que ya cuentan con un `resolution_hint` validado, inyectando las pistas directamente en el diagnóstico devuelto por `verify_and_gate_changes` ([[mcp-server]]).
4. Si `consecutive_failures` alcanza el umbral de **3 intentos consecutivos**, el módulo entra en estado `tripped_abort` (`ABORT_AND_ALERT`), congelando el *Git worktree* y exigiendo una entrada forense completa en este documento antes de ejecutar `ResetCircuitBreakerAndResolveIncident`.

---

## 2. Plantilla Estructural Obligatoria de Incidente (`FAIL-XXX`)

Todo incidente registrado por `[ARQUITECTO]` o `Nabu` debe adherirse estrictamente al siguiente esquema sin omitir ningún campo:

```markdown
## [FAIL-<MODULO>-<TIMESTAMP>-<SEQ>] — <Resumen Técnico del Defecto>

| Metadato Forense | Valor Registrado |
|---|---|
| **ID de Incidente** | `FAIL-<MODULO>-<TIMESTAMP>-<SEQ>` |
| **Fecha / Timestamp UTC** | `YYYY-MM-DDTHH:MM:SSZ` |
| **Módulo Objetivo** | `[[<id-del-modulo>]]` |
| **Archivo(s) Afectado(s)** | `src/<ruta>/<archivo>.ts` / `tests/<ruta>/<archivo>.test.ts` |
| **Compuerta Fallida** | `Gate 0 (Oxc AST)` \| `Gate 1 (Types/Lint)` \| `Gate 2 (SAST)` \| `Gate 3 (Sandbox Vitest)` \| `Gate 4 (Build/Utu)` |
| **Fase TDD Activa** | `red_expect_assertion_failure` \| `green_expect_pass` \| `refactor_full_gate` |
| **Intento / Estado Disyuntor** | `Intento X de 3` (`retry_allowed` \| `tripped_abort`) |
| **ADR Infringido** | `[[ADR-XXX-...]]` |

### 1. Stack Trace Real Capturado Fuera de Banda
```text
<Volcado exacto e inalterado del error emitido por oxc_validator, tsc, vitest JSON o cargo>
```

### 2. Tokens Clave FTS5 Extraídos (`build_safe_fts5_or_query`)
`"<token_1>" OR "<token_2>" OR "<token_3>" OR "<token_4>"`

### 3. Análisis de Causa Raíz (RCA)
Explicación técnica de nivel de sistemas detallando por qué se produjo el fallo (ej. violación de clausura léxica en transacción, ruta UNC no sanitizada, promesa flotante sin `await` o discrepancia de tipos nominales).

### 4. Heurística de Resolución (`resolution_hint` Persistido en SQLite)
Directiva accionable y determinista inyectada en `forensic_incidents.resolution_hint` para prevenir la recurrencia del fallo en turnos futuros.
```

---

## 3. Catálogo de Heurísticas Forenses Pre-Sembradas (Base de Conocimiento Inicial)

Para dotar al motor FTS5 de capacidad de autocuración desde el primer ciclo de construcción, se documentan las heurísticas canónicas asociadas a los modos de fallo de alta severidad prevenidos por el kernel:

### [FAIL-SANDBOX-SEED-001] — Denegación de Acceso en Node Permission Model por Prefijo Extendido UNC en Windows

| Metadato Forense | Valor Registrado |
|---|---|
| **ID de Incidente** | `FAIL-SANDBOX-SEED-001` |
| **Módulo Objetivo** | [[sandbox-runner]] |
| **Archivo(s) Afectado(s)** | `src/runner/vitest_sandbox.rs` |
| **Compuerta Fallida** | `Gate 3 (Sandbox Vitest)` |
| **Fase TDD Activa** | `red_expect_assertion_failure` |
| **Estado Disyuntor** | `closed_ok` (Mitigado en Especificación Canónica) |
| **ADR Relacionado** | [[ADR-005-node-permission-sandboxing]] |

#### 1. Stack Trace Real de Referencia
```text
node:internal/modules/esm/resolve:265
  throw new ERR_ACCESS_DENIED(
  ^
Error [ERR_ACCESS_DENIED]: Access to this API has been restricted. FileSystemRead
  resource: '\\?\C:\Github\alexandria-runtime\node_modules\vitest\vitest.mjs'
  code: 'ERR_ACCESS_DENIED'
```

#### 2. Tokens Clave FTS5 Extraídos
`"ERR_ACCESS_DENIED" OR "FileSystemRead" OR "vitest" OR "permission" OR "UNC"`

#### 3. Análisis de Causa Raíz (RCA)
En Windows, `std::fs::canonicalize()` antepone el prefijo de longitud extendida `\\?\` (o `\\?\UNC\`). El comparador de rutas del modelo `--permission` de Node.js y el cargador ESM de Vite no equiparan `\\?\C:\ruta` con `C:\ruta`, disparando `ERR_ACCESS_DENIED` o fallando al escribir archivos temporales en `TMP` si las variables `TMP`/`TEMP`/`TMPDIR` fueron eliminadas por `env_clear()`.

#### 4. Heurística de Resolución (`resolution_hint`)
Aplicar siempre `clean_canonical_path()` eliminando `\\?\UNC\` y `\\?\` antes de construir `--allow-fs-read` y `--allow-fs-write`, preservar `SystemRoot` en Windows e inyectar `TMP`, `TEMP` y `TMPDIR` apuntando al directorio temporal canonicalizado.

---

### [FAIL-STORAGE-SEED-002] — Corrupción de Tabla Virtual FTS5 (`SQLITE_CORRUPT_VTAB`) por Omisión de Triggers de Actualización/Borrado

| Metadato Forense | Valor Registrado |
|---|---|
| **ID de Incidente** | `FAIL-STORAGE-SEED-002` |
| **Módulo Objetivo** | [[database-schema]] |
| **Archivo(s) Afectado(s)** | `src/storage/schema.sql`, `src/storage/actor.rs` |
| **Compuerta Fallida** | `Gate 3 (Sandbox Vitest)` / Persistencia de Kernel |
| **Estado Disyuntor** | `closed_ok` (Mitigado en Especificación Canónica) |
| **ADR Relacionado** | [[ADR-004-single-writer-mpsc-storage]] |

#### 1. Stack Trace Real de Referencia
```text
Error SQL ejecutando consulta BM25 en incidents_fts: database disk image is malformed (code 11: SQLITE_CORRUPT_VTAB)
```

#### 2. Tokens Clave FTS5 Extraídos
`"SQLITE_CORRUPT_VTAB" OR "malformed" OR "incidents_fts" OR "skills_fts" OR "bm25"`

#### 3. Análisis de Causa Raíz (RCA)
Cuando una tabla virtual FTS5 declara `content='forensic_incidents'` y se ejecuta un `UPDATE` (por ejemplo, al poblar `resolution_hint` tras resolver un fallo) sin que exista el trigger `AFTER UPDATE` (`incidents_au`) que emite el comando `'delete'` de la fila previa seguido del `INSERT` de la fila nueva, el índice invertido de FTS5 apunta a offsets de tokens inexistentes y devuelve `SQLITE_CORRUPT_VTAB`.

#### 4. Heurística de Resolución (`resolution_hint`)
Garantizar que toda tabla virtual FTS5 con contenido externo (`skills_fts`, `incidents_fts`) declare invariablemente los tres triggers simétricos (`_ai` AFTER INSERT, `_ad` AFTER DELETE, `_au` AFTER UPDATE).

---

## 4. Incidentes Activos en el Espacio de Trabajo

> **ESTADO OPERATIVO ACTUAL:** `0 Incidentes Activos` — Todos los `circuit_breakers` se encuentran en estado `closed_ok` (`consecutive_failures = 0`).

---

## 5. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Esquema SQLite y Actor MPSC:** [[database-schema]]
- **Runtime Central y Disyuntor:** [[core-engine]]
- **Escáner AST Gate 0:** [[gate-guard-ast]]
- **Sandbox Runner Gate 3:** [[sandbox-runner]]
- **Bitácora de Cambios:** [[log]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]], [[ADR-005-node-permission-sandboxing]], [[ADR-006-oxc-ast-engine]]
