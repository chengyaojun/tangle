use crate::ir::graph::*;

pub fn lower_rule_table(table_markdown: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let entry_id = id_gen.fresh();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(),
        kind: IRNodeKind::Decision,
        label: "table.entry".into(),
        source_span: None, source_text: None,
        group: None, style: None,
    });

    let lines: Vec<&str> = table_markdown
        .lines()
        .filter(|l| l.contains('|'))
        .filter(|l| {
            !l.trim()
                .chars()
                .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
        })
        .collect();

    if lines.len() < 2 {
        return graph;
    }

    // Parse header
    let headers: Vec<String> = split_table_row(lines[0]);
    if headers.is_empty() {
        return graph;
    }

    let condition_count = headers.len().saturating_sub(1);

    // Parse data rows
    for (row_idx, line) in lines[1..].iter().enumerate() {
        let cells = split_table_row(line);
        if cells.len() < 2 {
            continue;
        }

        let action = cells.last().unwrap().clone();
        let mut conditions = vec![];

        for (i, cell) in cells
            .iter()
            .enumerate()
            .take(condition_count.min(cells.len().saturating_sub(1)))
        {
            let cond_val = cell.trim().to_string();
            if !cond_val.is_empty() && cond_val != "-" {
                let col_name = headers.get(i).map(|h| h.trim()).unwrap_or("?");
                conditions.push(format!("{} = {}", col_name, cond_val));
            }
        }

        let node_id = id_gen.fresh();
        let guard = if conditions.is_empty() {
            None
        } else {
            Some(conditions.join(" AND "))
        };

        graph.nodes.push(IRNode {
            id: node_id.clone(),
            kind: IRNodeKind::Action,
            label: action,
            source_span: None, source_text: None,
            group: None, style: None,
        });
        graph.edges.push(IREdge {
            from: entry_id.clone(),
            to: node_id,
            kind: IREdgeKind::Condition,
            guard,
            source_span: None,
            priority: Some(row_idx as u32), style: None,
        });
    }

    graph
}

fn split_table_row(line: &str) -> Vec<String> {
    line.split('|')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_assigns_priority_by_row_order() {
        let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| low | - | review |
| - | poor | reject |
";
        let mut id_gen = FreshNodeId::new();
        let graph = lower_rule_table(md, "test.md", &mut id_gen);

        // 3 data rows → 3 edges from entry, each with priority
        let entry_edges: Vec<&IREdge> = graph.edges.iter()
            .filter(|e| e.from == graph.entry_node_id)
            .collect();
        assert_eq!(entry_edges.len(), 3);
        assert_eq!(entry_edges[0].priority, Some(0));
        assert_eq!(entry_edges[1].priority, Some(1));
        assert_eq!(entry_edges[2].priority, Some(2));
    }

    #[test]
    fn table_wildcard_omits_condition() {
        let md = "\
| Income | Result |
|--------|--------|
| - | approve |
";
        let mut id_gen = FreshNodeId::new();
        let graph = lower_rule_table(md, "test.md", &mut id_gen);
        let edge = &graph.edges[0];
        assert!(edge.guard.is_none()); // wildcard → no guard
    }
}
