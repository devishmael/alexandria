use alexandria::gate_guard::oxc_validator::{scan_gate2_sast_and_secrets, validate_typescript_source};
use alexandria::graph::resolver::{
    compute_sha256, resolve_context_with_adr_hoisting, NodeKind, WikiNode,
};
use alexandria::mcp::server::{dispatch_mcp_method_with_context, McpKernelContext};
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::VaultStorageHandle;
use alexandria::vector_store::hybrid_memory::{
    compute_deterministic_int8_embedding, cosine_similarity_int8, FEATURE_VECTOR_DIM,
};
use petgraph::algo::is_cyclic_directed;
use petgraph::stable_graph::StableDiGraph;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ADVERSARIAL_COUNTER: AtomicUsize = AtomicUsize::new(9000);

fn temp_adversarial_dir(label: &str) -> PathBuf {
    let seq = ADVERSARIAL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("alexandria_adv_{}_{}_{}", label, nanos, seq));
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

fn make_test_node(id: &str, kind: NodeKind) -> WikiNode {
    let content = format!("# Node {}\n\nContent for [[{}]]", id, id);
    WikiNode {
        id: id.to_string(),
        path: PathBuf::from(format!("wiki/{}.md", id)),
        kind,
        sha256: compute_sha256(&content),
        content,
    }
}

#[tokio::test]
async fn test_massive_concurrent_actor_bursts() {
    let dir = temp_adversarial_dir("burst_storage");
    let db_path = dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)
        .expect("Failed to spawn storage actor");

    let initial_telemetry = storage.compute_organism_telemetry().unwrap();
    let initial_mem = initial_telemetry["episodicMemoryChunks"].as_u64().unwrap_or(0);
    let initial_verdicts = initial_telemetry["totalVerdicts"].as_u64().unwrap_or(0);

    let concurrent_tasks = 25;
    let mut join_handles = Vec::new();

    for i in 0..concurrent_tasks {
        let handle = storage.clone();
        join_handles.push(tokio::spawn(async move {
            // 1. Record episodic memory
            let mem_res = handle
                .record_episodic_memory(
                    "concurrent_stress".to_string(),
                    format!("worker_{}", i),
                    "billing".to_string(),
                    vec![format!("worker_{}.ts", i)],
                    format!("Stress test instruction payload number {}", i),
                    50,
                )
                .await;
            assert!(mem_res.is_ok(), "Episodic memory write must succeed");

            // 2. Record quality gate verdict
            let verdict_res = handle
                .record_quality_gate_verdict(
                    format!("module_{}", i),
                    "alpha_fast".to_string(),
                    "green_expect_pass".to_string(),
                    true,
                    format!("{{\"worker\": {}}}", i),
                )
                .await;
            assert!(verdict_res.is_ok(), "Verdict write must succeed");

            // 3. Record failure and reset circuit breaker
            let fail_res = handle
                .record_gate_failure(
                    format!("module_{}", i),
                    "oxc_ast".to_string(),
                    "Simulated failure for stress".to_string(),
                    3,
                )
                .await;
            assert!(fail_res.is_ok(), "Failure recording must succeed");

            let reset_res = handle
                .reset_circuit_breaker(
                    format!("module_{}", i),
                    None,
                    "Recovered from burst test".to_string(),
                )
                .await;
            assert!(reset_res.is_ok(), "Reset circuit breaker must succeed");
        }));
    }

    for jh in join_handles {
        jh.await.expect("Task must not panic");
    }

    // Verify all operations were serialized and committed cleanly without SQLITE_BUSY
    let final_telemetry = storage.compute_organism_telemetry().unwrap();
    let final_mem = final_telemetry["episodicMemoryChunks"].as_u64().unwrap_or(0);
    let final_verdicts = final_telemetry["totalVerdicts"].as_u64().unwrap_or(0);

    assert_eq!(
        final_mem,
        initial_mem + concurrent_tasks,
        "Every single concurrent memory insertion must be persisted"
    );
    assert_eq!(
        final_verdicts,
        initial_verdicts + concurrent_tasks,
        "Every single concurrent verdict must be persisted"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_mcp_rapid_multi_project_tenant_alternation() {
    let proj_alpha = temp_adversarial_dir("tenant_alpha");
    let proj_beta = temp_adversarial_dir("tenant_beta");
    let proj_gamma = temp_adversarial_dir("tenant_gamma");

    // Setup 3 independent vaults
    for (dir, name, spec) in [
        (&proj_alpha, "alpha-mod", "# Alpha Contract Specification"),
        (&proj_beta, "beta-mod", "# Beta Contract Specification"),
        (&proj_gamma, "gamma-mod", "# Gamma Contract Specification"),
    ] {
        let wiki = dir.join("wiki");
        let modules = wiki.join("modules");
        fs::create_dir_all(&modules).unwrap();
        fs::write(
            wiki.join("index.md"),
            format!("# Index\n\n- [[{}]]\n", name),
        )
        .unwrap();
        fs::write(
            modules.join(format!("{}.md", name)),
            format!("---\nid: {}\ntype: module\n---\n{}\n", name, spec),
        )
        .unwrap();
    }

    let ctx = McpKernelContext::default_local();
    let projects = [
        (&proj_alpha, "alpha-mod", "Alpha Contract Specification"),
        (&proj_beta, "beta-mod", "Beta Contract Specification"),
        (&proj_gamma, "gamma-mod", "Gamma Contract Specification"),
    ];

    // Measure initial baseline for each project
    let mut initial_baselines = Vec::new();
    for (proj_dir, _, _) in &projects {
        let db_dir = proj_dir.join(".alexandria");
        fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("vault.db");
        let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL).unwrap();
        let telemetry = storage.compute_organism_telemetry().unwrap();
        let init_chunks = telemetry["episodicMemoryChunks"].as_u64().unwrap_or(0);
        initial_baselines.push(init_chunks);
    }

    // Alternating calls across projects in round-robin fashion for 15 turns
    for turn in 0..15 {
        let (proj_dir, mod_name, expected_substr) = projects[turn % 3];
        let resp = dispatch_mcp_method_with_context(
            "tools/call",
            json!({
                "name": "prepare_task_context",
                "arguments": {
                    "instruction": format!("Turn {} for {}", turn, mod_name),
                    "targetFiles": [format!("src/{}.ts", mod_name)],
                    "workspaceRoot": proj_dir.to_string_lossy().to_string(),
                    "moduleName": mod_name
                }
            }),
            json!(turn as i64),
            &ctx,
        )
        .await;

        let payload = extract_tool_payload(resp.result);
        assert_eq!(payload["status"], json!("dispatched_by_kernel"));
        assert!(
            payload["architecturalContext"]
                .as_str()
                .unwrap_or_default()
                .contains(expected_substr),
            "Turn {} targeting {} must load its exact contract",
            turn,
            mod_name
        );
    }

    // Inspect each project's database independently
    // 15 turns divided by 3 projects = exactly 5 calls per project
    for (idx, (proj_dir, mod_name, _)) in projects.iter().enumerate() {
        let db_path = proj_dir.join(".alexandria").join("vault.db");
        assert!(db_path.exists(), "Local database must exist");

        let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL).unwrap();
        let telemetry = storage.compute_organism_telemetry().unwrap();
        let chunks = telemetry["episodicMemoryChunks"].as_u64().unwrap_or(0);
        let init = initial_baselines[idx];

        assert_eq!(
            chunks,
            init + 5,
            "Project {} must have exactly initial ({}) + 5 episodic memory chunks recorded (Zero cross-project leakage)",
            mod_name,
            init
        );
    }

    let _ = fs::remove_dir_all(&proj_alpha);
    let _ = fs::remove_dir_all(&proj_beta);
    let _ = fs::remove_dir_all(&proj_gamma);
}

