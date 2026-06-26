use crate::model::{SourceSpan, TangleHeading};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorVariant {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ErrorRegistry {
    variants: HashMap<String, ErrorVariant>,
}

impl ErrorRegistry {
    pub fn new() -> Self {
        ErrorRegistry {
            variants: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        name: &str,
        fields: Vec<(String, String)>,
        span: Option<SourceSpan>,
    ) {
        self.variants.insert(
            name.to_string(),
            ErrorVariant {
                name: name.to_string(),
                fields,
                span,
            },
        );
    }

    pub fn lookup(&self, name: &str) -> Option<&ErrorVariant> {
        self.variants.get(name)
    }

    pub fn is_error(&self, name: &str) -> bool {
        self.variants.contains_key(name)
    }

    pub fn all_variants(&self) -> Vec<&ErrorVariant> {
        self.variants.values().collect()
    }

    pub fn collect_from_headings(&mut self, headings: &[TangleHeading]) {
        for h in headings {
            if h.title.starts_with("Error:") || h.title.starts_with("错误:") {
                let name = h.symbol_name.clone().unwrap_or_else(|| {
                    h.title
                        .trim_start_matches("Error:")
                        .trim_start_matches("错误:")
                        .trim()
                        .to_string()
                });
                let fields = h
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.type_name.clone().unwrap_or_default()))
                    .collect();
                self.register(&name, fields, Some(h.span.clone()));
            }
            self.collect_from_headings(&h.children);
        }
    }
}
