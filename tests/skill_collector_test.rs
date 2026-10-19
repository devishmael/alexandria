use alexandria::collector::skill_etl::{
    AlexandriaCapsuleJson, AlexandriaCapsuleManifest, CANONICAL_CAPSULE_SCHEMA_VERSION,
    builtin_core_capsules, collect_and_refactor_directory, export_refactored_json_bundle,
    parse_raw_skill_or_hook, sanitize_skill_id_for_filename,
};
use alexandria::mcp::server::{McpKernelContext, dispatch_alexandria_tool_with_context};
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::VaultStorageHandle;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COLLECTOR_TEST_SEQ: AtomicUsize = AtomicUsize::new(1);

fn create_temp_test_dir(label: &str) -> PathBuf {
    let seq = COLLECTOR_TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "alexandria_collector_test_{}_{}_{}_{}",
        label,
        std::process::id(),
        nanos,
        seq
    ));
    fs::create_dir_all(&dir).expect("Temporary test directory must be created");
    dir
}

#[test]
fn test_parse_canonical_and_external_markdown_skills() {
    // 1. Canonical Alexandria Section 8 skill with full YAML frontmatter
    let canonical_md = r#"---
id: "database:prisma-acid-closure"
domain: "database"
profile_compatibility: ["mvp-fullstack-ts", "enterprise-microservices"]
description: "Strict ACID lexical closure rules inside interactive Prisma transactions."
trigger_keywords: ["prisma", "transaction", "acid", "closure"]
required_tools: ["prepare_task_context", "verify_and_gate_changes"]
invariants:
  - "NO_TX_ALIASING: Never assign or destructure the `tx` parameter."
  - "DIRECT_AWAIT_ONLY: Every `tx.<model>.<op>()` call must be immediately awaited."
hooks:
  - event: "pre_gate_0"
    action: "scan_ast_invariants"
    target_filter: "src/**/*.ts"
---

# Prisma Interactive Transaction Rules

Always execute interactive transactions in under 25ms with zero external network I/O.
"#;

    let canonical_capsule =
        parse_raw_skill_or_hook(Path::new("prisma-acid-closure.md"), canonical_md)
            .expect("Canonical Alexandria skill markdown must parse cleanly");

    assert_eq!(
        canonical_capsule.schema_version,
        CANONICAL_CAPSULE_SCHEMA_VERSION
    );
    assert_eq!(canonical_capsule.skill_id, "database:prisma-acid-closure");
    assert_eq!(canonical_capsule.domain, "database");
    assert_eq!(
        canonical_capsule.profile_compatibility,
        vec!["mvp-fullstack-ts", "enterprise-microservices"]
    );
    assert!(
        canonical_capsule
            .trigger_keywords
            .contains(&"prisma".to_string())
    );
    assert_eq!(
        canonical_capsule.required_tools,
        vec!["prepare_task_context", "verify_and_gate_changes"]
    );
    assert_eq!(canonical_capsule.invariants.len(), 2);
    assert_eq!(canonical_capsule.hooks.len(), 1);
    assert_eq!(canonical_capsule.hooks[0].event, "pre_gate_0");
    assert_eq!(canonical_capsule.hooks[0].action, "scan_ast_invariants");
    assert_eq!(canonical_capsule.content_sha256.len(), 64);

    // 2. External Antigravity / Claude SKILL.md with lightweight frontmatter (name, description)
    //    and Markdown sections (# Invariants, # Hooks) parsed via pulldown-cmark
    let external_skill_md = r#"---
name: vitest-red-green-runner
description: Enforces strict RED-GREEN-REFACTOR TDD workflow with Vitest JSON output reports.
---

# Overview

Execute unit tests in an isolated sandbox before mutating source files.

## Invariants
- Never modify `src/` without a failing test in `tests/`.
- Require `AssertionError` in RED phase.

## Hooks
- pre_execution: enforce_red_phase (tests/**/*.test.ts)
"#;

    let external_capsule =
        parse_raw_skill_or_hook(Path::new("skills/vitest/SKILL.md"), external_skill_md)
            .expect("External SKILL.md must be refactored into AlexandriaCapsuleJson");

    assert_eq!(
        external_capsule.schema_version,
        CANONICAL_CAPSULE_SCHEMA_VERSION
    );
    assert!(
        external_capsule
            .skill_id
            .contains("vitest-red-green-runner"),
        "Refactored skill_id must incorporate external name: {}",
        external_capsule.skill_id
    );
    assert_eq!(external_capsule.domain, "testing");
    assert!(
        !external_capsule.trigger_keywords.is_empty(),
        "Trigger keywords must be inferred automatically from name and description"
    );
    assert_eq!(
        external_capsule.invariants.len(),
        2,
        "pulldown-cmark must extract bullet points under ## Invariants"
    );
    assert!(
        !external_capsule.hooks.is_empty(),
        "pulldown-cmark must extract hooks under ## Hooks"
    );
    assert_eq!(external_capsule.hooks[0].event, "pre_execution");
    assert_eq!(external_capsule.hooks[0].action, "enforce_red_phase");
    assert_eq!(external_capsule.content_sha256.len(), 64);
}

