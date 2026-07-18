use crate::codegen::type_map::tangle_type_to_go;
use crate::ir::graph::*;
use std::collections::{HashSet, VecDeque};

/// Maximum recursion depth for `emit_branch_body`. Beyond this the emitter
/// stops recursing and emits a `// max depth reached` comment to prevent
/// stack overflow on pathological graphs.
const MAX_BRANCH_DEPTH: usize = 100;

/// Emit metadata comments (group, style) for a node. Returns comment lines with given indent.
fn emit_node_comments(node: &IRNode, indent: &str) -> String {
    let mut out = String::new();
    if let Some(ref group) = node.group {
        out.push_str(&format!("{}// group: {}\n", indent, group));
    }
    if let Some(ref style) = node.style {
        out.push_str(&format!("{}// style: {}\n", indent, style));
    }
    out
}

/// Emit metadata comments for an edge kind and style.
fn emit_edge_comments(edge: &IREdge, indent: &str) -> String {
    let mut out = String::new();
    match edge.kind {
        IREdgeKind::Dashed => out.push_str(&format!("{}// edge: dashed\n", indent)),
        IREdgeKind::Thick => out.push_str(&format!("{}// edge: thick\n", indent)),
        IREdgeKind::Crossed => out.push_str(&format!("{}// edge: crossed\n", indent)),
        IREdgeKind::Control | IREdgeKind::Condition | IREdgeKind::Error => {}
    }
    if let Some(ref style) = edge.style {
        out.push_str(&format!("{}// edge-style: {}\n", indent, style));
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

/// Recursively emit the body of a branch target inside an if/else block (Go).
fn emit_branch_body<'a>(
    target_id: &'a str,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
    depth: usize,
) -> String {
    if depth >= MAX_BRANCH_DEPTH {
        return format!("{}// max depth reached\n", indent);
    }
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
            out.push_str(&format!("{}// {}: {}\n", indent,
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
            out.push_str(&format!("{}return Ok(nil)\n", indent));
        }
        IRNodeKind::ErrorTerminal => {
            let label = node.label.replace('"', "\\\"");
            out.push_str(&format!("{}return Err(\"{}\")\n", indent, label));
        }
    }

    for edge in edges {
        if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, visited, indent, depth + 1));
        }
    }
    out
}

/// Emit if/else if/else chain for a Decision node with guarded outgoing edges (Go).
/// Key difference from JS/Py: Go requires `} else if` on the same line.
fn emit_decision_branch<'a>(
    node: &IRNode,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
    depth: usize,
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
        out.push_str(&format!("{}// decision: {} (no guarded branches)\n", indent, node.label));
        for e in &crossed {
            out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, e.to));
        }
        return out;
    }

    let inner_indent = format!("{}    ", indent);
    let mut all_branch_visited: HashSet<&str> = HashSet::new();

    for (i, edge) in guarded.iter().enumerate() {
        out.push_str(&emit_edge_comments(edge, indent));
        let guard = edge.guard.as_ref().unwrap();
        if i == 0 {
            out.push_str(&format!("{}if {} {{\n", indent, guard));
        } else {
            out.push_str(&format!("{}}} else if {} {{\n", indent, guard));
        }
        let mut branch_visited = visited.clone();
        out.push_str(&emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent, depth + 1));
        all_branch_visited.extend(branch_visited.iter().copied());
    }

    if !unguarded.is_empty() {
        out.push_str(&format!("{}}} else {{\n", indent));
        for edge in &unguarded {
            out.push_str(&emit_edge_comments(edge, &inner_indent));
            let mut branch_visited = visited.clone();
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent, depth + 1));
            all_branch_visited.extend(branch_visited.iter().copied());
        }
        out.push_str(&format!("{}}}\n", indent));
    } else if !guarded.is_empty() {
        out.push_str(&format!("{}}}\n", indent));
    }

    visited.extend(all_branch_visited.iter().copied());

    for edge in &crossed {
        out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, edge.to));
    }

    out
}

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

    // Stdlib prelude — only emit imported modules
    if !graph.imported_stdlib.is_empty() {
        out.push_str("// --- Tangle Standard Library (Go) ---\n");
        for module in &graph.imported_stdlib {
            if let Some(prelude) = crate::stdlib::bindings::stdlib_module_prelude(
                crate::stdlib::bindings::TargetHost::Go, module
            ) {
                out.push_str(prelude);
            }
        }
        out.push('\n');
    }

    if !graph.functions.is_empty() {
        out.push_str(&emit_multi_function_go(&graph.functions));
    } else {
        out.push_str(&emit_single_function_go(
            &graph.nodes,
            &graph.edges,
            &graph.entry_node_id,
            module_name,
        ));
    }

    out
}

