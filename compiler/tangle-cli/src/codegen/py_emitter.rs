use crate::ir::graph::*;
use std::collections::{HashSet, VecDeque};

fn sanitize_module_name(name: &str) -> String {
    name.replace(['.', '-'], "_")
}

/// Emit metadata comments (group, style) for a node. Returns comment lines with given indent.
fn emit_node_comments(node: &IRNode, indent: &str) -> String {
    let mut out = String::new();
    if let Some(ref group) = node.group {
        out.push_str(&format!("{}# group: {}\n", indent, group));
    }
    if let Some(ref style) = node.style {
        out.push_str(&format!("{}# style: {}\n", indent, style));
    }
    out
}

/// Emit metadata comments for an edge kind and style.
fn emit_edge_comments(edge: &IREdge, indent: &str) -> String {
    let mut out = String::new();
    match edge.kind {
        IREdgeKind::Dashed => out.push_str(&format!("{}# edge: dashed\n", indent)),
        IREdgeKind::Thick => out.push_str(&format!("{}# edge: thick\n", indent)),
        IREdgeKind::Crossed => out.push_str(&format!("{}# edge: crossed\n", indent)),
        IREdgeKind::Control | IREdgeKind::Condition | IREdgeKind::Error => {}
    }
    if let Some(ref style) = edge.style {
        out.push_str(&format!("{}# edge-style: {}\n", indent, style));
    }
    out
}

/// Sort edges by priority (lower = higher precedence). Edges without priority sort last.
fn sort_edges_by_priority<'a>(edges: &[&'a IREdge]) -> Vec<&'a IREdge> {
    let mut sorted = edges.to_vec();
    sorted.sort_by(|a, b| {
        match (a.priority, b.priority) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    sorted
}

/// Recursively emit the body of a branch target inside an if/elif block (Python).
fn emit_branch_body<'a>(
    target_id: &'a str,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
) -> String {
    let mut out = String::new();
    if visited.contains(target_id) {
        return out;
    }
    visited.insert(target_id);

    let node = match nodes.iter().find(|n| n.id == target_id) {
        Some(n) => n,
        None => return out,
    };

    out.push_str(&emit_node_comments(node, indent));

    match node.kind {
        IRNodeKind::Action | IRNodeKind::Compute | IRNodeKind::Decision => {
            let label = node.label.replace('\'', "\\'");
            out.push_str(&format!("{}# {}: {}\n", indent,
                match node.kind {
                    IRNodeKind::Action => "action",
                    IRNodeKind::Compute => "compute",
                    IRNodeKind::Decision => "decision",
                    _ => "step",
                },
                label
            ));
        }
        IRNodeKind::Terminal => {
            out.push_str(&format!("{}return Ok(None)\n", indent));
        }
        IRNodeKind::ErrorTerminal => {
            let label = node.label.replace('\'', "\\'");
            out.push_str(&format!("{}return Err('{}')\n", indent, label));
        }
    }

    for edge in edges {
        if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, visited, indent));
        }
    }
    out
}

/// Emit if/elif/else chain for a Decision node with guarded outgoing edges (Python).
fn emit_decision_branch<'a>(
    node: &IRNode,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
) -> String {
    let mut out = String::new();
    let out_edges: Vec<&IREdge> = edges.iter().filter(|e| e.from == node.id).collect();

    let mut guarded: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.guard.is_some() && e.kind != IREdgeKind::Crossed)
        .copied()
        .collect();
    let unguarded: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.guard.is_none() && e.kind != IREdgeKind::Crossed)
        .copied()
        .collect();
    let crossed: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.kind == IREdgeKind::Crossed)
        .copied()
        .collect();

    guarded = sort_edges_by_priority(&guarded);

    if guarded.is_empty() && unguarded.is_empty() {
        out.push_str(&format!("{}# decision: {} (no guarded branches)\n", indent, node.label));
        for e in &crossed {
            out.push_str(&format!("{}# skipped: crossed edge to {}\n", indent, e.to));
        }
        return out;
    }

    let inner_indent = format!("{}    ", indent);
    let mut all_branch_visited: HashSet<&str> = HashSet::new();
    let mut has_body = false;

    for (i, edge) in guarded.iter().enumerate() {
        out.push_str(&emit_edge_comments(edge, indent));
        let guard = edge.guard.as_ref().unwrap();
        if i == 0 {
            out.push_str(&format!("{}if ({}):\n", indent, guard));
        } else {
            out.push_str(&format!("{}elif ({}):\n", indent, guard));
        }
        let mut branch_visited = visited.clone();
        let body = emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent);
        if body.is_empty() {
            out.push_str(&format!("{}pass\n", inner_indent));
        } else {
            out.push_str(&body);
        }
        all_branch_visited.extend(branch_visited.iter().copied());
        has_body = true;
    }

    if !unguarded.is_empty() {
        out.push_str(&format!("{}else:\n", indent));
        for edge in &unguarded {
            out.push_str(&emit_edge_comments(edge, &inner_indent));
            let mut branch_visited = visited.clone();
            let body = emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent);
            if body.is_empty() {
                out.push_str(&format!("{}pass\n", inner_indent));
            } else {
                out.push_str(&body);
            }
            all_branch_visited.extend(branch_visited.iter().copied());
        }
        has_body = true;
    }

    if !has_body {
        out.push_str(&format!("{}pass\n", indent));
    }

    visited.extend(all_branch_visited.iter().copied());

    for edge in &crossed {
        out.push_str(&format!("{}# skipped: crossed edge to {}\n", indent, edge.to));
    }

    out
}

