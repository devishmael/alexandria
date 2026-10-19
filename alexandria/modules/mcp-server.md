---
id: mcp-server
title: "Contrato de Subsistema: Servidor MCP JSON-RPC 2.0 sobre Stdio y Esquemas Inmutables"
type: module
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - module
  - mcp
  - json-rpc
  - stdio
  - prefix-pinning
  - schemas
---

# Contrato de Subsistema: Servidor MCP JSON-RPC 2.0 sobre Stdio y Esquemas Inmutables

> **Documento Canónico de Módulo:** `alexandria/modules/mcp-server.md`  
> **Archivo de Destino en Kernel:** `src/mcp/server.rs`  
> **Versión del Protocolo MCP:** `2024-11-05`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-003-zero-trust-quality-gates]]

---

## 1. Arquitectura de Transporte y Aislamiento Estricto de Canales

El subsistema **MCP Server** expone las primitivas de espacio de kernel de Alexandria Runtime a clientes de espacio de usuario (Cursor, Windsurf, Claude Desktop, Antigravity IDE o agentes CLI) mediante el protocolo **JSON-RPC 2.0 sobre `stdio`**, cumpliendo rigurosamente la especificación oficial Model Context Protocol (`2024-11-05`).

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                         AISLAMIENTO FÍSICO DE DESCRIPTORES DE ARCHIVO                    │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│  FD 0 (stdin)   ──► Tokio AsyncBufReadExt::lines() ──► Deserializador JsonRpcRequest     │
│                                                                                          │
│  FD 1 (stdout)  ◄── Tramas JSON-RPC 2.0 puras delimitadas por '\n' + flush() inmediato   │
│                     (PROHIBIDO emitir logs, banners o secuencias ANSI en stdout)         │
│                                                                                          │
│  FD 2 (stderr)  ◄── tracing_subscriber::fmt().with_writer(std::io::stderr).with_ansi(false)│
│                     (Exclusivo para telemetría diagnóstica y trazas del kernel)          │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Invariantes del Transporte JSON-RPC 2.0
1. **Pureza de `stdout`:** Cualquier byte no perteneciente a un objeto JSON-RPC válido emitido en `stdout` corrompe el parser del cliente IDE y derriba la sesión MCP. Toda telemetría de Rust (`tracing::info!`, `tracing::warn!`, `eprintln!`) se enruta forzosamente a `stderr` con `with_ansi(false)`.
2. **Tratamiento de Notificaciones:** Los mensajes JSON-RPC entrantes que carecen del campo `id` (por ejemplo, `"method": "notifications/initialized"` o `"notifications/cancelled"`) son notificaciones unidireccionales. El servidor **nunca** debe emitir una respuesta en `stdout` para mensajes sin `id`.
3. **Inmutabilidad Bit a Bit (*Prefix Pinning*):** El arreglo devuelto por `tools/list` mantiene un orden determinista e inmutable durante todo el ciclo de vida del proceso, garantizando que el bloque de herramientas fijado en el prefijo del LLM conserve un $100\%$ de *KV-Cache Hit* ([[ADR-002-prefix-pinned-caching]]).

---

## 2. Contrato Tipado de Interfaz (`AlexandriaMCPServerV2_4`)

