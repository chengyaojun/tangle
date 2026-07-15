//! ir-diff: semantic comparison of two Tangle IR JSON files.
//!
//! Strips source spans (file/line/column/sourceText) and compares Rule Graph
//! structure. JSON key order is normalized (sorted) so ordering differences
//! don't cause false positives.
//!
//! Exit code 0 = MATCH, 1 = DIFF (prints first difference to stderr), 2 = usage
//! error.

use serde_json::{json, Map, Value};
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

    let (ts_normalized, rs_normalized) = compare_functions(ts, rs);

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

/// Phase 1 of normalization pipeline: lift functions[0] to top-level.
///
/// Rust IR wraps nodes/edges/entryNodeId inside `functions[0]`, while TS IR
/// places them at top level. This function promotes functions[0] to top level
/// and strips the `functions`, empty `importedStdlib`, and empty `stdlibImports`
/// keys. If `functions` is absent or empty, the input is returned unchanged
/// (minus empty stdlib arrays).
///
/// Superseded by `compare_functions` (which handles multi-function arrays) in
/// the production `main` path. Retained for the single-function unit tests
/// that still exercise this behavior directly.
#[allow(dead_code)]
#[allow(clippy::collapsible_match)]
fn lift_functions(v: Value) -> Value {
    let mut map = match v {
        Value::Object(m) => m,
        other => return other,
    };

    // Lift functions[0] if present
    if let Some(functions_val) = map.remove("functions") {
        if let Value::Array(arr) = functions_val {
            if let Some(first) = arr.into_iter().next() {
                if let Value::Object(func_map) = first {
                    // Only lift IR schema fields; strip function metadata (name/params/receiver)
                    const LIFT_FIELDS: &[&str] = &["nodes", "edges", "errorEdges", "entryNodeId"];
                    for (k, v) in func_map {
                        if LIFT_FIELDS.contains(&k.as_str()) {
                            // Don't overwrite existing top-level keys
                            map.entry(k).or_insert(v);
                        }
                    }
                }
            }
        }
    }

    // Strip empty stdlib arrays
    if let Some(Value::Array(arr)) = map.get("importedStdlib") {
        if arr.is_empty() {
            map.remove("importedStdlib");
        }
    }
    if let Some(Value::Array(arr)) = map.get("stdlibImports") {
        if arr.is_empty() {
            map.remove("stdlibImports");
        }
    }

    Value::Object(map)
}

/// Phase 2 of normalization pipeline: build a mapping from original node IDs
/// to positional IDs ("node0", "node1", ...).
///
/// TS IR uses semantic IDs (entry1, bind2, ret4), Rust IR uses positional IDs
/// (n0, n1, n2). Both share the same node array order, so positional remapping
/// produces identical IDs on both sides.
fn build_id_map(v: &Value) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
        for (i, node) in nodes.iter().enumerate() {
            if let Some(id) = node.get("id").and_then(|id| id.as_str()) {
                map.insert(id.to_string(), format!("node{}", i));
            }
        }
    }
    map
}

/// Recursively normalize an IR value for stable comparison:
/// - strip source span fields (see `SPAN_FIELDS`)
/// - strip null `guard` fields (F-008)
/// - normalize label "return" → "exit" (F-011)
/// - remap node IDs to positional ids via `id_map` (F-007)
/// - sort object keys for stable comparison
///
/// Arrays are compared element-wise in their original order (node IDs are
/// positional within the IR graph and meaningful).
fn normalize(v: Value, id_map: &std::collections::HashMap<String, String>) -> Value {
    match v {
        Value::Object(map) => {
            let mut filtered: Vec<(String, Value)> = Vec::with_capacity(map.len());
            for (k, v) in map {
                if SPAN_FIELDS.contains(&k.as_str()) {
                    continue;
                }
                // F-008: strip null guard
                if k == "guard" && v == Value::Null {
                    continue;
                }
                // F-011: normalize label "return" → "exit"
                if k == "label" {
                    if let Some(s) = v.as_str() {
                        if s == "return" {
                            filtered.push((k, Value::String("exit".into())));
                            continue;
                        }
                    }
                }
                // F-007: remap node IDs
                if k == "id" || k == "from" || k == "to" || k == "entryNodeId" {
                    if let Some(s) = v.as_str() {
                        if let Some(remapped) = id_map.get(s) {
                            filtered.push((k, Value::String(remapped.clone())));
                            continue;
                        }
                    }
                }
                filtered.push((k, normalize(v, id_map)));
            }
            // Sort keys for stable comparison (ignores JSON key order)
            filtered.sort_by(|a, b| a.0.cmp(&b.0));
            let collected: Map<String, Value> = filtered.into_iter().collect();
            Value::Object(collected)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(|v| normalize(v, id_map)).collect()),
        other => other,
    }
}