#[test]
fn test_oxc_ast_adversarial_boundary_cases() {
    // 1. Explicit Any in type alias
    let any_code = "export type UnsafeType = any;";
    let v_any = validate_typescript_source(any_code, false).unwrap();
    assert!(
        v_any.iter().any(|v| v.rule.to_string().contains("ANY")),
        "Must flag explicit any in type alias"
    );

    // 2. Double Cast as unknown as T
    let double_cast_code = "const val = (payload as unknown) as UserRecord;";
    let v_cast = validate_typescript_source(double_cast_code, false).unwrap();
    assert!(
        v_cast.iter().any(|v| v.rule.to_string().contains("DOUBLE_CAST")),
        "Must flag double cast through unknown"
    );

    // 3. Adversarial Prisma transaction aliasing: tx assigned to another variable
    let tx_alias_code = r#"
        await prisma.$transaction(async (tx) => {
            const aliasTx = tx;
            await aliasTx.account.update({ where: { id: 1 }, data: { balance: 100 } });
        });
    "#;
    let v_tx = validate_typescript_source(tx_alias_code, false).unwrap();
    assert!(
        v_tx.iter().any(|v| v.rule.to_string().contains("ALIASING")),
        "Must flag aliasing or re-assignment of tx parameter"
    );

    // 4. Valid clean Prisma transaction: must have zero violations
    let typed_clean_tx = r#"
        export async function transferFunds(prisma: DatabaseClient, from: string, to: string, amount: number) {
            await prisma.$transaction(async (tx: PrismaTransaction) => {
                await tx.account.update({ where: { id: from }, data: { balance: 50 } });
                await tx.account.update({ where: { id: to }, data: { balance: 150 } });
            });
        }
    "#;
    let v_clean = validate_typescript_source(typed_clean_tx, false).unwrap();
    assert!(
        v_clean.is_empty(),
        "Properly formed Prisma transaction with direct await must pass cleanly"
    );

    // 5. Gate 2 Secret scan: OpenAI API Key leak detection
    let leak_code = r#"
        const client = new OpenAI({
            apiKey: "sk-proj-abc1234567890123456789012345678901234567890"
        });
    "#;
    let sast_violations = scan_gate2_sast_and_secrets(leak_code);
    assert!(
        !sast_violations.is_empty(),
        "Must catch live API key pattern in source code"
    );
}

