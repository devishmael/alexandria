use alexandria::cli::{discover_workspace_context, ResolvedWorkspaceContext};
use alexandria::mcp::server::{dispatch_mcp_method_with_context, McpKernelContext};
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::VaultStorageHandle;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(500);

fn temp_test_dir(label: &str) -> PathBuf {
    let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("alexandria_dyn_iso_{}_{}_{}", label, nanos, seq));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn extract_tool_payload(resp_result: Option<Value>) -> Value {
    let res = resp_result.expect("JSON-RPC response must contain result");
    let text = res["content"][0]["text"]
        .as_str()
        .expect("JSON text in content[0]");
    serde_json::from_str(text).expect("Valid tool JSON payload")
}

#[test]
fn test_discover_workspace_context_with_file_hint() {
    let project_dir = temp_test_dir("file_hint_proj");
    let git_dir = project_dir.join(".git");
    fs::create_dir_all(&git_dir).expect("Created .git");

    let sub_src = project_dir.join("src").join("controllers");
    fs::create_dir_all(&sub_src).expect("Created src/controllers");
    let target_file = sub_src.join("auth.controller.ts");
    fs::write(&target_file, "export const auth = () => {};").expect("Created file");

    let resolved: ResolvedWorkspaceContext =
        discover_workspace_context(Some(&target_file), None, None);

    assert_eq!(
        resolved.workspace_root, project_dir,
        "Must traverse ancestor tree to find the .git project root"
    );
    assert_eq!(
        resolved.vault_dir,
        project_dir.join("wiki"),
        "Default vault must be ./wiki when none exists"
    );
    assert_eq!(
        resolved.db_path,
        project_dir.join(".alexandria").join("vault.db"),
        "Database path must be scoped to the discovered project root"
    );

    let _ = fs::remove_dir_all(&project_dir);
}

#[test]
fn test_discover_workspace_context_honors_alexandria_json_vault_path() {
    let project_dir = temp_test_dir("custom_vault_proj");
    fs::create_dir_all(&project_dir).expect("Created project dir");

    let config_content = json!({
        "vaultPath": "architecture-vault",
        "primaryModel": "claude-3-5-sonnet"
    })
    .to_string();

    fs::write(project_dir.join(".alexandria.json"), config_content)
        .expect("Wrote .alexandria.json");

    let resolved = discover_workspace_context(Some(&project_dir), None, None);

    assert_eq!(
        resolved.workspace_root, project_dir,
        "Root must match .alexandria.json location"
    );
    assert_eq!(
        resolved.vault_dir,
        project_dir.join("architecture-vault"),
        "Must honor custom vaultPath defined in .alexandria.json"
    );

    let _ = fs::remove_dir_all(&project_dir);
}

#[tokio::test]
async fn test_mcp_initialize_sets_session_default_workspace() {
    let project_dir = temp_test_dir("init_session_proj");
    let alexandria_vault = project_dir.join("alexandria");
    let modules_dir = alexandria_vault.join("modules");
    fs::create_dir_all(&modules_dir).expect("Created alexandria/modules");

    fs::write(
        alexandria_vault.join("index.md"),
        "# Root Index\n\n- [[service-contract]]\n",
    )
    .expect("Wrote index.md");

    fs::write(
        modules_dir.join("service-contract.md"),
        "---\nid: service-contract\ntype: module\n---\n# Service Contract Specification\n",
    )
    .expect("Wrote module");

    let ctx = McpKernelContext::default_local();

    // Send initialize with rootUri pointing to the new project
    let root_uri = format!("file:///{}", project_dir.to_string_lossy().replace('\\', "/"));
    let init_resp = dispatch_mcp_method_with_context(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "rootUri": root_uri
        }),
        json!(1),
        &ctx,
    )
    .await;

    assert!(init_resp.error.is_none());

    // Calling prepare_task_context without explicit args must now resolve against the initialized session workspace!
    let prep_resp = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Inspect architecture",
                "targetFiles": []
            }
        }),
        json!(2),
        &ctx,
    )
    .await;

    let payload = extract_tool_payload(prep_resp.result);
    assert_eq!(payload["status"], json!("dispatched_by_kernel"));
    assert!(
        payload["architecturalContext"]
            .as_str()
            .unwrap_or_default()
            .contains("Service Contract Specification"),
        "Must load contracts from the project configured during MCP initialize"
    );

    let _ = fs::remove_dir_all(&project_dir);
}

