use crate::model::SourceSpan;
use pulldown_cmark::{Event, Parser, Tag, TagEnd, HeadingLevel};

/// 简化的 Markdown 节点（对应 TS MarkdownNode）
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownNode {
    pub node_type: String,
    pub children: Vec<MarkdownNode>,
    pub value: Option<String>,
    pub depth: Option<usize>,
    pub lang: Option<String>,
    pub url: Option<String>,
    pub checked: Option<bool>,
    pub position: Option<MarkdownPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownPosition {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// 解析 Markdown 源文本为简化节点树
pub fn parse_markdown(source: &str, _file: &str) -> Vec<MarkdownNode> {
    let parser = Parser::new(source);
    let mut root_children: Vec<MarkdownNode> = Vec::new();
    let mut stack: Vec<MarkdownNode> = Vec::new();
    let mut current_text = String::new();
    let mut current_position: Option<MarkdownPosition> = None;

    for (event, range) in parser.into_offset_iter() {
        let start = range.start;
        let end = range.end;

        match event {
            Event::Start(tag) => {
                flush_text(&mut current_text, &mut current_position, &mut stack);

                let pos = offset_to_position(source, start, end);
                match tag {
                    Tag::Heading { level, .. } => {
                        let depth = match level {
                            HeadingLevel::H1 => 1, HeadingLevel::H2 => 2,
                            HeadingLevel::H3 => 3, HeadingLevel::H4 => 4,
                            HeadingLevel::H5 => 5, HeadingLevel::H6 => 6,
                        };
                        stack.push(MarkdownNode {
                            node_type: "heading".into(), children: vec![], value: None,
                            depth: Some(depth), lang: None, url: None, checked: None,
                            position: Some(pos),
                        });
                    }
                    Tag::CodeBlock(kind) => {
                        let lang = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(l) => {
                                if l.is_empty() { None } else { Some(l.to_string()) }
                            }
                            pulldown_cmark::CodeBlockKind::Indented => None,
                        };
                        stack.push(MarkdownNode {
                            node_type: "code".into(), children: vec![], value: None,
                            depth: None, lang, url: None, checked: None, position: Some(pos),
                        });
                    }
                    Tag::List(..) => {
                        stack.push(MarkdownNode {
                            node_type: "list".into(), children: vec![], value: None,
                            depth: None, lang: None, url: None, checked: None, position: Some(pos),
                        });
                    }
                    Tag::Item => {
                        stack.push(MarkdownNode {
                            node_type: "listItem".into(), children: vec![], value: None,
                            depth: None, lang: None, url: None, checked: None, position: Some(pos),
                        });
                    }
                    Tag::Link { dest_url, .. } => {
                        stack.push(MarkdownNode {
                            node_type: "link".into(), children: vec![], value: None,
                            depth: None, lang: None, url: Some(dest_url.to_string()),
                            checked: None, position: Some(pos),
                        });
                    }
                    Tag::Paragraph | Tag::BlockQuote(_) => {
                        let nt = if matches!(tag, Tag::Paragraph) { "paragraph" } else { "blockquote" };
                        stack.push(MarkdownNode {
                            node_type: nt.into(), children: vec![], value: None,
                            depth: None, lang: None, url: None, checked: None, position: Some(pos),
                        });
                    }
                    Tag::Table(..) => {
                        stack.push(MarkdownNode {
                            node_type: "table".into(), children: vec![], value: None,
                            depth: None, lang: None, url: None, checked: None, position: Some(pos),
                        });
                    }
                    Tag::TableHead => {
                        stack.push(MarkdownNode {
                            node_type: "tableHead".into(), children: vec![], value: None,
                            depth: None, lang: None, url: None, checked: None, position: Some(pos),
                        });
                    }
                    Tag::TableRow => {
                        stack.push(MarkdownNode {
                            node_type: "tableRow".into(), children: vec![], value: None,
                            depth: None, lang: None, url: None, checked: None, position: Some(pos),
                        });
                    }
                    Tag::TableCell => {
                        stack.push(MarkdownNode {
                            node_type: "tableCell".into(), children: vec![], value: None,
                            depth: None, lang: None, url: None, checked: None, position: Some(pos),
                        });
                    }
                    _ => {}
                }
            }
            Event::End(tag_end) => {
                flush_text(&mut current_text, &mut current_position, &mut stack);
                match tag_end {
                    TagEnd::Heading(..) | TagEnd::CodeBlock | TagEnd::List(..)
                    | TagEnd::Item | TagEnd::Link | TagEnd::Paragraph
                    | TagEnd::BlockQuote | TagEnd::Table | TagEnd::TableHead | TagEnd::TableRow | TagEnd::TableCell => {
                        if let Some(completed) = stack.pop() {
                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(completed);
                            } else {
                                root_children.push(completed);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                if current_position.is_none() {
                    current_position = Some(offset_to_position(source, start, end));
                }
                current_text.push_str(&text);
            }
            Event::Code(text) => {
                if current_position.is_none() {
                    current_position = Some(offset_to_position(source, start, end));
                }
                // Preserve inline-code markup so consumers (e.g. param parsing)
                // can distinguish `name` from prose text.
                current_text.push('`');
                current_text.push_str(&text);
                current_text.push('`');
            }
            Event::InlineHtml(html) | Event::Html(html) => { current_text.push_str(&html); }
            Event::SoftBreak | Event::HardBreak => { current_text.push(' '); }
            Event::TaskListMarker(checked) => {
                if let Some(parent) = stack.last_mut() { parent.checked = Some(checked); }
            }
            _ => {}
        }
    }

    flush_text(&mut current_text, &mut current_position, &mut stack);
    root_children
}

fn flush_text(text: &mut String, position: &mut Option<MarkdownPosition>, stack: &mut [MarkdownNode]) {
    if text.trim().is_empty() { text.clear(); *position = None; return; }
    let text_node = MarkdownNode {
        node_type: "text".into(), children: vec![],
        value: Some(std::mem::take(text)), depth: None,
        lang: None, url: None, checked: None, position: *position,
    };
    if let Some(parent) = stack.last_mut() { parent.children.push(text_node); }
    *position = None;
}

fn offset_to_position(source: &str, start: usize, end: usize) -> MarkdownPosition {
    let start_line = source[..start].lines().count().max(1);
    let end_line = source[..end].lines().count().max(1);
    let start_col = source[..start].lines().last().map(|l| l.len()).unwrap_or(0) + 1;
    let end_col = source[..end].lines().last().map(|l| l.len()).unwrap_or(0) + 1;
    MarkdownPosition { start_line, start_column: start_col, end_line, end_column: end_col }
}

impl MarkdownNode {
    pub fn to_span(&self, file: &str) -> Option<SourceSpan> {
        self.position.map(|p| SourceSpan {
            file: file.to_string(),
            start_line: p.start_line,
            start_column: p.start_column,
            end_line: p.end_line,
            end_column: p.end_column,
        })
    }
}
