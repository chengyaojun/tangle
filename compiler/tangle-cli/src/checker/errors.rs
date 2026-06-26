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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HeadingRole, SourceSpan, TangleHeading, TangleParam};

    fn make_span() -> SourceSpan {
        SourceSpan {
            file: "test.tangle".to_string(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 10,
        }
    }

    fn make_heading(title: &str, symbol_name: Option<&str>, params: Vec<TangleParam>) -> TangleHeading {
        TangleHeading {
            id: "h1".to_string(),
            depth: 1,
            role: HeadingRole::Type,
            title: title.to_string(),
            symbol_name: symbol_name.map(|s| s.to_string()),
            directives: vec![],
            params,
            code_blocks: vec![],
            rule: None,
            span: make_span(),
            children: vec![],
        }
    }

    fn make_param(name: &str, type_name: &str) -> TangleParam {
        TangleParam {
            name: name.to_string(),
            description: String::new(),
            type_name: Some(type_name.to_string()),
            span: make_span(),
        }
    }

    // --- 1. Register and lookup an error variant ---

    #[test]
    fn register_and_lookup() {
        let mut reg = ErrorRegistry::new();
        reg.register(
            "PayFailed",
            vec![("amount".to_string(), "Int".to_string())],
            None,
        );

        let v = reg.lookup("PayFailed");
        assert!(v.is_some());
        let v = v.unwrap();
        assert_eq!(v.name, "PayFailed");
        assert_eq!(v.fields.len(), 1);
        assert_eq!(v.fields[0], ("amount".to_string(), "Int".to_string()));
        assert!(v.span.is_none());
    }

    // --- 2. is_error returns true for registered, false for unknown ---

    #[test]
    fn is_error_check() {
        let mut reg = ErrorRegistry::new();
        reg.register("Timeout", vec![], None);

        assert!(reg.is_error("Timeout"));
        assert!(!reg.is_error("NotFound"));
    }

    // --- 3. all_variants returns all registered variants ---

    #[test]
    fn all_variants_collection() {
        let mut reg = ErrorRegistry::new();
        reg.register("A", vec![], None);
        reg.register("B", vec![], None);
        reg.register("C", vec![], None);

        let mut names: Vec<&str> = reg.all_variants().iter().map(|v| v.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["A", "B", "C"]);
    }

    // --- 4. collect_from_headings — "Error: PayFailed" title ---

    #[test]
    fn collect_from_headings() {
        let heading = make_heading(
            "Error:PayFailed",
            None,
            vec![
                make_param("amount", "Int"),
                make_param("reason", "String"),
            ],
        );

        let mut reg = ErrorRegistry::new();
        reg.collect_from_headings(&[heading]);

        assert!(reg.is_error("PayFailed"));

        let v = reg.lookup("PayFailed").unwrap();
        assert_eq!(v.name, "PayFailed");
        assert_eq!(v.fields.len(), 2);
        assert_eq!(v.fields[0], ("amount".to_string(), "Int".to_string()));
        assert_eq!(v.fields[1], ("reason".to_string(), "String".to_string()));
        assert!(v.span.is_some());
    }
}