pub fn emit_python(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();
    let module_name = sanitize_module_name(module_name);

    // Python runtime prelude
    out.push_str("# Tangle-generated Python\n");
    out.push_str("from dataclasses import dataclass\n");
    out.push_str("from typing import Any, Optional\n\n");
    out.push_str("# Runtime helpers\n");
    out.push_str("class Result:\n");
    out.push_str("    def __init__(self, ok: bool, value: Any = None, error: str = ''):\n");
    out.push_str("        self.ok = ok\n");
    out.push_str("        self.value = value\n");
    out.push_str("        self.error = error\n\n");
    out.push_str("def Ok(value: Any = None) -> Result:\n");
    out.push_str("    return Result(True, value)\n\n");
    out.push_str("def Err(variant: str, value: Any = None) -> Result:\n");
    out.push_str("    return Result(False, value, variant)\n\n");
    out.push('\n');

    // Stdlib prelude — only emit imported modules
    if !graph.imported_stdlib.is_empty() {
        out.push_str("# --- Tangle Standard Library (Python) ---\n");
        for module in &graph.imported_stdlib {
            if let Some(prelude) = crate::stdlib::bindings::stdlib_module_prelude(
                crate::stdlib::bindings::TargetHost::Python, module
            ) {
                out.push_str(prelude);
            }
        }
        out.push('\n');
    }

    if !graph.functions.is_empty() {
        out.push_str(&emit_multi_function_py(&graph.functions));
    } else {
        out.push_str(&emit_single_function_py(
            &graph.nodes,
            &graph.edges,
            &graph.entry_node_id,
            &module_name,
        ));
    }

    out
}

/// Single-function fallback: one `def {module_name}()` wrapper + entry invocation.
fn emit_single_function_py(
    nodes: &[IRNode],
    edges: &[IREdge],
    entry_node_id: &str,
    module_name: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("def {}() -> Result:\n", module_name));
    out.push_str(&emit_python_function_body(nodes, edges, entry_node_id));
    out.push('\n');
    out.push_str("# Entry point\n");
    out.push_str("if __name__ == '__main__':\n");
    out.push_str(&format!("    result = {}()\n", module_name));
    out.push_str("    if not result.ok:\n");
    out.push_str("        print(f'Error: {result.error}', file=__import__('sys').stderr)\n");
    out.push_str("        exit(1)\n");
    out.push_str("    print(result.value)\n");
    out
}

/// Multi-function emission: one Python function per heading-defined callable.
/// The entry point calls `main()` only when a function named "main" exists,
/// capturing the Result, checking for errors, and printing the value —
/// symmetric with single-function mode and the JS multi-function entry.
fn emit_multi_function_py(functions: &[IRFunction]) -> String {
    let mut out = String::new();
    for func in functions {
        let name = &func.name;
        // params are currently empty (IRFunction.params will be expanded in a future phase)
        out.push_str(&format!("def {}() -> Result:\n", name));
        out.push_str(&emit_python_function_body(&func.nodes, &func.edges, &func.entry_node_id));
        out.push('\n');
    }
    // Entry: only when main is present
    let has_main = functions.iter().any(|f| f.name == "main");
    if has_main {
        out.push_str("# Entry point\n");
        out.push_str("if __name__ == '__main__':\n");
        out.push_str("    result = main()\n");
        out.push_str("    if not result.ok:\n");
        out.push_str("        print(f'Error: {result.error}', file=__import__('sys').stderr)\n");
        out.push_str("        exit(1)\n");
        out.push_str("    print(result.value)\n");
    }
    out
}

