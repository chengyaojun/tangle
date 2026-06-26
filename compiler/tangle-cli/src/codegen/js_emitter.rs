use crate::ir::graph::*;
use std::collections::{HashSet, VecDeque};

pub fn emit_js(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();

    // Runtime prelude
    out.push_str("// Tangle-generated JavaScript\n");
    out.push_str(crate::codegen::js_prelude::RUNTIME_PRELUDE);
    out.push('\n');

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
