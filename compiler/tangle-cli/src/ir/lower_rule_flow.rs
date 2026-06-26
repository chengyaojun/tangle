use crate::ir::graph::*;
use std::collections::HashMap;

pub fn lower_rule_flow(mermaid_source: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let mut node_map: HashMap<String, String> = HashMap::new();
    let mut nodes: Vec<IRNode> = vec![];
    let mut edges: Vec<IREdge> = vec![];
    let mut entry_id: Option<String> = None;

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

        // Try standalone node declaration: A[Label] or B(Label) or C{Label}
        if let Some(caps) = parse_node_decl(line) {
            let (mermaid_id, label, is_error) = caps;
            register_node(&mermaid_id, label, is_error, &mut node_map, &mut nodes, &mut entry_id, id_gen);
            continue;
        }

        // Try edge: may contain inline node declarations
        if let Some((from_part, guard, to_part)) = parse_edge_parts(line) {
            // Extract and register nodes from inline declarations
            if let Some((from_id, from_label)) = extract_inline_node(&from_part) {
                register_node(&from_id, from_label, false, &mut node_map, &mut nodes, &mut entry_id, id_gen);
            }
            if let Some((to_id, to_label)) = extract_inline_node(&to_part) {
                let is_error = to_label.to_lowercase().starts_with("error:")
                    || to_label.starts_with("错误:");
                register_node(&to_id, to_label, is_error, &mut node_map, &mut nodes, &mut entry_id, id_gen);
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
                });
            }
        }
    }

    let entry_node_id = entry_id.unwrap_or_else(|| {
        let id = id_gen.next();
        nodes.push(IRNode {
            id: id.clone(),
            kind: IRNodeKind::Terminal,
            label: "empty".into(),
            source_span: None,
        });
        id
    });

    RuleGraph { nodes, edges, error_edges: vec![], entry_node_id }
}

fn register_node(
    mermaid_id: &str, label: String, is_error: bool,
    node_map: &mut HashMap<String, String>, nodes: &mut Vec<IRNode>,
    entry_id: &mut Option<String>, id_gen: &mut FreshNodeId,
) {
    if node_map.contains_key(mermaid_id) { return; }
    let node_id = id_gen.next();
    let kind = if is_error { IRNodeKind::ErrorTerminal } else { IRNodeKind::Action };
    node_map.insert(mermaid_id.to_string(), node_id.clone());
    if entry_id.is_none() {
        *entry_id = Some(node_id.clone());
    }
    nodes.push(IRNode { id: node_id, kind, label, source_span: None });
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
