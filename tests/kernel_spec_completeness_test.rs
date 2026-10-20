use alexandria::gate_guard::oxc_validator::{
    detect_public_export_signature_change, extract_public_export_signatures,
    scan_gate2_sast_and_secrets,
};
use alexandria::graph::resolver::{
    DisjunctionConflict, load_vault_graph_from_dir, resolve_mutable_import_footprint,
    start_vault_debounced_watcher, verify_worktree_disjunction,
};
use alexandria::mcp::server::{McpKernelContext, dispatch_alexandria_tool_with_context};
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::VaultStorageHandle;
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SPEC_TEST_SEQ: AtomicUsize = AtomicUsize::new(1);

fn create_temp_spec_dir(label: &str) -> PathBuf {
    let seq = SPEC_TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "alexandria_spec_test_{}_{}_{}_{}",
        label,
        std::process::id(),
        nanos,
        seq
    ));
    fs::create_dir_all(&dir).expect("Temporary spec test directory must be created");
    dir
}

fn parse_mcp_payload(resp_result: Option<Value>) -> Value {
    let res = resp_result.expect("MCP response must contain result");
    let text = res["content"][0]["text"]
        .as_str()
        .expect("Serialized JSON text");
    serde_json::from_str(text).expect("Valid tool JSON payload")
}

