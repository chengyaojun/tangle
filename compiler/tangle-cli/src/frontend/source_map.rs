use crate::markdown::MarkdownNode;
use crate::model::SourceSpan;

pub fn span_from_node(file: &str, node: &MarkdownNode) -> Option<SourceSpan> {
    node.to_span(file)
}