/// Compare two IRs' functions[] arrays (or wrap top-level as single function).
/// Returns (ts_normalized, rs_normalized) as JSON arrays sorted by function.name.
///
/// Replaces the single-function `lift_functions` path: Rust IR may carry
/// multiple functions (e.g. payment has `main` + `process`), and TS IR has
/// no `functions[]` concept (top-level nodes/edges). Both sides are normalized
/// to a `Vec<Function>` sorted by name so `main` aligns with `main` regardless
/// of input order.
fn compare_functions(ts: Value, rs: Value) -> (Value, Value) {
    let ts_arr = extract_functions_array(&ts);
    let rs_arr = extract_functions_array(&rs);

    // Sort by name so functions align across the two IRs
    let mut ts_sorted = ts_arr;
    let mut rs_sorted = rs_arr;
    ts_sorted.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    rs_sorted.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

    // Normalize each function independently (id map is per-function scope)
    let ts_norm: Vec<Value> = ts_sorted.iter().map(normalize_function).collect();
    let rs_norm: Vec<Value> = rs_sorted.iter().map(normalize_function).collect();

    (Value::Array(ts_norm), Value::Array(rs_norm))
}

/// Extract the `functions[]` array, or wrap top-level nodes/edges as a single
/// function named "module" (for TS-style flat IR without `functions[]`).
fn extract_functions_array(ir: &Value) -> Vec<Value> {
    if let Some(funcs) = ir.get("functions").and_then(|f| f.as_array()) {
        if !funcs.is_empty() {
            return funcs.clone();
        }
    }
    // Wrap top-level IR as a single function. Includes `errorEdges` so the
    // wrapped shape matches an extracted Rust function (which carries it).
    vec![json!({
        "name": "module",
        "nodes": ir.get("nodes").cloned().unwrap_or(Value::Array(vec![])),
        "edges": ir.get("edges").cloned().unwrap_or(Value::Array(vec![])),
        "errorEdges": ir.get("errorEdges").cloned().unwrap_or(Value::Array(vec![])),
        "entryNodeId": ir.get("entryNodeId").cloned().unwrap_or(Value::Null),
    })]
}

