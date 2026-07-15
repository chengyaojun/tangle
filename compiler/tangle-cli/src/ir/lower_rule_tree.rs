use crate::ir::graph::*;
use crate::model::{SourceSpan, TangleDiagnostic};

pub fn lower_rule_tree(
    list_markdown: &str,
    _file: &str,
    id_gen: &mut FreshNodeId,
) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
    let entry_id = id_gen.fresh();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(),
        kind: IRNodeKind::Decision,
        label: "tree.entry".into(),
        source_span: None,
        source_text: None,
        group: None,
        style: None,
    });

    let roots = parse_list_to_tree(list_markdown);

    for branch in &roots {
        if branch.children.is_empty() {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_RULE_EMPTY_BRANCH".into(),
                message: format!("branch '{}' has no conditions or action", branch.text),
                span: SourceSpan { file: _file.into(), start_line: branch.line, start_column: 0, end_line: branch.line, end_column: 0 },
            });
            continue;
        }

        let has_action = branch.children.iter().any(|c| c.text.starts_with("Action:"));
        if !has_action {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_RULE_NO_ACTION".into(),
                message: format!("branch '{}' has no Action: marker", branch.text),
                span: SourceSpan { file: _file.into(), start_line: branch.line, start_column: 0, end_line: branch.line, end_column: 0 },
            });
        }

        let conditions: Vec<&ListNode> = branch.children.iter()
            .filter(|c| !c.text.starts_with("Action:"))
            .collect();

        let first_cond_id = if conditions.is_empty() {
            None
        } else {
            let cond = conditions[0];
            let node_id = id_gen.fresh();
            graph.nodes.push(IRNode {
                id: node_id.clone(),
                kind: IRNodeKind::Decision,
                label: cond.text.clone(),
                source_span: None,
                source_text: None,
                group: None,
                style: None,
            });
            graph.edges.push(IREdge {
                from: entry_id.clone(),
                to: node_id.clone(),
                kind: IREdgeKind::Condition,
                guard: Some(cond.text.clone()),
                source_span: None,
                priority: None,
                style: None,
            });
            Some(node_id)
        };

        let mut prev_id = first_cond_id;
        for cond in conditions.iter().skip(1) {
            let node_id = id_gen.fresh();
            graph.nodes.push(IRNode {
                id: node_id.clone(),
                kind: IRNodeKind::Decision,
                label: cond.text.clone(),
                source_span: None,
                source_text: None,
                group: None,
                style: None,
            });
            let from = prev_id.clone().expect("first_cond_id guaranteed when conditions.len() >= 2");
            graph.edges.push(IREdge {
                from,
                to: node_id.clone(),
                kind: IREdgeKind::Condition,
                guard: Some(cond.text.clone()),
                source_span: None,
                priority: None,
                style: None,
            });
            prev_id = Some(node_id);
        }

        // Multiple Action: markers in a branch create parallel action nodes
        // (all connected from the same prev_id). This is intentional for
        // multi-action semantics.
        for child in &branch.children {
            if let Some(action_label) = child.text.strip_prefix("Action:") {
                let action_id = id_gen.fresh();
                graph.nodes.push(IRNode {
                    id: action_id.clone(),
                    kind: IRNodeKind::Action,
                    label: action_label.trim().to_string(),
                    source_span: None,
                    source_text: None,
                    group: None,
                    style: None,
                });
                let from = prev_id.clone().unwrap_or_else(|| entry_id.clone());
                graph.edges.push(IREdge {
                    from,
                    to: action_id,
                    kind: IREdgeKind::Control,
                    guard: None,
                    source_span: None,
                    priority: None,
                    style: None,
                });
            }
        }
    }

    (graph, diagnostics)
}

/// 缩进感知的列表树节点
#[derive(Debug, Clone)]
pub struct ListNode {
    pub text: String,
    pub depth: usize,
    pub line: usize,
    pub children: Vec<ListNode>,
}

/// 解析嵌套列表为缩进感知的树结构。
/// 每 4 空格或 1 tab = 1 级深度。
pub fn parse_list_to_tree(markdown: &str) -> Vec<ListNode> {
    let mut items: Vec<(usize, String, usize)> = vec![]; // (depth, text, line)
    for (line_no, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("* ") && !trimmed.starts_with("- ") {
            continue;
        }
        let leading = &line[..line.len() - trimmed.len()];
        let depth = compute_depth_from_str(leading);
        let text = trimmed
            .trim_start_matches("* ")
            .trim_start_matches("- ")
            .trim()
            .to_string();
        items.push((depth, text, line_no + 1)); // 1-based
    }
    let mut idx = 0;
    build_tree(&items, 0, &mut idx)
}

