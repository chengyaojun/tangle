use crate::ir::graph::*;
use std::collections::{HashSet, VecDeque};

/// Maximum recursion depth for `emit_branch_body`. Beyond this the emitter
/// stops recursing and emits a `// max depth reached` comment to prevent
/// stack overflow on pathological graphs.
const MAX_BRANCH_DEPTH: usize = 100;

fn sanitize_module_name(name: &str) -> String {
    name.replace(['.', '-'], "_")
}

/// Translate a Tangle source statement into JS satisfying the Result runtime protocol.
/// Bare `return X` becomes `return Ok(X)`; already-wrapped `Ok(...)`/`Err(...)` is left as-is.
/// Struct construction `Type { ... }` / `this { ... }` becomes `{ ... }`;
/// record update `var { ... }` becomes `{ ...var, ... }`.
/// Propagation `expr?` becomes `__unwrap(expr)`.
fn translate_stmt_to_js(src: &str) -> String {
    let trimmed = src.trim();
    if trimmed == "return" {
        return "return Ok(undefined)".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("return ") {
        let expr = rest.trim();
        if expr.is_empty() {
            return "return Ok(undefined)".to_string();
        }
        let translated = translate_propagation(&translate_struct_literals(expr));
        if translated.starts_with("Ok(") || translated.starts_with("Err(") {
            return format!("return {}", translated);
        }
        return format!("return Ok({})", translated);
    }
    translate_propagation(&translate_struct_literals(trimmed))
}

/// Rewrite Tangle propagation `expr?` into `__unwrap(expr)`.
/// Scans for `?` outside string literals and wraps the immediately preceding atom.
fn translate_propagation(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' {
            let quote = c;
            out.push(c);
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    out.push(chars[i]);
                    out.push(chars[i + 1]);
                    i += 2;
                } else {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            if i < chars.len() {
                out.push(chars[i]);
                i += 1;
            }
            continue;
        }
        if c == '?' {
            let atom_start = find_atom_start(&out);
            let atom = out[atom_start..].to_string();
            out.truncate(atom_start);
            out.push_str("__unwrap(");
            out.push_str(&atom);
            out.push(')');
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Find the byte offset where the expression atom immediately preceding the end of `s` starts.
/// An atom is a balanced bracket group optionally prefixed by identifier chars and `.` (member access).
fn find_atom_start(s: &str) -> usize {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = chars.len();
    while i > 0 && chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    while i > 0 {
        let c = chars[i - 1].1;
        if c == ')' || c == ']' || c == '}' {
            let open = match c { ')' => '(', ']' => '[', '}' => '{', _ => unreachable!() };
            let mut depth = 1;
            i -= 1;
            while i > 0 && depth > 0 {
                i -= 1;
                if chars[i].1 == c { depth += 1; }
                else if chars[i].1 == open { depth -= 1; }
            }
        } else if c.is_alphanumeric() || c == '_' || c == '.' {
            i -= 1;
        } else {
            break;
        }
    }
    if i < chars.len() { chars[i].0 } else { 0 }
}

/// Rewrite Tangle struct-literal syntax into JS object literals.
/// `Identifier {` directly followed by `{` is treated as a struct:
///  - `this`/Capitalized → construction, drop the name: `Order { a: 1 }` → `{ a: 1 }`
///  - lowercase (non-keyword) → record update, spread: `order { a: 1 }` → `{ ...order, a: 1 }`
fn translate_struct_literals(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && chars[j] == '{' && !is_block_keyword(&ident) {
                if ident == "this" || ident.chars().next().is_some_and(|c| c.is_uppercase()) {
                    out.push('{');
                    i = j + 1;
                    continue;
                } else {
                    out.push_str("{ ...");
                    out.push_str(&ident);
                    out.push(',');
                    i = j + 1;
                    continue;
                }
            }
            out.push_str(&ident);
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn is_block_keyword(ident: &str) -> bool {
    matches!(ident, "else" | "try" | "finally" | "do")
}

/// True if `?` appears outside a string literal (i.e. as a propagation operator).
fn statement_uses_propagation(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() { i += 2; }
                else { i += 1; }
            }
            if i < chars.len() { i += 1; }
        } else if c == '?' {
            return true;
        } else {
            i += 1;
        }
    }
    false
}

pub fn emit_js(graph: &RuleGraph, module_name: &str) -> String {
    let module_name = sanitize_module_name(module_name);
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
        // Function-level imports: [println](fmt) -> const println = fmt.println;
        for (alias, module) in &graph.stdlib_imports {
            if alias != module {
                // Check if alias is comma-separated: [print, println](fmt)
                for fn_name in alias.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    if fn_name != module {
                        out.push_str(&format!("const {} = {}.{};\n", fn_name, module, fn_name));
                    }
                }
            }
        }
        out.push('\n');
    }

    if !graph.functions.is_empty() {
        out.push_str(&emit_multi_function_js(&graph.functions));
    } else {
        out.push_str(&emit_single_function_js(&graph.nodes, &graph.edges, &graph.entry_node_id, &module_name));
    }

    out
}

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