#[test]
fn test_collect_directory_and_export_json_manifest() {
    let temp_dir = create_temp_test_dir("collect_export");
    let source_dir = temp_dir.join("raw_skills");
    let output_dir = temp_dir.join("dist_capsules");
    fs::create_dir_all(source_dir.join("nested")).expect("Created nested source dir");

    // 1. Write a Markdown skill
    fs::write(
        source_dir.join("branded-types.md"),
        r#"---
name: nominal-branded-types
domain: backend
description: Forbids explicit any and enforces nominal Branded Types in TypeScript.
invariants:
  - "NO_EXPLICIT_ANY"
  - "NO_DOUBLE_CAST_UNKNOWN"
---
Use `type Brand<K, T> = K & { __brand: T }` for domain identifiers.
"#,
    )
    .expect("Wrote branded-types.md");

    // 2. Write a YAML hook/tool specification
    fs::write(
        source_dir.join("nested").join("security-sast-hook.yaml"),
        r#"skill_id: "security:sast-secret-guard"
domain: "security"
description: "Scans diffs for leaked credentials and enforces stateless Utu audit."
trigger_keywords:
  - "security"
  - "sast"
  - "secret"
required_tools:
  - "verify_and_gate_changes"
invariants:
  - "ZERO_SECRET_LEAKS"
hooks:
  - event: "post_gate_3"
    action: "run_stateless_utu_audit"
    target_filter: "src/**"
instructions_markdown: "Verify all environment variables are excluded from source control."
"#,
    )
    .expect("Wrote security-sast-hook.yaml");

    let capsules = collect_and_refactor_directory(&source_dir)
        .expect("Directory collection must succeed");
    assert_eq!(capsules.len(), 2, "Must collect both .md and .yaml capsules");

    let manifest_path = export_refactored_json_bundle(&capsules, &output_dir)
        .expect("Exporting JSON bundle must succeed");
    assert!(
        manifest_path.exists(),
        "alexandria-capsules.manifest.json must be created"
    );

    let manifest_raw = fs::read_to_string(&manifest_path).expect("Read manifest JSON");
    let manifest: AlexandriaCapsuleManifest =
        serde_json::from_str(&manifest_raw).expect("Valid manifest JSON structure");
    assert_eq!(manifest.schema_version, CANONICAL_CAPSULE_SCHEMA_VERSION);
    assert_eq!(manifest.total_capsules, 2);
    assert_eq!(manifest.capsules.len(), 2);

    for cap in &capsules {
        let expected_filename = format!("{}.capsule.json", sanitize_skill_id_for_filename(&cap.skill_id));
        let cap_path = output_dir.join(&expected_filename);
        assert!(
            cap_path.exists(),
            "Individual capsule JSON file must exist: {:?}",
            cap_path
        );
        let parsed_cap: AlexandriaCapsuleJson =
            serde_json::from_str(&fs::read_to_string(&cap_path).unwrap())
                .expect("Capsule file must deserialize to AlexandriaCapsuleJson");
        assert_eq!(parsed_cap.skill_id, cap.skill_id);
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_builtin_capsules_auto_seeded_in_vault_db() {
    let builtins = builtin_core_capsules();
    assert_eq!(builtins.len(), 4, "Exactly 4 factory core capsules must exist");

    let temp_dir = create_temp_test_dir("builtin_seed");
    let db_path = temp_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path.clone(), CANONICAL_SCHEMA_SQL)
        .expect("VaultStorageHandle must initialize and auto-seed built-in capsules");

    let expected_ids = [
        "testing:tdd-vitest",
        "database:prisma-acid-closure",
        "typescript:nominal-branded-types",
        "security:zero-trust-sast",
    ];

    // 1. Verify exact_id resolution for all 4 built-in capsules
    for expected_id in expected_ids {
        let resolved = storage
            .resolve_skill_capsule(Some(expected_id), None)
            .expect("Query must succeed")
            .unwrap_or_else(|| panic!("Built-in skill '{}' must be auto-seeded", expected_id));
        assert_eq!(resolved.skill_id, expected_id);
        assert_eq!(resolved.resolution_method, "exact_id");
        assert!(!resolved.required_tools_json.is_empty());
        assert!(!resolved.hooks_json.is_empty());
    }

    // 2. Verify FTS5 BM25 search resolution for built-in capsules
    let bm25_match = storage
        .resolve_skill_capsule(None, Some("nominal branded types explicit any typescript"))
        .expect("FTS5 query must succeed")
        .expect("FTS5 BM25 must resolve typescript:nominal-branded-types");
    assert_eq!(bm25_match.skill_id, "typescript:nominal-branded-types");
    assert_eq!(bm25_match.resolution_method, "fts5_bm25_fallback");

    // 3. Verify MCP tool `resolve_skill_capsule` returns real `allowedTools` and `hooks` from SQLite
    let ctx = McpKernelContext::new(temp_dir.clone(), temp_dir.join("wiki"), Some(storage));
    let mcp_resp = dispatch_alexandria_tool_with_context(
        "resolve_skill_capsule",
        json!({ "skillId": "testing:tdd-vitest" }),
        json!(901),
        &ctx,
    )
    .await;

    let result = mcp_resp.result.expect("MCP response must contain result");
    let payload: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["skillId"], json!("testing:tdd-vitest"));
    assert!(
        payload["allowedTools"]
            .as_array()
            .is_some_and(|arr| !arr.is_empty()),
        "MCP resolve_skill_capsule must return stored allowedTools"
    );
    assert!(
        payload["hooks"]
            .as_array()
            .is_some_and(|arr| !arr.is_empty()),
        "MCP resolve_skill_capsule must return stored hooks"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_collect_subcommand_e2e() {
    let bin = env!("CARGO_BIN_EXE_alexandria");
    let temp_dir = create_temp_test_dir("cli_collect_e2e");
    let raw_input_dir = temp_dir.join("input_skills");
    let custom_out_dir = temp_dir.join("custom_capsules_out");
    fs::create_dir_all(&raw_input_dir).expect("Created input_skills directory");

    fs::write(
        raw_input_dir.join("SKILL.md"),
        r#"---
name: custom-e2e-collector-skill
description: E2E verification skill for alexandria collect CLI subcommand.
---

# Invariants
- Enforce deterministic JSON output.
"#,
    )
    .expect("Wrote SKILL.md");

    let output = Command::new(bin)
        .current_dir(&temp_dir)
        .args([
            "collect",
            raw_input_dir.to_str().unwrap(),
            "--out",
            custom_out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute alexandria collect subcommand");

    assert!(
        output.status.success(),
        "alexandria collect must exit with code 0. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let summary_json: Value = serde_json::from_str(stdout_str.trim())
        .unwrap_or_else(|e| panic!("Expected JSON summary on stdout, got '{}': {}", stdout_str, e));

    assert_eq!(summary_json["status"], json!("collected"));
    assert_eq!(summary_json["totalCapsules"], json!(1));
    assert!(
        custom_out_dir
            .join("alexandria-capsules.manifest.json")
            .exists(),
        "Manifest file must be generated in --out directory"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
