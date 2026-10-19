use alexandria::mcp::server::{
    McpKernelContext, dispatch_alexandria_tool_with_context, dispatch_mcp_method_with_context,
};
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::VaultStorageHandle;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn create_temp_integration_env(label: &str) -> (PathBuf, McpKernelContext, VaultStorageHandle) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base_dir = std::env::temp_dir().join(format!(
        "alexandria_mcp_integ_{}_{}_{}",
        label,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&base_dir).expect("Temporary integration environment created");

    let db_path = base_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)
        .expect("SQLite WAL actor initialized");

    // Hermetic self-contained temporary vault inside base_dir.join("wiki") for CI isolation
    let wiki_dir = base_dir.join("wiki");
    let modules_dir = wiki_dir.join("modules");
    let decisions_dir = wiki_dir.join("decisions");
    fs::create_dir_all(&modules_dir).expect("Created wiki/modules directory");
    fs::create_dir_all(&decisions_dir).expect("Created wiki/decisions directory");

    fs::write(
        wiki_dir.join("index.md"),
        "# Alexandria Vault Index\n\n- [[gate-guard-ast]]\n- [[ADR-006-oxc-ast-engine]]\n",
    )
    .expect("Wrote wiki/index.md");

    fs::write(
        modules_dir.join("gate-guard-ast.md"),
        "---\nid: gate-guard-ast\ntype: module\n---\n\n# Subsystem Contract: Gate 0 AST Scanner (`oxc_parser`)\n\nGoverned by [[ADR-006-oxc-ast-engine]]. Enforces zero explicit `any` and strict Prisma transaction lexical closure.\n",
    )
    .expect("Wrote wiki/modules/gate-guard-ast.md");

    fs::write(
        decisions_dir.join("ADR-006-oxc-ast-engine.md"),
        "---\nid: ADR-006-oxc-ast-engine\ntype: adr\nstatus: accepted\n---\n\n# ADR-006: Native Oxc AST Engine for Gate 0\n\nAll TypeScript and TSX files are validated in memory using `oxc_parser`.\n",
    )
    .expect("Wrote wiki/decisions/ADR-006-oxc-ast-engine.md");

    let ctx = McpKernelContext::new(base_dir.clone(), wiki_dir, Some(storage.clone()));
    (base_dir, ctx, storage)
}

fn extract_tool_payload(resp_result: Option<Value>) -> Value {
    let res = resp_result.expect("JSON-RPC response must contain result");
    assert_eq!(res["isError"], json!(false));
    let text = res["content"][0]["text"]
        .as_str()
        .expect("JSON text in content[0]");
    serde_json::from_str(text).expect("Valid tool JSON payload")
}

