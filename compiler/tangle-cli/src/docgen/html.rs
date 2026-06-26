use crate::model::{SymbolKind, TangleModule};

/// Generate documentation HTML from a compiled Tangle module
pub fn generate_doc_html(module: &TangleModule, _source: &str) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str(&format!(
        "<title>{} — Tangle Docs</title>\n",
        module.module_name
    ));
    html.push_str("<style>\n");
    html.push_str(
        "body { font-family: system-ui, sans-serif; max-width: 900px; margin: 0 auto; padding: 2rem; }\n",
    );
    html.push_str("h1 { border-bottom: 2px solid #333; }\n");
    html.push_str("h2 { border-bottom: 1px solid #999; }\n");
    html.push_str("h3 { color: #444; }\n");
    html.push_str("h4 { color: #666; }\n");
    html.push_str(
        ".struct { background: #f5f5f5; padding: 1rem; border-radius: 4px; margin: 1rem 0; }\n",
    );
    html.push_str(".field { font-family: monospace; margin: 0.25rem 0; }\n");
    html.push_str(".method { margin: 0.5rem 0 0.5rem 1rem; }\n");
    html.push_str(".deprecated { text-decoration: line-through; color: #999; }\n");
    html.push_str("code { background: #eee; padding: 0.2em 0.4em; border-radius: 2px; }\n");
    html.push_str("</style>\n</head>\n<body>\n");

    // Module header
    html.push_str(&format!("<h1>{}</h1>\n", module.module_name));
    html.push_str(&format!("<p><code>{}</code></p>\n", module.file));

    // Symbol index
    html.push_str("<h2>Symbols</h2>\n<ul>\n");
    for sym in &module.symbols {
        let export_mark = if sym.exported { "" } else { " 🔒" };
        let deprecated = "";
        html.push_str(&format!(
            "<li{}>{}{} <code>{}</code></li>\n",
            deprecated,
            sym.name,
            export_mark,
            kind_to_str(sym.kind)
        ));
    }
    html.push_str("</ul>\n");

    // Headings
    for heading in &module.headings {
        render_heading_html(&mut html, heading, 1);
    }

    // Imports
    if !module.imports.is_empty() {
        html.push_str("<h2>Imports</h2>\n<ul>\n");
        for imp in &module.imports {
            html.push_str(&format!(
                "<li><code>{}</code> → {}</li>\n",
                imp.alias, imp.target
            ));
        }
        html.push_str("</ul>\n");
    }

    html.push_str("</body>\n</html>\n");
    html
}

fn kind_to_str(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Entry => "entry",
        SymbolKind::Type => "type",
        SymbolKind::Callable => "callable",
        SymbolKind::SemanticInternal => "internal",
    }
}

fn render_heading_html(html: &mut String, heading: &crate::model::TangleHeading, _depth: usize) {
    let tag = match heading.depth {
        1 => "h1",
        2 => "h2",
        3 => "h3",
        4 => "h4",
        5 => "h5",
        _ => "h6",
    };

    let deprecated_class = if heading.title.contains("~~") {
        " class=\"deprecated\""
    } else {
        ""
    };
    let clean_title = heading.title.replace("~~", "");

    html.push_str(&format!(
        "<{} {}>{}</{}>\n",
        tag, deprecated_class, clean_title, tag
    ));

    // Params/fields
    if !heading.params.is_empty() {
        html.push_str("<div class=\"struct\">\n");
        for param in &heading.params {
            let ty = param.type_name.as_deref().unwrap_or("_");
            html.push_str(&format!(
                "<div class=\"field\"><code>{}: {}</code> — {}</div>\n",
                param.name, ty, param.description
            ));
        }
        html.push_str("</div>\n");
    }

    // Children
    for child in &heading.children {
        render_heading_html(html, child, heading.depth + 1);
    }
}
