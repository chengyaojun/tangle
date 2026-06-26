use crate::ir::graph::*;
use std::collections::HashMap;

pub fn lower_rule_flow(mermaid_source: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let mut node_map: HashMap<String, String> = HashMap::new(); // mermaid_id -> generated_id
    let mut nodes: Vec<IRNode> = vec![];
    let mut edges: Vec<IREdge> = vec![];
    let mut entry_id: Option<String> = None;

    for line in mermaid_source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("graph ") || line.starts_with("graph\t") {
            continue;
        }

        // Try node declaration: A[Label] or B(Label) or C{Label}
        if let Some(caps) = parse_node_decl(line) {
            let (mermaid_id, label, is_error) = caps;
            let node_id = id_gen.next();
            let kind = if is_error {
                IRNodeKind::ErrorTerminal
            } else {
                IRNodeKind::Action
            };
            node_map.insert(mermaid_id, node_id.clone());
            if entry_id.is_none() {
                entry_id = Some(node_id.clone());
            }
            nodes.push(IRNode {
                id: node_id,
                kind,
                label,
                source_span: None,
            });
            continue;
        }

        // Try edge: A -->|guard| B or A --> B
        if let Some((from_id, guard, to_id)) = parse_edge(line) {
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

    RuleGraph {
        nodes,
        edges,
        error_edges: vec![],
        entry_node_id,
    }
}

/// Parse node declaration like A[Start: Review] -> (mermaid_id, label, is_error)
/// Mermaid IDs are alphanumeric+underscore; bracket types: [], (), {}
fn parse_node_decl(line: &str) -> Option<(String, String, bool)> {
    let trimmed = line.trim();

    // Find end of mermaid ID (first non-alphanumeric, non-underscore char)
    let id_end = trimmed.find(|c: char| !c.is_ascii_alphanumeric() && c != '_')?;
    let mermaid_id = trimmed[..id_end].to_string();
    let rest = trimmed[id_end..].trim_start();

    // Match opening bracket
    let (_open, close) = if rest.starts_with('[') {
        ('[', ']')
    } else if rest.starts_with('(') {
        ('(', ')')
    } else if rest.starts_with('{') {
        ('{', '}')
    } else {
        return None;
    };

    if !rest.ends_with(close) {
        return None;
    }
    let label = rest[1..rest.len() - 1].trim().to_string();
    let is_error =
        label.to_lowercase().starts_with("error:") || label.starts_with("错误:");

    Some((mermaid_id, label, is_error))
}

/// Parse edge like A -->|approved| B -> (from_id, guard, to_id)
/// or A --> B -> (from_id, None, to_id)
fn parse_edge(line: &str) -> Option<(String, Option<String>, String)> {
    let trimmed = line.trim();
    let arrow_pos = trimmed.find("-->")?;
    let from_id = trimmed[..arrow_pos].trim().to_string();
    let after_arrow = trimmed[arrow_pos + 3..].trim();

    if let Some(pipe_start) = after_arrow.find('|') {
        let pipe_end = after_arrow[pipe_start + 1..].find('|')?;
        let guard = after_arrow[pipe_start + 1..pipe_start + 1 + pipe_end]
            .trim()
            .to_string();
        let to_id = after_arrow[pipe_start + 1 + pipe_end + 1..]
            .trim()
            .to_string();
        Some((from_id, Some(guard), to_id))
    } else {
        let to_id = after_arrow.trim().to_string();
        Some((from_id, None, to_id))
    }
}