/// Emit the body of one function (BFS traversal + Result protocol), 4-space indented.
fn emit_python_function_body(nodes: &[IRNode], edges: &[IREdge], entry_node_id: &str) -> String {
    let mut out = String::new();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&IRNode> = VecDeque::new();

    if let Some(entry) = nodes.iter().find(|n| n.id == entry_node_id) {
        queue.push_back(entry);
    }

    while let Some(node) = queue.pop_front() {
        if visited.contains(node.id.as_str()) {
            continue;
        }
        visited.insert(&node.id);

        out.push_str(&emit_node_comments(node, "    "));

        match node.kind {
            IRNodeKind::Action | IRNodeKind::Compute => {
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!(
                    "    # {}: {}\n",
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        _ => "step",
                    },
                    label
                ));
            }
            IRNodeKind::Decision => {
                let has_guarded = edges.iter().any(|e| {
                    e.from == node.id && e.guard.is_some() && e.kind != IREdgeKind::Crossed
                });
                if has_guarded {
                    out.push_str(&emit_decision_branch(node, nodes, edges, &mut visited, "    "));
                } else {
                    let label = node.label.replace('\'', "\\'");
                    out.push_str(&format!("    # decision: {}\n", label));
                }
            }
            IRNodeKind::Terminal => {
                out.push_str("    return Ok(None)\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!("    return Err('{}')\n", label));
            }
        }

        for edge in edges {
            if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
                if node.kind == IRNodeKind::Decision && edge.guard.is_some() {
                    continue;
                }
                out.push_str(&emit_edge_comments(edge, "    "));
                if let Some(target) = nodes.iter().find(|n| n.id == edge.to) {
                    queue.push_back(target);
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, kind: IRNodeKind, label: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind,
            label: label.to_string(),
            source_span: None, source_text: None,
            group: None, style: None,
        }
    }

    fn make_edge(from: &str, to: &str) -> IREdge {
        IREdge {
            from: from.to_string(),
            to: to.to_string(),
            kind: IREdgeKind::Control,
            guard: None,
            source_span: None,
            priority: None, style: None,
        }
    }

    #[test]
    fn emit_minimal_graph_produces_valid_python() {
        let graph = RuleGraph {
            nodes: vec![
                make_node("n0", IRNodeKind::Action, "entry"),
                make_node("n1", IRNodeKind::Terminal, "done"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };

        let output = emit_python(&graph, "test_module");

        assert!(!output.is_empty());
        assert!(output.contains("def test_module()"));
        assert!(output.contains("Tangle-generated Python"));
        assert!(output.contains("class Result"));
        assert!(output.contains("return Ok(None)"));
        assert!(output.contains("if __name__ == '__main__':"));
    }

    #[test]
    fn emit_graph_with_action_and_error_terminal() {
        let graph = RuleGraph {
            nodes: vec![
                make_node("n0", IRNodeKind::Action, "start"),
                make_node("n1", IRNodeKind::Action, "do_work"),
                make_node("n2", IRNodeKind::ErrorTerminal, "fail"),
            ],
            edges: vec![
                make_edge("n0", "n1"),
                make_edge("n1", "n2"),
            ],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };

        let output = emit_python(&graph, "workflow");

        assert!(output.contains("# action: do_work"), "expected action comment, got:\n{}", output);
        assert!(output.contains("return Err('fail')"), "expected error return, got:\n{}", output);
        assert!(output.contains("def workflow()"));
        // module names with special chars are sanitized
        let graph2 = RuleGraph {
            nodes: vec![
                make_node("n0", IRNodeKind::Action, "x"),
                make_node("n1", IRNodeKind::Terminal, "y"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };
        let output2 = emit_python(&graph2, "my.module_name");
        assert!(output2.contains("def my_module_name()"), "special chars should be sanitized");
    }
}
