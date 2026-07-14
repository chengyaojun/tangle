use crate::ir::graph::*;
use crate::model::{SourceSpan, TangleDiagnostic};

/// 检测两行条件是否重叠（都能匹配同一输入）。`-` = 通配。
pub fn rows_overlap(row_a: &[String], row_b: &[String]) -> bool {
    row_a
        .iter()
        .zip(row_b.iter())
        .all(|(a, b)| a == "-" || b == "-" || a == b)
}

pub fn lower_rule_table_with_diagnostics(
    table_markdown: &str,
    file: &str,
    id_gen: &mut FreshNodeId,
) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
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
        return (graph, diagnostics);
    }

    // Parse header
    let headers: Vec<String> = split_table_row(lines[0]);
    if headers.is_empty() {
        return (graph, diagnostics);
    }

    let condition_count = headers.len().saturating_sub(1);

    // 解析所有数据行为条件值数组（用于重叠检测）
    let mut parsed_rows: Vec<Vec<String>> = vec![];
    let mut parsed_actions: Vec<String> = vec![];
    for line in &lines[1..] {
        let cells = split_table_row(line);
        if cells.len() < 2 {
            continue;
        }
        let conds: Vec<String> = {
            let mut c: Vec<String> = cells
                .iter()
                .take(condition_count.min(cells.len().saturating_sub(1)))
                .map(|c| c.trim().to_string())
                .collect();
            // Pad with wildcard "-" to ensure all rows have equal length
            while c.len() < condition_count {
                c.push("-".into());
            }
            c
        };
        parsed_rows.push(conds);
        parsed_actions.push(cells.last().unwrap().clone());
    }

    // 重叠检测：对每对行 (i, j) 且 i < j
    for i in 0..parsed_rows.len() {
        for j in (i + 1)..parsed_rows.len() {
            if rows_overlap(&parsed_rows[i], &parsed_rows[j]) {
                // 检查是否完全相同（duplicate）
                if parsed_rows[i] == parsed_rows[j] {
                    diagnostics.push(TangleDiagnostic {
                        code: "TANGLE_RULE_DUPLICATE".into(),
                        message: format!("rows {} and {} are identical", i + 1, j + 1),
                        // TODO: track line numbers through table parsing to provide accurate spans
                        span: SourceSpan {
                            file: file.into(),
                            start_line: 0,
                            start_column: 0,
                            end_line: 0,
                            end_column: 0,
                        },
                    });
                } else {
                    // 检查行 i 是否完全覆盖行 j（j 不可达）
                    let i_covers_j = parsed_rows[i]
                        .iter()
                        .zip(parsed_rows[j].iter())
                        .all(|(a, b)| a == "-" || a == b);
                    if i_covers_j {
                        diagnostics.push(TangleDiagnostic {
                            code: "TANGLE_RULE_UNREACHABLE".into(),
                            message: format!(
                                "row {} is unreachable (covered by row {})",
                                j + 1,
                                i + 1
                            ),
                            // TODO: track line numbers through table parsing to provide accurate spans
                            span: SourceSpan {
                                file: file.into(),
                                start_line: 0,
                                start_column: 0,
                                end_line: 0,
                                end_column: 0,
                            },
                        });
                    } else {
                        diagnostics.push(TangleDiagnostic {
                            code: "TANGLE_RULE_OVERLAP".into(),
                            message: format!(
                                "rows {} and {} overlap; row {} wins by priority",
                                i + 1,
                                j + 1,
                                i + 1
                            ),
                            // TODO: track line numbers through table parsing to provide accurate spans
                            span: SourceSpan {
                                file: file.into(),
                                start_line: 0,
                                start_column: 0,
                                end_line: 0,
                                end_column: 0,
                            },
                        });
                    }
                }
            }
        }
    }

    // 生成 IR 节点和边
    for (row_idx, conds) in parsed_rows.iter().enumerate() {
        let action = &parsed_actions[row_idx];
        let mut conditions = vec![];

        for (i, cond_val) in conds.iter().enumerate() {
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
            label: action.clone(),
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

    (graph, diagnostics)
}

/// 向后兼容包装器（无 diagnostics）
pub fn lower_rule_table(table_markdown: &str, file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    lower_rule_table_with_diagnostics(table_markdown, file, id_gen).0
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

    #[test]
    fn rows_overlap_detects_wildcard_intersection() {
        assert!(rows_overlap(&["high".into(), "-".into()], &["high".into(), "good".into()]));
        assert!(rows_overlap(&["-".into(), "-".into()], &["low".into(), "bad".into()]));
        assert!(!rows_overlap(&["high".into(), "good".into()], &["low".into(), "good".into()]));
    }

    #[test]
    fn table_overlap_emits_warning() {
        let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| high | - | approve |
| - | good | review |
";
        let mut id_gen = FreshNodeId::new();
        let (_graph, diags) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
        assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_OVERLAP"));
    }

    #[test]
    fn table_no_overlap_no_warning() {
        let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| low | poor | reject |
";
        let mut id_gen = FreshNodeId::new();
        let (_graph, diags) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
        assert!(!diags.iter().any(|d| d.code == "TANGLE_RULE_OVERLAP"));
    }

    #[test]
    fn table_unreachable_emits_info() {
        let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| - | - | approve |
| high | good | reject |
";
        let mut id_gen = FreshNodeId::new();
        let (_graph, diags) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
        assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_UNREACHABLE"));
    }

    #[test]
    fn table_duplicate_emits_warning() {
        let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| high | good | reject |
";
        let mut id_gen = FreshNodeId::new();
        let (_graph, diags) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
        assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_DUPLICATE"));
    }
}
