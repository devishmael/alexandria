use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::{VaultStorageHandle, build_safe_fts5_or_query};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn temp_vault_db_path(test_label: &str) -> PathBuf {
    let seq = TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut dir = std::env::temp_dir();
    dir.push(format!("alexandria_test_{}_{}_{}.db", test_label, nanos, seq));
    dir
}

fn cleanup_db_files(db_path: &PathBuf) {
    let _ = std::fs::remove_file(db_path);
    let wal = PathBuf::from(format!("{}-wal", db_path.display()));
    let shm = PathBuf::from(format!("{}-shm", db_path.display()));
    let _ = std::fs::remove_file(wal);
    let _ = std::fs::remove_file(shm);
}

#[tokio::test]
async fn test_a_schema_initialization_and_symmetric_fts5_triggers() {
    let db_path = temp_vault_db_path("schema_triggers");
    let _handle = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("El VaultStorageHandle debe inicializar vault.db sin errores");

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Debe abrirse conexión de solo lectura en modo WAL");

    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'trigger' ORDER BY name")
        .expect("Consulta a sqlite_master válida");

    let triggers: HashSet<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("Mapeo de triggers válido")
        .filter_map(Result::ok)
        .collect();

    for expected in [
        "skills_ai",
        "skills_ad",
        "skills_au",
        "incidents_ai",
        "incidents_ad",
        "incidents_au",
    ] {
        assert!(
            triggers.contains(expected),
            "Falta el trigger simétrico FTS5 obligatorio: {}",
            expected
        );
    }

    cleanup_db_files(&db_path);
}

#[tokio::test]
async fn test_b_massive_mpsc_concurrency_without_sqlite_busy() {
    let db_path = temp_vault_db_path("mpsc_concurrency");
    let handle = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("Debe iniciar actor MPSC");

    let total_tasks = 64usize;
    let mut join_handles = Vec::with_capacity(total_tasks);

    for i in 0..total_tasks {
        let h = handle.clone();
        join_handles.push(tokio::spawn(async move {
            let module_name = format!("module_concurrent_{}", i % 4);
            h.record_gate_failure(
                module_name,
                "Gate 0 (Oxc AST)".to_string(),
                format!("TSAnyKeyword violation at byte offset {}", i * 10),
                3,
            )
            .await
        }));
    }

    for jh in join_handles {
        let res = jh
            .await
            .expect("La tarea de Tokio no debe entrar en pánico")
            .expect("Ninguna operación debe fallar por SQLITE_BUSY");
        let (_failure_id, state, count) = res;
        assert!(
            matches!(state.as_str(), "retry_allowed" | "tripped_abort"),
            "Estado del circuit breaker inesperado: {}",
            state
        );
        assert!(count >= 1);
    }

    cleanup_db_files(&db_path);
}

#[tokio::test]
async fn test_c_monotonic_unique_ids_under_same_millisecond_burst() {
    let db_path = temp_vault_db_path("monotonic_seq");
    let handle = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("Debe iniciar actor MPSC");

    let burst_size = 32usize;
    let mut futures = Vec::with_capacity(burst_size);
    for _ in 0..burst_size {
        futures.push(handle.record_gate_failure(
            "burst_module".to_string(),
            "Gate 1".to_string(),
            "tsc error TS2322".to_string(),
            10,
        ));
    }

    let mut unique_ids = HashSet::with_capacity(burst_size);
    for fut in futures {
        let (fid, _, _) = fut.await.expect("Registro en ráfaga exitoso");
        assert!(
            unique_ids.insert(fid.clone()),
            "ID duplicado en el mismo milisegundo: {}",
            fid
        );
    }
    assert_eq!(unique_ids.len(), burst_size);

    cleanup_db_files(&db_path);
}

