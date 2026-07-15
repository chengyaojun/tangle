//! ir-diff: semantic comparison of two Tangle IR JSON files.
//!
//! Strips source spans (file/line/column/sourceText) and compares Rule Graph
//! structure. JSON key order is normalized (sorted) so ordering differences
//! don't cause false positives.
//!
//! Exit code 0 = MATCH, 1 = DIFF (prints first difference to stderr), 2 = usage
//! error.

use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::process::exit;

/// Span-related field names stripped during normalization.
///
/// Both Tangle IR JSON outputs (Rust `tangle-cli` and the TypeScript reference)
/// serialize `SourceSpan` with `serde(rename_all = "camelCase")`, so the actual
/// keys are `sourceSpan`, `startLine`, `startColumn`, etc. The snake_case
/// variants are included defensively in case either side changes its
/// serialization convention.
const SPAN_FIELDS: &[&str] = &[
    // Container objects holding a full span
    "sourceSpan",
    "span",
    // Rust IR carries the original source text on IRNode — not structural
    "sourceText",
    "source_text",
    // Legacy / generic "source" blob
    "source",
    // Inner span fields (camelCase — what the IR actually emits today)
    "file",
    "startLine",
    "startColumn",
    "endLine",
    "endColumn",
    // Inner span fields (snake_case — defensive)
    "start_line",
    "start_column",
    "end_line",
    "end_column",
];

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: ir-diff <ts-ir.json> <rs-ir.json>");
        exit(2);
    }
    let ts_json = fs::read_to_string(&args[1]).expect("read ts-ir");
    let rs_json = fs::read_to_string(&args[2]).expect("read rs-ir");

    let ts: Value = serde_json::from_str(&ts_json).expect("parse ts-ir JSON");
    let rs: Value = serde_json::from_str(&rs_json).expect("parse rs-ir JSON");

    let ts_normalized = normalize(ts);
    let rs_normalized = normalize(rs);

    if ts_normalized == rs_normalized {
        println!("MATCH");
        exit(0);
    } else {
        eprintln!("DIFF");
        let ts_str = serde_json::to_string_pretty(&ts_normalized).unwrap();
        let rs_str = serde_json::to_string_pretty(&rs_normalized).unwrap();
        eprintln!("--- ts-ir normalized ---\n{}", ts_str);
        eprintln!("--- rs-ir normalized ---\n{}", rs_str);
        exit(1);
    }
}

/// Recursively strip source span fields and sort object keys for stable
/// comparison. Arrays are compared element-wise in their original order (node
/// IDs are positional within the IR graph and meaningful).
fn normalize(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut filtered: Vec<(String, Value)> = Vec::with_capacity(map.len());
            for (k, v) in map {
                if SPAN_FIELDS.contains(&k.as_str()) {
                    continue;
                }
                filtered.push((k, normalize(v)));
            }
            // Sort keys for stable comparison (ignores JSON key order)
            filtered.sort_by(|a, b| a.0.cmp(&b.0));
            let collected: Map<String, Value> = filtered.into_iter().collect();
            Value::Object(collected)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize).collect()),
        other => other,
    }
}
