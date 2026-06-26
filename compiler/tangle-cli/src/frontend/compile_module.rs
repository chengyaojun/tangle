use crate::markdown::parse_markdown;
use crate::model::{
    HeadingRole, RuleData, RuleKind, SourceSpan, SymbolKind, TangleCodeBlock,
    TangleDiagnostic, TangleHeading, TangleModule, TangleParam, TangleSymbol,
};
use crate::frontend::headings::{heading_role_for_depth, parse_heading_text};
use crate::frontend::blocks::{collect_links, is_tangle_code_block, parse_param_item};
use crate::frontend::source_map::span_from_node;

pub struct CompileModuleInput {
    pub file: String,
    pub source: String,
}

/// 主编译入口：Markdown 源文本 → TangleModule
pub fn compile_module(input: CompileModuleInput) -> TangleModule {
    let mut diagnostics: Vec<TangleDiagnostic> = vec![];
    let nodes = parse_markdown(&input.source, &input.file);

    let imports = collect_links(&input.file, &nodes);
    let flat_headings = extract_headings(&nodes, &input.file, &input.source, &mut diagnostics);
    let headings = build_heading_tree(flat_headings);
    let symbols = build_symbols(&headings);
    validate_symbol_rules(&symbols, &mut diagnostics);

    let module_name = module_name_from_file(&input.file);

    TangleModule {
        file: input.file,
        module_name,
        imports,
        headings,
        symbols,
        diagnostics,
    }
}

fn is_rule_heading(title: &str) -> bool {
    title.starts_with("Rule:") || title.starts_with("rule:")
}

fn determine_rule_kind(source: &str) -> Option<RuleKind> {
    // Priority order matters:
    // 1. Mermaid fenced code block -> Flow
    if source.contains("```mermaid") || source.contains("graph TD") || source.contains("graph LR") {
        return Some(RuleKind::Flow);
    }
    // 2. Pipe table -> Table (at least 2 lines with |)
    let pipe_lines: Vec<_> = source.lines().filter(|l| l.contains('|')).collect();
    if pipe_lines.len() >= 2 {
        return Some(RuleKind::Table);
    }
    // 3. Checkbox items -> Toggle
    if source.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("- [") || t.starts_with("* [")
    }) {
        return Some(RuleKind::Toggle);
    }
    // 4. Bullet list items -> Tree
    if source.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("* ") || t.starts_with("- ")
    }) {
        return Some(RuleKind::Tree);
    }
    None
}

