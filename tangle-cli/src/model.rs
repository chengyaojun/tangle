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

/// 标题节点（树结构 — 子标题嵌套）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TangleHeading {
    pub id: String,
    pub depth: usize,
    pub role: HeadingRole,
    pub title: String,
    pub symbol_name: Option<String>,
    pub directives: Vec<TangleDirective>,
    pub params: Vec<TangleParam>,
    pub code_blocks: Vec<TangleCodeBlock>,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TangleModule {
    pub file: String,
    pub module_name: String,
    pub imports: Vec<TangleImport>,
    pub headings: Vec<TangleHeading>,
    pub symbols: Vec<TangleSymbol>,
    pub diagnostics: Vec<TangleDiagnostic>,
}
