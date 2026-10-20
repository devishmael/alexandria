use alexandria::gate_guard::oxc_validator::prune_typescript_context_for_symbol;
use alexandria::lsp::server::handle_lsp_jsonrpc_message;
use alexandria::mcp::server::{
    McpKernelContext, build_agentic_git_iam_seal, dispatch_alexandria_tool_with_context,
};
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::{VaultStorageHandle, write_telemetry_and_markdown_dashboard};
use alexandria::vector_store::hybrid_memory::{
    compute_deterministic_int8_embedding, cosine_similarity_int8, export_vector_store_snapshot,
    query_hybrid_vector_memory,
};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static V25_TEST_SEQ: AtomicUsize = AtomicUsize::new(1);

fn create_temp_v25_dir(label: &str) -> PathBuf {
    let seq = V25_TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "alexandria_v25_test_{}_{}_{}_{}",
        label,
        std::process::id(),
        nanos,
        seq
    ));
    fs::create_dir_all(&dir).expect("Temporary v2.5 test directory must be created");
    dir
}

fn unwrap_mcp_json(resp_result: Option<Value>) -> Value {
    let res = resp_result.expect("MCP response must contain result");
    let text = res["content"][0]["text"]
        .as_str()
        .expect("Serialized JSON text");
    serde_json::from_str(text).expect("Valid tool JSON payload")
}

