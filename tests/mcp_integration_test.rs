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
    fs::create_dir_all(&base_dir).expect("Creación de entorno temporal exitosa");

    let db_path = base_dir.join("vault.db");
    let storage = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)
        .expect("Actor SQLite WAL inicializado");

    let repo_vault = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("alexandria");

    let ctx = McpKernelContext::new(base_dir.clone(), repo_vault, Some(storage.clone()));
    (base_dir, ctx, storage)
}

fn extract_tool_payload(resp_result: Option<Value>) -> Value {
    let res = resp_result.expect("Respuesta JSON-RPC con result");
    assert_eq!(res["isError"], json!(false));
    let text = res["content"][0]["text"]
        .as_str()
        .expect("Texto JSON en content[0]");
    serde_json::from_str(text).expect("Payload JSON de herramienta válido")
}

#[tokio::test]
async fn test_mcp_real_prepare_task_context_classification() {
    let (temp_dir, ctx, _storage) = create_temp_integration_env("prepare_ctx");

    // 1. Tocar prisma/schema.prisma debe forzar omega_pantheon y elevar ADRs desde alexandria/
    let omega_resp = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Añadir tabla de suscripciones en prisma/schema.prisma",
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
        "Modificar prisma/schema.prisma debe imponer obligatoriamente omega_pantheon"
    );
    assert!(
        omega_payload["architecturalContext"]
            .as_str()
            .is_some_and(|c| c.contains("Gate 0") || c.contains("oxc_parser")),
        "Debe cargar el contrato real desde alexandria/modules/gate-guard-ast.md"
    );
    let hoisted = omega_payload["hoistedAdrs"]
        .as_array()
        .expect("hoistedAdrs debe ser un array");
    assert!(
        !hoisted.is_empty(),
        "Debe elevar los ADRs constitucionales enlazados desde el módulo"
    );

    // 2. Tocar únicamente src/ui/button.tsx debe asignar alpha_fast
    let alpha_resp = dispatch_mcp_method_with_context(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Ajustar padding del botón primario",
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
        "Modificar src/ui/button.tsx sin impacto estructural debe asignar alpha_fast"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_mcp_real_verify_changes_rejects_any() {
    let (temp_dir, ctx, _storage) = create_temp_integration_env("verify_reject_any");

    // Escribimos un archivo TypeScript físico con `let x: any = 10;` en el workspace temporal
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir).expect("Directorio src creado");
    let bad_file_rel = "src/violating_service.ts";
    fs::write(
        temp_dir.join(bad_file_rel),
        "export function compute(): number {\n  let x: any = 10;\n  return x;\n}\n",
    )
    .expect("Archivo TS escrito");

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
        "El servidor MCP debe rechazar código con 'any' explícito"
    );

    let violated_rules = payload["violatedRules"]
        .as_array()
        .expect("violatedRules presente");
    assert!(
        violated_rules.contains(&json!("NO_EXPLICIT_ANY")),
        "Debe reportar explícitamente la regla NO_EXPLICIT_ANY: {:?}",
        violated_rules
    );
    assert!(
        payload["diagnostics"]
            .as_str()
            .is_some_and(|d| d.contains("NO_EXPLICIT_ANY")),
        "El diagnóstico debe detallar la infracción NO_EXPLICIT_ANY"
    );
    assert_eq!(payload["circuitBreakerState"], json!("retry_allowed"));
    assert!(
        payload["failureId"]
            .as_str()
            .is_some_and(|id| id.starts_with("FAIL-BILLING-")),
        "Debe registrar el incidente forense en vault.db"
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
            "Garantiza clausura léxica estricta en transacciones interactivas Prisma".to_string(),
            r#"["prisma", "transaction", "acid", "deadlock"]"#.to_string(),
            r#"["NO_TX_ALIASING", "DIRECT_AWAIT_ONLY", "NO_EXTERNAL_FETCH_IN_TX"]"#.to_string(),
            "Usa siempre `await tx.<model>.<op>()` directamente dentro del callback inline."
                .to_string(),
        )
        .await
        .expect("Inserción de skill en vault.db exitosa");

    // 1. Resolución por ID exacto (`exact_id`)
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

    // 2. Resolución por búsqueda léxica FTS5 BM25 (`fts5_bm25_fallback`) usando intentQuery
    let fts_resp = dispatch_alexandria_tool_with_context(
        "resolve_skill_capsule",
        json!({
            "intentQuery": "Cómo prevenir deadlock en transaction interactiva de prisma"
        }),
        json!(302),
        &ctx,
    )
    .await;

    let fts_payload = extract_tool_payload(fts_resp.result);
    assert_eq!(
        fts_payload["skillId"],
        json!("skill-prisma-acid-closure"),
        "FTS5 BM25 con stemmer Porter debe recuperar la skill por intentQuery"
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
