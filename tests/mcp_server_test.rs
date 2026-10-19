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
        "Initialize handshake must not produce an error"
    );

    let result = response
        .result
        .expect("Result must be present in initialize");
    assert_eq!(result["protocolVersion"], json!("2024-11-05"));
    assert_eq!(result["serverInfo"]["name"], json!("alexandria-runtime"));
    assert_eq!(
        result["serverInfo"]["version"],
        json!("2.4.0-OmniPragmatic")
    );
    assert_eq!(
        result["capabilities"]["tools"]["listChanged"],
        json!(false),
        "Prefix Pinning requires listChanged to be false"
    );
}

#[tokio::test]
async fn test_mcp_tools_list_schema_completeness() {
    let response = dispatch_mcp_method("tools/list", json!({}), json!(2)).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!(2));
    assert!(response.error.is_none());

    let result = response
        .result
        .expect("Result must be present in tools/list");
    let tools = result["tools"]
        .as_array()
        .expect("Field 'tools' must be a JSON array");

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
        "Exactly the 6 official kernel tools must be exposed"
    );

    for (idx, expected_name) in expected_names.iter().enumerate() {
        let tool = &tools[idx];
        assert_eq!(
            tool["name"].as_str(),
            Some(*expected_name),
            "Tool ordering must be deterministic (Prefix Pinning) at index {}",
            idx
        );
        assert!(
            tool["description"]
                .as_str()
                .is_some_and(|d| !d.trim().is_empty()),
            "Tool {} must have a non-empty description",
            expected_name
        );

        let schema = &tool["inputSchema"];
        assert_eq!(
            schema["type"],
            json!("object"),
            "inputSchema of {} must be of type object",
            expected_name
        );
        assert_eq!(
            schema["additionalProperties"],
            json!(false),
            "inputSchema of {} must forbid additionalProperties (additionalProperties: false)",
            expected_name
        );
        assert!(
            schema["properties"].is_object(),
            "inputSchema of {} must define its properties map",
            expected_name
        );
    }
}

#[tokio::test]
async fn test_mcp_tools_call_dispatch() {
    // 1. Invocation of prepare_task_context via tools/call
    let call_prepare = dispatch_mcp_method(
        "tools/call",
        json!({
            "name": "prepare_task_context",
            "arguments": {
                "instruction": "Audit payment transaction closure",
                "targetFiles": ["src/billing/service.ts"],
                "moduleName": "billing"
            }
        }),
        json!(10),
    )
    .await;

    assert!(call_prepare.error.is_none());
    let prep_result = call_prepare.result.expect("Result in tools/call");
    assert_eq!(prep_result["isError"], json!(false));

    let content_arr = prep_result["content"]
        .as_array()
        .expect("content must be an MCP array");
    assert_eq!(content_arr.len(), 1);
    assert_eq!(content_arr[0]["type"], json!("text"));

    let inner_payload: Value = serde_json::from_str(
        content_arr[0]["text"]
            .as_str()
            .expect("Serialized JSON text"),
    )
    .expect("Valid inner JSON payload");
    assert_eq!(inner_payload["tool"], json!("prepare_task_context"));
    assert_eq!(inner_payload["status"], json!("dispatched_by_kernel"));
    assert_eq!(
        inner_payload["receivedArgs"]["moduleName"],
        json!("billing")
    );

    // 2. Invocation of verify_and_gate_changes via tools/call
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
    let verify_result = call_verify
        .result
        .expect("Result in verify_and_gate_changes");
    assert_eq!(verify_result["isError"], json!(false));
    let verify_payload: Value =
        serde_json::from_str(verify_result["content"][0]["text"].as_str().expect("JSON text"))
            .expect("Valid JSON");
    assert_eq!(verify_payload["tool"], json!("verify_and_gate_changes"));
    assert_eq!(
        verify_payload["receivedArgs"]["tddPhase"],
        json!("red_expect_assertion_failure")
    );

    // 3. Verification of non-existent tool (-32602 Invalid params / unknown tool)
    let unknown_tool_resp =
        dispatch_alexandria_tool("non_existent_tool", json!({}), json!(12)).await;
    assert!(unknown_tool_resp.result.is_none());
    let err = unknown_tool_resp
        .error
        .expect("Unknown tool must return an error");
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
        .expect("Unknown method must return JSON-RPC error object");
    assert_eq!(
        err.code, -32601,
        "Standard JSON-RPC 2.0 error code for Method not found must be -32601"
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

    // 1. Structural verification on JsonRpcRequest
    let parsed: JsonRpcRequest =
        serde_json::from_str(raw_notification).expect("Valid notification deserialization");
    assert_eq!(parsed.jsonrpc, "2.0");
    assert_eq!(parsed.method, "notifications/initialized");
    assert!(
        parsed.id.is_none(),
        "A JSON-RPC notification does not have an id field"
    );

    // 2. Functional verification: line handler must emit no response on stdout
    let maybe_response = handle_raw_jsonrpc_line(raw_notification).await;
    assert!(
        maybe_response.is_none(),
        "Notifications without 'id' must never generate a stdout response"
    );

    // 3. Additional verification: blank lines also emit no response
    assert!(handle_raw_jsonrpc_line("   \n  ").await.is_none());
}