#[tokio::test]
async fn test_mcp_dynamic_workspace_and_storage_isolation() {
    let proj_a = temp_test_dir("dyn_proj_a");
    let proj_b = temp_test_dir("dyn_proj_b");

    // Setup Project A
    let vault_a = proj_a.join("wiki");
    fs::create_dir_all(vault_a.join("modules")).unwrap();
    fs::write(
        vault_a.join("index.md"),
        "# Proj A Index\n\n- [[auth-module]]\n",
    )
    .unwrap();
    fs::write(
        vault_a.join("modules").join("auth-module.md"),
        "---\nid: auth-module\ntype: module\n---\n# Project A Authentication Contract\n",
    )
    .unwrap();

    // Setup Project B
    let vault_b = proj_b.join("wiki");
    fs::create_dir_all(vault_b.join("modules")).unwrap();
    fs::write(
        vault_b.join("index.md"),
        "# Proj B Index\n\n- [[billing-module]]\n",
    )
    .unwrap();
    fs::write(
        vault_b.join("modules").join("billing-module.md"),
        "---\nid: billing-module\ntype: module\n---\n# Project B Billing Contract\n",
    )
    .unwrap();

    let ctx = McpKernelContext::default_local();

    // Spawn handles to inspect the baseline before MCP tool calls
    let db_a_path = proj_a.join(".alexandria").join("vault.db");
    let db_b_path = proj_b.join(".alexandria").join("vault.db");

    // Discover context to trigger directory setup
    let res_a = discover_workspace_context(Some(&proj_a), None, None);
    let res_b = discover_workspace_context(Some(&proj_b), None, None);

    let storage_a = VaultStorageHandle::spawn(res_a.db_path.clone(), CANONICAL_SCHEMA_SQL).unwrap();
    let storage_b = VaultStorageHandle::spawn(res_b.db_path.clone(), CANONICAL_SCHEMA_SQL).unwrap();

    let initial_a = storage_a.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);
    let initial_b = storage_b.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);

    // 1. Tool call targeting Project A explicitly via workspaceRoot
    let resp_a = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Implement OAuth2 callback for customer portal",
                "targetFiles": ["src/auth.ts"],
                "workspaceRoot": proj_a.to_string_lossy().to_string()
            }
        }),
        json!(10),
        &ctx,
    )
    .await;

    let payload_a = extract_tool_payload(resp_a.result);
    assert_eq!(payload_a["status"], json!("dispatched_by_kernel"));
    assert!(
        payload_a["architecturalContext"]
            .as_str()
            .unwrap_or_default()
            .contains("Project A Authentication Contract"),
        "Must load Project A contracts"
    );

    // Verify Project A has exactly +1 memory, and Project B is untouched
    let current_a_after_call1 = storage_a.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);
    let current_b_after_call1 = storage_b.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);
    assert_eq!(
        current_a_after_call1,
        initial_a + 1,
        "Project A must have exactly 1 additional episodic memory chunk"
    );
    assert_eq!(
        current_b_after_call1,
        initial_b,
        "Project B must NOT receive memories when Project A is called"
    );

    // 2. Tool call targeting Project B explicitly via targetFiles path inside Project B
    let target_file_b = proj_b.join("src").join("invoice.ts");
    let resp_b = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Generate monthly invoices and tax withholding",
                "targetFiles": [target_file_b.to_string_lossy().to_string()]
            }
        }),
        json!(20),
        &ctx,
    )
    .await;

    let payload_b = extract_tool_payload(resp_b.result);
    assert_eq!(payload_b["status"], json!("dispatched_by_kernel"));
    assert!(
        payload_b["architecturalContext"]
            .as_str()
            .unwrap_or_default()
            .contains("Project B Billing Contract"),
        "Must load Project B contracts based on target file hint"
    );

    // 3. Verify Database Scopes are isolated:
    // Project B database has +1 episodic memory recorded, and Project A count has not changed
    assert!(db_a_path.exists(), "Project A database must exist locally");
    assert!(db_b_path.exists(), "Project B database must exist locally");

    let final_a = storage_a.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);
    let final_b = storage_b.compute_organism_telemetry().unwrap()["episodicMemoryChunks"].as_u64().unwrap_or(0);

    assert_eq!(
        final_a,
        initial_a + 1,
        "Project A count must remain initial + 1 (Zero Cross-Project Pollution)"
    );
    assert_eq!(
        final_b,
        initial_b + 1,
        "Project B must have exactly 1 additional episodic memory chunk recorded"
    );

    let _ = fs::remove_dir_all(&proj_a);
    let _ = fs::remove_dir_all(&proj_b);
}

#[tokio::test]
async fn test_prepare_task_context_returns_vault_unresolved_on_empty_vault() {
    let empty_proj = temp_test_dir("empty_vault_proj");
    let empty_wiki = empty_proj.join("wiki");
    fs::create_dir_all(&empty_wiki).expect("Created empty wiki dir");

    let ctx = McpKernelContext::default_local();

    let resp = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Build user onboarding",
                "targetFiles": ["src/onboarding.ts"],
                "workspaceRoot": empty_proj.to_string_lossy().to_string()
            }
        }),
        json!(99),
        &ctx,
    )
    .await;

    let payload = extract_tool_payload(resp.result);
    assert_eq!(
        payload["status"],
        json!("vault_unresolved"),
        "Must return explicit vault_unresolved status when vault has 0 documents"
    );
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("No architectural vault found"),
        "Must provide explicit message directing user/agent to alexandria init"
    );

    let _ = fs::remove_dir_all(&empty_proj);
}
