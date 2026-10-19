---
id: database-schema
title: "Esquema SQLite WAL, Triggers FTS5, Tuning de Memoria y Actor Transaccional MPSC"
type: architecture
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - architecture
  - sqlite
  - wal
  - fts5
  - mpsc
  - actor
  - persistence
---

# Esquema SQLite WAL, Triggers FTS5, Tuning de Memoria y Actor Transaccional MPSC

> **Documento Canónico de Persistencia:** `alexandria/architecture/database-schema.md`  
> **Archivos de Destino en Kernel:** `src/storage/schema.sql` y `src/storage/actor.rs`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-004-single-writer-mpsc-storage]]

---

## 1. Arquitectura de Almacenamiento y Tuning del Motor SQLite

El estado persistente del arnés se aloja en `~/.alexandria/vault.db`. Para garantizar latencias de lectura inferiores a $0.5\text{ ms}$ y erradicar por completo los bloqueos de concurrencia (`SQLITE_BUSY` / `SQLITE_LOCKED`) cuando múltiples sub-sesiones o workers paralelos operan simultáneamente, el motor aplica cuatro mecanismos de ingeniería de sistemas:

1. **Tuning Estricto por PRAGMAs de Kernel:**
   - `PRAGMA journal_mode = WAL;`: Activa *Write-Ahead Logging*, permitiendo que múltiples hilos lectores (`SQLITE_OPEN_READ_ONLY`) consulten la base de datos simultáneamente sin bloquear al hilo escritor ni ser bloqueados por este.
   - `PRAGMA synchronous = NORMAL;`: En modo WAL, garantiza durabilidad ante fallos de proceso evitando llamadas `fsync()` redundantes en cada transacción individual, delegando la sincronización completa a los *checkpoints* del WAL.
   - `PRAGMA busy_timeout = 5000;`: Configura un respaldo de espera de 5000 ms a nivel de motor C de SQLite ante contenciones transitorias de cierre de archivo.
   - `PRAGMA mmap_size = 268435456;`: Mapea hasta 256 MB de páginas de la base de datos directamente en el espacio de direcciones virtuales del proceso (`mmap`), eliminando copias de buffer entre el kernel del sistema operativo y el espacio de usuario en lecturas frecuentes.
   - `PRAGMA temp_store = MEMORY;`: Fuerza que todas las tablas temporales, índices transitorios y árboles B de ordenación (`ORDER BY bm25(...)`) residan exclusivamente en RAM.
   - `PRAGMA cache_size = -64000;`: Establece un límite máximo de caché de páginas de 64 MB por conexión activa, acotando el crecimiento de memoria bajo cargas intensivas.

2. **Tablas Virtuales FTS5 de Contenido Externo Blindadas contra `SQLITE_CORRUPT_VTAB`:**
   - Las tablas virtuales `skills_fts` e `incidents_fts` declaran `content='<tabla_base>'` y `content_rowid='rowid'` para evitar duplicar el texto pesado de instrucciones Markdown y stack traces en el archivo `.db`.
   - En SQLite FTS5 con contenido externo, la omisión de cualquiera de los tres triggers de sincronización (`AFTER INSERT`, `AFTER DELETE`, `AFTER UPDATE`) desincroniza el índice invertido respecto a la tabla relacional, provocando el error fatal `database disk image is malformed (SQLITE_CORRUPT_VTAB)`. El esquema canónico define explícitamente la tríada simétrica de triggers (`_ai`, `_ad`, `_au`) para cada tabla indexada.

3. **Actor de Escritura Única (`VaultWriterActor`) con Drenaje por Lotes (*Batching*):**
   - Ningún hilo de Tokio abre conexiones de escritura directas. Toda mutación se envía por un canal acotado `tokio::sync::mpsc::Sender<VaultWriteCommand>` hacia un único hilo nativo dedicado del sistema operativo (`alexandria-sqlite-writer`).
   - El hilo escritor bloquea esperando el primer comando (`blocking_recv()`) e inmediatamente drena hasta 63 comandos adicionales acumulados en la cola mediante `try_recv()`, agrupando hasta 64 mutaciones dentro de una única transacción física (`BEGIN IMMEDIATE ... COMMIT`).