#[tokio::test]
async fn test_hybrid_vector_memory_rrf_and_token_budget() {
    let temp_dir = create_temp_v25_dir("hybrid_vector");
    let db_path = temp_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)
        .expect("VaultStorageHandle initialized");

    let vec_a = compute_deterministic_int8_embedding("prisma transaction lexical closure acid");
    let vec_b = compute_deterministic_int8_embedding("prisma transaction lexical closure acid");
    assert_eq!(vec_a, vec_b, "Embedding must be 100% deterministic");
    let self_sim = cosine_similarity_int8(&vec_a, &vec_b);
    assert!(
        self_sim > 0.99,
        "Cosine similarity of identical non-empty text must be ~1.0, got {}",
        self_sim
    );

    // Insert multiple episodic memories (user-supplied and agent-executed)
    let id1 = storage
        .record_episodic_memory(
            "user_supplied".to_string(),
            "bel".to_string(),
            "billing".to_string(),
            vec!["prisma".to_string(), "transaction".to_string()],
            "User requested strict lexical closure inside Prisma interactive transactions for billing settlement under 25ms.".to_string(),
            60,
        )
        .await
        .expect("Memory 1 recorded");
    assert!(!id1.is_empty());

    let _id2 = storage
        .record_episodic_memory(
            "agent_execution".to_string(),
            "enki".to_string(),
            "billing".to_string(),
            vec!["vitest".to_string(), "prisma".to_string()],
            "Enki refactored src/billing/settlement.ts to eliminate tx aliasing and verified RED->GREEN TDD in Vitest.".to_string(),
            60,
        )
        .await
        .expect("Memory 2 recorded");

    let _id3 = storage
        .record_episodic_memory(
            "agent_execution".to_string(),
            "enki".to_string(),
            "ui_theme".to_string(),
            vec!["tailwind".to_string(), "css".to_string()],
            "Adjusted dark mode contrast tokens on primary navigation header component.".to_string(),
            60,
        )
        .await
        .expect("Memory 3 recorded");

    // Query with strict token budget
    let recalled = query_hybrid_vector_memory(&storage, "prisma transaction billing closure", 45)
        .expect("Hybrid RRF query succeeded");
    assert!(
        !recalled.is_empty(),
        "RRF hybrid query must return relevant memory chunks"
    );
    assert_eq!(
        recalled[0].module_name, "billing",
        "Top RRF match must belong to the billing module"
    );
    let total_tokens: usize = recalled.iter().map(|r| r.token_estimate).sum();
    assert!(
        total_tokens <= 45,
        "Recalled chunks must strictly respect max_token_budget (45), got {}",
        total_tokens
    );

    // Export snapshot to .alexandria/vector-store/memory-index.json
    let vector_dir = temp_dir.join(".alexandria").join("vector-store");
    let index_path = export_vector_store_snapshot(&storage, &vector_dir)
        .expect("Vector store snapshot exported");
    assert!(
        index_path.exists(),
        ".alexandria/vector-store/memory-index.json must exist"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_ast_context_pruning_for_target_symbol() {
    let ts_source = r#"import { CurrencyCode } from './types';

export interface OrderInput {
    id: string;
    amountCents: number;
    currency: CurrencyCode;
}

function validateAmount(cents: number): boolean {
    return Number.isInteger(cents) && cents > 0;
}

export function processOrder(input: OrderInput): boolean {
    if (!validateAmount(input.amountCents)) {
        return false;
    }
    return true;
}

export function unrelatedLegacyReport(year: number): string {
    const lines: string[] = [];
    for (let m = 1; m <= 12; m++) {
        lines.push(`Month ${m} of ${year}: archived ledger summary entry with verbose padding`);
    }
    return lines.join('\n');
}
"#;

    let pruned = prune_typescript_context_for_symbol(ts_source, "processOrder", false)
        .expect("AST context pruning must succeed");

    assert_eq!(pruned.target_symbol, "processOrder");
    assert!(
        pruned.retained_symbols.contains(&"processOrder".to_string()),
        "Target function processOrder must be retained"
    );
    assert!(
        pruned
            .retained_symbols
            .contains(&"validateAmount".to_string()),
        "Direct local helper validateAmount called by processOrder must be retained"
    );
    assert!(
        pruned
            .pruned_symbols
            .contains(&"unrelatedLegacyReport".to_string()),
        "Unrelated function unrelatedLegacyReport must be pruned"
    );
    assert!(
        pruned
            .pruned_source
            .contains("/* [Alexandria AST Pruned] */"),
        "Pruned source must replace unrelated function body with stub marker"
    );
    assert!(
        pruned.pruned_bytes < pruned.original_bytes,
        "Pruned source ({}) must be smaller than original source ({})",
        pruned.pruned_bytes,
        pruned.original_bytes
    );
    assert!(pruned.estimated_tokens_saved > 0);
}

#[tokio::test]
async fn test_court_martial_docket_on_third_failure_and_agentic_git_iam() {
    let temp_dir = create_temp_v25_dir("court_martial_iam");
    let db_path = temp_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL).unwrap();
    let ctx = McpKernelContext::new(temp_dir.clone(), temp_dir.join("wiki"), Some(storage));

    let bad_ts = "export function broken(): any { return 1 as unknown as any; }";

    // Trigger 3 consecutive failures on module "payments" to trip the Circuit Breaker
    let mut third_payload = json!({});
    for i in 1..=3 {
        let resp = dispatch_alexandria_tool_with_context(
            "verify_and_gate_changes",
            json!({
                "targetModule": "payments",
                "targetFiles": ["src/payments.ts"],
                "executionPath": "alpha_fast",
                "tddPhase": "green_expect_pass",
                "sourceCode": bad_ts
            }),
            json!(700 + i),
            &ctx,
        )
        .await;
        third_payload = unwrap_mcp_json(resp.result);
    }

    assert_eq!(third_payload["circuitBreakerState"], json!("tripped_abort"));
    assert_eq!(third_payload["attemptNumber"], json!(3));

    let docket = &third_payload["courtMartialDocket"];
    assert!(
        docket.is_object(),
        "courtMartialDocket must be emitted when circuitBreakerState == tripped_abort"
    );
    assert_eq!(
        docket["status"],
        json!("awaiting_bel_single_turn_verdict")
    );
    assert_eq!(docket["defendantAgent"], json!("enki"));
    assert_eq!(docket["prosecutorAgent"], json!("utu"));
    assert_eq!(docket["presidingJudgeAgent"], json!("bel"));
    assert_eq!(docket["targetModule"], json!("payments"));

    // Verify Agentic Git IAM seal directly and via finalize_task_and_log
    let iam_direct = build_agentic_git_iam_seal("enki", "payments", "Fixed explicit any in payments");
    assert_eq!(
        iam_direct["ephemeralBranch"],
        json!("alexandria/task/payments")
    );
    assert!(
        iam_direct["gitAuthor"]
            .as_str()
            .is_some_and(|a| a.contains("Enki"))
    );
    assert!(
        iam_direct["gitCommitter"]
            .as_str()
            .is_some_and(|c| c.contains("Nabu"))
    );
    assert!(
        iam_direct["cryptographicSealSha256"]
            .as_str()
            .is_some_and(|h| h.len() == 64)
    );

    let fin_resp = dispatch_alexandria_tool_with_context(
        "finalize_task_and_log",
        json!({
            "moduleName": "payments",
            "summary": "Replaced explicit any with Branded PaymentId and passed Gate 0-4"
        }),
        json!(710),
        &ctx,
    )
    .await;
    let fin_payload = unwrap_mcp_json(fin_resp.result);
    assert_eq!(
        fin_payload["agenticGitIam"]["ephemeralBranch"],
        json!("alexandria/task/payments")
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_live_telemetry_mttr_and_markdown_dashboard_generation() {
    let temp_dir = create_temp_v25_dir("telemetry_dashboard");
    let wiki_dir = temp_dir.join("wiki");
    fs::create_dir_all(&wiki_dir).unwrap();
    let db_path = temp_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL).unwrap();

    // Record a gate failure followed by a passing verdict
    let _ = storage
        .record_gate_failure(
            "orders".to_string(),
            "oxc_ast".to_string(),
            "NO_EXPLICIT_ANY at offset 12".to_string(),
            3,
        )
        .await
        .unwrap();
    let _ = storage
        .record_quality_gate_verdict(
            "orders".to_string(),
            "alpha_fast".to_string(),
            "green_expect_pass".to_string(),
            false,
            r#"{"passed":false}"#.to_string(),
        )
        .await
        .unwrap();
    let _ = storage
        .record_quality_gate_verdict(
            "orders".to_string(),
            "alpha_fast".to_string(),
            "green_expect_pass".to_string(),
            true,
            r#"{"passed":true}"#.to_string(),
        )
        .await
        .unwrap();

    let (telemetry_json_path, dashboard_md_path) =
        write_telemetry_and_markdown_dashboard(&storage, &temp_dir, &wiki_dir)
            .expect("Telemetry and Markdown dashboard must be generated");

    assert!(
        telemetry_json_path.exists(),
        ".alexandria/telemetry/organism-health.json must exist"
    );
    assert!(dashboard_md_path.exists(), "wiki/dashboard.md must exist");

    let health_val: Value =
        serde_json::from_str(&fs::read_to_string(&telemetry_json_path).unwrap()).unwrap();
    assert_eq!(health_val["totalVerdicts"], json!(2));
    assert_eq!(health_val["passedVerdicts"], json!(1));
    assert_eq!(health_val["failedVerdicts"], json!(1));
    assert!(health_val["meanTimeToRepairSeconds"].is_number());

    let dashboard_md = fs::read_to_string(&dashboard_md_path).unwrap();
    assert!(dashboard_md.contains("Alexandria Organism Telemetry Dashboard"));
    assert!(dashboard_md.contains("MTTR"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_lsp_server_publishes_gate0_and_gate2_diagnostics() {
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    })
    .to_string();

    let init_resp =
        handle_lsp_jsonrpc_message(&init_req).expect("LSP initialize must return a response");
    assert_eq!(
        init_resp["result"]["capabilities"]["textDocumentSync"],
        json!(1)
    );

    let did_open_req = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/src/unsafe_payment.ts",
                "languageId": "typescript",
                "version": 1,
                "text": "export function leak(x: any): string {\n  const token = \"ghp_1234567890abcdefghijklmnopqrstuv\";\n  return token;\n}\n"
            }
        }
    })
    .to_string();

    let diag_notification = handle_lsp_jsonrpc_message(&did_open_req)
        .expect("LSP textDocument/didOpen must emit publishDiagnostics notification");
    assert_eq!(
        diag_notification["method"],
        json!("textDocument/publishDiagnostics")
    );
    let diagnostics = diag_notification["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics array must be present");

    let codes: Vec<&str> = diagnostics
        .iter()
        .filter_map(|d| d["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"NO_EXPLICIT_ANY"),
        "LSP diagnostics must include Gate 0 NO_EXPLICIT_ANY: {:?}",
        codes
    );
    assert!(
        codes.contains(&"GATE2_HARDCODED_SECRET"),
        "LSP diagnostics must include Gate 2 GATE2_HARDCODED_SECRET: {:?}",
        codes
    );
}