#[tokio::test]
async fn test_mcp_real_prepare_task_context_classification() {
    let (temp_dir, ctx, _storage) = create_temp_integration_env("prepare_ctx");

    // 1. Touching prisma/schema.prisma must enforce omega_pantheon and hoist ADRs from wiki/
    let omega_resp = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Add subscription table in prisma/schema.prisma",
                "targetFiles": ["prisma/schema.prisma", "src/billing/repo.ts"],
                "moduleName": "gate-guard-ast"
            }
        }),
        json!(101),
        &ctx,
    )
    .await;

    let omega_payload = extract_tool_payload(omega_resp.result);
    assert_eq!(
        omega_payload["enforcedPath"],
        json!("omega_pantheon"),
        "Modifying prisma/schema.prisma must enforce omega_pantheon"
    );
    assert!(
        omega_payload["architecturalContext"]
            .as_str()
            .is_some_and(|c| c.contains("Gate 0") || c.contains("oxc_parser")),
        "Must load the real contract from wiki/modules/gate-guard-ast.md"
    );
    let hoisted = omega_payload["hoistedAdrs"]
        .as_array()
        .expect("hoistedAdrs must be an array");
    assert!(
        hoisted.contains(&json!("ADR-006-oxc-ast-engine")),
        "Must hoist constitutional ADRs linked from the module"
    );

    // 2. Touching only src/ui/button.tsx must assign alpha_fast
    let alpha_resp = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Adjust primary button padding",
                "targetFiles": ["src/ui/button.tsx"]
            }
        }),
        json!(102),
        &ctx,
    )
    .await;

    let alpha_payload = extract_tool_payload(alpha_resp.result);
    assert_eq!(
        alpha_payload["enforcedPath"],
        json!("alpha_fast"),
        "Modifying src/ui/button.tsx without structural impact must assign alpha_fast"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_mcp_real_verify_changes_rejects_any() {
    let (temp_dir, ctx, _storage) = create_temp_integration_env("verify_reject_any");

    // Write a physical TypeScript file containing `let x: any = 10;` in the temporary workspace
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir).expect("Created src directory");
    let bad_file_rel = "src/violating_service.ts";
    fs::write(
        temp_dir.join(bad_file_rel),
        "export function compute(): number {\n  let x: any = 10;\n  return x;\n}\n",
    )
    .expect("Wrote TypeScript file");

    let verify_resp = dispatch_alexandria_tool_with_context(
        "verify_and_gate_changes",
        json!({
            "targetModule": "billing",
            "targetFiles": [bad_file_rel],
            "executionPath": "alpha_fast",
            "tddPhase": "green_expect_pass"
        }),
        json!(201),
        &ctx,
    )
    .await;

    let payload = extract_tool_payload(verify_resp.result);
    assert_eq!(
        payload["passed"],
        json!(false),
        "MCP server must reject code containing explicit 'any'"
    );

    let violated_rules = payload["violatedRules"]
        .as_array()
        .expect("violatedRules must be present");
    assert!(
        violated_rules.contains(&json!("NO_EXPLICIT_ANY")),
        "Must explicitly report rule NO_EXPLICIT_ANY: {:?}",
        violated_rules
    );
    assert!(
        payload["diagnostics"]
            .as_str()
            .is_some_and(|d| d.contains("NO_EXPLICIT_ANY")),
        "Diagnostics must detail the NO_EXPLICIT_ANY violation"
    );
    assert_eq!(payload["circuitBreakerState"], json!("retry_allowed"));
    assert!(
        payload["failureId"]
            .as_str()
            .is_some_and(|id| id.starts_with("FAIL-BILLING-")),
        "Must record the forensic incident in vault.db"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_mcp_real_skill_resolution_and_fts_fallback() {
    let (temp_dir, ctx, storage) = create_temp_integration_env("skill_fts");

    storage
        .upsert_skill(
            "skill-prisma-acid-closure".to_string(),
            "database".to_string(),
            "Enforces strict lexical closure inside interactive Prisma transactions".to_string(),
            r#"["prisma", "transaction", "acid", "deadlock"]"#.to_string(),
            r#"["NO_TX_ALIASING", "DIRECT_AWAIT_ONLY", "NO_EXTERNAL_FETCH_IN_TX"]"#.to_string(),
            "Always use `await tx.<model>.<op>()` directly inside the inline callback.".to_string(),
        )
        .await
        .expect("Skill insertion into vault.db succeeded");

    // 1. Resolution by exact ID (`exact_id`)
    let exact_resp = dispatch_alexandria_tool_with_context(
        "resolve_skill_capsule",
        json!({
            "skillId": "skill-prisma-acid-closure"
        }),
        json!(301),
        &ctx,
    )
    .await;

    let exact_payload = extract_tool_payload(exact_resp.result);
    assert_eq!(
        exact_payload["skillId"],
        json!("skill-prisma-acid-closure")
    );
    assert_eq!(exact_payload["resolutionMethod"], json!("exact_id"));
    assert!(
        exact_payload["instructions"]
            .as_str()
            .is_some_and(|s| s.contains("await tx.<model>.<op>()"))
    );
    assert_eq!(
        exact_payload["invariants"],
        json!([
            "NO_TX_ALIASING",
            "DIRECT_AWAIT_ONLY",
            "NO_EXTERNAL_FETCH_IN_TX"
        ])
    );

    // 2. Resolution via FTS5 BM25 lexical search (`fts5_bm25_fallback`) using intentQuery
    let fts_resp = dispatch_alexandria_tool_with_context(
        "resolve_skill_capsule",
        json!({
            "intentQuery": "How to prevent deadlock in interactive prisma transaction"
        }),
        json!(302),
        &ctx,
    )
    .await;

    let fts_payload = extract_tool_payload(fts_resp.result);
    assert_eq!(
        fts_payload["skillId"],
        json!("skill-prisma-acid-closure"),
        "FTS5 BM25 with Porter stemmer must retrieve the skill via intentQuery"
    );
    assert_eq!(
        fts_payload["resolutionMethod"],
        json!("fts5_bm25_fallback")
    );
    assert_eq!(
        fts_payload["invariants"],
        json!([
            "NO_TX_ALIASING",
            "DIRECT_AWAIT_ONLY",
            "NO_EXTERNAL_FETCH_IN_TX"
        ])
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
