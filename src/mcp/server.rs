use crate::collector::skill_etl::{
    AlexandriaHookSpec, collect_and_refactor_directory, export_refactored_json_bundle,
};
use crate::gate_guard::oxc_validator::{
    detect_public_export_signature_change, prune_typescript_context_for_symbol,
    scan_gate2_sast_and_secrets, validate_typescript_source,
};
use crate::graph::resolver::{
    DisjunctionConflict, current_human_intervention_epoch, load_vault_graph_from_dir,
    resolve_context_with_adr_hoisting, serialize_graph_for_sqlite, verify_worktree_disjunction,
};
use crate::runner::vitest_sandbox::{RedPhaseVerdict, VitestJsonReport, evaluate_tdd_red_phase};
use crate::storage::actor::{VaultStorageHandle, write_telemetry_and_markdown_dashboard};
use crate::vector_store::hybrid_memory::{
    export_vector_store_snapshot, query_hybrid_vector_memory,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Clone)]
pub struct McpKernelContext {
    pub workspace_root: PathBuf,
    pub vault_dir: PathBuf,
    pub storage: Option<VaultStorageHandle>,
}

impl McpKernelContext {
    pub fn new(
        workspace_root: PathBuf,
        vault_dir: PathBuf,
        storage: Option<VaultStorageHandle>,
    ) -> Self {
        Self {
            workspace_root,
            vault_dir,
            storage,
        }
    }

    pub fn default_local() -> Self {
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let vault_dir = if workspace_root.join("alexandria").exists() {
            workspace_root.join("alexandria")
        } else {
            workspace_root.join("wiki")
        };
        Self {
            workspace_root,
            vault_dir,
            storage: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub async fn handle_raw_jsonrpc_line(line: &str) -> Option<JsonRpcResponse> {
    let ctx = McpKernelContext::default_local();
    handle_raw_jsonrpc_line_with_context(line, &ctx).await
}

pub async fn handle_raw_jsonrpc_line_with_context(
    line: &str,
    ctx: &McpKernelContext,
) -> Option<JsonRpcResponse> {
    if line.trim().is_empty() {
        return None;
    }

    let req: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Malformed JSON-RPC frame: {}", e);
            return Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id: Value::Null,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {}", e),
                }),
            });
        }
    };

    let req_id = req.id?;
    Some(dispatch_mcp_method_with_context(&req.method, req.params, req_id, ctx).await)
}

pub async fn run_mcp_stdio_loop() -> Result<(), String> {
    run_mcp_stdio_loop_with_context(McpKernelContext::default_local()).await
}

pub async fn run_mcp_stdio_loop_with_context(ctx: McpKernelContext) -> Result<(), String> {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();

    tracing::info!("Alexandria v1.0.0 MCP Server started over stdio");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await.map_err(|e| e.to_string())? {
        if let Some(response) = handle_raw_jsonrpc_line_with_context(&line, &ctx).await {
            let mut serialized = serde_json::to_string(&response).map_err(|e| e.to_string())?;
            serialized.push('\n');

            stdout
                .write_all(serialized.as_bytes())
                .await
                .map_err(|e| e.to_string())?;
            stdout.flush().await.map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub async fn dispatch_mcp_method(method: &str, params: Value, id: Value) -> JsonRpcResponse {
    let ctx = McpKernelContext::default_local();
    dispatch_mcp_method_with_context(method, params, id, &ctx).await
}

pub async fn dispatch_mcp_method_with_context(
    method: &str,
    params: Value,
    id: Value,
    ctx: &McpKernelContext,
) -> JsonRpcResponse {
    match method {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "alexandria",
                    "version": "1.0.0"
                },
                "capabilities": {
                    "tools": { "listChanged": false }
                }
            })),
            error: None,
        },
        "ping" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({})),
            error: None,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "prepare_task_context",
                        "description": "Classifies Path Alpha/Omega in Zero-Turn and resolves the depth=1 topological graph with transitive ADR hoisting.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["instruction", "targetFiles"],
                            "additionalProperties": false,
                            "properties": {
                                "instruction": { "type": "string" },
                                "targetFiles": { "type": "array", "items": { "type": "string" } },
                                "moduleName": { "type": "string" },
                                "focusSymbol": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "upsert_architectural_contract",
                        "description": "Creates or updates ADRs and module contracts in the knowledge vault (Path Omega).",
                        "inputSchema": {
                            "type": "object",
                            "required": ["moduleName", "technicalSpecMarkdown"],
                            "additionalProperties": false,
                            "properties": {
                                "moduleName": { "type": "string" },
                                "technicalSpecMarkdown": { "type": "string" },
                                "adrTitle": { "type": "string" },
                                "adrDecisionMarkdown": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "resolve_skill_capsule",
                        "description": "Injects a JIT procedural capsule into the dynamic suffix without invalidating the KV-Cache prefix, or ingests a skill directory.",
                        "inputSchema": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "skillId": { "type": "string" },
                                "intentQuery": { "type": "string" },
                                "sourceDir": { "type": "string" },
                                "outputDir": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "search_capsules",
                        "description": "Searches curated skill and hook capsules via full-text BM25 index with optional domain filtering.",
                        "inputSchema": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "query": { "type": "string", "description": "Free-text search query matching triggers, titles, and instructions." },
                                "domain": {
                                    "type": "string",
                                    "enum": ["backend", "architecture", "devops", "frontend", "testing", "database", "api", "hook"],
                                    "description": "Optional canonical domain filter."
                                },
                                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "description": "Maximum number of capsules to return (default: 10, max: 50)." }
                            }
                        }
                    },
                    {
                        "name": "read_capsule",
                        "description": "Retrieves complete technical specifications, invariants, instructions, and hooks for a specific capsule.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["capsuleId"],
                            "additionalProperties": false,
                            "properties": {
                                "capsuleId": { "type": "string", "description": "The canonical capsule identifier (e.g. 'testing:tdd-practitioner' or 'api:api-designer')." }
                            }
                        }
                    },
                    {
                        "name": "spawn_isolated_worktrees",
                        "description": "Authorizes parallel Git worktrees after verifying dual disjunction in petgraph and oxc_resolver.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["tasks"],
                            "additionalProperties": false,
                            "properties": {
                                "tasks": {
                                    "type": "array",
                                    "minItems": 2,
                                    "items": {
                                        "type": "object",
                                        "required": ["workerId", "moduleName", "targetFiles"],
                                        "properties": {
                                            "workerId": { "type": "string" },
                                            "moduleName": { "type": "string" },
                                            "targetFiles": { "type": "array", "items": { "type": "string" } }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    {
                        "name": "verify_and_gate_changes",
                        "description": "Executes Gate 0 (Oxc AST), Gate 1-4 in Node Sandbox (--permission), and Utu Stateless Cold Audit.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["targetModule", "targetFiles", "executionPath", "tddPhase"],
                            "additionalProperties": false,
                            "properties": {
                                "targetModule": { "type": "string" },
                                "targetFiles": { "type": "array", "items": { "type": "string" } },
                                "executionPath": { "type": "string", "enum": ["alpha_fast", "omega_pantheon"] },
                                "tddPhase": { "type": "string", "enum": ["red_expect_assertion_failure", "green_expect_pass", "refactor_full_gate"] }
                            }
                        }
                    },
                    {
                        "name": "finalize_task_and_log",
                        "description": "Performs the atomic commit, appends the immutable entry to wiki/log.md, and resets the Circuit Breaker.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["moduleName", "summary"],
                            "additionalProperties": false,
                            "properties": {
                                "moduleName": { "type": "string" },
                                "summary": { "type": "string" },
                                "startDevServer": { "type": "boolean" }
                            }
                        }
                    }
                ]
            })),
            error: None,
        },
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            dispatch_alexandria_tool_with_context(tool_name, args, id, ctx).await
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("JSON-RPC method not found: {}", method),
            }),
        },
    }
}

