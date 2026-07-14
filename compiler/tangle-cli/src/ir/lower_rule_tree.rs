use crate::ir::graph::*;

pub fn lower_rule_tree(list_markdown: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let entry_id = id_gen.fresh();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(),
        kind: IRNodeKind::Decision,
        label: "tree.entry".into(),
        source_span: None, source_text: None,
        group: None, style: None,
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
        let node_id = id_gen.fresh();
        graph.nodes.push(IRNode {
            id: node_id.clone(),
            kind: IRNodeKind::Decision,
            label: item.clone(),
            source_span: None, source_text: None,
            group: None, style: None,
        });
        graph.edges.push(IREdge {
            from: prev_id,
            to: node_id.clone(),
            kind: IREdgeKind::Condition,
            guard: Some(item),
            source_span: None,
            priority: None, style: None,
        });
        prev_id = node_id;
    }

    graph
}

/// 缩进感知的列表树节点
#[derive(Debug, Clone)]
pub struct ListNode {
    pub text: String,
    pub depth: usize,
    pub children: Vec<ListNode>,
}

/// 解析嵌套列表为缩进感知的树结构。
/// 每 4 空格或 1 tab = 1 级深度。
pub fn parse_list_to_tree(markdown: &str) -> Vec<ListNode> {
    let mut items: Vec<(usize, String)> = vec![];
    for line in markdown.lines() {
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
        items.push((depth, text));
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

fn build_tree(items: &[(usize, String)], target_depth: usize, idx: &mut usize) -> Vec<ListNode> {
    let mut nodes = vec![];
    while *idx < items.len() {
        let (depth, ref text) = items[*idx];
        if depth < target_depth {
            break;
        }
        if depth == target_depth {
            *idx += 1;
            let children = build_tree(items, target_depth + 1, idx);
            nodes.push(ListNode {
                text: text.clone(),
                depth: target_depth,
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
}