fn compute_depth_from_str(leading: &str) -> usize {
    let mut depth = 0;
    let mut spaces = 0;
    for c in leading.chars() {
        match c {
            '\t' => {
                depth += 1;
                spaces = 0;
            }
            ' ' => {
                spaces += 1;
                if spaces == 4 {
                    depth += 1;
                    spaces = 0;
                }
            }
            _ => break,
        }
    }
    depth
}

fn build_tree(items: &[(usize, String, usize)], target_depth: usize, idx: &mut usize) -> Vec<ListNode> {
    let mut nodes = vec![];
    while *idx < items.len() {
        let (depth, ref text, line) = items[*idx];
        if depth < target_depth {
            break;
        }
        if depth == target_depth {
            *idx += 1;
            let children = build_tree(items, target_depth + 1, idx);
            nodes.push(ListNode {
                text: text.clone(),
                depth: target_depth,
                line,
                children,
            });
        } else {
            // depth > target_depth：不应发生（由上层 build_tree 处理），跳过
            *idx += 1;
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_list_tracks_depth() {
        let md = "\
* Branch A
    * Cond 1
    * Cond 2
* Branch B
    * Cond 3
";
        let roots = parse_list_to_tree(md);
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].text, "Branch A");
        assert_eq!(roots[0].depth, 0);
        assert_eq!(roots[0].children.len(), 2);
        assert_eq!(roots[0].children[0].text, "Cond 1");
        assert_eq!(roots[0].children[0].depth, 1);
        assert_eq!(roots[1].text, "Branch B");
        assert_eq!(roots[1].children.len(), 1);
    }

    #[test]
    fn parse_list_ignores_non_list_lines() {
        let md = "\
Some intro text
* Item 1
More text
    * Item 2
";
        let roots = parse_list_to_tree(md);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].text, "Item 1");
        assert_eq!(roots[0].children.len(), 1);
    }

    #[test]
    fn parse_list_handles_tab_indent() {
        let md = "\
* Branch
\t* Tabbed child
";
        let roots = parse_list_to_tree(md);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].children.len(), 1);
        assert_eq!(roots[0].children[0].text, "Tabbed child");
    }

    #[test]
    fn parse_list_depth_jump_skips_node() {
        // 深度跳跃（depth 0 → depth 2，跳过 depth 1）时，跳跃的节点被跳过。
        // 这是设计行为：有效 DNF 输入不应出现深度跳跃。
        let md = "\
* A
        * B
";
        let roots = parse_list_to_tree(md);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].text, "A");
        assert_eq!(roots[0].children.len(), 0); // B 被跳过
    }

    #[test]
    fn parse_list_handles_dash_marker() {
        let md = "\
- Branch A
    - Cond 1
";
        let roots = parse_list_to_tree(md);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].text, "Branch A");
        assert_eq!(roots[0].children.len(), 1);
        assert_eq!(roots[0].children[0].text, "Cond 1");
    }

    #[test]
    fn lower_tree_dnf_basic() {
        let md = "\
* Branch A
    * Income: high
    * Credit: good
    * Action: approve
* Branch B
    * Income: low
    * Action: reject
";
        let mut id_gen = FreshNodeId::new();
        let (graph, diags) = lower_rule_tree(md, "test.md", &mut id_gen);

        // entry + [Income:high, Credit:good, Action:approve] + [Income:low, Action:reject]
        // = 1 + 3 + 2 = 6 nodes
        assert_eq!(graph.nodes.len(), 6);
        // edges: entry→Income:high, Income:high→Credit:good, Credit:good→Action:approve
        //        entry→Income:low, Income:low→Action:reject = 5 edges
        assert_eq!(graph.edges.len(), 5);
        assert!(diags.is_empty());
    }

    #[test]
    fn lower_tree_no_action_warns() {
        let md = "\
* Branch A
    * Income: high
";
        let mut id_gen = FreshNodeId::new();
        let (_graph, diags) = lower_rule_tree(md, "test.md", &mut id_gen);
        assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_NO_ACTION"));
    }

    #[test]
    fn lower_tree_empty_branch_warns() {
        let md = "* Branch A\n";
        let mut id_gen = FreshNodeId::new();
        let (_graph, diags) = lower_rule_tree(md, "test.md", &mut id_gen);
        assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_EMPTY_BRANCH"));
    }

    #[test]
    fn lower_tree_action_node_kind() {
        let md = "\
* Branch A
    * Action: approve
";
        let mut id_gen = FreshNodeId::new();
        let (graph, _diags) = lower_rule_tree(md, "test.md", &mut id_gen);
        let action_node = graph.nodes.iter().find(|n| n.label == "approve").unwrap();
        assert_eq!(action_node.kind, IRNodeKind::Action);
    }
}
