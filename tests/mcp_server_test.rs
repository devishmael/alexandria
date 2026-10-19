use alexandria::mcp::server::{
    JsonRpcRequest, dispatch_alexandria_tool, dispatch_mcp_method, handle_raw_jsonrpc_line,
};
use serde_json::{Value, json};

#[tokio::test]
async fn test_mcp_initialize_handshake() {
    let response = dispatch_mcp_method(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "antigravity-host", "version": "2.0.0" }
        }),
        json!(1),
    )
    .await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!(1));
    assert!(
        response.error.is_none(),
        "El handshake initialize no debe producir error"
    );

    let result = response.result.expect("Resultado presente en initialize");
    assert_eq!(result["protocolVersion"], json!("2024-11-05"));
    assert_eq!(result["serverInfo"]["name"], json!("alexandria-runtime"));
    assert_eq!(
        result["serverInfo"]["version"],
        json!("2.4.0-OmniPragmatic")
    );
    assert_eq!(
        result["capabilities"]["tools"]["listChanged"],
        json!(false),
        "Prefix Pinning exige que listChanged sea false"
    );
}

#[tokio::test]
async fn test_mcp_tools_list_schema_completeness() {
    let response = dispatch_mcp_method("tools/list", json!({}), json!(2)).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!(2));
    assert!(response.error.is_none());

    let result = response.result.expect("Resultado presente en tools/list");
    let tools = result["tools"]
        .as_array()
        .expect("El campo 'tools' debe ser un arreglo JSON");

    let expected_names = [
        "prepare_task_context",
        "upsert_architectural_contract",
        "resolve_skill_capsule",
        "spawn_isolated_worktrees",
        "verify_and_gate_changes",
        "finalize_task_and_log",
    ];

    assert_eq!(
        tools.len(),
        expected_names.len(),
        "Deben exponerse exactamente las 6 herramientas oficiales del kernel"
    );

    for (idx, expected_name) in expected_names.iter().enumerate() {
        let tool = &tools[idx];
        assert_eq!(
            tool["name"].as_str(),
            Some(*expected_name),
            "El orden de las herramientas debe ser determinista (Prefix Pinning) en el índice {}",
            idx
        );
        assert!(
            tool["description"]
                .as_str()
                .is_some_and(|d| !d.trim().is_empty()),
            "La herramienta {} debe poseer descripción no vacía",
            expected_name
        );

        let schema = &tool["inputSchema"];
        assert_eq!(
            schema["type"],
            json!("object"),
            "inputSchema de {} debe ser de tipo object",
            expected_name
        );
        assert_eq!(
            schema["additionalProperties"],
            json!(false),
            "inputSchema de {} debe bloquear propiedades adicionales (additionalProperties: false)",
            expected_name
        );
        assert!(
            schema["properties"].is_object(),
            "inputSchema de {} debe definir su mapa de properties",
            expected_name
        );
    }
}

#[tokio::test]
async fn test_mcp_tools_call_dispatch() {
    // 1. Invocación de prepare_task_context vía tools/call
    let call_prepare = dispatch_mcp_method(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Auditar transacción de pagos",
                "targetFiles": ["src/billing/service.ts"],
                "moduleName": "billing"
            }
        }),
        json!(10),
    )
    .await;

    assert!(call_prepare.error.is_none());
    let prep_result = call_prepare.result.expect("Result en tools/call");
    assert_eq!(prep_result["isError"], json!(false));

    let content_arr = prep_result["content"]
        .as_array()
        .expect("content debe ser un array MCP");
    assert_eq!(content_arr.len(), 1);
    assert_eq!(content_arr[0]["type"], json!("text"));

    let inner_payload: Value = serde_json::from_str(
        content_arr[0]["text"]
            .as_str()
            .expect("Texto JSON serializado"),
    )
    .expect("Payload interno JSON válido");
    assert_eq!(inner_payload["tool"], json!("prepare_task_context"));
    assert_eq!(inner_payload["status"], json!("dispatched_by_kernel"));
    assert_eq!(
        inner_payload["receivedArgs"]["moduleName"],
        json!("billing")
    );

    // 2. Invocación de verify_and_gate_changes vía tools/call
    let call_verify = dispatch_mcp_method(
        "tools/call",
        json!({
            "name": "verify_and_gate_changes",
            "arguments": {
                "targetModule": "billing",
                "targetFiles": ["src/billing/service.ts"],
                "executionPath": "alpha_fast",
                "tddPhase": "red_expect_assertion_failure"
            }
        }),
        json!(11),
    )
    .await;

    assert!(call_verify.error.is_none());
    let verify_result = call_verify.result.expect("Result en verify_and_gate_changes");
    assert_eq!(verify_result["isError"], json!(false));
    let verify_payload: Value = serde_json::from_str(
        verify_result["content"][0]["text"]
            .as_str()
            .expect("Texto JSON"),
    )
    .expect("JSON válido");
    assert_eq!(verify_payload["tool"], json!("verify_and_gate_changes"));
    assert_eq!(
        verify_payload["receivedArgs"]["tddPhase"],
        json!("red_expect_assertion_failure")
    );

    // 3. Verificación de herramienta inexistente (-32602 Invalid params / unknown tool)
    let unknown_tool_resp =
        dispatch_alexandria_tool("non_existent_tool", json!({}), json!(12)).await;
    assert!(unknown_tool_resp.result.is_none());
    let err = unknown_tool_resp
        .error
        .expect("Herramienta desconocida debe retornar error");
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn test_mcp_unknown_method_error_code() {
    let response =
        dispatch_mcp_method("resources/subscribe_non_existent", json!({}), json!(404)).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!(404));
    assert!(response.result.is_none());

    let err = response
        .error
        .expect("Método inexistente debe retornar objeto error JSON-RPC");
    assert_eq!(
        err.code, -32601,
        "El código estándar JSON-RPC 2.0 para Method not found debe ser -32601"
    );
    assert!(err.message.contains("resources/subscribe_non_existent"));
}

#[tokio::test]
async fn test_mcp_ping_pong() {
    let response = dispatch_mcp_method("ping", json!({}), json!("ping-42")).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!("ping-42"));
    assert!(response.error.is_none());
    assert_eq!(response.result, Some(json!({})));
}

#[tokio::test]
async fn test_mcp_notification_silence() {
    let raw_notification = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;

    // 1. Verificación estructural sobre JsonRpcRequest
    let parsed: JsonRpcRequest =
        serde_json::from_str(raw_notification).expect("Deserialización válida de notificación");
    assert_eq!(parsed.jsonrpc, "2.0");
    assert_eq!(parsed.method, "notifications/initialized");
    assert!(
        parsed.id.is_none(),
        "Una notificación JSON-RPC no posee campo id"
    );

    // 2. Verificación funcional: el manejador de línea no debe emitir respuesta alguna en stdout
    let maybe_response = handle_raw_jsonrpc_line(raw_notification).await;
    assert!(
        maybe_response.is_none(),
        "Las notificaciones sin 'id' jamás deben generar respuesta en stdout"
    );

    // 3. Verificación adicional: líneas en blanco tampoco emiten respuesta
    assert!(handle_raw_jsonrpc_line("   \n  ").await.is_none());
}