```typescript
export interface AlexandriaMCPServerV2_4 {
  /**
   * 1. Clasificación Zero-Turn (Path Alpha vs. Path Omega) y resolución en RAM (< 0.2 ms)
   *    del subgrafo Depth=1 más ADR Hoisting transitivo (Depth <= 6).
   */
  prepare_task_context(params: {
    instruction: string;
    targetFiles: string[];
    moduleName?: string;
  }): Promise<{
    enforcedPath: 'alpha_fast' | 'omega_pantheon';
    architecturalContext: string;
    hoistedAdrs: string[];
    activeInvariants: string[];
    requiredGates: Array<'oxc_ast' | 'types' | 'security' | 'tests_sandbox' | 'build_and_utu'>;
  }>;

  /**
   * 2. Creación o actualización transaccional de contratos en alexandria/modules/
   *    y registros constitucionales [[ADR-XXX]] en alexandria/decisions/.
   */
  upsert_architectural_contract(params: {
    moduleName: string;
    technicalSpecMarkdown: string;
    adrTitle?: string;
    adrDecisionMarkdown?: string;
  }): Promise<{
    adrPath?: string;
    moduleContractPath: string;
    graphUpdated: boolean;
  }>;

  /**
   * 3. Inyección dinámica de Cápsulas JIT en el sufijo volátil de la ventana
   *    mediante ID exacto, similitud vectorial o fallback FTS5 BM25 Porter.
   */
  resolve_skill_capsule(params: {
    skillId?: string;
    intentQuery?: string;
  }): Promise<{
    skillId: string;
    resolutionMethod: 'exact_id' | 'vector_similarity' | 'fts5_bm25_fallback';
    instructions: string;
    invariants: string[];
    allowedTools: string[];
  }>;

  /**
   * 4. Verificación en <= 2 ms de la doble disyunción (subgrafos en petgraph y
   *    árbol de imports en oxc_resolver) antes de crear Git worktrees paralelos.
   */
  spawn_isolated_worktrees(params: {
    tasks: Array<{
      workerId: string;
      moduleName: string;
      targetFiles: string[];
    }>;
  }): Promise<{
    approved: boolean;
    worktreePaths?: Record<string, string>;
    conflictType?: 'wiki_subgraph_overlap' | 'ast_import_collision';
    collidingNodesOrFiles?: string[];
  }>;

  /**
   * 5. Ejecución fuera de banda de los Quality Gates (Gate 0 a Gate 4), validación
   *    semántica de la fase TDD activa y gestión del Circuit Breaker.
   */
  verify_and_gate_changes(params: {
    targetModule: string;
    targetFiles: string[];
    executionPath: 'alpha_fast' | 'omega_pantheon';
    tddPhase: 'red_expect_assertion_failure' | 'green_expect_pass' | 'refactor_full_gate';
  }): Promise<{
    passed: boolean;
    redPhaseFailureKind?: 'valid_assertion_fail' | 'invalid_syntax_or_import_error' | 'unexpected_pass';
    circuitBreakerState: 'closed_ok' | 'retry_allowed' | 'tripped_abort';
    attemptNumber: number;
    diagnostics: string;
    utuColdAuditVerdict?: {
      approved: boolean;
      findings: string[];
    };
    failureId?: string;
    historicalHintsFromMemory?: string[];
  }>;

  /**
   * 6. Cierre canónico: commit Git atómico, registro en alexandria/log.md,
   *    reinicio del Circuit Breaker a closed_ok y levantamiento opcional de servidor dev.
   */
  finalize_task_and_log(params: {
    moduleName: string;
    summary: string;
    startDevServer?: boolean;
  }): Promise<{
    committedHash: string;
    wikiLogUpdated: boolean;
    localUrl?: string;
  }>;
}
```

---

## 3. Especificación de Esquemas JSON de Entrada y Salida por Herramienta

### 3.1. `prepare_task_context`
- **Input JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["instruction", "targetFiles"],
    "additionalProperties": false,
    "properties": {
      "instruction": {
        "type": "string",
        "description": "Directiva técnica o de producto a ejecutar."
      },
      "targetFiles": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Lista de rutas relativas de archivos que se planea inspeccionar o modificar."
      },
      "moduleName": {
        "type": "string",
        "description": "Identificador canónico del módulo en alexandria/modules/."
      }
    }
  }
  ```
- **Output JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["enforcedPath", "architecturalContext", "hoistedAdrs", "activeInvariants", "requiredGates"],
    "properties": {
      "enforcedPath": { "type": "string", "enum": ["alpha_fast", "omega_pantheon"] },
      "architecturalContext": { "type": "string" },
      "hoistedAdrs": { "type": "array", "items": { "type": "string" } },
      "activeInvariants": { "type": "array", "items": { "type": "string" } },
      "requiredGates": {
        "type": "array",
        "items": { "type": "string", "enum": ["oxc_ast", "types", "security", "tests_sandbox", "build_and_utu"] }
      }
    }
  }
  ```

### 3.2. `upsert_architectural_contract`
- **Input JSON Schema:**
  ```json
  {
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
  ```