fn extract_headings(
    nodes: &[crate::markdown::MarkdownNode],
    file: &str,
    source: &str,
    diagnostics: &mut Vec<TangleDiagnostic>,
) -> Vec<TangleHeading> {
    let mut headings: Vec<TangleHeading> = vec![];
    let mut current_heading: Option<TangleHeading> = None;
    let mut pending_params: Vec<TangleParam> = vec![];
    let mut pending_code_blocks: Vec<TangleCodeBlock> = vec![];
    let mut pending_is_rule: bool = false;
    let mut pending_rule_line_start: usize = 0;
    let mut pending_rule_line_end: usize = 0;

    for node in nodes {
        if node.node_type == "heading" {
            if let Some(mut h) = current_heading.take() {
                h.params = std::mem::take(&mut pending_params);
                h.code_blocks = std::mem::take(&mut pending_code_blocks);
                // Extract rule data if this is a rule heading
                if pending_is_rule && pending_rule_line_start > 0 {
                    let rule_source = source.lines()
                        .skip(pending_rule_line_start.saturating_sub(1))
                        .take(pending_rule_line_end.saturating_sub(pending_rule_line_start) + 1)
                        .collect::<Vec<_>>()
                        .join("\n");
                    if let Some(kind) = determine_rule_kind(&rule_source) {
                        h.rule = Some(RuleData {
                            kind,
                            source: rule_source,
                            span: h.span.clone(),
                        });
                    }
                    pending_is_rule = false;
                    pending_rule_line_start = 0;
                    pending_rule_line_end = 0;
                }
                headings.push(h);
            }

            let depth = node.depth.unwrap_or(1);
            let text = node.children.iter()
                .find(|c| c.node_type == "text")
                .and_then(|c| c.value.clone())
                .unwrap_or_default();

            let role = heading_role_for_depth(depth);
            let span = span_from_node(file, node).unwrap_or_else(|| SourceSpan {
                file: file.to_string(), start_line: 1, start_column: 1,
                end_line: 1, end_column: 1,
            });

            let parsed = parse_heading_text(&text, depth, &span);
            diagnostics.extend(parsed.diagnostics);

            let id = stable_heading_id(&parsed.title);
            let is_rule = is_rule_heading(&parsed.title);

            current_heading = Some(TangleHeading {
                id, depth, role, title: parsed.title,
                symbol_name: parsed.symbol_name, directives: vec![],
                params: vec![], code_blocks: vec![], rule: None, span, children: vec![],
            });

            if is_rule {
                pending_is_rule = true;
            }
        } else if current_heading.is_some() {
            // Track rule body span
            if pending_is_rule {
                if let Some(ref pos) = node.position {
                    if pending_rule_line_start == 0 {
                        pending_rule_line_start = pos.start_line;
                    }
                    pending_rule_line_end = pos.end_line;
                }
            }

            if node.node_type == "list" {
                for item in &node.children {
                    let text = plain_text_recursive(item);
                    if let Some(span) = span_from_node(file, item) {
                        if let Some(param) = parse_param_item(&text, &span) {
                            pending_params.push(param);
                        }
                    }
                }
            } else if is_tangle_code_block(node) {
                let value = node.children.iter()
                    .find(|c| c.node_type == "text")
                    .and_then(|c| c.value.clone())
                    .unwrap_or_default();
                if let Some(span) = span_from_node(file, node) {
                    pending_code_blocks.push(TangleCodeBlock {
                        language: "@tangle".into(), value, span,
                    });
                }
            }
        }
    }

    if let Some(mut h) = current_heading.take() {
        h.params = std::mem::take(&mut pending_params);
        h.code_blocks = std::mem::take(&mut pending_code_blocks);
        // Extract rule data if this is a rule heading
        if pending_is_rule && pending_rule_line_start > 0 {
            let rule_source = source.lines()
                .skip(pending_rule_line_start.saturating_sub(1))
                .take(pending_rule_line_end.saturating_sub(pending_rule_line_start) + 1)
                .collect::<Vec<_>>()
                .join("\n");
            if let Some(kind) = determine_rule_kind(&rule_source) {
                h.rule = Some(RuleData {
                    kind,
                    source: rule_source,
                    span: h.span.clone(),
                });
            }
        }
        headings.push(h);
    }

    headings
}

fn plain_text_recursive(node: &crate::markdown::MarkdownNode) -> String {
    if node.node_type == "text" {
        return node.value.clone().unwrap_or_default();
    }
    if node.node_type == "inlineCode" {
        if let Some(ref v) = node.value {
            return format!("`{}`", v);
        }
    }
    node.children.iter().map(plain_text_recursive).collect::<Vec<_>>().join(" ")
}

fn build_heading_tree(flat: Vec<TangleHeading>) -> Vec<TangleHeading> {
    let mut root: Vec<TangleHeading> = vec![];
    let mut stack: Vec<TangleHeading> = vec![];

    for heading in flat {
        while let Some(top) = stack.last() {
            if top.depth < heading.depth { break; }
            let completed = stack.pop().unwrap();
            if let Some(parent) = stack.last_mut() {
                parent.children.push(completed);
            } else {
                root.push(completed);
            }
        }
        stack.push(heading);
    }

    while let Some(completed) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(completed);
        } else {
            root.push(completed);
        }
    }

    root
}

fn build_symbols(headings: &[TangleHeading]) -> Vec<TangleSymbol> {
    let mut symbols = vec![];
    build_symbols_recursive(headings, &mut symbols);
    symbols
}

fn build_symbols_recursive(headings: &[TangleHeading], out: &mut Vec<TangleSymbol>) {
    for h in headings {
        if let Some(ref name) = h.symbol_name {
            let exported = !name.starts_with('_');
            let kind = match h.role {
                HeadingRole::Program | HeadingRole::Section => SymbolKind::SemanticInternal,
                HeadingRole::Type => SymbolKind::Type,
                HeadingRole::Callable => {
                    if name == "main" && h.depth == 4 { SymbolKind::Entry }
                    else { SymbolKind::Callable }
                }
                HeadingRole::SemanticSection | HeadingRole::SemanticAtom => SymbolKind::SemanticInternal,
            };
            out.push(TangleSymbol { name: name.clone(), kind, exported, heading_id: h.id.clone(), span: h.span.clone() });
        }
        build_symbols_recursive(&h.children, out);
    }
}

