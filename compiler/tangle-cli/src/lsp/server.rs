use std::io::{self, BufRead, Read, Write};
use serde_json::{Value, json};

/// Minimal LSP server using stdio JSON-RPC
pub struct LspServer {
    diagnostics_enabled: bool,
}

impl LspServer {
    pub fn new() -> Self {
        LspServer {
            diagnostics_enabled: true,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = stdin.lock();
        let mut writer = stdout.lock();

        let mut buffer = String::new();

        loop {
            buffer.clear();
            // Read Content-Length header
            let mut content_length: Option<usize> = None;
            loop {
                buffer.clear();
                if reader.read_line(&mut buffer)? == 0 {
                    break;
                }
                let line = buffer.trim().to_string();
                if line.is_empty() {
                    break;
                } // end of headers
                if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                    content_length = len_str.trim().parse().ok();
                }
            }

            let len = match content_length {
                Some(l) => l,
                None => continue,
            };

            // Read message body
            let mut body = vec![0u8; len];
            reader.read_exact(&mut body)?;
            let request: Value = match serde_json::from_slice(&body) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let method = request
                .get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            let id = request.get("id").cloned();

            match method {
                "initialize" => {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "capabilities": {
                                "textDocumentSync": 1,
                                "diagnosticProvider": { "interFileDependencies": false }
                            },
                            "serverInfo": { "name": "tangle-lsp", "version": "0.1.0" }
                        }
                    });
                    Self::write_message(&mut writer, &response)?;
                }
                "textDocument/didOpen" | "textDocument/didChange" => {
                    if let Some(params) = request.get("params") {
                        if let Some(doc) = params.get("textDocument") {
                            if let Some(uri) = doc.get("uri").and_then(|u| u.as_str()) {
                                let text = doc
                                    .get("text")
                                    .or_else(|| {
                                        params
                                            .get("contentChanges")
                                            .and_then(|c| c[0].get("text"))
                                    })
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("");

                                // Run compiler diagnostics
                                let diagnostics = self.check_file(uri, text);
                                let notification = json!({
                                    "jsonrpc": "2.0",
                                    "method": "textDocument/publishDiagnostics",
                                    "params": {
                                        "uri": uri,
                                        "diagnostics": diagnostics
                                    }
                                });
                                Self::write_message(&mut writer, &notification)?;
                            }
                        }
                    }
                }
                "shutdown" => {
                    let response = json!({"jsonrpc": "2.0", "id": id, "result": null});
                    Self::write_message(&mut writer, &response)?;
                    break;
                }
                "exit" => break,
                _ => {
                    // Unknown method — respond with null result if id exists
                    if let Some(id_val) = &id {
                        let response =
                            json!({"jsonrpc": "2.0", "id": id_val, "result": null});
                        Self::write_message(&mut writer, &response)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn check_file(&self, uri: &str, source: &str) -> Vec<Value> {
        if !self.diagnostics_enabled {
            return vec![];
        }

        use crate::frontend::compile_module::{compile_module, CompileModuleInput};

        let input = CompileModuleInput {
            file: uri.to_string(),
            source: source.to_string(),
        };
        let module = compile_module(input);

        module
            .diagnostics
            .iter()
            .map(|d| {
                json!({
                    "range": {
                        "start": { "line": d.span.start_line - 1, "character": d.span.start_column - 1 },
                        "end": { "line": d.span.end_line - 1, "character": d.span.end_column - 1 }
                    },
                    "severity": 1,
                    "code": d.code,
                    "source": "tangle",
                    "message": d.message
                })
            })
            .collect()
    }

    fn write_message(writer: &mut dyn Write, msg: &Value) -> io::Result<()> {
        let body = serde_json::to_string(msg).unwrap_or_default();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        writer.write_all(header.as_bytes())?;
        writer.write_all(body.as_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}
