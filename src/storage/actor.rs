use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};

static INCIDENT_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCapsuleRecord {
    pub skill_id: String,
    pub domain: String,
    pub description: String,
    pub trigger_keywords_json: String,
    pub invariants_json: String,
    pub instructions_md: String,
    pub resolution_method: String,
}

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
    UpsertSkill {
        skill_id: String,
        domain: String,
        description: String,
        trigger_keywords_json: String,
        invariants_json: String,
        instructions_md: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
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

#[derive(Clone)]
pub struct VaultStorageHandle {
    writer_tx: mpsc::Sender<VaultWriteCommand>,
    db_path: PathBuf,
}

impl VaultStorageHandle {
    pub fn spawn(db_path: PathBuf, schema_sql: &str) -> Result<Self, String> {
        let init_conn =
            Connection::open(&db_path).map_err(|e| format!("Error abriendo vault.db: {}", e))?;
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

    pub async fn upsert_skill(
        &self,
        skill_id: String,
        domain: String,
        description: String,
        trigger_keywords_json: String,
        invariants_json: String,
        instructions_md: String,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(VaultWriteCommand::UpsertSkill {
                skill_id,
                domain,
                description,
                trigger_keywords_json,
                invariants_json,
                instructions_md,
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
            VaultWriteCommand::UpsertSkill { reply, .. } => {
                let _ = reply.send(Err(err_msg.to_string()));
            }
        }
    }

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

                let failure_id =
                    format!("FAIL-{}-{}-{}", module_name.to_uppercase(), now_ms, seq);

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
            VaultWriteCommand::UpsertSkill {
                skill_id,
                domain,
                description,
                trigger_keywords_json,
                invariants_json,
                instructions_md,
                reply,
            } => {
                let res = tx
                    .execute(
                        "INSERT INTO skills (skill_id, domain, description, trigger_keywords_json, invariants_json, instructions_md, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                         ON CONFLICT(skill_id) DO UPDATE SET
                            domain = excluded.domain,
                            description = excluded.description,
                            trigger_keywords_json = excluded.trigger_keywords_json,
                            invariants_json = excluded.invariants_json,
                            instructions_md = excluded.instructions_md,
                            updated_at = excluded.updated_at",
                        params![
                            skill_id,
                            domain,
                            description,
                            trigger_keywords_json,
                            invariants_json,
                            instructions_md,
                            now_ms
                        ],
                    )
                    .map(|_| ())
                    .map_err(|e| format!("Error SQL insertando skill: {}", e));

                Box::new(move |commit_status| match (commit_status, res) {
                    (Ok(()), Ok(())) => {
                        let _ = reply.send(Ok(()));
                    }
                    (Err(commit_err), _) => {
                        let _ = reply.send(Err(commit_err));
                    }
                    (_, Err(sql_err)) => {
                        let _ = reply.send(Err(sql_err));
                    }
                })
            }
        }
    }

    pub fn build_safe_fts5_or_query(raw_stack_trace: &str) -> Option<String> {
        build_safe_fts5_or_query(raw_stack_trace)
    }

    pub fn resolve_skill_capsule(
        &self,
        skill_id: Option<&str>,
        intent_query: Option<&str>,
    ) -> Result<Option<SkillCapsuleRecord>, String> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;

        if let Some(id) = skill_id.filter(|s| !s.trim().is_empty()) {
            let exact: Option<SkillCapsuleRecord> = conn
                .query_row(
                    "SELECT skill_id, domain, description, trigger_keywords_json, invariants_json, instructions_md
                     FROM skills WHERE skill_id = ?1",
                    params![id],
                    |row| {
                        Ok(SkillCapsuleRecord {
                            skill_id: row.get(0)?,
                            domain: row.get(1)?,
                            description: row.get(2)?,
                            trigger_keywords_json: row.get(3)?,
                            invariants_json: row.get(4)?,
                            instructions_md: row.get(5)?,
                            resolution_method: "exact_id".to_string(),
                        })
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?;

            if exact.is_some() {
                return Ok(exact);
            }
        }

        if let Some(query) = intent_query.filter(|s| !s.trim().is_empty()) {
            if let Some(safe_fts_query) = build_safe_fts5_or_query(query) {
                let matched_id: Option<String> = conn
                    .query_row(
                        "SELECT skill_id FROM skills_fts WHERE skills_fts MATCH ?1 ORDER BY bm25(skills_fts) LIMIT 1",
                        params![safe_fts_query],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;

                if let Some(found_id) = matched_id {
                    let fts_record: Option<SkillCapsuleRecord> = conn
                        .query_row(
                            "SELECT skill_id, domain, description, trigger_keywords_json, invariants_json, instructions_md
                             FROM skills WHERE skill_id = ?1",
                            params![found_id],
                            |row| {
                                Ok(SkillCapsuleRecord {
                                    skill_id: row.get(0)?,
                                    domain: row.get(1)?,
                                    description: row.get(2)?,
                                    trigger_keywords_json: row.get(3)?,
                                    invariants_json: row.get(4)?,
                                    instructions_md: row.get(5)?,
                                    resolution_method: "fts5_bm25_fallback".to_string(),
                                })
                            },
                        )
                        .optional()
                        .map_err(|e| e.to_string())?;

                    if fts_record.is_some() {
                        return Ok(fts_record);
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn search_similar_incidents_fts5(
        &self,
        raw_stack_trace: &str,
    ) -> Result<Vec<String>, String> {
        let Some(safe_fts_query) = build_safe_fts5_or_query(raw_stack_trace) else {
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