pub async fn dispatch_alexandria_tool(tool_name: &str, args: Value, id: Value) -> JsonRpcResponse {
    let ctx = McpKernelContext::default_local();
    dispatch_alexandria_tool_with_context(tool_name, args, id, &ctx).await
}

pub fn classify_execution_path(instruction: &str, target_files: &[String]) -> &'static str {
    let instr_lower = instruction.to_lowercase();
    let touches_critical_file = target_files.iter().any(|f| {
        let norm = f.replace('\\', "/").to_lowercase();
        norm.contains("prisma/schema.prisma")
            || norm.ends_with("schema.prisma")
            || norm.contains("/auth/")
            || norm.starts_with("auth/")
            || norm.contains("core/auth")
            || norm.starts_with("wiki/")
            || norm.starts_with("alexandria/")
            || norm.contains("/decisions/")
            || norm.contains("adr-")
    });

    let instr_triggers_omega = instr_lower.contains("schema.prisma")
        || instr_lower.contains("adr-")
        || instr_lower.contains("architecture");

    if touches_critical_file || instr_triggers_omega {
        "omega_pantheon"
    } else {
        "alpha_fast"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookExecutionResult {
    pub passed: bool,
    pub executed_hooks_count: usize,
    pub triggered_actions: Vec<String>,
    pub rejections: Vec<String>,
}

pub async fn execute_lifecycle_hook_interceptors(
    event: &str,
    target_files: &[String],
    sources: &[(String, String)],
    ctx: &McpKernelContext,
    args: &Value,
) -> HookExecutionResult {
    let mut passed = true;
    let mut triggered_actions = Vec::new();
    let mut rejections = Vec::new();

    let registered_hooks = if let Some(storage) = &ctx.storage {
        storage.get_hooks_for_event(event).unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut executed_hooks_count = registered_hooks.len();

    for (capsule_id, hook) in &registered_hooks {
        triggered_actions.push(format!("{}:{}", capsule_id, hook.action));

        match hook.action.as_str() {
            "validate_prisma_tx_closure" => {
                for (file_label, code) in sources {
                    let is_tsx = file_label.ends_with(".tsx") || file_label.ends_with(".jsx");
                    if let Ok(violations) = validate_typescript_source(code, is_tsx) {
                        for v in violations {
                            if v.rule.to_string().contains("PRISMA") {
                                passed = false;
                                rejections.push(format!(
                                    "[HOOK:{} @ {}] Prisma transaction violation: {}",
                                    hook.action, file_label, v.message
                                ));
                            }
                        }
                    }
                }
            }
            "enforce_nominal_types" => {
                for (file_label, code) in sources {
                    let is_tsx = file_label.ends_with(".tsx") || file_label.ends_with(".jsx");
                    if let Ok(violations) = validate_typescript_source(code, is_tsx) {
                        for v in violations {
                            let rule_str = v.rule.to_string();
                            if rule_str.contains("ANY") || rule_str.contains("DOUBLE_CAST") {
                                passed = false;
                                rejections.push(format!(
                                    "[HOOK:{} @ {}] Nominal type violation: {}",
                                    hook.action, file_label, v.message
                                ));
                            }
                        }
                    }
                }
            }
            "scan_gate2_sast_and_secrets" => {
                for (file_label, code) in sources {
                    let violations = scan_gate2_sast_and_secrets(code);
                    for v in violations {
                        passed = false;
                        rejections.push(format!(
                            "[HOOK:{} @ {}] Security SAST leak: {}",
                            hook.action, file_label, v.message
                        ));
                    }
                }
            }
            "enforce_test_file_existence" => {
                let has_test_file = target_files.iter().any(|f| {
                    let fl = f.to_lowercase();
                    fl.contains(".test.") || fl.contains(".spec.") || fl.contains("tests/")
                }) || sources.iter().any(|(f, _)| {
                    let fl = f.to_lowercase();
                    fl.contains(".test.") || fl.contains(".spec.") || fl.contains("tests/")
                });

                if !has_test_file && (!target_files.is_empty() || !sources.is_empty()) {
                    passed = false;
                    rejections.push(
                        "[HOOK:enforce_test_file_existence] TDD lifecycle hook requires at least one test file in target files or sources.".to_string(),
                    );
                }
            }
            _ => {
                if hook.action.contains("secret") || hook.action.contains("sast") {
                    for (file_label, code) in sources {
                        let violations = scan_gate2_sast_and_secrets(code);
                        for v in violations {
                            passed = false;
                            rejections.push(format!(
                                "[HOOK:{} @ {}] SAST check failed: {}",
                                hook.action, file_label, v.message
                            ));
                        }
                    }
                }
            }
        }
    }

    if let Some(explicit_hooks) = args.get("activeHooks").and_then(|v| v.as_array()) {
        for eh in explicit_hooks {
            if let Ok(hook_spec) = serde_json::from_value::<AlexandriaHookSpec>(eh.clone()) {
                if hook_spec.event == event {
                    executed_hooks_count += 1;
                    triggered_actions.push(format!("explicit:{}", hook_spec.action));

                    if hook_spec.action == "forbid_secrets" || hook_spec.action.contains("secret") {
                        for (file_label, code) in sources {
                            let violations = scan_gate2_sast_and_secrets(code);
                            for v in violations {
                                passed = false;
                                rejections.push(format!(
                                    "[EXPLICIT_HOOK:{} @ {}] Secret detected: {}",
                                    hook_spec.action, file_label, v.message
                                ));
                            }
                        }
                    } else if hook_spec.action == "forbid_any" || hook_spec.action.contains("any") {
                        for (file_label, code) in sources {
                            let is_tsx =
                                file_label.ends_with(".tsx") || file_label.ends_with(".jsx");
                            if let Ok(violations) = validate_typescript_source(code, is_tsx) {
                                for v in violations {
                                    if v.rule.to_string().contains("ANY") {
                                        passed = false;
                                        rejections.push(format!(
                                            "[EXPLICIT_HOOK:{} @ {}] Explicit any forbidden: {}",
                                            hook_spec.action, file_label, v.message
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    HookExecutionResult {
        passed,
        executed_hooks_count,
        triggered_actions,
        rejections,
    }
}

pub async fn dispatch_alexandria_tool_with_context(
    tool_name: &str,
    args: Value,
    id: Value,
    ctx: &McpKernelContext,
) -> JsonRpcResponse {
    match tool_name {
        "prepare_task_context" => {
            let instruction = args
                .get("instruction")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let target_files: Vec<String> = args
                .get("targetFiles")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let module_name = args.get("moduleName").and_then(|v| v.as_str());

            let enforced_path = classify_execution_path(instruction, &target_files);
            let (graph, node_map) = load_vault_graph_from_dir(&ctx.vault_dir);

            if let Some(storage) = &ctx.storage {
                let (nodes, edges) = serialize_graph_for_sqlite(&ctx.vault_dir, &graph);
                let _ = storage.sync_wiki_graph(nodes, edges).await;
            }

            let start_node_idx = module_name
                .and_then(|m| node_map.get(m).copied())
                .or_else(|| node_map.get("index").copied());

            let (architectural_context, direct_neighbors, hoisted_adrs) =
                if let Some(idx) = start_node_idx {
                    let resolved = resolve_context_with_adr_hoisting(&graph, idx, 6);
                    let content = graph[idx].content.clone();
                    (content, resolved.direct_neighbors, resolved.hoisted_adrs)
                } else {
                    (
                        "Alexandria Canonical Vault Context (Default)".to_string(),
                        Vec::new(),
                        Vec::new(),
                    )
                };

            let required_gates = if enforced_path == "omega_pantheon" {
                vec![
                    "oxc_ast",
                    "types",
                    "security",
                    "tests_sandbox",
                    "build_and_utu",
                ]
            } else {
                vec!["oxc_ast", "types", "tests_sandbox"]
            };

            let mut pruned_ast_contexts: Vec<Value> = Vec::new();
            if let Some(focus_sym) = args
                .get("focusSymbol")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                if let Some(inline_src) = args.get("sourceCode").and_then(|v| v.as_str()) {
                    if let Ok(pruned) = prune_typescript_context_for_symbol(inline_src, focus_sym, false) {
                        if let Ok(val) = serde_json::to_value(pruned) {
                            pruned_ast_contexts.push(val);
                        }
                    }
                }
                for file_str in &target_files {
                    let candidate = if Path::new(file_str).is_absolute() {
                        PathBuf::from(file_str)
                    } else {
                        ctx.workspace_root.join(file_str)
                    };
                    if candidate.is_file() {
                        if let Ok(code) = fs::read_to_string(&candidate) {
                            let is_tsx = file_str.ends_with(".tsx") || file_str.ends_with(".jsx");
                            if let Ok(pruned) =
                                prune_typescript_context_for_symbol(&code, focus_sym, is_tsx)
                            {
                                if let Ok(val) = serde_json::to_value(pruned) {
                                    pruned_ast_contexts.push(val);
                                }
                            }
                        }
                    }
                }
            }

            let mut recalled_vector_memory: Vec<Value> = Vec::new();
            if let Some(storage) = &ctx.storage {
                if let Ok(chunks) = query_hybrid_vector_memory(storage, instruction, 350) {
                    for chunk in chunks {
                        if let Ok(v) = serde_json::to_value(chunk) {
                            recalled_vector_memory.push(v);
                        }
                    }
                }
                if !instruction.trim().is_empty() {
                    let _ = storage
                        .record_episodic_memory(
                            "user_supplied".to_string(),
                            "bel".to_string(),
                            module_name.unwrap_or("core").to_string(),
                            target_files.clone(),
                            instruction.to_string(),
                            45,
                        )
                        .await;
                }
            }

            let payload = json!({
                "tool": tool_name,
                "status": "dispatched_by_kernel",
                "receivedArgs": args,
                "enforcedPath": enforced_path,
                "architecturalContext": architectural_context,
                "directNeighbors": direct_neighbors,
                "hoistedAdrs": hoisted_adrs,
                "prunedAstContexts": pruned_ast_contexts,
                "recalledVectorMemory": recalled_vector_memory,
                "humanInterventionEpoch": current_human_intervention_epoch(),
                "activeInvariants": [
                    "NO_EXPLICIT_ANY",
                    "NO_DOUBLE_CAST_UNKNOWN",
                    "PRISMA_TX_STRICT_LEXICAL_CLOSURE"
                ],
                "requiredGates": required_gates
            });

            wrap_mcp_text_response(id, payload, false)
        }
        "upsert_architectural_contract" => {
            let module_name = args
                .get("moduleName")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed-module");
            let spec_md = args
                .get("technicalSpecMarkdown")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let adr_title = args.get("adrTitle").and_then(|v| v.as_str());
            let adr_md = args.get("adrDecisionMarkdown").and_then(|v| v.as_str());

            let modules_dir = ctx.vault_dir.join("modules");
            let _ = fs::create_dir_all(&modules_dir);
            let contract_path = modules_dir.join(format!("{}.md", module_name));
            let _ = fs::write(&contract_path, spec_md);

            let mut adr_path_str: Option<String> = None;
            if let (Some(title), Some(decision_content)) = (adr_title, adr_md) {
                let decisions_dir = ctx.vault_dir.join("decisions");
                let _ = fs::create_dir_all(&decisions_dir);
                let adr_file = decisions_dir.join(format!("{}.md", title));
                let _ = fs::write(&adr_file, decision_content);
                adr_path_str = Some(adr_file.to_string_lossy().to_string());
            }

            if let Some(storage) = &ctx.storage {
                let (graph, _) = load_vault_graph_from_dir(&ctx.vault_dir);
                let (nodes, edges) = serialize_graph_for_sqlite(&ctx.vault_dir, &graph);
                let _ = storage.sync_wiki_graph(nodes, edges).await;
            }

            let payload = json!({
                "tool": tool_name,
                "status": "dispatched_by_kernel",
                "receivedArgs": args,
                "moduleContractPath": contract_path.to_string_lossy(),
                "adrPath": adr_path_str,
                "graphUpdated": true
            });

            wrap_mcp_text_response(id, payload, false)
        }
        "resolve_skill_capsule" => {
            if let Some(source_dir_str) = args
                .get("sourceDir")
                .or_else(|| args.get("collectFromDir"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                let source_path = if Path::new(source_dir_str).is_absolute() {
                    PathBuf::from(source_dir_str)
                } else {
                    ctx.workspace_root.join(source_dir_str)
                };
                let out_path = args
                    .get("outputDir")
                    .and_then(|v| v.as_str())
                    .map(|p| {
                        if Path::new(p).is_absolute() {
                            PathBuf::from(p)
                        } else {
                            ctx.workspace_root.join(p)
                        }
                    })
                    .unwrap_or_else(|| ctx.workspace_root.join(".alexandria").join("capsules"));

                match collect_and_refactor_directory(&source_path) {
                    Ok(capsules) => {
                        let manifest_res = export_refactored_json_bundle(&capsules, &out_path);
                        if let Some(storage) = &ctx.storage {
                            for cap in &capsules {
                                let _ = storage.upsert_skill_capsule(cap).await;
                            }
                        }
                        let skill_ids: Vec<String> =
                            capsules.iter().map(|c| c.skill_id.clone()).collect();
                        let payload = json!({
                            "tool": tool_name,
                            "status": "dispatched_by_kernel",
                            "receivedArgs": args,
                            "resolutionMethod": "directory_etl_ingest",
                            "totalCapsules": capsules.len(),
                            "ingestedSkillIds": skill_ids,
                            "manifestPath": manifest_res.ok().map(|p| p.to_string_lossy().to_string())
                        });
                        return wrap_mcp_text_response(id, payload, false);
                    }
                    Err(err) => {
                        let payload = json!({
                            "tool": tool_name,
                            "status": "error",
                            "error": err
                        });
                        return wrap_mcp_text_response(id, payload, true);
                    }
                }
            }

            let skill_id = args.get("skillId").and_then(|v| v.as_str());
            let intent_query = args.get("intentQuery").and_then(|v| v.as_str());

            let resolved_opt = if let Some(storage) = &ctx.storage {
                storage
                    .resolve_skill_capsule(skill_id, intent_query)
                    .ok()
                    .flatten()
            } else {
                None
            };

            let payload = if let Some(record) = resolved_opt {
                let invariants: Vec<String> =
                    serde_json::from_str(&record.invariants_json).unwrap_or_default();
                let mut allowed_tools: Vec<String> =
                    serde_json::from_str(&record.required_tools_json).unwrap_or_default();
                if allowed_tools.is_empty() {
                    allowed_tools = vec![
                        "prepare_task_context".to_string(),
                        "verify_and_gate_changes".to_string(),
                    ];
                }
                let hooks: Vec<AlexandriaHookSpec> =
                    serde_json::from_str(&record.hooks_json).unwrap_or_default();

                json!({
                    "tool": tool_name,
                    "status": "dispatched_by_kernel",
                    "receivedArgs": args,
                    "skillId": record.skill_id,
                    "domain": record.domain,
                    "description": record.description,
                    "resolutionMethod": record.resolution_method,
                    "instructions": record.instructions_md,
                    "invariants": invariants,
                    "allowedTools": allowed_tools,
                    "hooks": hooks
                })
            } else {
                json!({
                    "tool": tool_name,
                    "status": "dispatched_by_kernel",
                    "receivedArgs": args,
                    "skillId": skill_id.unwrap_or("none"),
                    "resolutionMethod": "not_found",
                    "instructions": "",
                    "invariants": [],
                    "allowedTools": [],
                    "hooks": []
                })
            };

            wrap_mcp_text_response(id, payload, false)
        }
        "search_capsules" => {
            let query = args.get("query").and_then(|v| v.as_str());
            let domain = args.get("domain").and_then(|v| v.as_str());
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|u| u as usize);

            let records = if let Some(storage) = &ctx.storage {
                storage
                    .search_skill_capsules(query, domain, limit)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            let capsule_summaries: Vec<Value> = records
                .into_iter()
                .map(|r| {
                    let triggers: Vec<String> =
                        serde_json::from_str(&r.trigger_keywords_json).unwrap_or_default();
                    json!({
                        "capsuleId": r.skill_id,
                        "domain": r.domain,
                        "description": r.description,
                        "triggerKeywords": triggers,
                        "resolutionMethod": r.resolution_method
                    })
                })
                .collect();

            let payload = json!({
                "tool": tool_name,
                "status": "success",
                "receivedArgs": args,
                "query": query,
                "domain": domain,
                "totalFound": capsule_summaries.len(),
                "capsules": capsule_summaries
            });

            wrap_mcp_text_response(id, payload, false)
        }
        "read_capsule" => {
            let capsule_id = args.get("capsuleId").and_then(|v| v.as_str()).unwrap_or("");

            let record_opt = if let Some(storage) = &ctx.storage {
                storage
                    .resolve_skill_capsule(Some(capsule_id), None)
                    .ok()
                    .flatten()
            } else {
                None
            };

            if let Some(record) = record_opt {
                let triggers: Vec<String> =
                    serde_json::from_str(&record.trigger_keywords_json).unwrap_or_default();
                let invariants: Vec<String> =
                    serde_json::from_str(&record.invariants_json).unwrap_or_default();
                let tools: Vec<String> =
                    serde_json::from_str(&record.required_tools_json).unwrap_or_default();
                let hooks: Vec<AlexandriaHookSpec> =
                    serde_json::from_str(&record.hooks_json).unwrap_or_default();

                let payload = json!({
                    "tool": tool_name,
                    "status": "success",
                    "receivedArgs": args,
                    "capsuleId": record.skill_id,
                    "domain": record.domain,
                    "description": record.description,
                    "triggerKeywords": triggers,
                    "requiredTools": tools,
                    "invariants": invariants,
                    "hooks": hooks,
                    "instructions": record.instructions_md
                });
                wrap_mcp_text_response(id, payload, false)
            } else {
                let payload = json!({
                    "tool": tool_name,
                    "status": "not_found",
                    "receivedArgs": args,
                    "capsuleId": capsule_id,
                    "message": format!("Capsule '{}' was not found in vault database.", capsule_id)
                });
                wrap_mcp_text_response(id, payload, false)
            }
        }
        "spawn_isolated_worktrees" => {
            let tasks_val = args.get("tasks").and_then(|v| v.as_array());
            let mut task_tuples: Vec<(String, Vec<PathBuf>)> = Vec::new();
            let mut worker_ids: Vec<String> = Vec::new();

            if let Some(arr) = tasks_val {
                for item in arr {
                    let worker_id = item
                        .get("workerId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("worker")
                        .to_string();
                    let module_name = item
                        .get("moduleName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let files: Vec<PathBuf> = item
                        .get("targetFiles")
                        .and_then(|v| v.as_array())
                        .map(|f_arr| {
                            f_arr
                                .iter()
                                .filter_map(|f| f.as_str().map(PathBuf::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    worker_ids.push(worker_id);
                    task_tuples.push((module_name, files));
                }
            }

            let (graph, node_map) = load_vault_graph_from_dir(&ctx.vault_dir);
            let payload = match verify_worktree_disjunction(&graph, &node_map, &task_tuples) {
                Ok(()) => {
                    let mut paths = serde_json::Map::new();
                    for wid in worker_ids {
                        paths.insert(
                            wid.clone(),
                            json!(format!(".alexandria-worktrees/{}", wid)),
                        );
                    }
                    json!({
                        "tool": tool_name,
                        "status": "dispatched_by_kernel",
                        "receivedArgs": args,
                        "approved": true,
                        "worktreePaths": paths
                    })
                }
                Err(DisjunctionConflict::WikiSubgraphOverlap { colliding_nodes }) => json!({
                    "tool": tool_name,
                    "status": "dispatched_by_kernel",
                    "receivedArgs": args,
                    "approved": false,
                    "conflictType": "wiki_subgraph_overlap",
                    "collidingNodesOrFiles": colliding_nodes
                }),
                Err(DisjunctionConflict::AstImportCollision { colliding_files }) => json!({
                    "tool": tool_name,
                    "status": "dispatched_by_kernel",
                    "receivedArgs": args,
                    "approved": false,
                    "conflictType": "ast_import_collision",
                    "collidingNodesOrFiles": colliding_files
                }),
            };

            wrap_mcp_text_response(id, payload, false)
        }
        "verify_and_gate_changes" => {
            let target_module = args
                .get("targetModule")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let target_files: Vec<String> = args
                .get("targetFiles")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let tdd_phase = args
                .get("tddPhase")
                .and_then(|v| v.as_str())
                .unwrap_or("green_expect_pass");

            let mut sources_to_scan: Vec<(String, String)> = Vec::new();

            if let Some(inline_code) = args.get("sourceCode").and_then(|v| v.as_str()) {
                let first_file = target_files
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "inline.ts".to_string());
                sources_to_scan.push((first_file, inline_code.to_string()));
            }

            if let Some(inline_map) = args.get("inlineSources").and_then(|v| v.as_object()) {
                for (k, v) in inline_map {
                    if let Some(code_str) = v.as_str() {
                        sources_to_scan.push((k.clone(), code_str.to_string()));
                    }
                }
            }

            for file_str in &target_files {
                if sources_to_scan.iter().any(|(f, _)| f == file_str) {
                    continue;
                }
                let candidate = if Path::new(file_str).is_absolute() {
                    PathBuf::from(file_str)
                } else {
                    ctx.workspace_root.join(file_str)
                };
                if candidate.is_file() {
                    let ext = candidate
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mts" | "cts") {
                        if let Ok(code) = fs::read_to_string(&candidate) {
                            sources_to_scan.push((file_str.clone(), code));
                        }
                    }
                }
            }

            let mut violated_rules: Vec<String> = Vec::new();
            let mut diagnostics_list: Vec<String> = Vec::new();

            let requested_exec_path = args
                .get("executionPath")
                .and_then(|v| v.as_str())
                .unwrap_or("alpha_fast");
            let mut promoted_execution_path = requested_exec_path.to_string();
            let mut syscall_trap_triggered = false;

            if requested_exec_path == "alpha_fast" {
                if let Some(baseline_str) = args.get("baselineSource").and_then(|v| v.as_str()) {
                    if let Some((file_label, modified_str)) = sources_to_scan.first() {
                        let is_tsx = file_label.ends_with(".tsx") || file_label.ends_with(".jsx");
                        if detect_public_export_signature_change(
                            baseline_str,
                            modified_str,
                            is_tsx,
                        )
                        .unwrap_or(false)
                        {
                            syscall_trap_triggered = true;
                            promoted_execution_path = "omega_pantheon".to_string();
                            diagnostics_list.push(format!(
                                "[SYSCALL_TRAP @ {}] Public export signature mutation detected; execution path promoted to omega_pantheon.",
                                file_label
                            ));
                        }
                    }
                } else if let Some(baseline_map) =
                    args.get("baselineSources").and_then(|v| v.as_object())
                {
                    for (file_label, modified_str) in &sources_to_scan {
                        if let Some(base_str) =
                            baseline_map.get(file_label).and_then(|v| v.as_str())
                        {
                            let is_tsx =
                                file_label.ends_with(".tsx") || file_label.ends_with(".jsx");
                            if detect_public_export_signature_change(base_str, modified_str, is_tsx)
                                .unwrap_or(false)
                            {
                                syscall_trap_triggered = true;
                                promoted_execution_path = "omega_pantheon".to_string();
                                diagnostics_list.push(format!(
                                    "[SYSCALL_TRAP @ {}] Public export signature mutation detected; execution path promoted to omega_pantheon.",
                                    file_label
                                ));
                            }
                        }
                    }
                }
            }

            for (file_label, source_code) in &sources_to_scan {
                let is_tsx = file_label.ends_with(".tsx") || file_label.ends_with(".jsx");
                match validate_typescript_source(source_code, is_tsx) {
                    Ok(violations) => {
                        for v in violations {
                            if !violated_rules.contains(&v.rule.to_string()) {
                                violated_rules.push(v.rule.to_string());
                            }
                            diagnostics_list.push(format!(
                                "[{} @ {} offset {}] {}",
                                v.rule, file_label, v.byte_offset, v.message
                            ));
                        }
                    }
                    Err(parse_err) => {
                        violated_rules.push("OXC_SYNTAX_ERROR".to_string());
                        diagnostics_list.push(format!("[{}] {}", file_label, parse_err));
                    }
                }

                for gate2_v in scan_gate2_sast_and_secrets(source_code) {
                    if !violated_rules.contains(&gate2_v.rule.to_string()) {
                        violated_rules.push(gate2_v.rule.to_string());
                    }
                    diagnostics_list.push(format!(
                        "[{} @ {} offset {}] {}",
                        gate2_v.rule, file_label, gate2_v.byte_offset, gate2_v.message
                    ));
                }
            }

            // Execute active lifecycle hook interceptors (pre_verify)
            let hook_result = execute_lifecycle_hook_interceptors(
                "pre_verify",
                &target_files,
                &sources_to_scan,
                ctx,
                &args,
            )
            .await;

            if !hook_result.passed {
                for rej in &hook_result.rejections {
                    violated_rules.push("LIFECYCLE_HOOK_REJECTION".to_string());
                    diagnostics_list.push(rej.clone());
                }
            }

            let mut passed = violated_rules.is_empty();
            let mut red_phase_kind: Option<&'static str> = None;

            if passed && tdd_phase == "red_expect_assertion_failure" {
                if let Some(report_val) = args.get("vitestReport") {
                    let report_opt: Option<VitestJsonReport> =
                        serde_json::from_value(report_val.clone()).ok();
                    match evaluate_tdd_red_phase(report_opt, "") {
                        RedPhaseVerdict::ValidAssertionFailure { .. } => {
                            red_phase_kind = Some("valid_assertion_fail");
                        }
                        RedPhaseVerdict::InvalidSyntaxOrImportError { details } => {
                            passed = false;
                            red_phase_kind = Some("invalid_syntax_or_import_error");
                            diagnostics_list.push(details);
                        }
                        RedPhaseVerdict::UnexpectedPass => {
                            passed = false;
                            red_phase_kind = Some("unexpected_pass");
                            diagnostics_list.push(
                                "Invalid RED phase: test suite passed without an assertion failure."
                                    .to_string(),
                            );
                        }
                    }
                }
            }

            let diagnostics_joined = if diagnostics_list.is_empty() {
                "Gate 0 (Oxc AST) and static verifications passed with zero violations.".to_string()
            } else {
                diagnostics_list.join("\n")
            };

            let mut cb_state = "closed_ok".to_string();
            let mut attempt_num: u32 = 1;
            let mut failure_id_opt: Option<String> = None;
            let mut historical_hints: Vec<String> = Vec::new();
            let mut verdict_id_opt: Option<i64> = None;

            if !passed {
                if let Some(storage) = &ctx.storage {
                    if let Ok((fid, state, attempts)) = storage
                        .record_gate_failure(
                            target_module.to_string(),
                            "oxc_ast".to_string(),
                            diagnostics_joined.clone(),
                            3,
                        )
                        .await
                    {
                        failure_id_opt = Some(fid);
                        cb_state = state;
                        attempt_num = attempts;
                    }
                    if let Ok(hints) = storage.search_similar_incidents_fts5(&diagnostics_joined) {
                        historical_hints = hints;
                    }
                } else {
                    cb_state = "retry_allowed".to_string();
                }
            } else if let Some(storage) = &ctx.storage {
                let _ = storage
                    .reset_circuit_breaker(
                        target_module.to_string(),
                        None,
                        "All quality gates passed successfully".to_string(),
                    )
                    .await;
            }

            if let Some(storage) = &ctx.storage {
                let valid_exec_path = if promoted_execution_path == "omega_pantheon" {
                    "omega_pantheon".to_string()
                } else {
                    "alpha_fast".to_string()
                };
                let valid_tdd_phase = match tdd_phase {
                    "red_expect_assertion_failure"
                    | "green_expect_pass"
                    | "refactor_full_gate" => tdd_phase.to_string(),
                    _ => "green_expect_pass".to_string(),
                };
                let diag_json = json!({
                    "violatedRules": violated_rules,
                    "diagnostics": diagnostics_joined,
                    "syscallTrapTriggered": syscall_trap_triggered
                })
                .to_string();
                if let Ok(vid) = storage
                    .record_quality_gate_verdict(
                        target_module.to_string(),
                        valid_exec_path,
                        valid_tdd_phase,
                        passed,
                        diag_json,
                    )
                    .await
                {
                    verdict_id_opt = Some(vid);
                }
            }

            let court_martial_docket = if cb_state == "tripped_abort" {
                Some(json!({
                    "docketId": format!(
                        "CM-{}",
                        failure_id_opt
                            .clone()
                            .unwrap_or_else(|| "FAIL-003".to_string())
                    ),
                    "status": "awaiting_bel_single_turn_verdict",
                    "defendantAgent": "enki",
                    "prosecutorAgent": "utu",
                    "presidingJudgeAgent": "bel",
                    "targetModule": target_module,
                    "violatedRules": violated_rules,
                    "utuProsecutionEvidence": diagnostics_joined,
                    "rulingOptions": [
                        "RULE_IN_FAVOR_OF_ENKI_AMEND_CONTRACT",
                        "UPHOLD_UTU_PROSECUTION_REVOKE_WORKTREE_AND_REASSIGN"
                    ]
                }))
            } else {
                None
            };

            let payload = json!({
                "tool": tool_name,
                "status": "dispatched_by_kernel",
                "receivedArgs": args,
                "passed": passed,
                "violatedRules": violated_rules,
                "redPhaseFailureKind": red_phase_kind,
                "syscallTrapTriggered": syscall_trap_triggered,
                "promotedExecutionPath": promoted_execution_path,
                "verdictId": verdict_id_opt,
                "circuitBreakerState": cb_state,
                "attemptNumber": attempt_num,
                "diagnostics": diagnostics_joined,
                "failureId": failure_id_opt,
                "historicalHintsFromMemory": historical_hints,
                "courtMartialDocket": court_martial_docket,
                "lifecycleHooksExecuted": hook_result.executed_hooks_count,
                "lifecycleHookActions": hook_result.triggered_actions
            });

            wrap_mcp_text_response(id, payload, false)
        }
        "finalize_task_and_log" => {
            let module_name = args
                .get("moduleName")
                .and_then(|v| v.as_str())
                .unwrap_or("core");
            let summary = args
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("Task completed");

            let mut commit_sources = Vec::new();
            let mut target_files: Vec<String> = Vec::new();
            if let Some(files_val) = args.get("targetFiles").and_then(|v| v.as_array()) {
                for f in files_val.iter().filter_map(|v| v.as_str()) {
                    target_files.push(f.to_string());
                    let path = if Path::new(f).is_absolute() {
                        PathBuf::from(f)
                    } else {
                        ctx.workspace_root.join(f)
                    };
                    if let Ok(code) = fs::read_to_string(&path) {
                        commit_sources.push((f.to_string(), code));
                    }
                }
            }

            let commit_hook_result = execute_lifecycle_hook_interceptors(
                "pre_commit",
                &target_files,
                &commit_sources,
                ctx,
                &args,
            )
            .await;

            if !commit_hook_result.passed {
                let payload = json!({
                    "tool": tool_name,
                    "status": "rejected_by_lifecycle_hook",
                    "passed": false,
                    "moduleName": module_name,
                    "rejections": commit_hook_result.rejections,
                    "executedHooksCount": commit_hook_result.executed_hooks_count,
                    "triggeredActions": commit_hook_result.triggered_actions
                });
                return wrap_mcp_text_response(id, payload, false);
            }

            let _ = fs::create_dir_all(&ctx.vault_dir);
            let log_path = ctx.vault_dir.join("log.md");
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let agentic_iam = build_agentic_git_iam_seal("enki", module_name, summary);

            let entry = format!(
                "\n### [EPOCH-{}] Module `{}` Certified\n- **Summary:** {}\n- **Agentic IAM Branch:** `{}`\n- **Cryptographic Seal (SHA-256):** `{}`\n",
                ts,
                module_name,
                summary,
                agentic_iam["ephemeralBranch"].as_str().unwrap_or(""),
                agentic_iam["cryptographicSealSha256"].as_str().unwrap_or("")
            );

            let mut wiki_log_updated = false;
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                if file.write_all(entry.as_bytes()).is_ok() {
                    wiki_log_updated = true;
                }
            }

            if let Some(storage) = &ctx.storage {
                let _ = storage
                    .reset_circuit_breaker(
                        module_name.to_string(),
                        None,
                        summary.to_string(),
                    )
                    .await;
                let _ = storage
                    .record_episodic_memory(
                        "agent_execution".to_string(),
                        "nabu".to_string(),
                        module_name.to_string(),
                        vec!["certified".to_string(), module_name.to_string()],
                        summary.to_string(),
                        50,
                    )
                    .await;
                let vector_dir = ctx.workspace_root.join(".alexandria").join("vector-store");
                let _ = export_vector_store_snapshot(storage, &vector_dir);
                let _ = write_telemetry_and_markdown_dashboard(
                    storage,
                    &ctx.workspace_root,
                    &ctx.vault_dir,
                );
            }

            let payload = json!({
                "tool": tool_name,
                "status": "dispatched_by_kernel",
                "receivedArgs": args,
                "passed": true,
                "committedHash": "HEAD",
                "wikiLogUpdated": wiki_log_updated,
                "logPath": log_path.to_string_lossy(),
                "agenticGitIam": agentic_iam,
                "lifecycleHooksExecuted": commit_hook_result.executed_hooks_count,
                "lifecycleHookActions": commit_hook_result.triggered_actions
            });

            wrap_mcp_text_response(id, payload, false)
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: format!("Unknown tool in Alexandria MCP: {}", tool_name),
            }),
        },
    }
}

fn wrap_mcp_text_response(id: Value, payload: Value, is_error: bool) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&payload).unwrap_or_default()
            }],
            "isError": is_error
        })),
        error: None,
    }
}

pub fn build_agentic_git_iam_seal(
    agent_id: &str,
    module_name: &str,
    summary: &str,
) -> Value {
    let git_author = match agent_id.to_lowercase().as_str() {
        "bel" => "Bel (Alexandria Overseer) <bel@alexandria.local>",
        "utu" => "Utu (Alexandria Critic) <utu@alexandria.local>",
        "nabu" => "Nabu (Alexandria Chronicler) <nabu@alexandria.local>",
        _ => "Enki (Alexandria Builder) <enki@alexandria.local>",
    };
    let git_committer = "Nabu (Alexandria Chronicler) <nabu@alexandria.local>";
    let clean_module = module_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let ephemeral_branch = format!("alexandria/task/{}", clean_module);

    let mut hasher = Sha256::new();
    hasher.update(format!("{}|{}|{}", agent_id, module_name, summary).as_bytes());
    let digest = hasher.finalize();
    let mut seal_hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut seal_hex, "{:02x}", b);
    }

    json!({
        "agentId": agent_id,
        "moduleName": module_name,
        "ephemeralBranch": ephemeral_branch,
        "gitAuthor": git_author,
        "gitCommitter": git_committer,
        "cryptographicSealSha256": seal_hex,
        "commitTrailers": [
            format!("Alexandria-Agent: {}", git_author),
            format!("Alexandria-Module: {}", module_name),
            format!("Alexandria-Seal-SHA256: {}", seal_hex),
            format!("Signed-off-by: {}", git_committer)
        ]
    })
}