fn validate_symbol_rules(symbols: &[TangleSymbol], diagnostics: &mut Vec<TangleDiagnostic>) {
    let entry_count = symbols.iter().filter(|s| s.kind == SymbolKind::Entry).count();
    if entry_count > 1 {
        for s in symbols.iter().filter(|s| s.kind == SymbolKind::Entry) {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_DUPLICATE_ENTRY".into(),
                message: "Multiple 'main' entry points found; only one allowed per module".into(),
                span: s.span.clone(),
            });
        }
    }
}

fn module_name_from_file(file: &str) -> String {
    std::path::Path::new(file)
        .file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string()
}

fn stable_heading_id(title: &str) -> String {
    title.to_lowercase().chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-').filter(|s| !s.is_empty())
        .collect::<Vec<_>>().join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_rule_heading() {
        assert!(is_rule_heading("Rule: Approval"));
        assert!(is_rule_heading("rule: ScoreCard"));
        assert!(is_rule_heading("Rule:CreditCheck"));
        assert!(!is_rule_heading("ApprovalTest"));
        assert!(!is_rule_heading("main"));
    }

    #[test]
    fn test_determine_rule_kind_flow() {
        let source = "```mermaid\ngraph TD\n    A --> B\n```";
        assert_eq!(determine_rule_kind(source), Some(RuleKind::Flow));
    }

    #[test]
    fn test_determine_rule_kind_table() {
        let source = "| A | B |\n| 1 | 2 |\n| 3 | 4 |";
        assert_eq!(determine_rule_kind(source), Some(RuleKind::Table));
    }

    #[test]
    fn test_determine_rule_kind_toggle() {
        let source = "- [x] enable_feature\n- [ ] disable_feature";
        assert_eq!(determine_rule_kind(source), Some(RuleKind::Toggle));
    }

    #[test]
    fn test_determine_rule_kind_tree() {
        let source = "* condition 1\n* condition 2";
        assert_eq!(determine_rule_kind(source), Some(RuleKind::Tree));
    }

    #[test]
    fn test_rule_heading_detection_flow() {
        let source = "# Test\n\n##### Rule: Approval\n\n```mermaid\ngraph TD\n    A --> B\n```\n";
        let module = compile_module(CompileModuleInput {
            file: "test.md".into(),
            source: source.into(),
        });

        // Find the rule heading (depth 5)
        let rule_heading: Vec<_> = module.headings.iter()
            .flat_map(|h| h.children.iter())
            .filter(|h| h.depth == 5 && is_rule_heading(&h.title))
            .collect();

        assert!(!rule_heading.is_empty(), "Should find Rule: heading");
        let heading = rule_heading[0];
        assert!(heading.rule.is_some(), "Rule heading should have rule data");
        let rule = heading.rule.as_ref().unwrap();
        assert_eq!(rule.kind, RuleKind::Flow);
        assert!(rule.source.contains("graph TD"));
    }

    #[test]
    fn test_rule_heading_detection_table() {
        let source = "# Test\n\n##### Rule: ScoreCard\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let module = compile_module(CompileModuleInput {
            file: "test.md".into(),
            source: source.into(),
        });

        let rule_heading: Vec<_> = module.headings.iter()
            .flat_map(|h| h.children.iter())
            .filter(|h| h.depth == 5 && is_rule_heading(&h.title))
            .collect();

        assert!(!rule_heading.is_empty());
        let rule = rule_heading[0].rule.as_ref().unwrap();
        assert_eq!(rule.kind, RuleKind::Table);
    }

    #[test]
    fn test_real_approval_flow_file() {
        let source = std::fs::read_to_string("../../tests/rules/approval-flow.tangle.md")
            .expect("Failed to read test file");
        let module = compile_module(CompileModuleInput {
            file: "approval-flow.tangle.md".into(),
            source,
        });

        // Find rule heading recursively
        fn find_rule(headings: &[TangleHeading]) -> Option<&TangleHeading> {
            for h in headings {
                if h.rule.is_some() { return Some(h); }
                if let Some(found) = find_rule(&h.children) { return Some(found); }
            }
            None
        }

        let rule_heading = find_rule(&module.headings);
        assert!(rule_heading.is_some(), "Should find rule heading in approval-flow.tangle.md");
        let rule = rule_heading.unwrap().rule.as_ref().unwrap();
        assert_eq!(rule.kind, RuleKind::Flow, "Expected Flow rule kind");
        assert!(rule.source.contains("graph TD"), "Rule source should contain graph TD");
    }

    #[test]
    fn test_full_pipeline_to_ir() {
        let source = std::fs::read_to_string("../../tests/rules/approval-flow.tangle.md")
            .expect("Failed to read test file");

        let module = compile_module(CompileModuleInput {
            file: "approval-flow.tangle.md".into(),
            source: source.clone(),
        });

        fn find_first_rule(headings: &[TangleHeading]) -> Option<&crate::model::RuleData> {
            for h in headings {
                if let Some(ref r) = h.rule { return Some(r); }
                if let Some(r) = find_first_rule(&h.children) { return Some(r); }
            }
            None
        }
        let rule_data = find_first_rule(&module.headings).expect("Should find rule");
        assert_eq!(rule_data.kind, RuleKind::Flow);

        // Debug: print the rule source
        eprintln!("=== Rule source ({} chars) ===", rule_data.source.len());
        for (i, line) in rule_data.source.lines().enumerate() {
            eprintln!("  L{}: {:?}", i+1, line);
        }

        use crate::ir::graph::FreshNodeId;
        use crate::ir::lower_rule_flow::lower_rule_flow;
        let mut id_gen = FreshNodeId::new();
        let graph = lower_rule_flow(&rule_data.source, "test.md", &mut id_gen);
        eprintln!("=== IR nodes ===");
        for n in &graph.nodes {
            eprintln!("  {}: {:?} \"{}\"", n.id, n.kind, n.label);
        }
        assert!(graph.nodes.len() >= 2, "Expected >=2 nodes, got {}: {:?}",
            graph.nodes.len(), graph.nodes.iter().map(|n| &n.label).collect::<Vec<_>>());
    }

    #[test]
    fn test_pipeline_table_to_ir() {
        let source = std::fs::read_to_string("../../tests/rules/decision-table.tangle.md")
            .expect("Failed to read test file");

        let module = compile_module(CompileModuleInput {
            file: "decision-table.tangle.md".into(),
            source,
        });

        use crate::checker::check_module::check_module;
        let checked = check_module(module);

        use crate::ir::compile_to_ir::compile_to_ir;
        let (graph, _diagnostics) = compile_to_ir(&checked);

        assert!(
            graph.nodes.len() >= 2,
            "Expected >=2 nodes in decision-table IR, got {}: {:?}",
            graph.nodes.len(),
            graph.nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_pipeline_tree_to_ir() {
        let source = std::fs::read_to_string("../../tests/rules/decision-tree.tangle.md")
            .expect("Failed to read test file");

        let module = compile_module(CompileModuleInput {
            file: "decision-tree.tangle.md".into(),
            source,
        });

        use crate::checker::check_module::check_module;
        let checked = check_module(module);

        use crate::ir::compile_to_ir::compile_to_ir;
        let (graph, _diagnostics) = compile_to_ir(&checked);

        assert!(
            graph.nodes.len() >= 2,
            "Expected >=2 nodes in decision-tree IR, got {}: {:?}",
            graph.nodes.len(),
            graph.nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_pipeline_toggle_to_ir() {
        let source = std::fs::read_to_string("../../tests/rules/feature-toggles.tangle.md")
            .expect("Failed to read test file");

        let module = compile_module(CompileModuleInput {
            file: "feature-toggles.tangle.md".into(),
            source,
        });

        use crate::checker::check_module::check_module;
        let checked = check_module(module);

        use crate::ir::compile_to_ir::compile_to_ir;
        let (graph, _diagnostics) = compile_to_ir(&checked);

        assert!(
            graph.nodes.len() >= 2,
            "Expected >=2 nodes in feature-toggles IR, got {}: {:?}",
            graph.nodes.len(),
            graph.nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
        );
    }
}