#[tokio::test]
async fn test_d_post_commit_oneshot_consistency_and_circuit_breaker_trip_reset() {
    let db_path = temp_vault_db_path("post_commit");
    let handle = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("Debe iniciar actor MPSC");

    // 1er fallo -> retry_allowed (1)
    let (fid1, state1, count1) = handle
        .record_gate_failure(
            "sandbox_runner".into(),
            "Gate 3".into(),
            "Error [ERR_ACCESS_DENIED]: FileSystemRead \\\\?\\C:\\Github\\vitest.mjs".into(),
            3,
        )
        .await
        .expect("Fallo 1 registrado");
    assert_eq!(state1, "retry_allowed");
    assert_eq!(count1, 1);

    // 2do fallo -> retry_allowed (2)
    let (_fid2, state2, count2) = handle
        .record_gate_failure(
            "sandbox_runner".into(),
            "Gate 3".into(),
            "Error [ERR_ACCESS_DENIED]: FileSystemRead \\\\?\\C:\\Github\\vitest.mjs".into(),
            3,
        )
        .await
        .expect("Fallo 2 registrado");
    assert_eq!(state2, "retry_allowed");
    assert_eq!(count2, 2);

    // 3er fallo -> tripped_abort (3)
    let (fid3, state3, count3) = handle
        .record_gate_failure(
            "sandbox_runner".into(),
            "Gate 3".into(),
            "Error [ERR_ACCESS_DENIED]: FileSystemRead \\\\?\\C:\\Github\\vitest.mjs".into(),
            3,
        )
        .await
        .expect("Fallo 3 registrado");
    assert_eq!(state3, "tripped_abort");
    assert_eq!(count3, 3);
    assert_ne!(fid1, fid3);

    let ro_conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Apertura Read-Only inmediata");
    let db_state: String = ro_conn
        .query_row(
            "SELECT state FROM circuit_breakers WHERE module_name = 'sandbox_runner'",
            [],
            |r| r.get(0),
        )
        .expect("Debe existir fila confirmada en circuit_breakers");
    assert_eq!(db_state, "tripped_abort");

    handle
        .reset_circuit_breaker(
            "sandbox_runner".into(),
            Some(fid3.clone()),
            "Limpiar prefijo UNC \\\\?\\ con clean_canonical_path antes de --allow-fs-read".into(),
        )
        .await
        .expect("Reset y actualización de pista deben confirmarse sin SQLITE_CORRUPT_VTAB");

    let db_state_after: (String, u32) = ro_conn
        .query_row(
            "SELECT state, consecutive_failures FROM circuit_breakers WHERE module_name = 'sandbox_runner'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("Lectura post-reset válida");
    assert_eq!(db_state_after.0, "closed_ok");
    assert_eq!(db_state_after.1, 0);

    cleanup_db_files(&db_path);
}

#[tokio::test]
async fn test_e_disjunctive_fts5_search_and_special_char_sanitization() {
    let db_path = temp_vault_db_path("fts5_search");
    let handle = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("Debe iniciar actor MPSC");

    let raw_tricky_trace =
        r#"Error: AND OR NOT NEAR "unbalanced:quote" at C:\Github\src\main.rs:42:10 (PRISMA_TX_DANGLING_PROMISE)"#;

    let sanitized = build_safe_fts5_or_query(raw_tricky_trace)
        .expect("El sanitizador debe producir una consulta FTS5 disyuntiva válida");
    assert!(!sanitized.contains("\"AND\""));
    assert!(!sanitized.contains("\"OR\""));
    assert!(!sanitized.contains("\"NOT\""));
    assert!(!sanitized.contains("\"NEAR\""));
    assert!(sanitized.contains("\"PRISMA_TX_DANGLING_PROMISE\""));

    let (fid, _, _) = handle
        .record_gate_failure(
            "gate_guard".into(),
            "Gate 0".into(),
            raw_tricky_trace.into(),
            3,
        )
        .await
        .expect("Debe registrar incidente");

    handle
        .reset_circuit_breaker(
            "gate_guard".into(),
            Some(fid.clone()),
            "Asegurar await inmediato en toda llamada a tx.<modelo>.<op>()".into(),
        )
        .await
        .expect("Debe registrar resolution_hint");

    let hints = handle
        .search_similar_incidents_fts5(
            "Fallo en Gate 0: detectado PRISMA_TX_DANGLING_PROMISE en C:\\Github\\src\\other.ts:99",
        )
        .expect("La consulta FTS5 BM25 no debe fallar por sintaxis ni corrupción");

    assert_eq!(hints.len(), 1);
    assert!(hints[0].contains(&fid));
    assert!(hints[0].contains("Asegurar await inmediato"));

    cleanup_db_files(&db_path);
}
