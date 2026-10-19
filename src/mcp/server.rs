use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

pub async fn run_mcp_stdio_loop() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Alexandria Runtime v2.4.0 MCP Server iniciado sobre stdio");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await.map_err(|e| e.to_string())? {
        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Trama JSON-RPC malformada: {}", e);
                let err_resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                let mut out = serde_json::to_string(&err_resp).unwrap_or_default();
                out.push('\n');
                let _ = stdout.write_all(out.as_bytes()).await;
                let _ = stdout.flush().await;
                continue;
            }
        };

        let Some(req_id) = req.id else {
            continue;
        };

        let response = dispatch_mcp_method(&req.method, req.params, req_id).await;
        let mut serialized = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        serialized.push('\n');

        stdout
            .write_all(serialized.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stdout.flush().await.map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn dispatch_mcp_method(method: &str, params: Value, id: Value) -> JsonRpcResponse {
    match method {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "alexandria-runtime",
                    "version": "2.4.0-OmniPragmatic"
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
                        "description": "Clasifica Path Alpha/Omega en Zero-Turn y resuelve el grafo depth=1 con ADR Hoisting.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["instruction", "targetFiles"],
                            "additionalProperties": false,
                            "properties": {
                                "instruction": { "type": "string" },
                                "targetFiles": { "type": "array", "items": { "type": "string" } },
                                "moduleName": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "upsert_architectural_contract",
                        "description": "Crea o actualiza ADRs y contratos de módulo en alexandria/ (Path Omega).",
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
                        "description": "Inyecta una cápsula JIT en el sufijo dinámico sin invalidar el Prefix Cache.",
                        "inputSchema": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "skillId": { "type": "string" },
                                "intentQuery": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "spawn_isolated_worktrees",
                        "description": "Autoriza Git worktrees paralelos verificando disyunción doble en petgraph y oxc_resolver.",
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
                        "description": "Ejecuta Gate 0 (Oxc AST), Gate 1-4 en Sandbox Node (--permission) y Auditoría Stateless de Utu.",
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
                        "description": "Realiza el commit atómico, actualiza alexandria/log.md y levanta el entorno de desarrollo.",
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
            dispatch_alexandria_tool(tool_name, args, id).await
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Método JSON-RPC no encontrado: {}", method),
            }),
        },
    }
}

pub async fn dispatch_alexandria_tool(tool_name: &str, args: Value, id: Value) -> JsonRpcResponse {
    match tool_name {
        "prepare_task_context"
        | "upsert_architectural_contract"
        | "resolve_skill_capsule"
        | "spawn_isolated_worktrees"
        | "verify_and_gate_changes"
        | "finalize_task_and_log" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&json!({
                        "tool": tool_name,
                        "status": "dispatched_by_kernel",
                        "receivedArgs": args
                    })).unwrap_or_default()
                }],
                "isError": false
            })),
            error: None,
        },
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: format!("Herramienta desconocida en Alexandria MCP: {}", tool_name),
            }),
        },
    }
}