#[tokio::test]
async fn test_wiki_graph_and_verdict_persistence_in_sqlite() {
    let temp_dir = create_temp_spec_dir("wiki_and_verdicts");
    let db_path = temp_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("VaultStorageHandle initialized");

    // 1. Persist wiki_nodes and wiki_edges via sync_wiki_graph
    let nodes = vec![
        (
            "billing".to_string(),
            "modules/billing.md".to_string(),
            "Module".to_string(),
            "a1b2c3d4".to_string(),
        ),
        (
            "ADR-004-single-writer-mpsc-storage".to_string(),
            "decisions/ADR-004-single-writer-mpsc-storage.md".to_string(),
            "ConstitutionalAdr".to_string(),
            "e5f6a7b8".to_string(),
        ),
    ];
    let edges = vec![(
        "billing".to_string(),
        "ADR-004-single-writer-mpsc-storage".to_string(),
    )];

    storage
        .sync_wiki_graph(nodes, edges)
        .await
        .expect("sync_wiki_graph must commit cleanly");

    let (node_count, edge_count) = storage
        .count_persisted_wiki_graph()
        .expect("count_persisted_wiki_graph must succeed");
    assert_eq!(node_count, 2, "Two wiki_nodes must be persisted in SQLite");
    assert_eq!(edge_count, 1, "One wiki_edge must be persisted in SQLite");

    // 2. Persist quality_gate_verdicts via record_quality_gate_verdict
    let verdict_id = storage
        .record_quality_gate_verdict(
            "billing".to_string(),
            "alpha_fast".to_string(),
            "green_expect_pass".to_string(),
            true,
            r#"{"gate0":"ok","gate2":"ok"}"#.to_string(),
        )
        .await
        .expect("record_quality_gate_verdict must commit cleanly");
    assert!(verdict_id >= 1, "Inserted verdict_id must be >= 1");

    let ro_conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Read-only connection open");
    let stored_verdict: (String, i64) = ro_conn
        .query_row(
            "SELECT module_name, passed FROM quality_gate_verdicts WHERE verdict_id = ?1",
            [verdict_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("Verdict row must exist in quality_gate_verdicts");
    assert_eq!(stored_verdict.0, "billing");
    assert_eq!(stored_verdict.1, 1);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_syscall_trap_on_public_export_signature_change() {
    let baseline = r#"
export interface InvoiceDto {
    id: string;
}
export function chargeCustomer(amountCents: number): boolean {
    return amountCents > 0;
}
"#;

    let internal_only_change = r#"
export interface InvoiceDto {
    id: string;
}
export function chargeCustomer(amountCents: number): boolean {
    const minimum = 50;
    return amountCents >= minimum;
}
"#;

    let signature_mutation = r#"
export interface InvoiceDto {
    id: string;
}
export function chargeCustomer(amountCents: number, currencyIso: string): boolean {
    return amountCents > 0 && currencyIso.length === 3;
}
"#;

    let base_sigs =
        extract_public_export_signatures(baseline, false).expect("Baseline signatures extracted");
    assert!(
        base_sigs.contains(&"export function chargeCustomer(1)".to_string()),
        "Expected 'export function chargeCustomer(1)' in {:?}",
        base_sigs
    );
    assert!(base_sigs.contains(&"export interface InvoiceDto".to_string()));

    let changed_internal =
        detect_public_export_signature_change(baseline, internal_only_change, false)
            .expect("Internal comparison succeeded");
    assert!(
        !changed_internal,
        "Modifying only the function body must not trigger a public signature change"
    );

    let changed_public = detect_public_export_signature_change(baseline, signature_mutation, false)
        .expect("Public signature comparison succeeded");
    assert!(
        changed_public,
        "Adding a parameter to an exported function must trigger a public signature change"
    );

    // Verify end-to-end Syscall Trap inside MCP verify_and_gate_changes
    let temp_dir = create_temp_spec_dir("syscall_trap");
    let db_path = temp_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL).unwrap();
    let ctx = McpKernelContext::new(temp_dir.clone(), temp_dir.join("wiki"), Some(storage));

    let resp = dispatch_alexandria_tool_with_context(
        "verify_and_gate_changes",
        json!({
            "targetModule": "billing",
            "targetFiles": ["src/billing/charge.ts"],
            "executionPath": "alpha_fast",
            "tddPhase": "green_expect_pass",
            "baselineSource": baseline,
            "sourceCode": signature_mutation
        }),
        json!(501),
        &ctx,
    )
    .await;

    let payload = parse_mcp_payload(resp.result);
    assert_eq!(
        payload["syscallTrapTriggered"],
        json!(true),
        "Mutating public export signature in alpha_fast must trigger Syscall Trap"
    );
    assert_eq!(payload["promotedExecutionPath"], json!("omega_pantheon"));
    assert!(
        payload["verdictId"].as_i64().is_some_and(|id| id >= 1),
        "verify_and_gate_changes must record verdictId in quality_gate_verdicts"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_gate2_sast_and_secret_scanner() {
    let unsafe_code = r#"
export function runDiagnostics(userInput: string): string {
    const githubToken = "ghp_1234567890abcdefghijklmnopqrstuvwxyz";
    const awsKey = "AKIAIOSFODNN7EXAMPLE";
    eval(userInput);
    return githubToken + awsKey;
}
"#;

    let gate2_violations = scan_gate2_sast_and_secrets(unsafe_code);
    assert!(
        gate2_violations
            .iter()
            .any(|v| v.rule == "GATE2_HARDCODED_SECRET"),
        "Gate 2 must detect hardcoded secrets (ghp_, AKIA)"
    );
    assert!(
        gate2_violations
            .iter()
            .any(|v| v.rule == "GATE2_SAST_UNSAFE_EXECUTION"),
        "Gate 2 must detect unsafe dynamic code execution (eval)"
    );

    let temp_dir = create_temp_spec_dir("gate2_sast");
    let ctx = McpKernelContext::new(temp_dir.clone(), temp_dir.join("wiki"), None);

    let resp = dispatch_alexandria_tool_with_context(
        "verify_and_gate_changes",
        json!({
            "targetModule": "security_mod",
            "targetFiles": ["src/unsafe.ts"],
            "executionPath": "alpha_fast",
            "tddPhase": "green_expect_pass",
            "sourceCode": unsafe_code
        }),
        json!(502),
        &ctx,
    )
    .await;

    let payload = parse_mcp_payload(resp.result);
    assert_eq!(payload["passed"], json!(false));
    let rules = payload["violatedRules"].as_array().unwrap();
    assert!(rules.contains(&json!("GATE2_HARDCODED_SECRET")));
    assert!(rules.contains(&json!("GATE2_SAST_UNSAFE_EXECUTION")));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_oxc_resolver_transitive_import_collision_in_worktrees() {
    let temp_dir = create_temp_spec_dir("oxc_resolver_worktree");
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir).expect("Created src dir");

    let shared_path = src_dir.join("shared.ts");
    let file_a = src_dir.join("a.ts");
    let file_b = src_dir.join("b.ts");

    fs::write(
        &shared_path,
        "export function helper(): number { return 42; }\n",
    )
    .expect("Wrote shared.ts");
    fs::write(
        &file_a,
        "import { helper } from './shared';\nexport function runA(): number { return helper(); }\n",
    )
    .expect("Wrote a.ts");
    fs::write(
        &file_b,
        "import { helper } from './shared';\nexport function runB(): number { return helper() + 1; }\n",
    )
    .expect("Wrote b.ts");

    let footprint_a = resolve_mutable_import_footprint(&[file_a.clone()]);
    assert!(
        footprint_a.len() >= 2,
        "Footprint of a.ts must include both a.ts and resolved ./shared.ts via oxc_resolver"
    );

    let (graph, node_map) = load_vault_graph_from_dir(&temp_dir.join("wiki"));
    let tasks = vec![
        ("worker_a_mod".to_string(), vec![file_a]),
        ("worker_b_mod".to_string(), vec![file_b]),
    ];

    let verdict = verify_worktree_disjunction(&graph, &node_map, &tasks);
    match verdict {
        Err(DisjunctionConflict::AstImportCollision { colliding_files }) => {
            assert!(
                colliding_files.iter().any(|f| f.contains("shared.ts")),
                "Collision must identify shared.ts resolved via oxc_resolver: {:?}",
                colliding_files
            );
        }
        other => panic!(
            "Expected Err(DisjunctionConflict::AstImportCollision) on shared.ts, got: {:?}",
            other
        ),
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_vault_debounced_watcher_initialization() {
    let temp_dir = create_temp_spec_dir("debounced_watcher");
    let wiki_dir = temp_dir.join("wiki");
    fs::create_dir_all(wiki_dir.join("modules")).expect("Created wiki/modules");
    fs::write(wiki_dir.join("index.md"), "# Vault Index\n").expect("Wrote index.md");

    let debouncer = start_vault_debounced_watcher(wiki_dir, 75)
        .expect("start_vault_debounced_watcher must initialize cleanly with 75ms debounce");
    drop(debouncer);

    let _ = fs::remove_dir_all(&temp_dir);
}