4. **Confirmación Estrictamente Post-Commit (`oneshot` Deferred Notification):**
   - Si el actor respondiera por el canal `oneshot` inmediatamente después de ejecutar `tx.execute(...)` pero antes de `tx.commit()`, un hilo lector despertado en Tokio podría abrir una conexión de solo lectura y no observar aún los datos (violación de consistencia *Read-Your-Writes*), o recibir un falso positivo si el `COMMIT` falla en disco. El actor retiene todos los cierres de notificación en un vector `pending_notifications` y los dispara **únicamente después** de que `tx.commit()` ha retornado exitosamente.

---

## 2. Script DDL Canónico (`src/storage/schema.sql`)

```sql
-- =============================================================================
-- ALEXANDRIA RUNTIME v2.4.0 — CANONICAL SQLITE WAL & FTS5 SCHEMA
-- Archivo objetivo: src/storage/schema.sql
-- =============================================================================

-- 1. Configuración de Rendimiento, Durabilidad y Mapeo en Memoria
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA mmap_size = 268435456;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -64000;

-- 2. Grafo Persistido de la Bóveda de Conocimiento (Plano 1 Relacional)
CREATE TABLE IF NOT EXISTS wiki_nodes (
    node_id TEXT PRIMARY KEY,
    relative_path TEXT NOT NULL UNIQUE,
    node_kind TEXT NOT NULL CHECK(node_kind IN ('Module', 'Architecture', 'ConstitutionalAdr', 'Skill', 'OperationalLog')),
    sha256_hex TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wiki_nodes_kind ON wiki_nodes(node_kind);

CREATE TABLE IF NOT EXISTS wiki_edges (
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    PRIMARY KEY (source_id, target_id),
    FOREIGN KEY (source_id) REFERENCES wiki_nodes(node_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_wiki_edges_target ON wiki_edges(target_id);

-- 3. Catálogo de Habilidades Procedimentales (Cápsulas JIT)
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

CREATE INDEX IF NOT EXISTS idx_skills_domain ON skills(domain);

-- 4. Índice Léxico FTS5 para Skills (Contenido Externo + Tokenizador Porter)
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

-- Triggers de Sincronización Simétrica para skills_fts
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

-- 5. Estado del Disyuntor Determinista (Circuit Breaker por Módulo)
CREATE TABLE IF NOT EXISTS circuit_breakers (
    module_name TEXT PRIMARY KEY,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL CHECK(state IN ('closed_ok', 'retry_allowed', 'tripped_abort')),
    last_failure_id TEXT,
    updated_at INTEGER NOT NULL
);

-- 6. Registro Forense de Incidentes (Nabu / FAIL-XXX)
CREATE TABLE IF NOT EXISTS forensic_incidents (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    failure_id TEXT NOT NULL UNIQUE,
    module_name TEXT NOT NULL,
    failed_gate TEXT NOT NULL,
    stack_trace TEXT NOT NULL,
    resolution_hint TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_forensic_incidents_module ON forensic_incidents(module_name, created_at DESC);

-- 7. Índice Léxico FTS5 para Incidentes Forenses (Previene SQLITE_CORRUPT_VTAB)
CREATE VIRTUAL TABLE IF NOT EXISTS incidents_fts USING fts5(
    failure_id,
    module_name,
    stack_trace,
    resolution_hint,
    content='forensic_incidents',
    content_rowid='rowid',
    tokenize='unicode61'
);

-- Triggers de Sincronización Simétrica para incidents_fts
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

-- 8. Historial Auditable de Veredictos de Compuertas de Calidad
CREATE TABLE IF NOT EXISTS quality_gate_verdicts (
    verdict_id INTEGER PRIMARY KEY AUTOINCREMENT,
    module_name TEXT NOT NULL,
    execution_path TEXT NOT NULL CHECK(execution_path IN ('alpha_fast', 'omega_pantheon')),
    tdd_phase TEXT NOT NULL CHECK(tdd_phase IN ('red_expect_assertion_failure', 'green_expect_pass', 'refactor_full_gate')),
    passed INTEGER NOT NULL CHECK(passed IN (0, 1)),
    diagnostics_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_verdicts_module_time ON quality_gate_verdicts(module_name, created_at DESC);
```

