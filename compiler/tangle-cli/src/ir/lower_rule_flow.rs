use crate::ir::graph::*;
use std::collections::HashMap;

pub fn lower_rule_flow(mermaid_source: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let mut node_map: HashMap<String, String> = HashMap::new();
    let mut nodes: Vec<IRNode> = vec![];
    let mut edges: Vec<IREdge> = vec![];
    let mut entry_id: Option<String> = None;

    let mut subgraph_stack: Vec<String> = vec![];
    let mut edge_styles: HashMap<usize, String> = HashMap::new();
    let mut node_styles: HashMap<String, String> = HashMap::new();
    let mut class_defs: HashMap<String, String> = HashMap::new();
    let mut class_assignments: HashMap<String, String> = HashMap::new();

    for line in mermaid_source.lines() {
        let line = line.trim();
        // Skip empty lines, graph declarations, and fence markers
        if line.is_empty()
            || line.starts_with("graph ")
            || line.starts_with("graph\t")
            || line.starts_with("```")
        {
            continue;
        }

        // subgraph start: "subgraph Approval" -> push first whitespace-delimited token
        if line.starts_with("subgraph ") {
            let name = line
                .trim_start_matches("subgraph ")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            subgraph_stack.push(name);
            continue;
        }

        // subgraph end
        if line == "end" {
            subgraph_stack.pop();
            continue;
        }

        // classDef <name> <style-def>
        if line.starts_with("classDef ") {
            let rest = line.trim_start_matches("classDef ");
            if let Some(sp) = rest.find(char::is_whitespace) {
                let class_name = rest[..sp].to_string();
                let style_def = rest[sp..].trim().to_string();
                class_defs.insert(class_name, style_def);
            }
            continue;
        }

        // class assignment: "class A,B className"
        if line.starts_with("class ") {
            let rest = line.trim_start_matches("class ");
            if let Some(sp) = rest.find(char::is_whitespace) {
                let node_ids = rest[..sp].split(',').map(|s| s.trim().to_string());
                let class_name = rest[sp..].trim().to_string();
                for nid in node_ids {
                    class_assignments.insert(nid, class_name.clone());
                }
            }
            continue;
        }

        // style <nodeId> <style-def>
        if line.starts_with("style ") {
            let rest = line.trim_start_matches("style ");
            if let Some(sp) = rest.find(char::is_whitespace) {
                let node_id = rest[..sp].to_string();
                let style_def = rest[sp..].trim().to_string();
                node_styles.insert(node_id, style_def);
            }
            continue;
        }

        // linkStyle <idx> <style-def>
        if line.starts_with("linkStyle ") {
            let rest = line.trim_start_matches("linkStyle ");
            if let Some(sp) = rest.find(char::is_whitespace) {
                // Parse failure: skip the line entirely (do NOT default to 0,
                // which would wrongly style the first edge).
                if let Ok(idx) = rest[..sp].parse::<usize>() {
                    let style_def = rest[sp..].trim().to_string();
                    edge_styles.insert(idx, style_def);
                }
            }
            continue;
        }

        let current_group = subgraph_stack.last().cloned();

        // Try standalone node declaration: A[Label] or B(Label) or C{Label}
        if let Some(caps) = parse_node_decl(line) {
            let (mermaid_id, label, is_error) = caps;
            register_node(
                &mermaid_id, label, is_error, current_group,
                &mut node_map, &mut nodes, &mut entry_id, id_gen,
            );
            continue;
        }

        // Try edge: may contain inline node declarations
        if let Some((from_part, guard, to_part)) = parse_edge_parts(line) {
            // Extract and register nodes from inline declarations
            if let Some((from_id, from_label)) = extract_inline_node(&from_part) {
                register_node(
                    &from_id, from_label, false, current_group.clone(),
                    &mut node_map, &mut nodes, &mut entry_id, id_gen,
                );
            }
            if let Some((to_id, to_label)) = extract_inline_node(&to_part) {
                let is_error = to_label.to_lowercase().starts_with("error:")
                    || to_label.starts_with("错误:");
                register_node(
                    &to_id, to_label, is_error, current_group,
                    &mut node_map, &mut nodes, &mut entry_id, id_gen,
                );
            }

            // Resolve edge endpoints (strip labels if present)
            let from_id = extract_node_id(&from_part);
            let to_id = extract_node_id(&to_part);

            if let (Some(from), Some(to)) = (node_map.get(&from_id), node_map.get(&to_id)) {
                let kind = if guard.is_some() {
                    IREdgeKind::Condition
                } else {
                    IREdgeKind::Control
                };
                edges.push(IREdge {
                    from: from.clone(),
                    to: to.clone(),
                    kind,
                    guard,
                    source_span: None,
                    priority: None, style: None,
                });
            }
        }
    }

    // Apply node styles (by mermaid id)
    for (mermaid_id, style) in &node_styles {
        if let Some(ir_id) = node_map.get(mermaid_id) {
            if let Some(node) = nodes.iter_mut().find(|n| &n.id == ir_id) {
                node.style = Some(style.clone());
            }
        }
    }
    // Apply class assignments — use class_defs resolved style text so that
    // node.style carries the parsed style (e.g. "fill:#ff0,stroke:#f00") rather
    // than the raw class name. Falls back to the class name if undefined.
    for (mermaid_id, class_name) in &class_assignments {
        if let Some(ir_id) = node_map.get(mermaid_id) {
            if let Some(node) = nodes.iter_mut().find(|n| &n.id == ir_id) {
                let style = class_defs
                    .get(class_name)
                    .cloned()
                    .unwrap_or_else(|| class_name.clone());
                node.style = Some(style);
            }
        }
    }
    // Apply edge styles (by index)
    for (idx, style) in &edge_styles {
        if *idx < edges.len() {
            edges[*idx].style = Some(style.clone());
        }
    }

    let entry_node_id = entry_id.unwrap_or_else(|| {
        let id = id_gen.fresh();
        nodes.push(IRNode {
            id: id.clone(),
            kind: IRNodeKind::Terminal,
            label: "empty".into(),
            source_span: None, source_text: None,
            group: None, style: None,
        });
        id
    });

    RuleGraph { nodes, edges, error_edges: vec![], entry_node_id, imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![] }
}

