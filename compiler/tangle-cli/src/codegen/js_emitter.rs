use crate::ir::graph::*;
use std::collections::{HashSet, VecDeque};

pub fn emit_js(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();

    // Runtime prelude
    out.push_str("// Tangle-generated JavaScript\n");
    out.push_str(crate::codegen::js_prelude::RUNTIME_PRELUDE);
    out.push('\n');

    // Stdlib prelude — only emit modules that are actually imported
    if !graph.imported_stdlib.is_empty() {
        out.push_str("// --- Tangle Standard Library (JS) ---\n");
        for module in &graph.imported_stdlib {
            if let Some(prelude) = crate::stdlib::bindings::stdlib_module_prelude(
                crate::stdlib::bindings::TargetHost::JavaScript, module
            ) {
                out.push_str(prelude);
            }
        }
        out.push('\n');
    }

    // Module function
    out.push_str(&format!("function {}() {{\n", module_name));

    // BFS traversal from entry node
    let mut visited: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&IRNode> = VecDeque::new();

    // Find entry node
    if let Some(entry) = graph.nodes.iter().find(|n| n.id == graph.entry_node_id) {
        queue.push_back(entry);
    }

    while let Some(node) = queue.pop_front() {
        if visited.contains(node.id.as_str()) {
            continue;
        }
        visited.insert(&node.id);

        match node.kind {
            IRNodeKind::Action | IRNodeKind::Compute | IRNodeKind::Decision => {
                out.push_str(&format!(
                    "  // {}: {}\n",
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        IRNodeKind::Decision => "decision",
                        _ => "unknown",
                    },
                    node.label
                ));
            }
            IRNodeKind::Terminal => {
                out.push_str("  return Ok(undefined);\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!("  return Err('{}');\n", label));
            }
        }

        // Follow edges
        for edge in &graph.edges {
            if edge.from == node.id {
                if let Some(target) = graph.nodes.iter().find(|n| n.id == edge.to) {
                    queue.push_back(target);
                }
            }
        }
    }

    out.push_str("}\n\n");

    // Entry point invocation
    out.push_str(&format!(
        "// Entry point\nconst __result = {module_name}();\nif (!__result.ok) {{ console.error('Error:', __result.error); process.exit(1); }}\nconsole.log(__result.value);\n",
        module_name = module_name
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry_node(id: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind: IRNodeKind::Action,
            label: "entry".to_string(),
            source_span: None,
        }
    }

    fn make_terminal_node(id: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind: IRNodeKind::Terminal,
            label: "done".to_string(),
            source_span: None,
        }
    }

    fn make_action_node(id: &str, label: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind: IRNodeKind::Action,
            label: label.to_string(),
            source_span: None,
        }
    }

    fn make_edge(from: &str, to: &str) -> IREdge {
        IREdge {
            from: from.to_string(),
            to: to.to_string(),
            kind: IREdgeKind::Control,
            guard: None,
            source_span: None,
        }
    }

    #[test]
    fn emit_minimal_graph_contains_function_and_prelude() {
        let graph = RuleGraph {
            nodes: vec![
                make_entry_node("n0"),
                make_terminal_node("n1"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![],
        };

        let output = emit_js(&graph, "test_module");

        assert!(output.contains("function test_module()"));
        assert!(output.contains("Tangle Runtime Prelude"));
        assert!(!output.contains("Tangle Standard Library"), "no stdlib prelude when no modules imported");
        assert!(output.contains("return Ok(undefined)"));
    }

    #[test]
    fn emit_graph_with_action_node_shows_label() {
        let graph = RuleGraph {
            nodes: vec![
                make_entry_node("n0"),
                make_action_node("n1", "do_work"),
                make_terminal_node("n2"),
            ],
            edges: vec![
                make_edge("n0", "n1"),
                make_edge("n1", "n2"),
            ],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![],
        };

        let output = emit_js(&graph, "workflow");

        assert!(output.contains("// action: do_work"), "expected action label 'do_work' in output:\n{}", output);
        assert!(output.contains("// action: entry"), "expected entry label in output:\n{}", output);
    }

    #[test]
    fn emit_empty_graph_produces_valid_js() {
        let graph = RuleGraph {
            nodes: vec![],
            edges: vec![],
            error_edges: vec![],
            entry_node_id: "missing".to_string(),
            imported_stdlib: vec![],
        };

        let output = emit_js(&graph, "empty_mod");

        assert!(!output.is_empty(), "output should not be empty");
        assert!(output.contains("Tangle Runtime Prelude"), "output should contain prelude");
        assert!(output.contains("function empty_mod()"), "output should contain function definition");
    }
}