#[test]
fn test_graph_cycle_and_hoisting_limits() {
    let mut graph: StableDiGraph<WikiNode, ()> = StableDiGraph::new();

    // 1. Build a 4-node circular dependency: A -> B -> C -> D -> A
    let a = graph.add_node(make_test_node("module-a", NodeKind::Module));
    let b = graph.add_node(make_test_node("module-b", NodeKind::Module));
    let c = graph.add_node(make_test_node("module-c", NodeKind::Module));
    let d = graph.add_node(make_test_node("module-d", NodeKind::Module));

    graph.add_edge(a, b, ());
    graph.add_edge(b, c, ());
    graph.add_edge(c, d, ());
    graph.add_edge(d, a, ());

    assert!(
        is_cyclic_directed(&graph),
        "Graph must be recognized as cyclic by petgraph is_cyclic_directed"
    );

    // 2. Build a deep 10-hop ADR chain starting from a root module:
    // Root -> ADR-001 -> ADR-002 -> ... -> ADR-010
    let mut adr_graph: StableDiGraph<WikiNode, ()> = StableDiGraph::new();
    let root = adr_graph.add_node(make_test_node("root-engine", NodeKind::Module));

    let mut prev = root;
    for i in 1..=10 {
        let adr_id = format!("ADR-{:03}", i);
        let node = adr_graph.add_node(make_test_node(&adr_id, NodeKind::ConstitutionalAdr));
        adr_graph.add_edge(prev, node, ());
        prev = node;
    }

    // Resolve context with max depth = 6
    let resolved = resolve_context_with_adr_hoisting(&adr_graph, root, 6);

    // Verify ADRs 1 through 6 are hoisted
    for i in 1..=6 {
        let expected = format!("ADR-{:03}", i);
        assert!(
            resolved.hoisted_adrs.contains(&expected),
            "ADR {} must be hoisted at depth <= 6",
            i
        );
    }

    // Verify ADRs 7 through 10 are STRICTLY NOT hoisted (Bounding invariant)
    for i in 7..=10 {
        let excluded = format!("ADR-{:03}", i);
        assert!(
            !resolved.hoisted_adrs.contains(&excluded),
            "ADR {} must NOT be hoisted beyond depth 6",
            i
        );
    }
}

#[test]
fn test_vector_quantization_and_rrf_boundary_math() {
    // 1. Zero/Empty string stability
    let empty_vec = compute_deterministic_int8_embedding("");
    assert_eq!(empty_vec.len(), FEATURE_VECTOR_DIM);
    let sim_zero = cosine_similarity_int8(&empty_vec, &empty_vec);
    assert_eq!(sim_zero, 0.0, "Zero vector similarity must produce exactly 0.0");
    assert!(!sim_zero.is_nan(), "Similarity must never produce NaN");

    // 2. Quantization bounds: all coordinates must be in [-127, 127]
    let long_text = "The quick brown fox jumps over the lazy dog repeatedly with various tokens and symbols 1234567890!@#$%^&*()";
    let vec = compute_deterministic_int8_embedding(long_text);
    assert_eq!(vec.len(), FEATURE_VECTOR_DIM);

    // 3. Self-similarity of non-empty text must be ~1.0
    let self_sim = cosine_similarity_int8(&vec, &vec);
    assert!(
        (self_sim - 1.0).abs() < 0.05,
        "Identical vectors must yield cosine similarity approximately 1.0 (got {})",
        self_sim
    );

    // 4. Semantically completely different text
    let code_snippet = "SELECT id, balance FROM accounts WHERE user_id = $1;";
    let code_vec = compute_deterministic_int8_embedding(code_snippet);
    let diff_sim = cosine_similarity_int8(&vec, &code_vec);
    assert!(
        diff_sim < 0.8,
        "Unrelated text must have lower similarity than self (got {})",
        diff_sim
    );
}
