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
    assert_eq!(result["serverInfo"]["name"], json!("alexandria"));
    assert_eq!(
        result["serverInfo"]["version"],
        json!(env!("CARGO_PKG_VERSION"))
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
        "search_capsules",
        "read_capsule",
        "spawn_isolated_worktrees",
        "verify_and_gate_changes",
        "finalize_task_and_log",
    ];

    assert_eq!(
        tools.len(),
        expected_names.len(),
        "Exactly the 8 official kernel tools must be exposed"
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
        dispatch_alexandria_tool("unknown_arbitrary_tool", json!({}), json!(12)).await;
    assert_eq!(unknown_tool_resp.jsonrpc, "2.0");
    assert_eq!(unknown_tool_resp.id, json!(12));
    assert!(unknown_tool_resp.error.is_some());
    let err = unknown_tool_resp.error.expect("Error on unknown tool");
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("unknown_arbitrary_tool"));
}

#[tokio::test]
async fn test_mcp_search_and_read_capsules_tools() {
    // 1. Test search_capsules via tools/call
    let search_resp = dispatch_mcp_method(
        "tools/call",
        json!({
            "name": "search_capsules",
            "arguments": {
                "query": "rust concurrency",
                "limit": 5
            }
        }),
        json!(20),
    )
    .await;

    assert!(search_resp.error.is_none());
    let s_res = search_resp.result.expect("Result in search_capsules");
    assert_eq!(s_res["isError"], json!(false));
    let s_payload: Value = serde_json::from_str(
        s_res["content"][0]["text"].as_str().expect("text"),
    )
    .expect("Valid JSON");
    assert_eq!(s_payload["tool"], json!("search_capsules"));
    assert_eq!(s_payload["status"], json!("success"));

    // 2. Test read_capsule via tools/call
    let read_resp = dispatch_mcp_method(
        "tools/call",
        json!({
            "name": "read_capsule",
            "arguments": {
                "capsuleId": "testing:tdd-practitioner"
            }
        }),
        json!(21),
    )
    .await;

    assert!(read_resp.error.is_none());
    let r_res = read_resp.result.expect("Result in read_capsule");
    assert_eq!(r_res["isError"], json!(false));
    let r_payload: Value = serde_json::from_str(
        r_res["content"][0]["text"].as_str().expect("text"),
    )
    .expect("Valid JSON");
    assert_eq!(r_payload["tool"], json!("read_capsule"));
}

#[tokio::test]
async fn test_mcp_lifecycle_hook_interception() {
    // 1. verify_and_gate_changes with an explicit hook forbidding any
    let call_verify_hook = dispatch_mcp_method(
        "tools/call",
        json!({
            "name": "verify_and_gate_changes",
            "arguments": {
                "targetModule": "billing",
                "targetFiles": ["src/billing/service.ts"],
                "executionPath": "alpha_fast",
                "tddPhase": "green_expect_pass",
                "activeHooks": [
                    {
                        "event": "pre_verify",
                        "action": "forbid_any"
                    }
                ]
            }
        }),
        json!(30),
    )
    .await;

    assert!(call_verify_hook.error.is_none());
    let verify_res = call_verify_hook.result.expect("Result");
    let verify_payload: Value = serde_json::from_str(
        verify_res["content"][0]["text"].as_str().expect("text"),
    )
    .expect("Valid JSON");
    assert_eq!(verify_payload["tool"], json!("verify_and_gate_changes"));
    assert!(verify_payload["lifecycleHooksExecuted"].as_u64().unwrap_or(0) >= 1);

    // 2. finalize_task_and_log with an explicit hook forbidding secrets
    let call_finalize_hook = dispatch_mcp_method(
        "tools/call",
        json!({
            "name": "finalize_task_and_log",
            "arguments": {
                "moduleName": "billing",
                "summary": "Completed billing audit",
                "activeHooks": [
                    {
                        "event": "pre_commit",
                        "action": "forbid_secrets"
                    }
                ]
            }
        }),
        json!(31),
    )
    .await;

    assert!(call_finalize_hook.error.is_none());
    let final_res = call_finalize_hook.result.expect("Result");
    let final_payload: Value = serde_json::from_str(
        final_res["content"][0]["text"].as_str().expect("text"),
    )
    .expect("Valid JSON");
    assert_eq!(final_payload["tool"], json!("finalize_task_and_log"));
    assert!(final_payload["passed"].as_bool().unwrap_or(false));
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