---

## 3. Implementación de Referencia del Actor MPSC (`src/storage/actor.rs`)

La siguiente implementación en Rust materializa el patrón *Single-Writer MPSC Batching Actor* con secuencia atómica monotónica `INCIDENT_SEQ`, confirmación post-commit y sanitización léxica para FTS5:

```rust
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
                    "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA foreign_keys = ON;
                     PRAGMA busy_timeout = 5000;
                     PRAGMA mmap_size = 268435456;
                     PRAGMA temp_store = MEMORY;
                     PRAGMA cache_size = -64000;",
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

    pub async fn record_gate_failure(
        &self,
        module_name: String,
        failed_gate: String,
        stack_trace: String,
        max_retries: u32,
    ) -> Result<(String, String, u32), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(VaultWriteCommand::RecordGateFailure {
                module_name,
                failed_gate,
                stack_trace,
                max_retries,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "Canal de VaultWriterActor cerrado".to_string())?;
        reply_rx
            .await
            .map_err(|_| "Respuesta oneshot cancelada en VaultWriterActor".to_string())?
    }

    pub async fn reset_circuit_breaker(
        &self,
        module_name: String,
        last_failure_id: Option<String>,
        resolution_hint: String,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(VaultWriteCommand::ResetCircuitBreakerAndResolveIncident {
                module_name,
                last_failure_id,
                resolution_hint,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "Canal de VaultWriterActor cerrado".to_string())?;
        reply_rx
            .await
            .map_err(|_| "Respuesta oneshot cancelada en VaultWriterActor".to_string())?
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

---

## 4. Consultas Optimizadas de Referencia con Ponderación `bm25()`

### 4.1. Enrutamiento Léxico de Cápsulas JIT (`resolve_skill_capsule`)
Pondera con mayor peso las coincidencias en `skill_id` ($10.0$) y `trigger_keywords_json` ($8.0$) sobre el cuerpo extenso de `instructions_md` ($1.0$). Dado que `bm25()` en SQLite FTS5 retorna valores negativos donde números más negativos indican mayor relevancia, un `ORDER BY bm25(...) ASC` posiciona el mejor candidato en primer lugar:

```sql
SELECT
    s.skill_id,
    s.domain,
    s.invariants_json,
    s.instructions_md,
    bm25(skills_fts, 10.0, 5.0, 3.0, 8.0, 1.0) AS relevance_score
FROM skills_fts
JOIN skills AS s ON s.rowid = skills_fts.rowid
WHERE skills_fts MATCH ?1
ORDER BY relevance_score ASC
LIMIT 1;
```

### 4.2. Recuperación Forense de Pistas ante Fallos de Compilación (`Nabu`)
Prioriza los incidentes resueltos que coinciden tanto en el módulo afectado ($5.0$) como en los símbolos del stack trace ($10.0$):

```sql
SELECT
    failure_id,
    module_name,
    resolution_hint,
    bm25(incidents_fts, 1.0, 5.0, 10.0, 2.0) AS forensic_score
FROM incidents_fts
WHERE incidents_fts MATCH ?1
  AND resolution_hint IS NOT NULL
ORDER BY forensic_score ASC
LIMIT 3;
```

---

## 5. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Motor de Ejecución:** [[core-engine]]
- **Servidor MCP:** [[mcp-server]]
- **Registro Forense:** [[failures]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-004-single-writer-mpsc-storage]]
