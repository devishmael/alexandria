use crate::collector::skill_etl::{
    AlexandriaCapsuleJson, AlexandriaHookSpec, builtin_core_capsules,
};
use crate::vector_store::hybrid_memory::{
    compute_deterministic_int8_embedding, distill_into_micro_chunk,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
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
    pub required_tools_json: String,
    pub invariants_json: String,
    pub hooks_json: String,
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
        required_tools_json: String,
        invariants_json: String,
        hooks_json: String,
        instructions_md: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SyncWikiGraph {
        nodes: Vec<(String, String, String, String)>,
        edges: Vec<(String, String)>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    RecordQualityGateVerdict {
        module_name: String,
        execution_path: String,
        tdd_phase: String,
        passed: bool,
        diagnostics_json: String,
        reply: oneshot::Sender<Result<i64, String>>,
    },
    RecordEpisodicMemoryChunk {
        source_kind: String,
        agent_id: String,
        module_name: String,
        tags_json: String,
        compact_summary: String,
        token_estimate: usize,
        embedding_int8_json: String,
        reply: oneshot::Sender<Result<String, String>>,
    },
}

/// Sanitizes a raw stack trace into a safe FTS5 disjunctive query (`"tok1" OR "tok2"`),
/// immune to syntax errors from `:`, `/`, `"` or reserved FTS5 operators (`AND`, `OR`, `NOT`, `NEAR`).
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
            Connection::open(&db_path).map_err(|e| format!("Failed to open vault.db: {}", e))?;
        init_conn
            .execute_batch(schema_sql)
            .map_err(|e| format!("Failed to initialize SQL schema: {}", e))?;
        let _ = init_conn.execute(
            "ALTER TABLE skills ADD COLUMN required_tools_json TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        let _ = init_conn.execute(
            "ALTER TABLE skills ADD COLUMN hooks_json TEXT NOT NULL DEFAULT '[]'",
            [],
        );

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let pantheon_archetypes: [(&str, &str, &str, i64, &str, &str); 4] = [
            (
                "bel",
                "Bel (The Overseer)",
                "Zero-Turn classifier, topological graph evaluator, and sub-session dispatcher.",
                5,
                r#"{"canModifySrc":false,"canReadVault":true,"canSpawnWorktrees":true}"#,
                r#"["prepare_task_context","spawn_isolated_worktrees","resolve_skill_capsule"]"#,
            ),
            (
                "enki",
                "Enki (The Builder)",
                "Polymorphic constructor via prefix-pinned sub-sessions (DB, Backend, UI) under strict TDD.",
                3,
                r#"{"canModifySrc":true,"requiresRedPhaseFirst":true,"canWriteTests":true}"#,
                r#"["prepare_task_context","resolve_skill_capsule","verify_and_gate_changes"]"#,
            ),
            (
                "utu",
                "Utu (The Critic)",
                "Stateless cold auditor for Gate 4 (read-only access to git diff, ADRs, and JSON reports).",
                1,
                r#"{"canModifySrc":false,"statelessColdContext":true,"readOnlyAudit":true}"#,
                r#"["verify_and_gate_changes"]"#,
            ),
            (
                "nabu",
                "Nabu (The Chronicler)",
                "Custodian of the vault (wiki/), contracts, ADRs, forensic incidents FAIL-XXX, and log.md.",
                1,
                r#"{"canModifySrc":false,"canWriteVault":true,"appendOnlyLedger":true}"#,
                r#"["upsert_architectural_contract","finalize_task_and_log"]"#,
            ),
        ];

        for (agent_id, display_name, role_archetype, max_subagents, rbac_json, tools_json) in
            pantheon_archetypes
        {
            init_conn
                .execute(
                    "INSERT OR IGNORE INTO agent_catalog (
                        agent_id, display_name, role_archetype, max_subagents,
                        rbac_permissions_json, authorized_tools_json, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        agent_id,
                        display_name,
                        role_archetype,
                        max_subagents,
                        rbac_json,
                        tools_json,
                        now_ms
                    ],
                )
                .map_err(|e| format!("Failed to seed Pantheon agent '{}': {}", agent_id, e))?;
        }

        for cap in builtin_core_capsules() {
            let trigger_keywords_json =
                serde_json::to_string(&cap.trigger_keywords).unwrap_or_else(|_| "[]".to_string());
            let required_tools_json =
                serde_json::to_string(&cap.required_tools).unwrap_or_else(|_| "[]".to_string());
            let invariants_json =
                serde_json::to_string(&cap.invariants).unwrap_or_else(|_| "[]".to_string());
            let hooks_json = serde_json::to_string(&cap.hooks).unwrap_or_else(|_| "[]".to_string());

            init_conn
                .execute(
                    "INSERT OR IGNORE INTO skills (
                        skill_id, domain, description, trigger_keywords_json,
                        required_tools_json, invariants_json, hooks_json,
                        instructions_md, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        cap.skill_id,
                        cap.domain,
                        cap.description,
                        trigger_keywords_json,
                        required_tools_json,
                        invariants_json,
                        hooks_json,
                        cap.instructions_markdown,
                        now_ms
                    ],
                )
                .map_err(|e| format!("Failed to seed built-in capsule '{}': {}", cap.skill_id, e))?;
        }

        // Auto-seed any capsules found in workspace .alexandria/capsules or ./capsules
        for search_dir in &[
            PathBuf::from(".alexandria/capsules"),
            PathBuf::from("capsules"),
        ] {
            if search_dir.exists() && search_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(search_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("json")
                            && path.file_name().and_then(|s| s.to_str()) != Some("alexandria-capsules.manifest.json")
                        {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(cap) = serde_json::from_str::<AlexandriaCapsuleJson>(&content) {
                                    let trigger_keywords_json = serde_json::to_string(&cap.trigger_keywords).unwrap_or_else(|_| "[]".to_string());
                                    let required_tools_json = serde_json::to_string(&cap.required_tools).unwrap_or_else(|_| "[]".to_string());
                                    let invariants_json = serde_json::to_string(&cap.invariants).unwrap_or_else(|_| "[]".to_string());
                                    let hooks_json = serde_json::to_string(&cap.hooks).unwrap_or_else(|_| "[]".to_string());
                                    let _ = init_conn.execute(
                                        "INSERT OR IGNORE INTO skills (
                                            skill_id, domain, description, trigger_keywords_json,
                                            required_tools_json, invariants_json, hooks_json,
                                            instructions_md, updated_at
                                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                        params![
                                            cap.skill_id,
                                            cap.domain,
                                            cap.description,
                                            trigger_keywords_json,
                                            required_tools_json,
                                            invariants_json,
                                            hooks_json,
                                            cap.instructions_markdown,
                                            now_ms
                                        ],
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Auto-seed episodic vector memory from workspace snapshot if present
        let snapshot_path = PathBuf::from(".alexandria/vector-store/memory-index.json");
        if snapshot_path.exists() {
            if let Ok(raw_json) = std::fs::read_to_string(&snapshot_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_json) {
                    if let Some(chunks) = val.get("chunks").and_then(|c| c.as_array()) {
                        for chunk in chunks {
                            let chunk_id = chunk.get("chunkId").and_then(|v| v.as_str()).unwrap_or("");
                            let source_kind = chunk.get("sourceKind").and_then(|v| v.as_str()).unwrap_or("episodic_turn");
                            let agent_id = chunk.get("agentId").and_then(|v| v.as_str()).unwrap_or("architect");
                            let module_name = chunk.get("moduleName").and_then(|v| v.as_str()).unwrap_or("core");
                            let tags = chunk.get("tags").cloned().unwrap_or_else(|| serde_json::json!([]));
                            let compact_summary = chunk.get("compactSummary").and_then(|v| v.as_str()).unwrap_or("");
                            let token_estimate = chunk.get("tokenEstimate").and_then(|v| v.as_i64()).unwrap_or(20);
                            let embedding = chunk.get("embeddingInt8").cloned().unwrap_or_else(|| serde_json::json!([]));
                            let created_at = chunk.get("createdAt").and_then(|v| v.as_i64()).unwrap_or(now_ms);

                            let tags_str = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
                            let emb_str = serde_json::to_string(&embedding).unwrap_or_else(|_| "[]".into());

                            let _ = init_conn.execute(
                                "INSERT OR IGNORE INTO episodic_vector_memory (
                                    chunk_id, source_kind, agent_id, module_name, tags_json,
                                    compact_summary, token_estimate, embedding_int8_json, created_at
                                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                params![
                                    chunk_id,
                                    source_kind,
                                    agent_id,
                                    module_name,
                                    tags_str,
                                    compact_summary,
                                    token_estimate,
                                    emb_str,
                                    created_at
                                ],
                            );
                        }
                    }
                }
            }
        }

        drop(init_conn);

        let (writer_tx, mut writer_rx) = mpsc::channel::<VaultWriteCommand>(512);
        let thread_db_path = db_path.clone();

        std::thread::Builder::new()
            .name("alexandria-sqlite-writer".into())
            .spawn(move || {
                let mut conn = Connection::open(&thread_db_path)
                    .expect("Failed to open write connection on vault.db");
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
                                .map_err(|e| format!("SQLite batch COMMIT failed: {}", e));

                            for notify in pending_notifications {
                                notify(commit_res.clone());
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to begin SQLite transaction: {}", e);
                            for cmd in batch {
                                Self::abort_command(cmd, &err_msg);
                            }
                        }
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn VaultWriterActor thread: {}", e))?;

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
            .map_err(|_| "VaultWriterActor channel closed".to_string())?;
        reply_rx
            .await
            .map_err(|_| "VaultWriterActor oneshot reply cancelled".to_string())?
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
            .map_err(|_| "VaultWriterActor channel closed".to_string())?;
        reply_rx
            .await
            .map_err(|_| "VaultWriterActor oneshot reply cancelled".to_string())?
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
                required_tools_json: r#"["prepare_task_context","verify_and_gate_changes"]"#
                    .to_string(),
                invariants_json,
                hooks_json: "[]".to_string(),
                instructions_md,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "VaultWriterActor channel closed".to_string())?;
        reply_rx
            .await
            .map_err(|_| "VaultWriterActor oneshot reply cancelled".to_string())?
    }

    pub async fn upsert_skill_capsule(
        &self,
        capsule: &AlexandriaCapsuleJson,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let trigger_keywords_json =
            serde_json::to_string(&capsule.trigger_keywords).unwrap_or_else(|_| "[]".to_string());
        let required_tools_json =
            serde_json::to_string(&capsule.required_tools).unwrap_or_else(|_| "[]".to_string());
        let invariants_json =
            serde_json::to_string(&capsule.invariants).unwrap_or_else(|_| "[]".to_string());
        let hooks_json = serde_json::to_string(&capsule.hooks).unwrap_or_else(|_| "[]".to_string());

        self.writer_tx
            .send(VaultWriteCommand::UpsertSkill {
                skill_id: capsule.skill_id.clone(),
                domain: capsule.domain.clone(),
                description: capsule.description.clone(),
                trigger_keywords_json,
                required_tools_json,
                invariants_json,
                hooks_json,
                instructions_md: capsule.instructions_markdown.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "VaultWriterActor channel closed".to_string())?;
        reply_rx
            .await
            .map_err(|_| "VaultWriterActor oneshot reply cancelled".to_string())?
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
            VaultWriteCommand::SyncWikiGraph { reply, .. } => {
                let _ = reply.send(Err(err_msg.to_string()));
            }
            VaultWriteCommand::RecordQualityGateVerdict { reply, .. } => {
                let _ = reply.send(Err(err_msg.to_string()));
            }
            VaultWriteCommand::RecordEpisodicMemoryChunk { reply, .. } => {
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
                    .map_err(|e| format!("SQL error recording forensic incident: {}", e));

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
                        sql_err = Some(format!("Failed to update forensic incident: {}", e));
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
                        sql_err = Some(format!("Failed to reset circuit breaker: {}", e));
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
                required_tools_json,
                invariants_json,
                hooks_json,
                instructions_md,
                reply,
            } => {
                let res = tx
                    .execute(
                        "INSERT INTO skills (
                            skill_id, domain, description, trigger_keywords_json,
                            required_tools_json, invariants_json, hooks_json,
                            instructions_md, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                         ON CONFLICT(skill_id) DO UPDATE SET
                            domain = excluded.domain,
                            description = excluded.description,
                            trigger_keywords_json = excluded.trigger_keywords_json,
                            required_tools_json = excluded.required_tools_json,
                            invariants_json = excluded.invariants_json,
                            hooks_json = excluded.hooks_json,
                            instructions_md = excluded.instructions_md,
                            updated_at = excluded.updated_at",
                        params![
                            skill_id,
                            domain,
                            description,
                            trigger_keywords_json,
                            required_tools_json,
                            invariants_json,
                            hooks_json,
                            instructions_md,
                            now_ms
                        ],
                    )
                    .map(|_| ())
                    .map_err(|e| format!("SQL error inserting skill: {}", e));

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
            VaultWriteCommand::SyncWikiGraph {
                nodes,
                edges,
                reply,
            } => {
                let mut sql_err: Option<String> = None;

                for (node_id, relative_path, node_kind, sha256_hex) in &nodes {
                    if let Err(e) = tx.execute(
                        "INSERT INTO wiki_nodes (node_id, relative_path, node_kind, sha256_hex, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5)
                         ON CONFLICT(node_id) DO UPDATE SET
                            relative_path = excluded.relative_path,
                            node_kind = excluded.node_kind,
                            sha256_hex = excluded.sha256_hex,
                            updated_at = excluded.updated_at",
                        params![node_id, relative_path, node_kind, sha256_hex, now_ms],
                    ) {
                        sql_err = Some(format!("SQL error upserting wiki_node '{}': {}", node_id, e));
                        break;
                    }
                }

                if sql_err.is_none() {
                    for (source_id, target_id) in &edges {
                        if let Err(e) = tx.execute(
                            "INSERT OR IGNORE INTO wiki_edges (source_id, target_id)
                             VALUES (?1, ?2)",
                            params![source_id, target_id],
                        ) {
                            sql_err = Some(format!(
                                "SQL error inserting wiki_edge '{} -> {}': {}",
                                source_id, target_id, e
                            ));
                            break;
                        }
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
            VaultWriteCommand::RecordQualityGateVerdict {
                module_name,
                execution_path,
                tdd_phase,
                passed,
                diagnostics_json,
                reply,
            } => {
                let passed_int: i64 = if passed { 1 } else { 0 };
                let res = tx
                    .execute(
                        "INSERT INTO quality_gate_verdicts (
                            module_name, execution_path, tdd_phase, passed, diagnostics_json, created_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            module_name,
                            execution_path,
                            tdd_phase,
                            passed_int,
                            diagnostics_json,
                            now_ms
                        ],
                    )
                    .map(|_| tx.last_insert_rowid())
                    .map_err(|e| format!("SQL error inserting quality_gate_verdict: {}", e));

                Box::new(move |commit_status| match (commit_status, res) {
                    (Ok(()), Ok(verdict_id)) => {
                        let _ = reply.send(Ok(verdict_id));
                    }
                    (Err(commit_err), _) => {
                        let _ = reply.send(Err(commit_err));
                    }
                    (_, Err(sql_err)) => {
                        let _ = reply.send(Err(sql_err));
                    }
                })
            }
            VaultWriteCommand::RecordEpisodicMemoryChunk {
                source_kind,
                agent_id,
                module_name,
                tags_json,
                compact_summary,
                token_estimate,
                embedding_int8_json,
                reply,
            } => {
                let chunk_id = format!(
                    "MEM-{}-{}-{}",
                    module_name.to_ascii_uppercase(),
                    now_ms,
                    seq
                );
                let res = tx
                    .execute(
                        "INSERT INTO episodic_vector_memory (
                            chunk_id, source_kind, agent_id, module_name,
                            tags_json, compact_summary, token_estimate,
                            embedding_int8_json, created_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        params![
                            chunk_id,
                            source_kind,
                            agent_id,
                            module_name,
                            tags_json,
                            compact_summary,
                            token_estimate as i64,
                            embedding_int8_json,
                            now_ms
                        ],
                    )
                    .map(|_| chunk_id)
                    .map_err(|e| format!("SQL error inserting episodic memory chunk: {}", e));

                Box::new(move |commit_status| match (commit_status, res) {
                    (Ok(()), Ok(cid)) => {
                        let _ = reply.send(Ok(cid));
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
                    "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, hooks_json, instructions_md
                     FROM skills WHERE skill_id = ?1",
                    params![id],
                    |row| {
                        Ok(SkillCapsuleRecord {
                            skill_id: row.get(0)?,
                            domain: row.get(1)?,
                            description: row.get(2)?,
                            trigger_keywords_json: row.get(3)?,
                            required_tools_json: row.get(4)?,
                            invariants_json: row.get(5)?,
                            hooks_json: row.get(6)?,
                            instructions_md: row.get(7)?,
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
                            "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, hooks_json, instructions_md
                             FROM skills WHERE skill_id = ?1",
                            params![found_id],
                            |row| {
                                Ok(SkillCapsuleRecord {
                                    skill_id: row.get(0)?,
                                    domain: row.get(1)?,
                                    description: row.get(2)?,
                                    trigger_keywords_json: row.get(3)?,
                                    required_tools_json: row.get(4)?,
                                    invariants_json: row.get(5)?,
                                    hooks_json: row.get(6)?,
                                    instructions_md: row.get(7)?,
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

    /// Searches curated skill and hook capsules with BM25 full-text ranking, optional domain filter,
    /// and clamped limit (default 10, max 50). Falls back to substring matching if FTS5 yields no results.
    pub fn search_skill_capsules(
        &self,
        query: Option<&str>,
        domain: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<SkillCapsuleRecord>, String> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;

        let lim = limit.unwrap_or(10).clamp(1, 50) as i64;
        let clean_query = query.map(|s| s.trim()).filter(|s| !s.is_empty());
        let clean_domain = domain.map(|s| s.trim()).filter(|s| !s.is_empty());

        let mut results = Vec::new();

        if let Some(q) = clean_query {
            if let Some(safe_fts) = build_safe_fts5_or_query(q) {
                if let Some(dom) = clean_domain {
                    let mut stmt = conn
                        .prepare(
                            "SELECT s.skill_id, s.domain, s.description, s.trigger_keywords_json, s.required_tools_json, s.invariants_json, s.hooks_json, s.instructions_md
                             FROM skills_fts f
                             JOIN skills s ON s.skill_id = f.skill_id
                             WHERE f.skills_fts MATCH ?1 AND s.domain = ?2
                             ORDER BY bm25(f.skills_fts)
                             LIMIT ?3",
                        )
                        .map_err(|e| e.to_string())?;

                    let rows = stmt
                        .query_map(params![safe_fts, dom, lim], |row| {
                            Ok(SkillCapsuleRecord {
                                skill_id: row.get(0)?,
                                domain: row.get(1)?,
                                description: row.get(2)?,
                                trigger_keywords_json: row.get(3)?,
                                required_tools_json: row.get(4)?,
                                invariants_json: row.get(5)?,
                                hooks_json: row.get(6)?,
                                instructions_md: row.get(7)?,
                                resolution_method: "fts5_bm25".to_string(),
                            })
                        })
                        .map_err(|e| e.to_string())?;

                    for r in rows {
                        if let Ok(rec) = r {
                            results.push(rec);
                        }
                    }
                } else {
                    let mut stmt = conn
                        .prepare(
                            "SELECT s.skill_id, s.domain, s.description, s.trigger_keywords_json, s.required_tools_json, s.invariants_json, s.hooks_json, s.instructions_md
                             FROM skills_fts f
                             JOIN skills s ON s.skill_id = f.skill_id
                             WHERE f.skills_fts MATCH ?1
                             ORDER BY bm25(f.skills_fts)
                             LIMIT ?2",
                        )
                        .map_err(|e| e.to_string())?;

                    let rows = stmt
                        .query_map(params![safe_fts, lim], |row| {
                            Ok(SkillCapsuleRecord {
                                skill_id: row.get(0)?,
                                domain: row.get(1)?,
                                description: row.get(2)?,
                                trigger_keywords_json: row.get(3)?,
                                required_tools_json: row.get(4)?,
                                invariants_json: row.get(5)?,
                                hooks_json: row.get(6)?,
                                instructions_md: row.get(7)?,
                                resolution_method: "fts5_bm25".to_string(),
                            })
                        })
                        .map_err(|e| e.to_string())?;

                    for r in rows {
                        if let Ok(rec) = r {
                            results.push(rec);
                        }
                    }
                }
            }

            // Fallback substring query if FTS5 produced 0 records
            if results.is_empty() {
                let like_term = format!("%{}%", q);
                if let Some(dom) = clean_domain {
                    let mut stmt = conn
                        .prepare(
                            "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, hooks_json, instructions_md
                             FROM skills
                             WHERE domain = ?1 AND (skill_id LIKE ?2 OR description LIKE ?2 OR trigger_keywords_json LIKE ?2)
                             ORDER BY skill_id ASC
                             LIMIT ?3",
                        )
                        .map_err(|e| e.to_string())?;

                    let rows = stmt
                        .query_map(params![dom, like_term, lim], |row| {
                            Ok(SkillCapsuleRecord {
                                skill_id: row.get(0)?,
                                domain: row.get(1)?,
                                description: row.get(2)?,
                                trigger_keywords_json: row.get(3)?,
                                required_tools_json: row.get(4)?,
                                invariants_json: row.get(5)?,
                                hooks_json: row.get(6)?,
                                instructions_md: row.get(7)?,
                                resolution_method: "substring_fallback".to_string(),
                            })
                        })
                        .map_err(|e| e.to_string())?;

                    for r in rows {
                        if let Ok(rec) = r {
                            results.push(rec);
                        }
                    }
                } else {
                    let mut stmt = conn
                        .prepare(
                            "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, hooks_json, instructions_md
                             FROM skills
                             WHERE skill_id LIKE ?1 OR description LIKE ?1 OR trigger_keywords_json LIKE ?1
                             ORDER BY skill_id ASC
                             LIMIT ?2",
                        )
                        .map_err(|e| e.to_string())?;

                    let rows = stmt
                        .query_map(params![like_term, lim], |row| {
                            Ok(SkillCapsuleRecord {
                                skill_id: row.get(0)?,
                                domain: row.get(1)?,
                                description: row.get(2)?,
                                trigger_keywords_json: row.get(3)?,
                                required_tools_json: row.get(4)?,
                                invariants_json: row.get(5)?,
                                hooks_json: row.get(6)?,
                                instructions_md: row.get(7)?,
                                resolution_method: "substring_fallback".to_string(),
                            })
                        })
                        .map_err(|e| e.to_string())?;

                    for r in rows {
                        if let Ok(rec) = r {
                            results.push(rec);
                        }
                    }
                }
            }
        } else if let Some(dom) = clean_domain {
            let mut stmt = conn
                .prepare(
                    "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, hooks_json, instructions_md
                     FROM skills
                     WHERE domain = ?1
                     ORDER BY skill_id ASC
                     LIMIT ?2",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map(params![dom, lim], |row| {
                    Ok(SkillCapsuleRecord {
                        skill_id: row.get(0)?,
                        domain: row.get(1)?,
                        description: row.get(2)?,
                        trigger_keywords_json: row.get(3)?,
                        required_tools_json: row.get(4)?,
                        invariants_json: row.get(5)?,
                        hooks_json: row.get(6)?,
                        instructions_md: row.get(7)?,
                        resolution_method: "domain_filter".to_string(),
                    })
                })
                .map_err(|e| e.to_string())?;

            for r in rows {
                if let Ok(rec) = r {
                    results.push(rec);
                }
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, hooks_json, instructions_md
                     FROM skills
                     ORDER BY skill_id ASC
                     LIMIT ?1",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map(params![lim], |row| {
                    Ok(SkillCapsuleRecord {
                        skill_id: row.get(0)?,
                        domain: row.get(1)?,
                        description: row.get(2)?,
                        trigger_keywords_json: row.get(3)?,
                        required_tools_json: row.get(4)?,
                        invariants_json: row.get(5)?,
                        hooks_json: row.get(6)?,
                        instructions_md: row.get(7)?,
                        resolution_method: "catalog_browse".to_string(),
                    })
                })
                .map_err(|e| e.to_string())?;

            for r in rows {
                if let Ok(rec) = r {
                    results.push(rec);
                }
            }
        }

        Ok(results)
    }

    /// Retrieves active hooks registered in the skills repository that match a given lifecycle event
    /// (e.g. `pre_verify`, `pre_commit`, `pre_gate_0`, `pre_gate_2`).
    pub fn get_hooks_for_event(
        &self,
        event: &str,
    ) -> Result<Vec<(String, AlexandriaHookSpec)>, String> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT skill_id, hooks_json FROM skills WHERE hooks_json != '[]' AND hooks_json != ''",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let skill_id: String = row.get(0)?;
                let hooks_json: String = row.get(1)?;
                Ok((skill_id, hooks_json))
            })
            .map_err(|e| e.to_string())?;

        let mut active_hooks = Vec::new();
        for r in rows {
            if let Ok((skill_id, hooks_json)) = r {
                if let Ok(hooks) = serde_json::from_str::<Vec<AlexandriaHookSpec>>(&hooks_json) {
                    for h in hooks {
                        let is_match = match event {
                            "pre_verify" => {
                                h.event == "pre_verify"
                                    || h.event == "pre_gate_0"
                                    || h.event == "pre_gate_2"
                                    || h.event == "pre_execution"
                            }
                            "pre_commit" => h.event == "pre_commit" || h.event == "post_gate_3",
                            other => h.event == other,
                        };
                        if is_match {
                            active_hooks.push((skill_id.clone(), h));
                        }
                    }
                }
            }
        }

        Ok(active_hooks)
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

    pub async fn sync_wiki_graph(
        &self,
        nodes: Vec<(String, String, String, String)>,
        edges: Vec<(String, String)>,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(VaultWriteCommand::SyncWikiGraph {
                nodes,
                edges,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "VaultWriterActor channel closed".to_string())?;
        reply_rx
            .await
            .map_err(|_| "VaultWriterActor oneshot reply cancelled".to_string())?
    }

    pub fn count_persisted_wiki_graph(&self) -> Result<(usize, usize), String> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;

        let node_count: usize = conn
            .query_row("SELECT COUNT(*) FROM wiki_nodes", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let edge_count: usize = conn
            .query_row("SELECT COUNT(*) FROM wiki_edges", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        Ok((node_count, edge_count))
    }

    pub async fn record_quality_gate_verdict(
        &self,
        module_name: String,
        execution_path: String,
        tdd_phase: String,
        passed: bool,
        diagnostics_json: String,
    ) -> Result<i64, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(VaultWriteCommand::RecordQualityGateVerdict {
                module_name,
                execution_path,
                tdd_phase,
                passed,
                diagnostics_json,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "VaultWriterActor channel closed".to_string())?;
        reply_rx
            .await
            .map_err(|_| "VaultWriterActor oneshot reply cancelled".to_string())?
    }

    pub async fn record_episodic_memory(
        &self,
        source_kind: String,
        agent_id: String,
        module_name: String,
        tags: Vec<String>,
        raw_text: String,
        max_chunk_tokens: usize,
    ) -> Result<String, String> {
        let valid_kind = match source_kind.as_str() {
            "user_supplied" | "agent_execution" | "architectural_decision" => source_kind,
            _ => "agent_execution".to_string(),
        };
        let (compact_summary, token_estimate) = distill_into_micro_chunk(&raw_text, max_chunk_tokens);
        let composite_text = format!("{} {} {}", module_name, tags.join(" "), compact_summary);
        let embedding_arr = compute_deterministic_int8_embedding(&composite_text);
        let embedding_int8_json =
            serde_json::to_string(&embedding_arr.to_vec()).unwrap_or_else(|_| "[]".to_string());
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(VaultWriteCommand::RecordEpisodicMemoryChunk {
                source_kind: valid_kind,
                agent_id,
                module_name,
                tags_json,
                compact_summary,
                token_estimate,
                embedding_int8_json,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "VaultWriterActor channel closed".to_string())?;
        reply_rx
            .await
            .map_err(|_| "VaultWriterActor oneshot reply cancelled".to_string())?
    }

    pub fn compute_organism_telemetry(&self) -> Result<serde_json::Value, String> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;

        let total_verdicts: usize = conn
            .query_row("SELECT COUNT(*) FROM quality_gate_verdicts", [], |r| r.get(0))
            .unwrap_or(0);
        let passed_verdicts: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM quality_gate_verdicts WHERE passed = 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let failed_verdicts = total_verdicts.saturating_sub(passed_verdicts);
        let bounce_rate = if total_verdicts > 0 {
            ((failed_verdicts as f64 / total_verdicts as f64) * 100.0 * 10.0).round() / 10.0
        } else {
            0.0
        };

        // Compute Mean Time To Repair (MTTR) in seconds across modules
        let mut mttr_samples_ms: Vec<f64> = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT module_name, passed, created_at FROM quality_gate_verdicts ORDER BY module_name, verdict_id ASC",
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            }) {
                let mut pending_fail_by_mod: std::collections::HashMap<String, i64> =
                    std::collections::HashMap::new();
                for (m_name, passed_flag, ts_ms) in rows.flatten() {
                    if passed_flag == 0 {
                        pending_fail_by_mod.entry(m_name).or_insert(ts_ms);
                    } else if let Some(fail_ts) = pending_fail_by_mod.remove(&m_name) {
                        let delta_ms = (ts_ms - fail_ts).max(1) as f64;
                        mttr_samples_ms.push(delta_ms);
                    }
                }
            }
        }

        let mttr_seconds = if !mttr_samples_ms.is_empty() {
            let avg_ms: f64 = mttr_samples_ms.iter().sum::<f64>() / (mttr_samples_ms.len() as f64);
            ((avg_ms / 1000.0) * 100.0).round() / 100.0
        } else {
            0.0
        };

        let memory_chunks: usize = conn
            .query_row("SELECT COUNT(*) FROM episodic_vector_memory", [], |r| r.get(0))
            .unwrap_or(0);
        let total_incidents: usize = conn
            .query_row("SELECT COUNT(*) FROM forensic_incidents", [], |r| r.get(0))
            .unwrap_or(0);
        let tripped_breakers: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM circuit_breakers WHERE state = 'tripped_abort'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let estimated_tokens_saved = (total_verdicts * 1_850) + (memory_chunks * 1_400);
        let estimated_usd_saved =
            (((estimated_tokens_saved as f64) / 1_000_000.0) * 15.0 * 1000.0).round() / 1000.0;

        Ok(json!({
            "schemaVersion": "alexandria.telemetry.v2.5",
            "totalVerdicts": total_verdicts,
            "passedVerdicts": passed_verdicts,
            "failedVerdicts": failed_verdicts,
            "adversarialBounceRatePercent": bounce_rate,
            "meanTimeToRepairSeconds": mttr_seconds,
            "forensicIncidentsCount": total_incidents,
            "trippedCircuitBreakers": tripped_breakers,
            "episodicMemoryChunks": memory_chunks,
            "estimatedTokensSaved": estimated_tokens_saved,
            "estimatedUsdSaved": estimated_usd_saved
        }))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

pub fn write_telemetry_and_markdown_dashboard(
    storage: &VaultStorageHandle,
    workspace_root: &Path,
    vault_dir: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    let telemetry = storage.compute_organism_telemetry()?;

    let telemetry_dir = workspace_root.join(".alexandria").join("telemetry");
    fs::create_dir_all(&telemetry_dir)
        .map_err(|e| format!("Failed to create telemetry directory {:?}: {}", telemetry_dir, e))?;
    let health_json_path = telemetry_dir.join("organism-health.json");
    let serialized = serde_json::to_string_pretty(&telemetry).map_err(|e| e.to_string())?;
    fs::write(&health_json_path, serialized)
        .map_err(|e| format!("Failed to write {:?}: {}", health_json_path, e))?;

    fs::create_dir_all(vault_dir)
        .map_err(|e| format!("Failed to create vault directory {:?}: {}", vault_dir, e))?;
    let dashboard_path = vault_dir.join("dashboard.md");

    let total_v = telemetry["totalVerdicts"].as_u64().unwrap_or(0);
    let passed_v = telemetry["passedVerdicts"].as_u64().unwrap_or(0);
    let failed_v = telemetry["failedVerdicts"].as_u64().unwrap_or(0);
    let bounce_pct = telemetry["adversarialBounceRatePercent"]
        .as_f64()
        .unwrap_or(0.0);
    let mttr_sec = telemetry["meanTimeToRepairSeconds"].as_f64().unwrap_or(0.0);
    let tokens_saved = telemetry["estimatedTokensSaved"].as_u64().unwrap_or(0);
    let usd_saved = telemetry["estimatedUsdSaved"].as_f64().unwrap_or(0.0);
    let mem_chunks = telemetry["episodicMemoryChunks"].as_u64().unwrap_or(0);
    let tripped = telemetry["trippedCircuitBreakers"].as_u64().unwrap_or(0);

    let dashboard_md = format!(
        "---\nid: dashboard\ntitle: \"Alexandria Organism Telemetry Dashboard\"\ntype: telemetry-dashboard\nstatus: canonical\nversion: \"1.0.0\"\ntotal_verdicts: {}\npassed_verdicts: {}\nfailed_verdicts: {}\nadversarial_bounce_rate_pct: {}\nmttr_seconds: {}\nestimated_tokens_saved: {}\nestimated_usd_saved: {}\n---\n\n# Alexandria Organism Telemetry Dashboard\n\n> **Live Kernel Health Projection** — Compatible with Obsidian Dataview & standard Markdown viewers.\n\n## 1. Key Organism Vital Signs (`MTTR` & Token ROI)\n\n| Metric | Value | Status |\n| :--- | :---: | :--- |\n| **Total Quality Gate Verdicts** | `{}` | Active |\n| **Passed / Failed Verdicts** | `{} / {}` | Gate 0-4 Enforced |\n| **Adversarial Bounce Rate (Utu vs. Enki)** | `{:.1}%` | Zero-Trust SAST |\n| **Mean Time To Repair (`MTTR`)** | `{:.2}s` | Self-Healing Loop |\n| **Tripped Circuit Breakers** | `{}` | Max Retries = 3 |\n| **Hybrid Vector Memory Chunks (`RRF`)** | `{}` | 64-dim `int8` |\n| **Estimated Context Tokens Saved** | `{}` | Depth=1 + AST Pruning |\n| **Estimated Compute Cost Saved (`USD`)** | `${:.3}` | Prefix-Pinned KV-Cache |\n\n## 2. Pantheon Governance Roster\n\n| Agent | Archetype | Concurrency Limit | Role |\n| :--- | :--- | :---: | :--- |\n| **`bel`** | Bel (The Overseer) | `5` | Zero-Turn Classifier & Court-Martial Judge |\n| **`enki`** | Enki (The Builder) | `3` | Strict RED-GREEN-REFACTOR Constructor |\n| **`utu`** | Utu (The Critic) | `1` | Stateless Cold SAST & ADR Auditor |\n| **`nabu`** | Nabu (The Chronicler) | `1` | Vault Custodian & Agentic Git IAM Signer |\n",
        total_v,
        passed_v,
        failed_v,
        bounce_pct,
        mttr_sec,
        tokens_saved,
        usd_saved,
        total_v,
        passed_v,
        failed_v,
        bounce_pct,
        mttr_sec,
        tripped,
        mem_chunks,
        tokens_saved,
        usd_saved
    );

    fs::write(&dashboard_path, dashboard_md)
        .map_err(|e| format!("Failed to write {:?}: {}", dashboard_path, e))?;

    Ok((health_json_path, dashboard_path))
}
