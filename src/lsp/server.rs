use crate::gate_guard::oxc_validator::{
    scan_gate2_sast_and_secrets, validate_typescript_source,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

fn byte_offset_to_line_col(source: &str, byte_offset: u32) -> (u32, u32) {
    let clamped = (byte_offset as usize).min(source.len());
    let prefix = &source[..clamped];
    let mut line: u32 = 0;
    let mut col: u32 = 0;
    for ch in prefix.chars() {
        if ch == '\n' {
            line += 1;
            col = 0;
        } else if ch != '\r' {
            col += 1;
        }
    }
    (line, col)
}

pub fn handle_lsp_jsonrpc_message(raw_json: &str) -> Option<Value> {
    let parsed: Value = serde_json::from_str(raw_json.trim()).ok()?;
    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = parsed.get("id").cloned().unwrap_or(Value::Null);
    let params = parsed.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "serverInfo": {
                    "name": "alexandria-lsp",
                    "version": "2.5.0-OmniPragmatic"
                },
                "capabilities": {
                    "textDocumentSync": 1,
                    "diagnosticProvider": {
                        "interFileDependencies": false,
                        "workspaceDiagnostics": false
                    }
                }
            }
        })),
        "shutdown" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": Value::Null
        })),
        "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .unwrap_or("file:///unknown.ts")
                .to_string();

            let text_opt = params
                .pointer("/textDocument/text")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    params
                        .pointer("/contentChanges/0/text")
                        .and_then(|v| v.as_str())
                })
                .or_else(|| params.get("text").and_then(|v| v.as_str()));

            let source_code = text_opt.unwrap_or("");
            let is_tsx = uri.ends_with(".tsx") || uri.ends_with(".jsx");

            let mut lsp_diagnostics: Vec<Value> = Vec::new();

            match validate_typescript_source(source_code, is_tsx) {
                Ok(violations) => {
                    for v in violations {
                        let (line, col) = byte_offset_to_line_col(source_code, v.byte_offset);
                        lsp_diagnostics.push(json!({
                            "range": {
                                "start": { "line": line, "character": col },
                                "end": { "line": line, "character": col.saturating_add(8) }
                            },
                            "severity": 1,
                            "code": v.rule,
                            "source": "alexandria-gateguard",
                            "message": v.message
                        }));
                    }
                }
                Err(parse_err) => {
                    lsp_diagnostics.push(json!({
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": { "line": 0, "character": 1 }
                        },
                        "severity": 1,
                        "code": "OXC_SYNTAX_ERROR",
                        "source": "alexandria-gateguard",
                        "message": parse_err
                    }));
                }
            }

            for gate2 in scan_gate2_sast_and_secrets(source_code) {
                let (line, col) = byte_offset_to_line_col(source_code, gate2.byte_offset);
                lsp_diagnostics.push(json!({
                    "range": {
                        "start": { "line": line, "character": col },
                        "end": { "line": line, "character": col.saturating_add(12) }
                    },
                    "severity": 1,
                    "code": gate2.rule,
                    "source": "alexandria-gateguard",
                    "message": gate2.message
                }));
            }

            Some(json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": {
                    "uri": uri,
                    "diagnostics": lsp_diagnostics
                }
            }))
        }
        _ => None,
    }
}

pub async fn run_lsp_stdio_loop() -> Result<(), String> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        let bytes_read = reader
            .read_line(&mut line_buf)
            .await
            .map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
            let content_len: usize = len_str.trim().parse().unwrap_or(0);
            loop {
                line_buf.clear();
                let header_read = reader
                    .read_line(&mut line_buf)
                    .await
                    .map_err(|e| e.to_string())?;
                if header_read == 0 || line_buf.trim().is_empty() {
                    break;
                }
            }
            if content_len > 0 {
                let mut payload_buf = vec![0u8; content_len];
                reader
                    .read_exact(&mut payload_buf)
                    .await
                    .map_err(|e| e.to_string())?;
                let raw_json = String::from_utf8_lossy(&payload_buf);
                if let Some(resp) = handle_lsp_jsonrpc_message(&raw_json) {
                    let body = serde_json::to_string(&resp).map_err(|e| e.to_string())?;
                    let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
                    stdout
                        .write_all(frame.as_bytes())
                        .await
                        .map_err(|e| e.to_string())?;
                    stdout.flush().await.map_err(|e| e.to_string())?;
                }
            }
        } else if trimmed.starts_with('{') {
            if let Some(resp) = handle_lsp_jsonrpc_message(trimmed) {
                let mut body = serde_json::to_string(&resp).map_err(|e| e.to_string())?;
                body.push('\n');
                stdout
                    .write_all(body.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
                stdout.flush().await.map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}
