use crate::ir::graph::*;

pub fn lower_rule_tree(list_markdown: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let entry_id = id_gen.next();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(),
        kind: IRNodeKind::Decision,
        label: "tree.entry".into(),
        source_span: None,
    });

    let items: Vec<String> = list_markdown
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("* ") || t.starts_with("- ")
        })
        .map(|l| {
            l.trim_start()
                .trim_start_matches("* ")
                .trim_start_matches("- ")
                .trim()
                .to_string()
        })
        .collect();

    let mut prev_id = entry_id;
    for item in items {
        let node_id = id_gen.next();
        graph.nodes.push(IRNode {
            id: node_id.clone(),
            kind: IRNodeKind::Decision,
            label: item.clone(),
            source_span: None,
        });
        graph.edges.push(IREdge {
            from: prev_id,
            to: node_id.clone(),
            kind: IREdgeKind::Condition,
            guard: Some(item),
            source_span: None,
        });
        prev_id = node_id;
    }

    graph
}
