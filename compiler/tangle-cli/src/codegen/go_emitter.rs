use crate::ir::graph::*;
use std::collections::{HashSet, VecDeque};

pub fn emit_go(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();

    // Go runtime prelude
    out.push_str("// Tangle-generated Go\n");
    out.push_str("package main\n\n");
    out.push_str("import (\n");
    out.push_str("    \"fmt\"\n");
    out.push_str("    \"os\"\n");
    out.push_str(")\n\n");
    out.push_str("type Result struct {\n");
    out.push_str("    Ok    bool\n");
    out.push_str("    Value interface{}\n");
    out.push_str("    Error string\n");
    out.push_str("}\n\n");
    out.push_str("func Ok(value interface{}) Result {\n");
    out.push_str("    return Result{Ok: true, Value: value}\n");
    out.push_str("}\n\n");
    out.push_str("func Err(variant string) Result {\n");
    out.push_str("    return Result{Ok: false, Error: variant}\n");
    out.push_str("}\n\n");

    // Stdlib prelude
    out.push_str(crate::stdlib::bindings::stdlib_prelude(
        crate::stdlib::bindings::TargetHost::Go
    ));
    out.push('\n');

    // Module function
    out.push_str(&format!("func {}() Result {{\n", to_camel(module_name)));

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
                out.push_str(&format!(
                    "    // {}: {}\n",
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        IRNodeKind::Decision => "decision",
                        _ => "step",
                    },
                    node.label
                ));
            }
            IRNodeKind::Terminal => {
                out.push_str("    return Ok(nil)\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('"', "\\\"");
                out.push_str(&format!("    return Err(\"{}\")\n", label));
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

    out.push_str("}\n\n");
    out.push_str("func main() {\n");
    out.push_str(&format!("    result := {}()\n", to_camel(module_name)));
    out.push_str("    if !result.Ok {\n");
    out.push_str("        fmt.Fprintf(os.Stderr, \"Error: %s\\n\", result.Error)\n");
    out.push_str("        os.Exit(1)\n");
    out.push_str("    }\n");
    out.push_str("    fmt.Println(result.Value)\n");
    out.push_str("}\n");

    out
}

fn to_camel(s: &str) -> String {
    s.split(&['.', '-', '_'][..])
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, kind: IRNodeKind, label: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind,
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
    fn emit_minimal_graph_produces_valid_go() {
        let graph = RuleGraph {
            nodes: vec![
                make_node("n0", IRNodeKind::Action, "entry"),
                make_node("n1", IRNodeKind::Terminal, "done"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
        };

        let output = emit_go(&graph, "test_module");

        assert!(!output.is_empty());
        assert!(output.contains("package main"));
        assert!(output.contains("func TestModule()"));
        assert!(output.contains("Tangle-generated Go"));
        assert!(output.contains("type Result struct"));
        assert!(output.contains("return Ok(nil)"));
        assert!(output.contains("func main()"));
        assert!(output.contains("result := TestModule()"));
    }

    #[test]
    fn emit_graph_with_action_node_and_sanitized_module_name() {
        let graph = RuleGraph {
            nodes: vec![
                make_node("n0", IRNodeKind::Action, "start"),
                make_node("n1", IRNodeKind::Action, "do_work"),
                make_node("n2", IRNodeKind::Terminal, "done"),
            ],
            edges: vec![
                make_edge("n0", "n1"),
                make_edge("n1", "n2"),
            ],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
        };

        let output = emit_go(&graph, "my.module-name");

        assert!(output.contains("// action: do_work"), "expected action comment, got:\n{}", output);
        // CamelCase sanitization
        assert!(output.contains("func MyModuleName()"), "expected CamelCase name, got:\n{}", output);
        assert!(output.contains("Tangle Standard Library"));
    }
}
