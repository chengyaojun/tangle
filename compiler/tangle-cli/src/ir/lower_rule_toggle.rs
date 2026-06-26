use crate::ir::graph::*;

pub fn lower_rule_toggle(checkbox_markdown: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let entry_id = id_gen.next();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(),
        kind: IRNodeKind::Compute,
        label: "toggle.entry".into(),
        source_span: None, source_text: None,
    });

    for line in checkbox_markdown.lines() {
        let t = line.trim_start();
        if !t.starts_with("- [") && !t.starts_with("* [") {
            continue;
        }

        let checked = t.contains("[x]") || t.contains("[X]");
        let rest = t
            .trim_start_matches("- [x]")
            .trim_start_matches("- [X]")
            .trim_start_matches("- [ ]")
            .trim_start_matches("* [x]")
            .trim_start_matches("* [X]")
            .trim_start_matches("* [ ]")
            .trim();

        // Extract name from backtick: `name`: desc
        let name = if let Some(tick_start) = rest.find('`') {
            let after_tick = &rest[tick_start + 1..];
            if let Some(tick_end) = after_tick.find('`') {
                after_tick[..tick_end].to_string()
            } else {
                "flag".to_string()
            }
        } else {
            "flag".to_string()
        };

        let node_id = id_gen.next();
        graph.nodes.push(IRNode {
            id: node_id.clone(),
            kind: IRNodeKind::Compute,
            label: format!("{} = {}", name, checked),
            source_span: None, source_text: None,
        });
        graph.edges.push(IREdge {
            from: entry_id.clone(),
            to: node_id,
            kind: IREdgeKind::Control,
            guard: None,
            source_span: None,
        });
    }

    graph
}
