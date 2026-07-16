use crate::ir::graph::*;
use crate::model::{SourceSpan, TangleDiagnostic};

/// Lower a single `@rule.toggle` block to IR.
///
/// # 跨块语义
///
/// 每次调用独立处理单个 toggle 块。跨 `@rule.toggle` 块的 group/style
/// 不继承——前一块的 pending_group/pending_style 不会流入下一块。
/// 如需为多个块设置统一 group，必须在每个块内显式声明。
///
/// # 单块内语义
///
/// `pending_group`/`pending_style` 缓存遇 `<!-- group: X -->` 行设置，
/// 遇 checkbox 行消费并清空，遇非注释非 checkbox 行清空。
pub fn lower_rule_toggle(checkbox_markdown: &str, file: &str, id_gen: &mut FreshNodeId) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
    let entry_id = id_gen.fresh();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(),
        kind: IRNodeKind::Compute,
        label: "toggle.entry".into(),
        source_span: None, source_text: None,
        group: None, style: None,
    });

    let mut pending_group: Option<String> = None;
    let mut pending_style: Option<String> = None;
    let mut toggle_index = 0u32;

    for (line_idx, line) in checkbox_markdown.lines().enumerate() {
        let line_no = line_idx + 1; // 1-based
        let t = line.trim_start();

        // Check for HTML comment metadata: <!-- group: X --> or <!-- style: Y -->
        if let Some(meta) = parse_html_comment(t) {
            match meta {
                ("group", value) => pending_group = Some(value),
                ("style", value) => pending_style = Some(value),
                _ => {}
            }
            continue;
        }

        // Skip non-checkbox lines (but clear pending metadata)
        if !t.starts_with("- [") && !t.starts_with("* [") {
            if !t.is_empty() && !t.starts_with("<!--") {
                pending_group = None;
                pending_style = None;
            }
            continue;
        }

        // Detect malformed checkbox: starts with - [ or * [ but doesn't contain [x]/[X]/[ ]
        let is_valid = t.contains("[x]") || t.contains("[X]") || t.contains("[ ]");
        if !is_valid {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_RULE_TOGGLE_MALFORMED".into(),
                message: format!("malformed checkbox: expected [x], [X], or [ ]: {}", t),
                span: SourceSpan {
                    file: file.into(),
                    start_line: line_no, start_column: 0,
                    end_line: line_no, end_column: 0,
                },
            });
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

        // Extract name: backtick first, then colon, then fallback
        let name = extract_name(rest);
        let name = match name {
            Some(n) => n,
            None => {
                diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_RULE_TOGGLE_MISSING_NAME".into(),
                    message: format!("could not extract toggle name from: {}", rest),
                    span: SourceSpan {
                        file: file.into(),
                        start_line: line_no, start_column: 0,
                        end_line: line_no, end_column: 0,
                    },
                });
                format!("toggle_{}", toggle_index)
            }
        };

        let node_id = id_gen.fresh();
        graph.nodes.push(IRNode {
            id: node_id.clone(),
            kind: IRNodeKind::Compute,
            label: format!("{} = {}", name, checked),
            source_span: Some(SourceSpan {
                file: file.into(),
                start_line: line_no, start_column: 0,
                end_line: line_no, end_column: 0,
            }),
            source_text: None,
            group: pending_group.take(),
            style: pending_style.take(),
        });
        graph.edges.push(IREdge {
            from: entry_id.clone(),
            to: node_id,
            kind: IREdgeKind::Control,
            guard: None,
            source_span: None,
            priority: None, style: None,
        });
        toggle_index += 1;
    }

    (graph, diagnostics)
}

/// Extract toggle name from the rest of a checkbox line.
/// Priority: backtick (`name`) > colon (name: value) > None.
fn extract_name(rest: &str) -> Option<String> {
    // 1. Backtick: `name`: desc
    if let Some(tick_start) = rest.find('`') {
        let after_tick = &rest[tick_start + 1..];
        if let Some(tick_end) = after_tick.find('`') {
            return Some(after_tick[..tick_end].to_string());
        }
    }
    // 2. Colon: name: value (name must be a valid identifier)
    if let Some(colon_pos) = rest.find(':') {
        let candidate = rest[..colon_pos].trim();
        if is_valid_identifier(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Check if a string is a valid identifier: [a-zA-Z_][a-zA-Z0-9_]*
fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse an HTML comment line like `<!-- group: X -->` or `<!-- style: Y -->`.
/// Returns (key, value) if the comment matches the metadata pattern.
fn parse_html_comment(line: &str) -> Option<(&'static str, String)> {
    let trimmed = line.trim();
    // 需要至少 "<!--x-->" (8 字符) 才能安全 slicing
    if trimmed.len() < 8 {
        return None;
    }
    if !trimmed.starts_with("<!--") || !trimmed.ends_with("-->") {
        return None;
    }
    let inner = trimmed[4..trimmed.len() - 3].trim();
    if let Some(rest) = inner.strip_prefix("group:") {
        return Some(("group", rest.trim().to_string()));
    }
    if let Some(rest) = inner.strip_prefix("style:") {
        return Some(("style", rest.trim().to_string()));
    }
    None
}
