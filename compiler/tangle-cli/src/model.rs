use serde::{Deserialize, Serialize};

/// 源码位置 span
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// 标题角色（6 级深度语义）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeadingRole {
    Program,         // depth 1
    Section,         // depth 2
    Type,            // depth 3
    Callable,        // depth 4
    SemanticSection, // depth 5
    SemanticAtom,    // depth 6
}

/// 指令（已消除所有 @ 指令，占位类型）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleDirective {
    pub raw: String,
    pub span: SourceSpan,
}

/// 模块导入（链接即导入）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleImport {
    pub alias: String,
    pub target: String,
    pub span: SourceSpan,
}

/// 函数参数 / 结构体字段
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleParam {
    pub name: String,
    pub description: String,
    pub type_name: Option<String>,
    pub span: SourceSpan,
}

/// @tangle 代码块
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleCodeBlock {
    pub language: String,
    pub value: String,
    pub span: SourceSpan,
}

/// 规则种类（四种规则形式）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleKind {
    Flow,    // Mermaid graph TD
    Table,   // Markdown pipe table
    Tree,    // Nested bullet lists
    Toggle,  // Checkbox lists
}

/// 规则数据（前端提取的规则体源码）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleData {
    pub kind: RuleKind,
    pub source: String,
    pub span: SourceSpan,
}

/// 标题节点（树结构 — 子标题嵌套）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleHeading {
    pub id: String,
    pub depth: usize,
    pub role: HeadingRole,
    pub title: String,
    pub symbol_name: Option<String>,
    pub directives: Vec<TangleDirective>,
    pub params: Vec<TangleParam>,
    pub code_blocks: Vec<TangleCodeBlock>,
    pub rule: Option<RuleData>,
    pub span: SourceSpan,
    pub children: Vec<TangleHeading>,
}

/// 符号种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Entry,
    Type,
    Callable,
    SemanticInternal,
}

/// 符号表条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub exported: bool,
    pub heading_id: String,
    pub span: SourceSpan,
}

/// 编译诊断
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleDiagnostic {
    pub code: String,
    pub message: String,
    pub span: SourceSpan,
}

/// DSL 编译单元（前端产出物）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TangleModule {
    pub file: String,
    pub module_name: String,
    pub imports: Vec<TangleImport>,
    pub headings: Vec<TangleHeading>,
    pub symbols: Vec<TangleSymbol>,
    pub diagnostics: Vec<TangleDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_span_construction() {
        let span = SourceSpan {
            file: "test.md".into(), start_line: 1, start_column: 1, end_line: 3, end_column: 10,
        };
        assert_eq!(span.file, "test.md");
        assert_eq!(span.start_line, 1);
        assert_eq!(span.end_line, 3);
    }

    #[test]
    fn test_heading_role_values() {
        assert_eq!(heading_role_for_depth_for_test(1), HeadingRole::Program);
        assert_eq!(heading_role_for_depth_for_test(3), HeadingRole::Type);
        assert_eq!(heading_role_for_depth_for_test(5), HeadingRole::SemanticSection);
    }

    fn heading_role_for_depth_for_test(depth: usize) -> HeadingRole {
        match depth {
            1 => HeadingRole::Program, 2 => HeadingRole::Section,
            3 => HeadingRole::Type, 4 => HeadingRole::Callable,
            5 => HeadingRole::SemanticSection, 6 => HeadingRole::SemanticAtom,
            _ => HeadingRole::Section,
        }
    }

    #[test]
    fn test_tangle_heading_tree() {
        let span = SourceSpan { file: "t.md".into(), start_line: 1, start_column: 1, end_line: 1, end_column: 5 };
        let child = TangleHeading {
            id: "child".into(), depth: 5, role: HeadingRole::SemanticSection,
            title: "Rule: Test".into(), symbol_name: None, directives: vec![],
            params: vec![], code_blocks: vec![], rule: None, span: span.clone(), children: vec![],
        };
        let parent = TangleHeading {
            id: "parent".into(), depth: 1, role: HeadingRole::Program,
            title: "Test".into(), symbol_name: Some("Test".into()), directives: vec![],
            params: vec![], code_blocks: vec![], rule: None, span: span.clone(),
            children: vec![child],
        };
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].title, "Rule: Test");
    }

    #[test]
    fn test_rule_kind_values() {
        assert_eq!(RuleKind::Flow as u8, 0); // discriminant check
        assert_ne!(RuleKind::Flow, RuleKind::Table);
    }

    #[test]
    fn test_rule_data_construction() {
        let data = RuleData {
            kind: RuleKind::Flow,
            source: "graph TD\nA-->B".into(),
            span: SourceSpan { file: "x.md".into(), start_line: 1, start_column: 1, end_line: 2, end_column: 5 },
        };
        assert_eq!(data.kind, RuleKind::Flow);
        assert!(data.source.contains("graph TD"));
    }

    #[test]
    fn test_tangle_module_construction() {
        let module = TangleModule {
            file: "test.md".into(), module_name: "test".into(),
            imports: vec![], headings: vec![], symbols: vec![], diagnostics: vec![],
        };
        assert_eq!(module.module_name, "test");
    }

    #[test]
    fn test_symbol_kind_values() {
        assert_eq!(SymbolKind::Entry as u8, 0);
        assert_eq!(SymbolKind::Type as u8, 1);
        assert_eq!(SymbolKind::Callable as u8, 2);
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = TangleDiagnostic {
            code: "E001".into(), message: "test error".into(),
            span: SourceSpan { file: "f.md".into(), start_line: 1, start_column: 1, end_line: 1, end_column: 1 },
        };
        assert_eq!(diag.code, "E001");
        assert!(diag.message.contains("test"));
    }

    #[test]
    fn test_serde_source_span_roundtrip() {
        let span = SourceSpan { file: "x.md".into(), start_line: 2, start_column: 3, end_line: 4, end_column: 5 };
        let json = serde_json::to_string(&span).unwrap();
        let back: SourceSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(span, back);
    }

    #[test]
    fn test_serde_rule_kind_roundtrip() {
        for kind in [RuleKind::Flow, RuleKind::Table, RuleKind::Tree, RuleKind::Toggle] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: RuleKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn test_serde_tangle_module_roundtrip() {
        let module = TangleModule {
            file: "m.md".into(), module_name: "m".into(),
            imports: vec![], headings: vec![], symbols: vec![], diagnostics: vec![],
        };
        let json = serde_json::to_string(&module).unwrap();
        let back: TangleModule = serde_json::from_str(&json).unwrap();
        assert_eq!(module.module_name, back.module_name);
    }

    #[test]
    fn test_heading_with_rule_data() {
        let span = SourceSpan { file: "h.md".into(), start_line: 1, start_column: 1, end_line: 1, end_column: 1 };
        let heading = TangleHeading {
            id: "h1".into(), depth: 5, role: HeadingRole::SemanticSection,
            title: "Rule: Test".into(), symbol_name: None, directives: vec![],
            params: vec![], code_blocks: vec![],
            rule: Some(RuleData { kind: RuleKind::Flow, source: "graph TD".into(), span: span.clone() }),
            span, children: vec![],
        };
        assert!(heading.rule.is_some());
        assert_eq!(heading.rule.unwrap().kind, RuleKind::Flow);
    }
}