/// Single-function fallback: one `func {CamelCase(module_name)}()` wrapper + `func main()` entry.
fn emit_single_function_go(
    nodes: &[IRNode],
    edges: &[IREdge],
    entry_node_id: &str,
    module_name: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("func {}() Result {{\n", to_camel(module_name)));
    out.push_str(&emit_go_function_body(nodes, edges, entry_node_id));
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

/// Format a parameter for Go: `name type`. Untyped params (type_ is None)
/// get `any` as their Go type. Typed params use tangle_type_to_go mapping.
fn format_go_param(p: &IRParam) -> String {
    match &p.type_ {
        Some(ty) => format!("{} {}", p.name, tangle_type_to_go(ty)),
        None => format!("{} any", p.name),
    }
}

/// Multi-function emission: one Go function per heading-defined callable.
/// Non-main functions get a `Result` return type so their body's
/// `return Ok(nil)` / `return Err(...)` statements compile. The `main`
/// function is wrapped as `mainImpl() Result` plus a separate `func main()`
/// entry that handles errors and prints the value — symmetric with
/// single-function mode (`func main()` itself cannot have a return type).
fn emit_multi_function_go(functions: &[IRFunction]) -> String {
    let mut out = String::new();
    for func in functions {
        let name = &func.name;
        let params_str = func.params.iter().map(format_go_param).collect::<Vec<_>>().join(", ");
        // Phase 6c 设计 A：外部签名恒为 Result，return_type 仅作 IR 元数据
        let ret_ty = "Result";
        if name == "main" {
            // Wrap main as mainImpl() Result so its `return Ok/Err` statements
            // compile; `func main()` itself cannot declare a return type.
            // mainImpl 不带 params（main 的 params 通常是空的，且 main 是入口不应暴露 params）
            out.push_str(&format!("func mainImpl() {} {{\n", ret_ty));
            out.push_str(&emit_go_function_body(&func.nodes, &func.edges, &func.entry_node_id));
            out.push_str("}\n\n");
            out.push_str("func main() {\n");
            out.push_str("    result := mainImpl()\n");
            out.push_str("    if !result.Ok {\n");
            out.push_str("        fmt.Fprintf(os.Stderr, \"Error: %s\\n\", result.Error)\n");
            out.push_str("        os.Exit(1)\n");
            out.push_str("    }\n");
            out.push_str("    fmt.Println(result.Value)\n");
            out.push_str("}\n");
        } else {
            out.push_str(&format!("func {}({}) {} {{\n", name, params_str, ret_ty));
            out.push_str(&emit_go_function_body(&func.nodes, &func.edges, &func.entry_node_id));
            out.push_str("}\n\n");
        }
    }
    out
}

/// Emit the body of one function (BFS traversal + Result protocol), 4-space indented.
fn emit_go_function_body(nodes: &[IRNode], edges: &[IREdge], entry_node_id: &str) -> String {
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
                out.push_str(&format!(
                    "    // {}: {}\n",
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        _ => "step",
                    },
                    node.label
                ));
            }
            IRNodeKind::Decision => {
                let has_guarded = edges.iter().any(|e| {
                    e.from == node.id && e.guard.is_some() && e.kind != IREdgeKind::Crossed
                });
                if has_guarded {
                    out.push_str(&emit_decision_branch(node, nodes, edges, &mut visited, "    ", 0));
                } else {
                    out.push_str(&format!("    // decision: {}\n", node.label));
                }
            }
            IRNodeKind::Terminal => {
                out.push_str("    return Ok(nil)\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('"', "\\\"");
                out.push_str(&format!("    return Err(\"{}\")\n", label));
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
    fn emit_minimal_graph_produces_valid_go() {
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
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };

        let output = emit_go(&graph, "my.module-name");

        assert!(output.contains("// action: do_work"), "expected action comment, got:\n{}", output);
        // CamelCase sanitization
        assert!(output.contains("func MyModuleName()"), "expected CamelCase name, got:\n{}", output);
        assert!(!output.contains("Tangle Standard Library"), "no stdlib prelude when no modules imported");
    }
}