- **Output JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["moduleContractPath", "graphUpdated"],
    "properties": {
      "adrPath": { "type": "string" },
      "moduleContractPath": { "type": "string" },
      "graphUpdated": { "type": "boolean" }
    }
  }
  ```

### 3.3. `resolve_skill_capsule`
- **Input JSON Schema:**
  ```json
  {
    "type": "object",
    "additionalProperties": false,
    "properties": {
      "skillId": { "type": "string" },
      "intentQuery": { "type": "string" }
    }
  }
  ```
- **Output JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["skillId", "resolutionMethod", "instructions", "invariants", "allowedTools"],
    "properties": {
      "skillId": { "type": "string" },
      "resolutionMethod": { "type": "string", "enum": ["exact_id", "vector_similarity", "fts5_bm25_fallback"] },
      "instructions": { "type": "string" },
      "invariants": { "type": "array", "items": { "type": "string" } },
      "allowedTools": { "type": "array", "items": { "type": "string" } }
    }
  }
  ```

### 3.4. `spawn_isolated_worktrees`
- **Input JSON Schema:**
  ```json
  {
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
  ```
- **Output JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["approved"],
    "properties": {
      "approved": { "type": "boolean" },
      "worktreePaths": { "type": "object", "additionalProperties": { "type": "string" } },
      "conflictType": { "type": "string", "enum": ["wiki_subgraph_overlap", "ast_import_collision"] },
      "collidingNodesOrFiles": { "type": "array", "items": { "type": "string" } }
    }
  }
  ```

### 3.5. `verify_and_gate_changes`
- **Input JSON Schema:**
  ```json
  {
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
  ```
- **Output JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["passed", "circuitBreakerState", "attemptNumber", "diagnostics"],
    "properties": {
      "passed": { "type": "boolean" },
      "redPhaseFailureKind": {
        "type": "string",
        "enum": ["valid_assertion_fail", "invalid_syntax_or_import_error", "unexpected_pass"]
      },
      "circuitBreakerState": { "type": "string", "enum": ["closed_ok", "retry_allowed", "tripped_abort"] },
      "attemptNumber": { "type": "integer", "minimum": 1 },
      "diagnostics": { "type": "string" },
      "utuColdAuditVerdict": {
        "type": "object",
        "required": ["approved", "findings"],
        "properties": {
          "approved": { "type": "boolean" },
          "findings": { "type": "array", "items": { "type": "string" } }
        }
      },
      "failureId": { "type": "string" },
      "historicalHintsFromMemory": { "type": "array", "items": { "type": "string" } }
    }
  }
  ```

### 3.6. `finalize_task_and_log`
- **Input JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["moduleName", "summary"],
    "additionalProperties": false,
    "properties": {
      "moduleName": { "type": "string" },
      "summary": { "type": "string" },
      "startDevServer": { "type": "boolean" }
    }
  }
  ```
- **Output JSON Schema:**
  ```json
  {
    "type": "object",
    "required": ["committedHash", "wikiLogUpdated"],
    "properties": {
      "committedHash": { "type": "string" },
      "wikiLogUpdated": { "type": "boolean" },
      "localUrl": { "type": "string" }
    }
  }
  ```

---

## 4. Implementación de Referencia en Rust (`src/mcp/server.rs`)

```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

pub async fn handle_raw_jsonrpc_line(line: &str) -> Option<JsonRpcResponse> {
    if line.trim().is_empty() {
        return None;
    }

    let req: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Trama JSON-RPC malformada: {}", e);
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

    // Las notificaciones JSON-RPC 2.0 (sin campo `id`) jamás deben emitir respuesta
    let req_id = req.id?;
    Some(dispatch_mcp_method(&req.method, req.params, req_id).await)
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
        if let Some(response) = handle_raw_jsonrpc_line(&line).await {
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
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
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
```

---

## 5. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Runtime Central:** [[core-engine]]
- **Motor Topológico:** [[topological-graph]]
- **Escáner AST Gate 0:** [[gate-guard-ast]]
- **Sandbox Runner Gate 3:** [[sandbox-runner]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-003-zero-trust-quality-gates]]
- **Bitácora de Cambios:** [[log]]