/// Normalize a single function's IR: build a per-function id map and apply
/// the standard `normalize` transform (strip spans, remap ids, sort keys).
///
/// Strips known function metadata fields (`name`/`params`/`receiver`) before
/// normalize, ensuring extracted Rust functions and wrapped TS modules compare
/// equal regardless of function name (e.g. an extracted Rust `main` matches a
/// TS-wrapped `module` when their graph structure is identical). Only
/// structural IR fields (`nodes`/`edges`/`errorEdges`/`entryNodeId`)
/// participate in comparison.
fn normalize_function(func: &Value) -> Value {
    let id_map = build_id_map(func);
    let mut stripped = func.clone();
    if let Value::Object(map) = &mut stripped {
        map.remove("name");
        map.remove("params");
        map.remove("receiver");
    }
    normalize(stripped, &id_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lift_functions_promotes_first_function_to_top_level() {
        let input = json!({
            "functions": [{
                "nodes": [{"id": "n0", "kind": "action", "label": "do"}],
                "edges": [{"from": "n0", "to": "n1", "kind": "control"}],
                "entryNodeId": "n0",
                "name": "main",
                "params": [],
                "receiver": null
            }],
            "importedStdlib": [],
            "stdlibImports": []
        });
        let result = lift_functions(input);
        assert!(result.get("functions").is_none(), "functions should be removed");
        assert!(result.get("importedStdlib").is_none(), "empty importedStdlib should be removed");
        assert!(result.get("stdlibImports").is_none(), "empty stdlibImports should be removed");
        assert!(result.get("name").is_none(), "function metadata 'name' should NOT be lifted");
        assert!(result.get("params").is_none(), "function metadata 'params' should NOT be lifted");
        assert!(result.get("receiver").is_none(), "function metadata 'receiver' should NOT be lifted");
        assert_eq!(result["entryNodeId"], "n0");
        assert_eq!(result["nodes"][0]["id"], "n0");
        assert_eq!(result["edges"][0]["from"], "n0");
    }

    #[test]
    fn lift_functions_preserves_flat_ir_without_functions_key() {
        let input = json!({
            "nodes": [{"id": "entry", "kind": "action", "label": "do"}],
            "edges": [],
            "entryNodeId": "entry"
        });
        let result = lift_functions(input);
        assert_eq!(result["nodes"][0]["id"], "entry");
        assert_eq!(result["entryNodeId"], "entry");
    }

    #[test]
    fn id_remap_normalizes_n0_to_node0() {
        let input = json!({
            "nodes": [{"id": "n0"}, {"id": "n1"}],
            "edges": [{"from": "n0", "to": "n1"}],
            "entryNodeId": "n0"
        });
        let id_map = build_id_map(&input);
        assert_eq!(id_map.get("n0"), Some(&"node0".to_string()));
        assert_eq!(id_map.get("n1"), Some(&"node1".to_string()));
    }

    #[test]
    fn id_remap_normalizes_entry1_to_node0() {
        let input = json!({
            "nodes": [{"id": "entry1"}, {"id": "bind2"}],
            "entryNodeId": "entry1"
        });
        let id_map = build_id_map(&input);
        assert_eq!(id_map.get("entry1"), Some(&"node0".to_string()));
        assert_eq!(id_map.get("bind2"), Some(&"node1".to_string()));
    }

    #[test]
    fn id_remap_applies_to_edges_and_entry() {
        let input = json!({
            "nodes": [{"id": "n0"}, {"id": "n1"}],
            "edges": [{"from": "n0", "to": "n1", "kind": "control"}],
            "entryNodeId": "n0"
        });
        let id_map = build_id_map(&input);
        let normalized = normalize(input, &id_map);
        assert_eq!(normalized["nodes"][0]["id"], "node0");
        assert_eq!(normalized["nodes"][1]["id"], "node1");
        assert_eq!(normalized["edges"][0]["from"], "node0");
        assert_eq!(normalized["edges"][0]["to"], "node1");
        assert_eq!(normalized["entryNodeId"], "node0");
    }

    #[test]
    fn end_to_end_expression_style_fixture_matches() {
        // Simulate TS IR (semantic IDs, flat structure, return label)
        let ts = json!({
            "nodes": [
                {"id": "entry1", "kind": "action", "label": "main"},
                {"id": "ret4", "kind": "terminal", "label": "return"}
            ],
            "edges": [
                {"from": "entry1", "to": "ret4", "kind": "control", "guard": null}
            ],
            "entryNodeId": "entry1"
        });
        // Simulate Rust IR (positional IDs, functions wrapper, exit label)
        let rs = json!({
            "functions": [{
                "nodes": [
                    {"id": "n0", "kind": "action", "label": "main"},
                    {"id": "n1", "kind": "terminal", "label": "exit"}
                ],
                "edges": [
                    {"from": "n0", "to": "n1", "kind": "control", "guard": null}
                ],
                "entryNodeId": "n0"
            }],
            "importedStdlib": [],
            "stdlibImports": []
        });

        let ts_lifted = lift_functions(ts);
        let rs_lifted = lift_functions(rs);
        let ts_id_map = build_id_map(&ts_lifted);
        let rs_id_map = build_id_map(&rs_lifted);
        let ts_norm = normalize(ts_lifted, &ts_id_map);
        let rs_norm = normalize(rs_lifted, &rs_id_map);
        assert_eq!(ts_norm, rs_norm, "TS and Rust IR should match after normalization");
    }

    #[test]
    fn null_guard_stripped() {
        let input = json!({
            "edges": [{"from": "n0", "to": "n1", "kind": "control", "guard": null}]
        });
        let id_map = std::collections::HashMap::new();
        let result = normalize(input, &id_map);
        assert!(result["edges"][0].get("guard").is_none(), "null guard should be stripped");
    }

    #[test]
    fn non_null_guard_preserved() {
        let input = json!({
            "edges": [{"from": "n0", "to": "n1", "kind": "condition", "guard": "x > 0"}]
        });
        let id_map = std::collections::HashMap::new();
        let result = normalize(input, &id_map);
        assert_eq!(result["edges"][0]["guard"], "x > 0");
    }

    #[test]
    fn return_label_normalized_to_exit() {
        let input = json!({
            "nodes": [{"id": "n0", "kind": "terminal", "label": "return"}]
        });
        let id_map = std::collections::HashMap::new();
        let result = normalize(input, &id_map);
        assert_eq!(result["nodes"][0]["label"], "exit");
    }

    #[test]
    fn compare_functions_aligns_by_name() {
        // 模拟 Rust IR: functions[main, process]
        let rs = json!({
            "functions": [
                {"name": "main", "nodes": [{"id": "n0", "kind": "compute", "label": "a"}], "edges": [], "entryNodeId": "n0"},
                {"name": "process", "nodes": [{"id": "n1", "kind": "compute", "label": "b"}], "edges": [], "entryNodeId": "n1"}
            ]
        });
        // 模拟 TS IR: functions[process, main]（顺序不同）
        let ts = json!({
            "functions": [
                {"name": "process", "nodes": [{"id": "entry1", "kind": "compute", "label": "b"}], "edges": [], "entryNodeId": "entry1"},
                {"name": "main", "nodes": [{"id": "entry2", "kind": "compute", "label": "a"}], "edges": [], "entryNodeId": "entry2"}
            ]
        });

        let (ts_norm, rs_norm) = compare_functions(ts, rs);
        let ts_arr = ts_norm.as_array().unwrap();
        let rs_arr = rs_norm.as_array().unwrap();
        assert_eq!(ts_arr.len(), 2);
        assert_eq!(rs_arr.len(), 2);
        // 按 name 排序后对齐：main(a) 与 main(a) 对齐，process(b) 与 process(b) 对齐
        // (name 字段在归一化时被剥离，因此通过 node label 验证对齐)
        assert_eq!(ts_arr[0]["nodes"][0]["label"], rs_arr[0]["nodes"][0]["label"]);
        assert_eq!(ts_arr[1]["nodes"][0]["label"], rs_arr[1]["nodes"][0]["label"]);
        // 排序后第一项是 label "a" (name "main")，第二项是 label "b" (name "process")
        assert_eq!(ts_arr[0]["nodes"][0]["label"], "a");
        assert_eq!(ts_arr[1]["nodes"][0]["label"], "b");
    }

    #[test]
    fn compare_functions_wraps_single_when_no_functions_array() {
        // 无 functions[] 的 IR（如 expression）包装为单 function 数组
        let rs = json!({
            "nodes": [{"id": "n0", "kind": "compute", "label": "a"}],
            "edges": [],
            "entryNodeId": "n0"
        });
        let ts = json!({
            "nodes": [{"id": "entry1", "kind": "compute", "label": "a"}],
            "edges": [],
            "entryNodeId": "entry1"
        });

        let (ts_norm, rs_norm) = compare_functions(ts, rs);
        let ts_arr = ts_norm.as_array().unwrap();
        let rs_arr = rs_norm.as_array().unwrap();
        assert_eq!(ts_arr.len(), 1);
        assert_eq!(rs_arr.len(), 1);
        // 包装后应保留原 nodes 的内容（name 等元数据被剥离）
        assert_eq!(ts_arr[0]["nodes"][0]["label"], "a");
        assert_eq!(rs_arr[0]["nodes"][0]["label"], "a");
        assert_eq!(ts_arr[0], rs_arr[0], "wrapped ts and rs should be equal after normalization");
    }

    #[test]
    fn compare_functions_matches_cross_side_extracted_vs_wrapped() {
        // Rust: functions[main] 携带元数据 (name/params/receiver/errorEdges)
        let rs = json!({
            "functions": [
                {"name": "main", "nodes": [{"id": "n0", "kind": "compute", "label": "a"}], "edges": [], "errorEdges": [], "entryNodeId": "n0", "params": [], "receiver": null}
            ]
        });
        // TS: 无 functions[]，顶层 nodes/edges (包装为 module)
        let ts = json!({
            "nodes": [{"id": "entry1", "kind": "compute", "label": "a"}],
            "edges": [],
            "entryNodeId": "entry1"
        });

        let (ts_norm, rs_norm) = compare_functions(ts, rs);
        // 元数据被剥离后，两边应 MATCH（expression/hello 场景）
        assert_eq!(ts_norm, rs_norm, "extracted function should match wrapped module after metadata strip");
    }
}
