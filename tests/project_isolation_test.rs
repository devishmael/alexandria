use alexandria::cli::resolve_vault_db_path;
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::VaultStorageHandle;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(200);

fn temp_test_dir(label: &str) -> PathBuf {
    let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("alexandria_iso_{}_{}_{}", label, nanos, seq));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_resolve_vault_db_path_prefers_local_project_root() {
    let project_dir = temp_test_dir("proj_local");
    // Ensure the env var is not interfering
    unsafe {
        std::env::remove_var("ALEXANDRIA_VAULT_DB");
    }

    let resolved = resolve_vault_db_path(Some(&project_dir))
        .expect("Must resolve project-local vault path");

    let expected = project_dir.join(".alexandria").join("vault.db");
    assert_eq!(resolved, expected, "Database path must be scoped to the project root");
    assert!(
        project_dir.join(".alexandria").exists(),
        ".alexandria directory must be created inside project root"
    );

    let _ = fs::remove_dir_all(&project_dir);
}

#[tokio::test]
async fn test_isolated_projects_have_independent_episodic_memory() {
    unsafe {
        std::env::remove_var("ALEXANDRIA_VAULT_DB");
    }

    let proj_a_dir = temp_test_dir("proj_a");
    let proj_b_dir = temp_test_dir("proj_b");

    let db_path_a = resolve_vault_db_path(Some(&proj_a_dir)).unwrap();
    let db_path_b = resolve_vault_db_path(Some(&proj_b_dir)).unwrap();

    let storage_a = VaultStorageHandle::spawn(db_path_a, CANONICAL_SCHEMA_SQL)
        .expect("Project A storage must initialize");
    let storage_b = VaultStorageHandle::spawn(db_path_b, CANONICAL_SCHEMA_SQL)
        .expect("Project B storage must initialize");

    let initial_a = storage_a.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);
    let initial_b = storage_b.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);

    // Record an episodic memory in Project A only
    storage_a
        .record_episodic_memory(
            "user_supplied".into(),
            "architect".into(),
            "billing_service".into(),
            vec!["stripe".into(), "credit_card".into()],
            "Stripe tokenized billing integration secret key rotation policy".into(),
            50,
        )
        .await
        .expect("Failed to record memory in Project A");

    // Verify Project A contains the new memory
    let telemetry_a = storage_a.compute_organism_telemetry().unwrap();
    let current_a = telemetry_a["episodicMemoryChunks"].as_u64().unwrap_or(0);
    assert_eq!(
        current_a,
        initial_a + 1,
        "Project A must have exactly 1 additional episodic memory chunk"
    );

    // Verify Project B is completely clean and isolated from Project A's mutations!
    let telemetry_b = storage_b.compute_organism_telemetry().unwrap();
    let current_b = telemetry_b["episodicMemoryChunks"].as_u64().unwrap_or(0);
    assert_eq!(
        current_b,
        initial_b,
        "Project B count MUST NOT change when Project A receives a memory (Zero Cross-Project Pollution)"
    );

    let _ = fs::remove_dir_all(&proj_a_dir);
    let _ = fs::remove_dir_all(&proj_b_dir);
}
