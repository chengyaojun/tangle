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

    // Stdlib prelude
    out.push_str(crate::stdlib::bindings::stdlib_prelude(
        crate::stdlib::bindings::TargetHost::Python
    ));
    out.push('\n');

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
