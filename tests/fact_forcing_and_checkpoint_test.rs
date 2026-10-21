use alexandria::gate_guard::fact_forcing::{FactForcingError, FactForcingTracker};
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::{SessionCheckpoint, VaultStorageHandle};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(100);

fn temp_test_dir(label: &str) -> PathBuf {
    let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("alexandria_test_{}_{}_{}", label, nanos, seq));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_fact_forcing_gate_rejects_uninspected_file_mutation() {
    let dir = temp_test_dir("fact_reject");
    let target_file = dir.join("existing_service.ts");
    fs::write(&target_file, "export const auth = () => true;").expect("Failed to write mock file");

    let tracker = FactForcingTracker::new();

    // The file exists on disk, but has NOT been inspected in this session.
    let result = tracker.verify_mutation(&target_file);

    assert!(
        result.is_err(),
        "Fact-Forcing Gate MUST reject mutation on an existing file that was not inspected"
    );

    match result {
        Err(FactForcingError::UninspectedFileMutation { path, message }) => {
            assert_eq!(path, target_file);
            assert!(
                message.contains("E_FACT_FORCING"),
                "Error message must include canonical E_FACT_FORCING error code"
            );
        }
        Ok(_) => panic!("Fact-Forcing Gate MUST reject uninspected mutation"),
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_fact_forcing_gate_allows_inspected_file_mutation() {
    let dir = temp_test_dir("fact_allow");
    let target_file = dir.join("existing_service.ts");
    fs::write(&target_file, "export const auth = () => true;").expect("Failed to write mock file");

    let mut tracker = FactForcingTracker::new();

    // Record that the agent inspected/read the file
    tracker.record_read(&target_file);

    // Now mutation must be allowed
    let result = tracker.verify_mutation(&target_file);
    assert!(result.is_ok(), "Fact-Forcing Gate MUST permit mutation once file has been read");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_fact_forcing_gate_allows_new_file_creation() {
    let dir = temp_test_dir("fact_new");
    let non_existent_file = dir.join("brand_new_component.tsx");

    let tracker = FactForcingTracker::new();

    // New file does not exist on disk, creating it is permitted without prior read
    let result = tracker.verify_mutation(&non_existent_file);
    assert!(result.is_ok(), "Fact-Forcing Gate permits creation of brand new files");
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_storage_actor_session_checkpoint_persistence_and_retrieval() {
    let dir = temp_test_dir("session_chk");
    let db_path = dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("Failed to initialize storage handle");

    let checkpoint = SessionCheckpoint {
        checkpoint_id: "chk_test_001".into(),
        session_id: "sess_alpha_99".into(),
        trigger_kind: "pre_compact".into(),
        active_adrs_json: "[\"ADR-001\", \"ADR-008\"]".into(),
        modified_files_json: "[\"src/main.rs\", \"src/lib.rs\"]".into(),
        test_status_json: "{\"passed\": 48, \"failed\": 0}".into(),
        summary_markdown: "## Pre-Compact Checkpoint Summary\nSession was at 80% token threshold.".into(),
        created_at: 1774000000,
    };

    storage
        .save_session_checkpoint(checkpoint.clone())
        .await
        .expect("Failed to save session checkpoint");

    let retrieved = storage
        .get_latest_session_checkpoint("sess_alpha_99".into())
        .await
        .expect("Failed to fetch session checkpoint");

    assert!(retrieved.is_some(), "Checkpoint must be persisted and retrievable");
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.checkpoint_id, "chk_test_001");
    assert_eq!(retrieved.trigger_kind, "pre_compact");
    assert_eq!(retrieved.summary_markdown, checkpoint.summary_markdown);
    let _ = fs::remove_dir_all(&dir);
}
