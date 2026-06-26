use crate::ir::graph::*;
use std::collections::{HashSet, VecDeque};

fn sanitize_module_name(name: &str) -> String {
    name.replace(['.', '-'], "_")
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

    // Module function
    out.push_str(&format!("def {}() -> Result:\n", module_name));

    // BFS traversal
    let mut visited: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&IRNode> = VecDeque::new();

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
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!(
                    "    # {}: {}\n",
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
                out.push_str("    return Ok(None)\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!("    return Err('{}')\n", label));
            }
        }

        for edge in &graph.edges {
            if edge.from == node.id {
                if let Some(target) = graph.nodes.iter().find(|n| n.id == edge.to) {
                    queue.push_back(target);
                }
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, kind: IRNodeKind, label: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind,
            label: label.to_string(),
            source_span: None, source_text: None,
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
    fn emit_minimal_graph_produces_valid_python() {
        let graph = RuleGraph {
            nodes: vec![
                make_node("n0", IRNodeKind::Action, "entry"),
                make_node("n1", IRNodeKind::Terminal, "done"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![],
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
            imported_stdlib: vec![], stdlib_imports: vec![],
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
            imported_stdlib: vec![], stdlib_imports: vec![],
        };
        let output2 = emit_python(&graph2, "my.module_name");
        assert!(output2.contains("def my_module_name()"), "special chars should be sanitized");
    }
}