#[allow(clippy::too_many_arguments)]
fn register_node(
    mermaid_id: &str, label: String, is_error: bool, group: Option<String>,
    node_map: &mut HashMap<String, String>, nodes: &mut Vec<IRNode>,
    entry_id: &mut Option<String>, id_gen: &mut FreshNodeId,
) {
    if node_map.contains_key(mermaid_id) { return; }
    let node_id = id_gen.fresh();
    let kind = if is_error { IRNodeKind::ErrorTerminal } else { IRNodeKind::Action };
    node_map.insert(mermaid_id.to_string(), node_id.clone());
    if entry_id.is_none() {
        *entry_id = Some(node_id.clone());
    }
    nodes.push(IRNode { id: node_id, kind, label, source_span: None, source_text: None, group, style: None });
}

/// Parse standalone node declaration: A[Label] -> (id, label, is_error)
fn parse_node_decl(line: &str) -> Option<(String, String, bool)> {
    let trimmed = line.trim();
    let id_end = trimmed.find(|c: char| !c.is_ascii_alphanumeric() && c != '_')?;
    let mermaid_id = trimmed[..id_end].to_string();
    let rest = trimmed[id_end..].trim_start();

    let close = if rest.starts_with('[') { ']' }
    else if rest.starts_with('(') { ')' }
    else if rest.starts_with('{') { '}' }
    else { return None; };

    if !rest.ends_with(close) { return None; }
    let label = rest[1..rest.len()-1].trim().to_string();
    let is_error = label.to_lowercase().starts_with("error:") || label.starts_with("错误:");

    // Verify this is a standalone node (no edge on the same line)
    if trimmed.contains("-->") { return None; }

    Some((mermaid_id, label, is_error))
}

