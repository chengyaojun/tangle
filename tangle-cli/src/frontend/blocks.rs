use crate::markdown::MarkdownNode;
use crate::model::{SourceSpan, TangleImport, TangleParam};

pub fn collect_links(file: &str, nodes: &[MarkdownNode]) -> Vec<TangleImport> {
    let mut imports = vec![];
    collect_links_recursive(file, nodes, &mut imports);
    imports
}

fn collect_links_recursive(file: &str, nodes: &[MarkdownNode], out: &mut Vec<TangleImport>) {
    for node in nodes {
        if node.node_type == "link" {
            if let Some(ref url) = node.url {
                if url.ends_with(".md") {
                    let alias = node.children.iter()
                        .find(|c| c.node_type == "text")
                        .and_then(|c| c.value.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    if let Some(span) = node.to_span(file) {
                        out.push(TangleImport { alias, target: url.clone(), span });
                    }
                }
            }
        }
        collect_links_recursive(file, &node.children, out);
    }
}

pub fn parse_param_item(text: &str, span: &SourceSpan) -> Option<TangleParam> {
    let text = text.trim();
    if !text.starts_with('`') { return None; }
    let end_tick = text[1..].find('`')?;
    let name = &text[1..=end_tick];
    let after_name = text[end_tick + 2..].trim();
    let after_name = after_name.strip_prefix(':')?.trim();
    let (description, type_name) = if let Some(open) = after_name.rfind('(') {
        if let Some(close) = after_name.rfind(')') {
            if close > open {
                (after_name[..open].trim().to_string(), Some(after_name[open + 1..close].to_string()))
            } else { (after_name.to_string(), None) }
        } else { (after_name.to_string(), None) }
    } else { (after_name.to_string(), None) };
    Some(TangleParam { name: name.to_string(), description, type_name, span: span.clone() })
}

pub fn is_tangle_code_block(node: &MarkdownNode) -> bool {
    node.node_type == "code" && node.lang.as_deref() == Some("@tangle")
}

pub fn plain_text(node: &MarkdownNode) -> String {
    if node.node_type == "text" { return node.value.clone().unwrap_or_default(); }
    node.children.iter().map(plain_text).collect::<Vec<_>>().join("")
}