/// Recursively emit the body of a branch target inside an if/else block.
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
            if let Some(ref src) = node.source_text {
                out.push_str(&format!("{}{};\n", indent, translate_stmt_to_js(src)));
            } else {
                out.push_str(&format!("{}// {}: {}\n", indent,
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        IRNodeKind::Decision => "decision",
                        _ => "unknown",
                    },
                    node.label
                ));
            }
        }
        IRNodeKind::Terminal => {
            out.push_str(&format!("{}return Ok(undefined);\n", indent));
        }
        IRNodeKind::ErrorTerminal => {
            let label = node.label.replace('\'', "\\'");
            out.push_str(&format!("{}return Err('{}');\n", indent, label));
        }
    }

    // Recurse into non-Crossed successors
    for edge in edges {
        if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, visited, indent, depth + 1));
        }
    }
    out
}

/// Emit if/else-if chain for a Decision node with guarded outgoing edges.
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
        // No emitable edges; fall back to comment
        out.push_str(&format!("{}// decision: {} (no guarded branches)\n", indent, node.label));
        for e in &crossed {
            out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, e.to));
        }
        return out;
    }

    let inner_indent = format!("{}  ", indent);

    // 收集所有分支访问的节点，最后一次性合并，避免污染后续分支的克隆
    let mut all_branch_visited: HashSet<&str> = HashSet::new();

    for (i, edge) in guarded.iter().enumerate() {
        out.push_str(&emit_edge_comments(edge, indent));
        let guard = edge.guard.as_ref().unwrap();
        if i == 0 {
            out.push_str(&format!("{}if ({}) {{\n", indent, guard));
        } else {
            out.push_str(&format!("{}else if ({}) {{\n", indent, guard));
        }
        let mut branch_visited = visited.clone();
        out.push_str(&emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent, depth + 1));
        all_branch_visited.extend(branch_visited.iter().copied());
        out.push_str(&format!("{}}}\n", indent));
    }

    if !unguarded.is_empty() {
        out.push_str(&format!("{}else {{\n", indent));
        for edge in &unguarded {
            out.push_str(&emit_edge_comments(edge, &inner_indent));
            let mut branch_visited = visited.clone();
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent, depth + 1));
            all_branch_visited.extend(branch_visited.iter().copied());
        }
        out.push_str(&format!("{}}}\n", indent));
    }

    // 所有分支完成后，一次性合并回 visited
    visited.extend(all_branch_visited.iter().copied());

    for edge in &crossed {
        out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, edge.to));
    }

    out
}

/// Emit the body of one function (BFS traversal + Result protocol), wrapped in
/// try/catch when any statement uses the `?` propagation operator. Returns the
/// indented body lines WITHOUT the surrounding `function ... { }`.
fn emit_js_function_body(nodes: &[IRNode], edges: &[IREdge], entry_node_id: &str) -> String {
    let uses_propagation = nodes.iter().any(|n| {
        n.source_text.as_ref().is_some_and(|s| statement_uses_propagation(s))
    });
    let mut out = String::new();
    if uses_propagation {
        out.push_str("  try {\n");
    }

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

        out.push_str(&emit_node_comments(node, "  "));

        match node.kind {
            IRNodeKind::Action | IRNodeKind::Compute => {
                if let Some(ref src) = node.source_text {
                    out.push_str(&format!("  {};\n", translate_stmt_to_js(src)));
                } else {
                    out.push_str(&format!("  // {}: {}\n",
                        match node.kind {
                            IRNodeKind::Action => "action",
                            IRNodeKind::Compute => "compute",
                            _ => "unknown",
                        },
                        node.label
                    ));
                }
            }
            IRNodeKind::Decision => {
                // Check if this Decision has guarded outgoing edges
                let has_guarded = edges.iter().any(|e| {
                    e.from == node.id && e.guard.is_some() && e.kind != IREdgeKind::Crossed
                });
                if has_guarded {
                    out.push_str(&emit_decision_branch(node, nodes, edges, &mut visited, "  ", 0));
                } else {
                    // Fall back to existing linear behavior
                    if let Some(ref src) = node.source_text {
                        out.push_str(&format!("  {};\n", translate_stmt_to_js(src)));
                    } else {
                        out.push_str(&format!("  // decision: {}\n", node.label));
                    }
                }
            }
            IRNodeKind::Terminal => {
                out.push_str("  return Ok(undefined);\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!("  return Err('{}');\n", label));
            }
        }

        for edge in edges {
            if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
                // Decision 节点的 guarded 边已在 if/else 中处理，跳过入队
                if node.kind == IRNodeKind::Decision && edge.guard.is_some() {
                    continue;
                }
                if let Some(target) = nodes.iter().find(|n| n.id == edge.to) {
                    queue.push_back(target);
                }
            }
        }
    }

    // Guarantee the function always returns a Result (rule graphs may have no Terminal node)
    out.push_str("  return Ok(undefined);\n");
    if uses_propagation {
        out.push_str("  } catch (e) { if (e && e.ok === false) return e; throw e; }\n");
    }
    out
}

