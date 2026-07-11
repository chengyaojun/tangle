//! LSP probe: runs a fixed LSP protocol sequence against `tangle lsp`
//! for each example file and reports all publishDiagnostics notifications.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lsp-probe <tangle-cli-target-debug> [example.md ...]");
        std::process::exit(2);
    }
    let tangle_bin = &args[1];
    let examples: Vec<String> = if args.len() > 2 {
        args[2..].iter().cloned().collect()
    } else {
        std::fs::read_dir("examples")
            .expect("read examples/")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|s| s.to_str()) == Some("md")
            })
            .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
            .collect()
    };

    let mut total_diags = 0usize;
    for example in &examples {
        eprintln!("--- Probing {} ---", example);
        let mut child = match spawn_lsp(tangle_bin) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ERROR] spawn failed: {}", e);
                continue;
            }
        };
        let stdin = child.stdin.as_mut().unwrap();
        let stdout = child.stdout.as_mut().unwrap();
        let mut reader = BufReader::new(stdout);

        // initialize
        send_msg(
            stdin,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": null,
                    "capabilities": {}
                }
            }),
        );
        let _init_resp = read_msg(&mut reader);

        // initialized notification
        send_msg(
            stdin,
            json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );

        // didOpen
        let uri = format!(
            "file://{}",
            std::fs::canonicalize(example)
                .unwrap_or_default()
                .display()
        );
        let text = std::fs::read_to_string(example).unwrap_or_default();
        send_msg(
            stdin,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "tangle",
                        "version": 1,
                        "text": text
                    }
                }
            }),
        );

        // The LSP server sends exactly one publishDiagnostics notification
        // per didOpen. Block-read it (the server responds immediately), then
        // break. A deadline guards against a hung server.
        let mut diag_count = 0usize;
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if std::time::Instant::now() >= deadline {
                break;
            }
            let msg = read_msg(&mut reader);
            match msg {
                Some(m) if m
                    .get("method")
                    .and_then(|m| m.as_str())
                    == Some("textDocument/publishDiagnostics") =>
                {
                    if let Some(diags) = m
                        .pointer("/params/diagnostics")
                        .and_then(|d| d.as_array())
                    {
                        diag_count += diags.len();
                        for d in diags {
                            eprintln!("  [diag] {}", d);
                        }
                    }
                    break;
                }
                Some(_) => continue,
                None => break,
            }
        }

        eprintln!("  -> {} diagnostics", diag_count);
        total_diags += diag_count;

        // shutdown
        send_msg(
            stdin,
            json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "shutdown"
            }),
        );
        let _ = read_msg(&mut reader);
        send_msg(
            stdin,
            json!({
                "jsonrpc": "2.0",
                "method": "exit"
            }),
        );
        let _ = child.wait();
    }

    eprintln!(
        "=== Total diagnostics across all examples: {} ===",
        total_diags
    );
    if total_diags > 0 {
        std::process::exit(1);
    }
}

fn spawn_lsp(tangle_bin: &str) -> std::io::Result<Child> {
    Command::new(tangle_bin)
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn send_msg(stdin: &mut dyn Write, msg: Value) {
    let s = serde_json::to_string(&msg).unwrap();
    let _ = write!(stdin, "Content-Length: {}\r\n\r\n{}", s.len(), s);
    let _ = stdin.flush();
}

fn read_msg(reader: &mut dyn BufRead) -> Option<Value> {
    let mut content_length = None;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(v) = trimmed.strip_prefix("Content-Length: ") {
            content_length = v.parse::<usize>().ok();
        }
    }
    let len = content_length?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}