/// Extract node ID from a part like "A" or "A[Label]" -> "A"
fn extract_node_id(part: &str) -> String {
    let part = part.trim();
    if let Some(pos) = part.find(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        part[..pos].to_string()
    } else {
        part.to_string()
    }
}

/// Extract inline node: "A[Label]" -> ("A", "Label"), "A" -> None (bare ID)
fn extract_inline_node(part: &str) -> Option<(String, String)> {
    let part = part.trim();
    let id_end = part.find(|c: char| !c.is_ascii_alphanumeric() && c != '_')?;
    let id = part[..id_end].to_string();
    let rest = part[id_end..].trim_start();

    let close = if rest.starts_with('[') { ']' }
    else if rest.starts_with('(') { ')' }
    else if rest.starts_with('{') { '}' }
    else { return None; };

    let close_pos = rest.find(close)?;
    let label = rest[1..close_pos].trim().to_string();
    Some((id, label))
}

/// Parse edge: "A -->|guard| B" -> ("A", Some("guard"), "B")
/// Also handles inline labels: "A[Label] -->|guard| B(Label)"
fn parse_edge_parts(line: &str) -> Option<(String, Option<String>, String)> {
    let trimmed = line.trim();
    let arrow_pos = trimmed.find("-->")?;
    let from_part = trimmed[..arrow_pos].trim().to_string();
    let after_arrow = trimmed[arrow_pos + 3..].trim();

    if let Some(pipe_start) = after_arrow.find('|') {
        let pipe_end = after_arrow[pipe_start + 1..].find('|')?;
        let guard = after_arrow[pipe_start + 1..pipe_start + 1 + pipe_end].trim().to_string();
        let to_part = after_arrow[pipe_start + 1 + pipe_end + 1..].trim().to_string();
        Some((from_part, Some(guard), to_part))
    } else {
        let to_part = after_arrow.trim().to_string();
        Some((from_part, None, to_part))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_subgraph_assigns_group() {
        let md = "\
graph TD
    A[Start] --> B{Decision}
    subgraph Approval
        B -->|yes| C[Approve]
    end
    subgraph Rejection
        B -->|no| E[Reject]
    end
";
        let mut id_gen = FreshNodeId::new();
        let graph = lower_rule_flow(md, "test.md", &mut id_gen);

        let node_c = graph.nodes.iter().find(|n| n.label == "Approve").unwrap();
        assert_eq!(node_c.group.as_deref(), Some("Approval"));
        let node_e = graph.nodes.iter().find(|n| n.label == "Reject").unwrap();
        assert_eq!(node_e.group.as_deref(), Some("Rejection"));
        let node_a = graph.nodes.iter().find(|n| n.label == "Start").unwrap();
        assert!(node_a.group.is_none());
    }

    #[test]
    fn flow_no_subgraph_group_none() {
        let md = "\
graph TD
    A[Start] --> B[End]
";
        let mut id_gen = FreshNodeId::new();
        let graph = lower_rule_flow(md, "test.md", &mut id_gen);
        for node in &graph.nodes {
            assert!(node.group.is_none());
        }
    }

    #[test]
    fn flow_class_assigns_style() {
        let md = "\
graph TD
    A[Start] --> B[End]
    classDef highlight fill:#ff0,stroke:#f00
    class B highlight
";
        let mut id_gen = FreshNodeId::new();
        let graph = lower_rule_flow(md, "test.md", &mut id_gen);
        let node_b = graph.nodes.iter().find(|n| n.label == "End").unwrap();
        assert_eq!(node_b.style.as_deref(), Some("fill:#ff0,stroke:#f00"));
    }

    #[test]
    fn flow_style_assigns_to_node() {
        let md = "\
graph TD
    A[Start] --> B[End]
    style B fill:#cfc
";
        let mut id_gen = FreshNodeId::new();
        let graph = lower_rule_flow(md, "test.md", &mut id_gen);
        let node_b = graph.nodes.iter().find(|n| n.label == "End").unwrap();
        assert_eq!(node_b.style.as_deref(), Some("fill:#cfc"));
    }
}