/// Single-function fallback: one `module_name()` wrapper + entry invocation.
fn emit_single_function_js(nodes: &[IRNode], edges: &[IREdge], entry_node_id: &str, module_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("function {}() {{\n", module_name));
    out.push_str(&emit_js_function_body(nodes, edges, entry_node_id));
    out.push_str("}\n\n");
    out.push_str(&format!(
        "// Entry point\nconst __result = {module_name}();\nif (!__result.ok) {{ console.error('Error:', __result.error); process.exit(1); }}\nif (__result.value !== undefined) {{ console.log(__result.value); }}\n",
        module_name = module_name
    ));
    out
}

/// Multi-function emission: one JS function per heading-defined callable, with
/// params as arguments. Methods (`receiver = Some`) attach to a receiver object
/// (e.g. `Order.create = function(...)`); free functions are plain declarations.
/// The entry point is the function named `main`.
fn emit_multi_function_js(functions: &[IRFunction]) -> String {
    let mut out = String::new();

    // Declare receiver objects for methods (e.g. `const Order = {};`)
    let mut receivers: Vec<&str> = vec![];
    for f in functions {
        if let Some(r) = &f.receiver {
            if !receivers.contains(&r.as_str()) {
                receivers.push(r.as_str());
            }
        }
    }
    for r in &receivers {
        out.push_str(&format!("const {} = {{}};\n", r));
    }
    if !receivers.is_empty() {
        out.push('\n');
    }

    for f in functions {
        let params = f.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ");
        match &f.receiver {
            Some(r) => out.push_str(&format!("{}.{} = function({}) {{\n", r, f.name, params)),
            None => out.push_str(&format!("function {}({}) {{\n", f.name, params)),
        }
        out.push_str(&emit_js_function_body(&f.nodes, &f.edges, &f.entry_node_id));
        match &f.receiver {
            Some(_) => out.push_str("};\n\n"),
            None => out.push_str("}\n\n"),
        }
    }

    // Entry point: call `main()` (the Callable heading named "main").
    let entry_name = functions.iter().find(|f| f.name == "main")
        .map(|f| f.name.as_str())
        .unwrap_or_else(|| functions.first().map(|f| f.name.as_str()).unwrap_or("main"));
    out.push_str(&format!(
        "// Entry point\nconst __result = {entry}();\nif (!__result.ok) {{ console.error('Error:', __result.error); process.exit(1); }}\nif (__result.value !== undefined) {{ console.log(__result.value); }}\n",
        entry = entry_name
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
            source_span: None, source_text: None,
            group: None, style: None,
        }
    }

    fn make_terminal_node(id: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind: IRNodeKind::Terminal,
            label: "done".to_string(),
            source_span: None, source_text: None,
            group: None, style: None,
        }
    }

    fn make_action_node(id: &str, label: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind: IRNodeKind::Action,
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
    fn emit_minimal_graph_contains_function_and_prelude() {
        let graph = RuleGraph {
            nodes: vec![
                make_entry_node("n0"),
                make_terminal_node("n1"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
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
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
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
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };

        let output = emit_js(&graph, "empty_mod");

        assert!(!output.is_empty(), "output should not be empty");
        assert!(output.contains("Tangle Runtime Prelude"), "output should contain prelude");
        assert!(output.contains("function empty_mod()"), "output should contain function definition");
    }

    #[test]
    fn emit_graph_sanitizes_module_name_with_special_chars() {
        let graph = RuleGraph {
            nodes: vec![
                make_entry_node("n0"),
                make_terminal_node("n1"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };

        let output = emit_js(&graph, "io-system.tangle");

        assert!(output.contains("function io_system_tangle()"), "module name should be sanitized, got:\n{}", output);
        assert!(output.contains("const __result = io_system_tangle();"), "entry invocation should use sanitized name, got:\n{}", output);
    }

    #[test]
    fn translate_wraps_bare_return_value_in_ok() {
        assert_eq!(translate_stmt_to_js("return \"hello\""), "return Ok(\"hello\")");
        assert_eq!(translate_stmt_to_js("return a"), "return Ok(a)");
        assert_eq!(translate_stmt_to_js("return x + 1"), "return Ok(x + 1)");
    }

    #[test]
    fn translate_bare_return_becomes_ok_undefined() {
        assert_eq!(translate_stmt_to_js("return"), "return Ok(undefined)");
        assert_eq!(translate_stmt_to_js("return "), "return Ok(undefined)");
    }

    #[test]
    fn translate_leaves_already_wrapped_returns() {
        assert_eq!(translate_stmt_to_js("return Ok(x)"), "return Ok(x)");
        assert_eq!(translate_stmt_to_js("return Err(\"fail\")"), "return Err(\"fail\")");
    }

    #[test]
    fn translate_passes_through_non_return_statements() {
        assert_eq!(translate_stmt_to_js("let x = 42"), "let x = 42");
        assert_eq!(translate_stmt_to_js("fmt.println(\"hi\")"), "fmt.println(\"hi\")");
    }

    #[test]
    fn translate_struct_construction_drops_type_name() {
        assert_eq!(
            translate_stmt_to_js("return Order { id: id, status: \"created\" }"),
            "return Ok({ id: id, status: \"created\" })"
        );
    }

    #[test]
    fn translate_this_struct_construction() {
        assert_eq!(
            translate_stmt_to_js("return this { id: 1, name: \"active\" }"),
            "return Ok({ id: 1, name: \"active\" })"
        );
    }

    #[test]
    fn translate_record_update_becomes_spread() {
        assert_eq!(
            translate_stmt_to_js("return Ok(order { status: \"paid\" })"),
            "return Ok({ ...order, status: \"paid\" })"
        );
        assert_eq!(
            translate_stmt_to_js("return order { status: \"confirmed\" }"),
            "return Ok({ ...order, status: \"confirmed\" })"
        );
    }

    #[test]
    fn translate_leaves_block_keywords_before_brace() {
        assert_eq!(translate_struct_literals("else { x }"), "else { x }");
        assert_eq!(translate_struct_literals("try { x }"), "try { x }");
        assert_eq!(translate_struct_literals("if (c) { x }"), "if (c) { x }");
    }

    #[test]
    fn translate_propagation_wraps_call() {
        assert_eq!(translate_propagation("process()"), "process()");
        assert_eq!(translate_propagation("process()?"), "__unwrap(process())");
        assert_eq!(
            translate_propagation("Err(\"PayFailed\", \"Invalid amount\")?"),
            "__unwrap(Err(\"PayFailed\", \"Invalid amount\"))"
        );
        assert_eq!(
            translate_propagation("Order.create(\"ord-1\", 100)?"),
            "__unwrap(Order.create(\"ord-1\", 100))"
        );
    }

    #[test]
    fn translate_propagation_in_assignment() {
        assert_eq!(
            translate_stmt_to_js("result = process()?"),
            "result = __unwrap(process())"
        );
    }

    #[test]
    fn translate_propagation_ignores_question_in_strings() {
        assert_eq!(translate_propagation("println(\"is it? yes\")"), "println(\"is it? yes\")");
        assert!(!statement_uses_propagation("println(\"is it? yes\")"));
        assert!(statement_uses_propagation("x = f()?"));
    }

    fn make_return_node(id: &str, src: &str) -> IRNode {
        IRNode {
            id: id.to_string(),
            kind: IRNodeKind::Action,
            label: "return".to_string(),
            source_span: None, source_text: Some(src.to_string()),
            group: None, style: None,
        }
    }

    #[test]
    fn emit_wraps_return_source_in_ok() {
        let graph = RuleGraph {
            nodes: vec![
                make_return_node("n0", "return \"done\""),
                make_terminal_node("n1"),
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };

        let output = emit_js(&graph, "mod");

        assert!(output.contains("return Ok(\"done\");"), "bare return should be wrapped in Ok, got:\n{}", output);
    }

    #[test]
    fn emit_rule_graph_without_terminal_still_returns_result() {
        // Mimics a rule graph: compute nodes with no source_text, no Terminal node
        let graph = RuleGraph {
            nodes: vec![
                IRNode {
                    id: "n0".to_string(), kind: IRNodeKind::Compute,
                    label: "toggle.entry".to_string(),
                    source_span: None, source_text: None,
                    group: None, style: None,
                },
                IRNode {
                    id: "n1".to_string(), kind: IRNodeKind::Compute,
                    label: "flag = true".to_string(),
                    source_span: None, source_text: None,
                    group: None, style: None,
                },
            ],
            edges: vec![make_edge("n0", "n1")],
            error_edges: vec![],
            entry_node_id: "n0".to_string(),
            imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
        };

        let output = emit_js(&graph, "toggles");

        assert!(output.contains("return Ok(undefined);"), "rule graph must end with a Result return, got:\n{}", output);
        assert!(output.contains("const __result = toggles();"), "entry invocation should be present, got:\n{}", output);
    }
}
