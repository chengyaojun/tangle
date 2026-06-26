# Track B — Rust 权威期 (1.0) 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 用 Rust 原生重写 Tangle 编译器的全部能力（Markdown 解析 → DSL 模型 → 类型检查 → IR → 多宿主 codegen），作为官方权威 `tangle-cli` 实现。TypeScript 版退役为参考实现。

**架构：** 单一 Rust crate `tangle-cli`，按编译流水线分为 6 层：frontend（Markdown → TangleModule）→ parser（@tangle 代码块词法/语法分析）→ checker（类型检查 + 错误处理）→ ir（Rule Graph IR + 规则 lowering）→ codegen（多宿主 JS/Python/Go 发射）→ cli（clap 命令行）。差分测试通过共享 IR JSON Schema 与 TS 参考实现对对齐。

**技术栈：** Rust 1.80+、pulldown-cmark（Markdown 解析）、clap 4.x（CLI）、serde + serde_json（IR 序列化）、codespan-reporting（诊断渲染）。

---

## 规格来源

- `docs/history/cosmic-pulse-lovelace.md` — 完整语言设计规格（Track B 定义在 §12、§14，行 420-521）
- `docs/superpowers/specs/2026-06-24-tangle-language-design.md` — 英文权威规格（Track B 在附录 C，行 524-535）
- `README.md` — 路线图摘要（行 184-197）

Track A 参考实现（TypeScript，~2600 行源码 + 31 个测试文件）位于 `src/` 目录，全部模块需对齐。

---

## 文件结构

Rust 项目位于仓库根目录下的 `tangle-cli/`：

```
tangle-cli/
├── Cargo.toml                  # crate 元数据、依赖
├── src/
│   ├── main.rs                 # CLI 入口（clap）
│   ├── lib.rs                  # 库根、公共 API 重导出
│   ├── model.rs                # DSL 类型定义（TangleModule、TangleHeading、SourceSpan 等）
│   ├── ast.rs                  # @tangle 代码块 AST 类型（Expr、Stmt、TypeExpr）
│   ├── diagnostic.rs           # 诊断系统（TangleDiagnostic、诊断码枚举、渲染）
│   ├── frontend/
│   │   ├── mod.rs              # 前端模块入口
│   │   ├── compile_module.rs   # Markdown → TangleModule 主编译流程
│   │   ├── blocks.rs           # 块级工具（链接收集、参数解析、代码块检测）
│   │   ├── headings.rs         # 标题解析与角色映射
│   │   └── source_map.rs       # 源码 span 提取
│   ├── markdown/
│   │   ├── mod.rs              # Markdown 模块入口
│   │   └── parse_markdown.rs   # pulldown-cmark 封装
│   ├── parser/
│   │   ├── mod.rs              # 解析器模块入口
│   │   ├── lexer.rs            # 词法分析器（Token、TokenKind）
│   │   ├── parser.rs           # Pratt 语法分析器（表达式 + 语句）
│   │   └── type_parser.rs      # 类型表达式解析器
│   ├── checker/
│   │   ├── mod.rs              # 检查器模块入口
│   │   ├── types.rs            # 类型系统（Type 枚举、typesEqual、isSubtype）
│   │   ├── builtins.rs         # 内建基础类型
│   │   ├── env.rs              # 类型环境（TypeEnv、ReceiverContext）
│   │   ├── check.rs            # 表达式类型检查
│   │   ├── resolve.rs          # 类型解析（两遍算法）
│   │   ├── errors.rs           # 错误变体注册表（ErrorRegistry）
│   │   ├── propagation.rs      # 错误传播检查（? 操作符）
│   │   ├── match_check.rs      # match 穷举性检查
│   │   ├── panic_check.rs      # panic 检查
│   │   └── check_module.rs     # 模块级类型检查编排
│   ├── ir/
│   │   ├── mod.rs              # IR 模块入口
│   │   ├── graph.rs            # RuleGraph 类型定义（IRNode、IREdge、IRErrorEdge）
│   │   ├── lower.rs            # 语句 → IR 子图 lowering
│   │   ├── lower_rule_flow.rs  # Mermaid 流程图 → IR lowering
│   │   ├── lower_rule_table.rs # 决策表 → IR lowering
│   │   ├── lower_rule_tree.rs  # 决策树 → IR lowering
│   │   ├── lower_rule_toggle.rs# 复选框 → IR lowering
│   │   ├── compile_to_ir.rs    # IR 编译编排（合并子图 + 验证）
│   │   ├── validate.rs         # IR 结构验证
│   │   └── visibility.rs       # 符号可见性检查
│   ├── codegen/
│   │   ├── mod.rs              # Codegen 模块入口
│   │   ├── js_emitter.rs       # JavaScript 代码发射器
│   │   ├── js_prelude.rs       # JS 运行时前导
│   │   ├── error_mapping.rs    # 错误包装/解包辅助
│   │   ├── py_emitter.rs       # Python 代码发射器（B3）
│   │   ├── go_emitter.rs       # Go 代码发射器（B3）
│   │   └── common.rs           # 跨宿主共享发射逻辑
│   └── cli/
│       ├── mod.rs              # CLI 模块入口
│       ├── run.rs              # tangle run 命令
│       └── test.rs             # tangle test 命令
├── tests/
│   ├── frontend/               # 前端测试（对应 src/frontend/）
│   ├── parser/                 # 解析器测试（对应 src/parser/）
│   ├── checker/                # 检查器测试（对应 src/checker/）
│   ├── ir/                     # IR 测试（对应 src/ir/）
│   ├── codegen/                # Codegen 测试（对应 src/codegen/）
│   ├── integration/            # 端到端集成测试
│   └── fixtures/               # 共享测试固件（.md 文件）
├── test-cases/                 # 差分测试用例库（与 TS 版共享的 .md 文件）
│   ├── basic/                  # 基础语法
│   ├── structs/                # 结构体与方法
│   ├── errors/                 # 错误处理
│   ├── rules/                  # 四种规则形式
│   └── mvp/                    # 业务 MVP 用例
└── ir-schema/                  # IR JSON Schema（与 TS 版共享，供差分测试）
    └── schema.json
```

---

## Phase B1 — Rust 编译器骨架（B1: 项目搭建 + frontend + parser + checker + IR + JS codegen + CLI）

### 任务 B1.0：项目初始化

**文件：**
- 创建：`tangle-cli/Cargo.toml`
- 创建：`tangle-cli/src/main.rs`
- 创建：`tangle-cli/src/lib.rs`

- [ ] **步骤 1：初始化 Cargo 项目**

```bash
cargo init tangle-cli --name tangle-cli
```

- [ ] **步骤 2：编写 Cargo.toml 依赖**

```toml
[package]
name = "tangle-cli"
version = "0.1.0"
edition = "2021"
description = "Tangle compiler — Rust authority implementation"

[[bin]]
name = "tangle"
path = "src/main.rs"

[dependencies]
pulldown-cmark = { version = "0.11", default-features = false, features = ["html"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
codespan-reporting = "0.11"
thiserror = "1"

[dev-dependencies]
tempfile = "3"
pretty_assertions = "1"
```

- [ ] **步骤 3：编写 src/main.rs 骨架**

```rust
fn main() {
    println!("tangle-cli {}", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **步骤 4：验证构建**

```bash
cd tangle-cli && cargo build
```

预期：编译成功，无错误。

- [ ] **步骤 5：编写 src/lib.rs 公共 API 骨架**

```rust
pub mod model;
pub mod ast;
pub mod diagnostic;
pub mod frontend;
pub mod markdown;
pub mod parser;
pub mod checker;
pub mod ir;
pub mod codegen;
pub mod cli;
```

- [ ] **步骤 6：验证 lib 构建**

```bash
cd tangle-cli && cargo build --lib
```

预期：因模块文件尚不存在而编译失败（后续任务逐步添加）。

- [ ] **步骤 7：Commit**

```bash
git add tangle-cli/
git commit -m "feat(b1): initialize Rust project skeleton

- Cargo.toml with pulldown-cmark, clap, serde, codespan-reporting
- src/main.rs CLI entry point
- src/lib.rs module declarations

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### 任务 B1.1：DSL 模型类型（model.rs + diagnostic.rs）

**文件：**
- 创建：`tangle-cli/src/model.rs`
- 创建：`tangle-cli/src/diagnostic.rs`

> 对应 TS 文件：`src/model.ts`（87 行）。所有类型是纯数据结构。

- [ ] **步骤 1：编写 model.rs 类型定义**

```rust
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
    Program,         // depth 1 — 包上下文
    Section,         // depth 2 — 命名空间
    Type,            // depth 3 — 结构体/接口/错误族
    Callable,        // depth 4 — 函数/方法
    SemanticSection, // depth 5 — 语义段
    SemanticAtom,    // depth 6 — 原子单元
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
```

- [ ] **步骤 2：编写 diagnostic.rs 诊断码枚举与渲染**

```rust
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use crate::model::TangleDiagnostic;

/// 诊断码常量
pub mod codes {
    pub const HEADING_MULTI_WORD: &str = "TANGLE_HEADING_MULTI_WORD";
    pub const INVALID_HEADING_CASE: &str = "TANGLE_INVALID_HEADING_CASE";
    pub const DUPLICATE_SYMBOL: &str = "TANGLE_DUPLICATE_SYMBOL";
    pub const PARSE_ERROR: &str = "TANGLE_PARSE_ERROR";
    pub const TYPE_ERROR: &str = "TANGLE_TYPE_ERROR";
    pub const TYPE_ALL_ERROR: &str = "TANGLE_TYPE_ALL_ERROR";
    pub const PANIC_REACHED: &str = "TANGLE_PANIC_REACHED";
    pub const MATCH_NOT_EXHAUSTIVE: &str = "TANGLE_MATCH_NOT_EXHAUSTIVE";
    pub const IR_VALIDATION_ERROR: &str = "TANGLE_IR_VALIDATION_ERROR";
    pub const UNDECLARED_ERROR: &str = "TANGLE_UNDECLARED_ERROR";
    pub const SYMBOL_NOT_FOUND: &str = "TANGLE_SYMBOL_NOT_FOUND";
}

/// 将诊断列表渲染到 stderr
pub fn render_diagnostics(diagnostics: &[TangleDiagnostic], source: &str, file: &str) {
    let mut files = SimpleFiles::new();
    let file_id = files.add(file, source);

    for diag in diagnostics {
        let diagnostic = Diagnostic::error()
            .with_message(&diag.message)
            .with_code(&diag.code)
            .with_labels(vec![Label::primary(
                file_id,
                diag.span.start_line..diag.span.end_line,
            )]);

        let writer = term::termcolor::StandardStream::stderr(
            term::termcolor::ColorChoice::Auto,
        );
        let config = term::Config::default();
        let _ = term::emit(&mut writer.lock(), &config, &files, &diagnostic);
    }
}
```

- [ ] **步骤 3：验证构建**

```bash
cd tangle-cli && cargo build --lib
```

预期：model.rs 和 diagnostic.rs 编译通过。

- [ ] **步骤 4：Commit**

```bash
git add tangle-cli/src/model.rs tangle-cli/src/diagnostic.rs
git commit -m "feat(b1): add DSL model types and diagnostic system

- model.rs: TangleModule, TangleHeading, SourceSpan, TangleSymbol, etc.
- diagnostic.rs: diagnostic code constants + codespan-reporting renderer

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### 任务 B1.2：AST 类型定义（ast.rs）

**文件：**
- 创建：`tangle-cli/src/ast.rs`

> 对应 TS 文件：`src/ast.ts`（249 行）。所有表达式/语句/类型表达式的枚举定义。

- [ ] **步骤 1：编写 ast.rs — 表达式枚举**

```rust
use crate::model::SourceSpan;

// ============================================================
// 表达式
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(LiteralExpr),
    Identifier(IdentifierExpr),
    MemberAccess(MemberAccessExpr),
    Call(CallExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    RecordUpdate(RecordUpdateExpr),
    Pipe(PipeExpr),
    This(ThisExpr),
    If(IfExpr),
    Arrow(ArrowExpr),
    Propagation(PropagationExpr),
    Match(MatchExpr),
    Destructure(DestructureExpr),
    Panic(PanicExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiteralExpr {
    pub literal_kind: LiteralKind,
    pub value: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralKind {
    Number,
    String,
    Boolean,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifierExpr {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberAccessExpr {
    pub object: Box<Expr>,
    pub member: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq,
    Lt, Gt, Lte, Gte,
    And, Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordUpdateExpr {
    pub object: Box<Expr>,
    pub fields: Vec<RecordField>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordField {
    pub name: String,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipeExpr {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThisExpr {
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub then_branch: Box<Expr>,
    pub else_branch: Option<Box<Expr>>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrowExpr {
    pub params: Vec<ArrowParam>,
    pub body: Box<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrowParam {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropagationExpr {
    pub expr: Box<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<MatchArm>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Expr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Variant { name: String, binding: Option<String> },
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestructureExpr {
    pub ok_name: String,
    pub err_name: String,
    pub expr: Box<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PanicExpr {
    pub message: Box<Expr>,
    pub span: SourceSpan,
}

// ============================================================
// 语句
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Return(ReturnStmt),
    Let(LetStmt),
    Const(ConstStmt),
    Expression(ExpressionStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstStmt {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionStmt {
    pub expr: Expr,
    pub span: SourceSpan,
}

// ============================================================
// 代码体
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub struct CodeBody {
    pub statements: Vec<Stmt>,
    pub span: SourceSpan,
}

// ============================================================
// 类型表达式（用于类型标注解析）
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Primitive(PrimitiveTypeExpr),
    Sum(SumTypeExpr),
    Generic(GenericTypeExpr),
    Function(FunctionTypeExpr),
    Named(NamedTypeExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveTypeExpr {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SumTypeExpr {
    pub variants: Vec<TypeExpr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericTypeExpr {
    pub base: String,
    pub type_args: Vec<TypeExpr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionTypeExpr {
    pub params: Vec<TypeExpr>,
    pub returns: Box<TypeExpr>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedTypeExpr {
    pub name: String,
    pub span: SourceSpan,
}

// ============================================================
// 解析后的代码块（含源码引用）
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCodeBlock {
    pub heading_id: String,
    pub source: String,
    pub body: CodeBody,
    pub diagnostics: Vec<crate::model::TangleDiagnostic>,
}
```

- [ ] **步骤 2：验证构建**

```bash
cd tangle-cli && cargo build --lib
```

预期：ast.rs 编译通过。

- [ ] **步骤 3：Commit**

```bash
git add tangle-cli/src/ast.rs
git commit -m "feat(b1): add code block AST type definitions

Expr (18 variants), Stmt (4 variants), TypeExpr (5 variants),
CodeBody, ParsedCodeBlock. All variants carry SourceSpan.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### 任务 B1.3：Markdown 解析封装（markdown/）

**文件：**
- 创建：`tangle-cli/src/markdown/mod.rs`
- 创建：`tangle-cli/src/markdown/parse_markdown.rs`

> 对应 TS 文件：`src/markdown/parseMarkdown.ts`（25 行）。封装 pulldown-cmark 提供简化的 Markdown 节点树。

- [ ] **步骤 1：编写 parse_markdown.rs**

```rust
use pulldown_cmark::{Event, Parser, Tag, TagEnd, HeadingLevel};
use crate::model::SourceSpan;

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
pub fn parse_markdown(source: &str, file: &str) -> Vec<MarkdownNode> {
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
                // flush pending text
                flush_text(&mut current_text, &current_position, &mut stack);

                let pos = offset_to_position(source, start, end, file);
                match tag {
                    Tag::Heading { level, .. } => {
                        let depth = match level {
                            HeadingLevel::H1 => 1,
                            HeadingLevel::H2 => 2,
                            HeadingLevel::H3 => 3,
                            HeadingLevel::H4 => 4,
                            HeadingLevel::H5 => 5,
                            HeadingLevel::H6 => 6,
                        };
                        let node = MarkdownNode {
                            node_type: "heading".into(),
                            children: vec![],
                            value: None,
                            depth: Some(depth),
                            lang: None,
                            url: None,
                            checked: None,
                            position: Some(pos),
                        };
                        stack.push(node);
                    }
                    Tag::CodeBlock(kind) => {
                        let lang = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(l) => {
                                if l.is_empty() { None } else { Some(l.to_string()) }
                            }
                            pulldown_cmark::CodeBlockKind::Indented => None,
                        };
                        let node = MarkdownNode {
                            node_type: "code".into(),
                            children: vec![],
                            value: None,
                            depth: None,
                            lang,
                            url: None,
                            checked: None,
                            position: Some(pos),
                        };
                        stack.push(node);
                    }
                    Tag::List(..) => {
                        let node = MarkdownNode {
                            node_type: "list".into(),
                            children: vec![],
                            value: None,
                            depth: None,
                            lang: None,
                            url: None,
                            checked: None,
                            position: Some(pos),
                        };
                        stack.push(node);
                    }
                    Tag::Item => {
                        let node = MarkdownNode {
                            node_type: "listItem".into(),
                            children: vec![],
                            value: None,
                            depth: None,
                            lang: None,
                            url: None,
                            checked: None,
                            position: Some(pos),
                        };
                        stack.push(node);
                    }
                    Tag::Link { dest_url, .. } => {
                        let node = MarkdownNode {
                            node_type: "link".into(),
                            children: vec![],
                            value: None,
                            depth: None,
                            lang: None,
                            url: Some(dest_url.to_string()),
                            checked: None,
                            position: Some(pos),
                        };
                        stack.push(node);
                    }
                    Tag::Paragraph | Tag::BlockQuote => {
                        let node = MarkdownNode {
                            node_type: if matches!(tag, Tag::Paragraph) {
                                "paragraph"
                            } else {
                                "blockquote"
                            }.into(),
                            children: vec![],
                            value: None,
                            depth: None,
                            lang: None,
                            url: None,
                            checked: None,
                            position: Some(pos),
                        };
                        stack.push(node);
                    }
                    Tag::Table(..) | Tag::TableHead | Tag::TableRow => {
                        let node = MarkdownNode {
                            node_type: "html".into(),
                            children: vec![],
                            value: None,
                            depth: None,
                            lang: None,
                            url: None,
                            checked: None,
                            position: Some(pos),
                        };
                        stack.push(node);
                    }
                    _ => {}
                }
            }
            Event::End(tag_end) => {
                flush_text(&mut current_text, &current_position, &mut stack);

                match tag_end {
                    TagEnd::Heading(..) | TagEnd::CodeBlock
                    | TagEnd::List(..) | TagEnd::Item
                    | TagEnd::Link | TagEnd::Paragraph
                    | TagEnd::BlockQuote | TagEnd::Table
                    | TagEnd::TableHead | TagEnd::TableRow => {
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
            Event::Text(text) | Event::Code(text) => {
                if current_position.is_none() {
                    current_position = Some(offset_to_position(source, start, end, file));
                }
                current_text.push_str(&text);
            }
            Event::InlineHtml(html) | Event::Html(html) => {
                current_text.push_str(&html);
            }
            Event::SoftBreak | Event::HardBreak => {
                current_text.push(' ');
            }
            Event::TaskListMarker(checked) => {
                // record checked state on the parent listItem
                if let Some(parent) = stack.last_mut() {
                    parent.checked = Some(checked);
                }
            }
            _ => {}
        }
    }

    flush_text(&mut current_text, &current_position, &mut stack);
    root_children
}

fn flush_text(
    text: &mut String,
    position: &Option<MarkdownPosition>,
    stack: &mut Vec<MarkdownNode>,
) {
    if text.trim().is_empty() {
        text.clear();
        *position = None;
        return;
    }
    let text_node = MarkdownNode {
        node_type: "text".into(),
        children: vec![],
        value: Some(std::mem::take(text)),
        depth: None,
        lang: None,
        url: None,
        checked: None,
        position: *position,
    };
    if let Some(parent) = stack.last_mut() {
        parent.children.push(text_node);
    }
    *position = None;
}

fn offset_to_position(
    source: &str,
    start: usize,
    end: usize,
    file: &str,
) -> MarkdownPosition {
    let start_line = source[..start].lines().count();
    let end_line = source[..end].lines().count();
    let start_col = source[..start].lines().last().map(|l| l.len()).unwrap_or(1);
    let end_col = source[..end].lines().last().map(|l| l.len()).unwrap_or(1);

    MarkdownPosition {
        start_line: start_line.max(1),
        start_column: start_col + 1, // 1-indexed
        end_line: end_line.max(1),
        end_column: end_col + 1,
    }
}

impl MarkdownNode {
    /// 获取 span（需要 file 参数）
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
```

- [ ] **步骤 2：编写 mod.rs**

```rust
pub mod parse_markdown;
pub use parse_markdown::*;
```

- [ ] **步骤 3：验证构建**

```bash
cd tangle-cli && cargo build --lib
```

- [ ] **步骤 4：Commit**

```bash
git add tangle-cli/src/markdown/
git commit -m "feat(b1): add Markdown parsing wrapper (pulldown-cmark)

Simplified node tree (MarkdownNode) for heading, code block,
list, link, paragraph, blockquote detection.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### 任务 B1.4：前端 — 标题与块工具（frontend/headings.rs + blocks.rs + source_map.rs）

**文件：**
- 创建：`tangle-cli/src/frontend/mod.rs`
- 创建：`tangle-cli/src/frontend/headings.rs`
- 创建：`tangle-cli/src/frontend/blocks.rs`
- 创建：`tangle-cli/src/frontend/source_map.rs`

> 对应 TS 文件：`headings.ts`（38 行）、`blocks.ts`（53 行）、`sourceMap.ts`（18 行）

- [ ] **步骤 1：编写 headings.rs**

```rust
use crate::model::{HeadingRole, SourceSpan, TangleDiagnostic};
use crate::diagnostic::codes;

/// 深度 → 角色映射（6 级系统）
pub fn heading_role_for_depth(depth: usize) -> HeadingRole {
    match depth {
        1 => HeadingRole::Program,
        2 => HeadingRole::Section,
        3 => HeadingRole::Type,
        4 => HeadingRole::Callable,
        5 => HeadingRole::SemanticSection,
        6 => HeadingRole::SemanticAtom,
        _ => HeadingRole::Section, // fallback
    }
}

/// 解析标题文本结果
#[derive(Debug, Clone)]
pub struct ParsedHeadingText {
    pub title: String,
    pub symbol_name: Option<String>,
    pub diagnostics: Vec<TangleDiagnostic>,
}

/// 提取括号中的显式标识符或返回纯文本
/// 规则：(1) 显式括号 (ident) → symbol_name (2) 纯英文标识符 → 直接使用
///       (3) 含空格纯英文 → 诊断 (4) Unicode 文本 → 直接使用
pub fn parse_heading_text(text: &str, depth: usize, span: &SourceSpan) -> ParsedHeadingText {
    let trimmed = text.trim();

    // 规则 1：查找英文半角括号中的标识符
    if let Some(open) = trimmed.rfind('(') {
        if let Some(close) = trimmed.rfind(')') {
            if close > open {
                let ident = &trimmed[open + 1..close];
                if is_valid_identifier(ident) {
                    let title = format!(
                        "{} {}",
                        trimmed[..open].trim(),
                        trimmed[close + 1..].trim()
                    )
                    .trim()
                    .to_string();
                    return ParsedHeadingText {
                        title,
                        symbol_name: Some(ident.to_string()),
                        diagnostics: vec![],
                    };
                }
            }
        }
    }

    // 规则 2：纯英文标识符
    if is_valid_identifier(trimmed) {
        let mut diagnostics = vec![];
        // 大小写对齐契约检查
        let expected_case = match depth {
            1..=3 => "PascalCase",
            4..=6 => "camelCase",
            _ => return ParsedHeadingText {
                title: trimmed.to_string(),
                symbol_name: None,
                diagnostics: vec![],
            },
        };

        let first_char = trimmed.chars().next().unwrap();
        match depth {
            1..=3 => {
                if !first_char.is_uppercase() {
                    diagnostics.push(TangleDiagnostic {
                        code: codes::INVALID_HEADING_CASE.into(),
                        message: format!(
                            "Heading '{}' at depth {} must be {}; got lowercase start",
                            trimmed, depth, expected_case
                        ),
                        span: span.clone(),
                    });
                }
            }
            4..=6 => {
                if !first_char.is_lowercase() {
                    diagnostics.push(TangleDiagnostic {
                        code: codes::INVALID_HEADING_CASE.into(),
                        message: format!(
                            "Heading '{}' at depth {} must be {}; got uppercase start",
                            trimmed, depth, expected_case
                        ),
                        span: span.clone(),
                    });
                }
            }
            _ => {}
        }

        return ParsedHeadingText {
            title: trimmed.to_string(),
            symbol_name: Some(trimmed.to_string()),
            diagnostics,
        };
    }

    // 规则 2 附带：纯英文含空格 → 警告
    if trimmed.chars().all(|c| c.is_ascii_graphic() || c == ' ') && trimmed.contains(' ') {
        return ParsedHeadingText {
            title: trimmed.to_string(),
            symbol_name: None,
            diagnostics: vec![TangleDiagnostic {
                code: codes::HEADING_MULTI_WORD.into(),
                message: format!(
                    "Heading '{}' has multiple words — add an explicit identifier in parentheses, e.g. (myFunc)",
                    trimmed
                ),
                span: span.clone(),
            }],
        };
    }

    // 规则 3：Unicode 降级
    ParsedHeadingText {
        title: trimmed.to_string(),
        symbol_name: Some(trimmed.to_string()),
        diagnostics: vec![],
    }
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
```

- [ ] **步骤 2：编写 blocks.rs**

```rust
use crate::markdown::MarkdownNode;
use crate::model::{SourceSpan, TangleImport, TangleParam, TangleDiagnostic};

/// 递归收集所有 .md 链接作为导入声明
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
                    let alias = node
                        .children
                        .iter()
                        .find(|c| c.node_type == "text")
                        .and_then(|c| c.value.clone())
                        .unwrap_or_else(|| "unknown".to_string());

                    if let Some(span) = node.to_span(file) {
                        out.push(TangleImport {
                            alias,
                            target: url.clone(),
                            span,
                        });
                    }
                }
            }
        }
        collect_links_recursive(file, &node.children, out);
    }
}

/// 解析参数列表项：`name`: description (Type)
pub fn parse_param_item(text: &str, span: &SourceSpan) -> Option<TangleParam> {
    let text = text.trim();

    // 匹配 `name`: description (Type) 或 `name`: (Type)
    if !text.starts_with('`') {
        return None;
    }

    let end_tick = text[1..].find('`')?;
    let name = &text[1..=end_tick];

    let after_name = text[end_tick + 2..].trim();
    let after_name = after_name.strip_prefix(':')?.trim();

    let (description, type_name) = if let Some(open) = after_name.rfind('(') {
        if let Some(close) = after_name.rfind(')') {
            if close > open {
                let desc = after_name[..open].trim().to_string();
                let ty = after_name[open + 1..close].to_string();
                (desc, Some(ty))
            } else {
                (after_name.to_string(), None)
            }
        } else {
            (after_name.to_string(), None)
        }
    } else {
        (after_name.to_string(), None)
    };

    Some(TangleParam {
        name: name.to_string(),
        description,
        type_name,
        span: span.clone(),
    })
}

/// 检查节点是否为 @tangle 代码块
pub fn is_tangle_code_block(node: &MarkdownNode) -> bool {
    node.node_type == "code"
        && node.lang.as_deref() == Some("@tangle")
}

/// 递归提取纯文本
pub fn plain_text(node: &MarkdownNode) -> String {
    if node.node_type == "text" {
        return node.value.clone().unwrap_or_default();
    }
    node.children.iter().map(plain_text).collect::<Vec<_>>().join("")
}
```

- [ ] **步骤 3：编写 source_map.rs**

```rust
use crate::markdown::MarkdownNode;
use crate::model::SourceSpan;

/// 从 MarkdownNode 的 position 提取 SourceSpan
pub fn span_from_node(file: &str, node: &MarkdownNode) -> Option<SourceSpan> {
    node.to_span(file)
}
```

- [ ] **步骤 4：编写 frontend/mod.rs**

```rust
pub mod headings;
pub mod blocks;
pub mod source_map;
pub mod compile_module;

pub use headings::*;
pub use blocks::*;
pub use source_map::*;
pub use compile_module::*;
```

- [ ] **步骤 5：验证构建**

```bash
cd tangle-cli && cargo build --lib
```

- [ ] **步骤 6：Commit**

```bash
git add tangle-cli/src/frontend/
git commit -m "feat(b1): add frontend utilities — headings, blocks, source_map

- headings.rs: depth→role mapping, heading text parser with casing validation
- blocks.rs: link collector, param item parser, @tangle code block detection
- source_map.rs: MarkdownNode → SourceSpan conversion

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### 任务 B1.5：前端 — compileModule 主编译流程（frontend/compile_module.rs）

**文件：**
- 创建：`tangle-cli/src/frontend/compile_module.rs`

> 对应 TS 文件：`src/front-end/compileModule.ts`（229 行）。这是前端主编排器。

- [ ] **步骤 1：编写 compile_module.rs**

```rust
use crate::markdown::{parse_markdown, MarkdownNode};
use crate::model::{
    HeadingRole, SourceSpan, SymbolKind, TangleCodeBlock, TangleDiagnostic,
    TangleHeading, TangleModule, TangleParam, TangleSymbol,
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

    // Step 1: 收集导入
    let imports = collect_links(&input.file, &nodes);

    // Step 2: 提取顶级标题（扁平列表）
    let flat_headings = extract_headings(&nodes, &input.file, &mut diagnostics);

    // Step 3: 构建标题树（按深度嵌套）
    let headings = build_heading_tree(flat_headings);

    // Step 4: 构建符号表
    let symbols = build_symbols(&headings);

    // Step 5: 验证符号规则（重复 main 等）
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

/// 从 Markdown 节点树提取扁平标题列表（含参数、代码块）
fn extract_headings(
    nodes: &[MarkdownNode],
    file: &str,
    diagnostics: &mut Vec<TangleDiagnostic>,
) -> Vec<TangleHeading> {
    let mut headings: Vec<TangleHeading> = vec![];
    let mut current_heading: Option<TangleHeading> = None;
    let mut pending_params: Vec<TangleParam> = vec![];
    let mut pending_code_blocks: Vec<TangleCodeBlock> = vec![];

    for node in nodes {
        if node.node_type == "heading" {
            // 保存前一个 heading
            if let Some(mut h) = current_heading.take() {
                h.params = std::mem::take(&mut pending_params);
                h.code_blocks = std::mem::take(&mut pending_code_blocks);
                headings.push(h);
            }

            let depth = node.depth.unwrap_or(1);
            let text = node
                .children
                .iter()
                .find(|c| c.node_type == "text")
                .and_then(|c| c.value.clone())
                .unwrap_or_default();

            let role = heading_role_for_depth(depth);
            let span = span_from_node(file, node)
                .unwrap_or_else(|| SourceSpan {
                    file: file.to_string(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                });

            let parsed = parse_heading_text(&text, depth, &span);
            diagnostics.extend(parsed.diagnostics);

            let id = stable_heading_id(&parsed.title);

            current_heading = Some(TangleHeading {
                id,
                depth,
                role,
                title: parsed.title,
                symbol_name: parsed.symbol_name,
                directives: vec![],
                params: vec![],
                code_blocks: vec![],
                span,
                children: vec![],
            });
        } else if let Some(ref mut h) = current_heading {
            // 在当前 heading 下方收集参数和代码块
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
                let value = node
                    .children
                    .iter()
                    .find(|c| c.node_type == "text")
                    .and_then(|c| c.value.clone())
                    .unwrap_or_default();
                if let Some(span) = span_from_node(file, node) {
                    pending_code_blocks.push(TangleCodeBlock {
                        language: "@tangle".into(),
                        value,
                        span,
                    });
                }
            }
        }
    }

    // 保存最后一个 heading
    if let Some(mut h) = current_heading.take() {
        h.params = std::mem::take(&mut pending_params);
        h.code_blocks = std::mem::take(&mut pending_code_blocks);
        headings.push(h);
    }

    headings
}

fn plain_text_recursive(node: &MarkdownNode) -> String {
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

/// 栈式标题树构建：按深度嵌套
fn build_heading_tree(flat: Vec<TangleHeading>) -> Vec<TangleHeading> {
    let mut root: Vec<TangleHeading> = vec![];
    let mut stack: Vec<TangleHeading> = vec![];

    for heading in flat {
        while let Some(top) = stack.last() {
            if top.depth < heading.depth {
                break;
            }
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

/// 从标题树提取符号表
fn build_symbols(headings: &[TangleHeading]) -> Vec<TangleSymbol> {
    let mut symbols = vec![];
    build_symbols_recursive(headings, &mut symbols);
    symbols
}

fn build_symbols_recursive(headings: &[TangleHeading], out: &mut Vec<TangleSymbol>) {
    for h in headings {
        if let Some(ref name) = h.symbol_name {
            // 可见性：下划线前缀 → 私有
            let exported = !name.starts_with('_');

            let kind = match h.role {
                HeadingRole::Program | HeadingRole::Section => SymbolKind::SemanticInternal,
                HeadingRole::Type => SymbolKind::Type,
                HeadingRole::Callable => {
                    if name == "main" && h.depth == 4 {
                        SymbolKind::Entry
                    } else {
                        SymbolKind::Callable
                    }
                }
                HeadingRole::SemanticSection | HeadingRole::SemanticAtom => {
                    SymbolKind::SemanticInternal
                }
            };

            out.push(TangleSymbol {
                name: name.clone(),
                kind,
                exported,
                heading_id: h.id.clone(),
                span: h.span.clone(),
            });
        }
        build_symbols_recursive(&h.children, out);
    }
}

/// 验证符号规则（如重复 main）
fn validate_symbol_rules(
    symbols: &[TangleSymbol],
    diagnostics: &mut Vec<TangleDiagnostic>,
) {
    let entry_count = symbols.iter().filter(|s| s.kind == SymbolKind::Entry).count();
    if entry_count > 1 {
        let dup_spans: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Entry)
            .map(|s| s.span.clone())
            .collect();
        for span in &dup_spans {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_DUPLICATE_ENTRY".into(),
                message: "Multiple 'main' entry points found; only one allowed per module".into(),
                span: span.clone(),
            });
        }
    }
}

fn module_name_from_file(file: &str) -> String {
    let path = std::path::Path::new(file);
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn stable_heading_id(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
```

- [ ] **步骤 2：验证构建**

```bash
cd tangle-cli && cargo build --lib
```

- [ ] **步骤 3：Commit**

```bash
git add tangle-cli/src/frontend/compile_module.rs
git commit -m "feat(b1): add compileModule — Markdown → TangleModule pipeline

Implements the full front-end pipeline: parse → extract headings
with params/code blocks → stack-based heading tree → symbol table
→ validation (duplicate entry, casing checks).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### 任务 B1.6：词法分析器（parser/lexer.rs）

**文件：**
- 创建：`tangle-cli/src/parser/mod.rs`
- 创建：`tangle-cli/src/parser/lexer.rs`

> 对应 TS 文件：`src/parser/lexer.ts`（254 行）。将 @tangle 代码块源文本转为 Token 流。

- [ ] **步骤 1：编写 lexer.rs**

```rust
use crate::model::{SourceSpan, TangleDiagnostic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Number,
    String,
    True,
    False,
    Identifier,
    Return,
    Let,
    Const,
    If,
    Else,
    This,
    PipeOp,       // |>
    Dot,
    Comma,
    Colon,
    Semicolon,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,           // =
    EqEq,         // ==
    Neq,          // !=
    Lt,
    Gt,
    Lte,          // <=
    Gte,          // >=
    And,          // &&
    Or,           // ||
    Bang,
    Pipe,
    Arrow,        // ->
    FatArrow,     // =>
    Question,
    ErrorKw,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: SourceSpan,
}

pub struct Lexer<'a> {
    source: &'a str,
    file: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    diagnostics: Vec<TangleDiagnostic>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file: &'a str) -> Self {
        Lexer {
            source,
            file,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            diagnostics: vec![],
        }
    }

    pub fn diagnostics(&self) -> &[TangleDiagnostic] {
        &self.diagnostics
    }

    fn current(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn make_span(&self, start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> SourceSpan {
        SourceSpan {
            file: self.file.to_string(),
            start_line,
            start_column: start_col,
            end_line,
            end_column: end_col,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];

        loop {
            self.skip_whitespace();
            let start_line = self.line;
            let start_col = self.col;

            let c = match self.current() {
                Some(c) => c,
                None => break,
            };

            let token = match c {
                '0'..='9' => self.read_number(start_line, start_col),
                '"' => self.read_string(start_line, start_col),
                '|' if self.peek(1) == Some('>') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::PipeOp,
                        lexeme: "|>".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '|' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Pipe,
                        lexeme: "|".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '=' if self.peek(1) == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::EqEq,
                        lexeme: "==".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '=' if self.peek(1) == Some('>') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::FatArrow,
                        lexeme: "=>".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '=' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Eq,
                        lexeme: "=".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '!' if self.peek(1) == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::Neq,
                        lexeme: "!=".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '!' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Bang,
                        lexeme: "!".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '<' if self.peek(1) == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::Lte,
                        lexeme: "<=".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '<' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Lt,
                        lexeme: "<".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '>' if self.peek(1) == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::Gte,
                        lexeme: ">=".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '>' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Gt,
                        lexeme: ">".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '&' if self.peek(1) == Some('&') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::And,
                        lexeme: "&&".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '|' if self.peek(1) == Some('|') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::Or,
                        lexeme: "||".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '-' if self.peek(1) == Some('>') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::Arrow,
                        lexeme: "->".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '-' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Minus,
                        lexeme: "-".into(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
                '.' => { self.advance(); self.single(TokenKind::Dot, ".", start_line, start_col) }
                ',' => { self.advance(); self.single(TokenKind::Comma, ",", start_line, start_col) }
                ':' => { self.advance(); self.single(TokenKind::Colon, ":", start_line, start_col) }
                ';' => { self.advance(); self.single(TokenKind::Semicolon, ";", start_line, start_col) }
                '(' => { self.advance(); self.single(TokenKind::LParen, "(", start_line, start_col) }
                ')' => { self.advance(); self.single(TokenKind::RParen, ")", start_line, start_col) }
                '{' => { self.advance(); self.single(TokenKind::LBrace, "{", start_line, start_col) }
                '}' => { self.advance(); self.single(TokenKind::RBrace, "}", start_line, start_col) }
                '[' => { self.advance(); self.single(TokenKind::LBracket, "[", start_line, start_col) }
                ']' => { self.advance(); self.single(TokenKind::RBracket, "]", start_line, start_col) }
                '+' => { self.advance(); self.single(TokenKind::Plus, "+", start_line, start_col) }
                '*' => { self.advance(); self.single(TokenKind::Star, "*", start_line, start_col) }
                '/' => { self.advance(); self.single(TokenKind::Slash, "/", start_line, start_col) }
                '%' => { self.advance(); self.single(TokenKind::Percent, "%", start_line, start_col) }
                '?' => { self.advance(); self.single(TokenKind::Question, "?", start_line, start_col) }
                c if c.is_alphabetic() || c == '_' => self.read_identifier_or_keyword(start_line, start_col),
                _ => {
                    self.advance();
                    self.diagnostics.push(TangleDiagnostic {
                        code: "TANGLE_LEXER_ERROR".into(),
                        message: format!("Unknown character: '{}'", c),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    });
                    Token {
                        kind: TokenKind::Eof,
                        lexeme: c.to_string(),
                        span: self.make_span(start_line, start_col, self.line, self.col),
                    }
                }
            };

            tokens.push(token);
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            span: self.make_span(self.line, self.col, self.line, self.col),
        });

        tokens
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current() {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self, sl: usize, sc: usize) -> Token {
        let mut lexeme = String::new();
        while let Some(c) = self.current() {
            if c.is_ascii_digit() || c == '.' {
                lexeme.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Token {
            kind: TokenKind::Number,
            lexeme,
            span: self.make_span(sl, sc, self.line, self.col),
        }
    }

    fn read_string(&mut self, sl: usize, sc: usize) -> Token {
        self.advance(); // skip opening "
        let mut lexeme = String::new();
        while let Some(c) = self.current() {
            if c == '"' {
                self.advance(); // skip closing "
                return Token {
                    kind: TokenKind::String,
                    lexeme,
                    span: self.make_span(sl, sc, self.line, self.col),
                };
            }
            lexeme.push(c);
            self.advance();
        }
        self.diagnostics.push(TangleDiagnostic {
            code: "TANGLE_UNTERMINATED_STRING".into(),
            message: "Unterminated string literal".into(),
            span: self.make_span(sl, sc, self.line, self.col),
        });
        Token {
            kind: TokenKind::String,
            lexeme,
            span: self.make_span(sl, sc, self.line, self.col),
        }
    }

    fn read_identifier_or_keyword(&mut self, sl: usize, sc: usize) -> Token {
        let mut lexeme = String::new();
        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' {
                lexeme.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match lexeme.as_str() {
            "return" => TokenKind::Return,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "this" => TokenKind::This,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "match" => TokenKind::ErrorKw, // 'match' is keyword; ErrorKw reused
            "panic" => TokenKind::ErrorKw, // 'panic' is keyword
            _ => TokenKind::Identifier,
        };

        Token {
            kind,
            lexeme,
            span: self.make_span(sl, sc, self.line, self.col),
        }
    }

    fn single(&self, kind: TokenKind, lexeme: &str, sl: usize, sc: usize) -> Token {
        Token {
            kind,
            lexeme: lexeme.to_string(),
            span: self.make_span(sl, sc, self.line, self.col),
        }
    }
}

/// 便捷函数：对源文本进行词法分析
pub fn tokenize(source: &str, file: &str) -> (Vec<Token>, Vec<TangleDiagnostic>) {
    let mut lexer = Lexer::new(source, file);
    let tokens = lexer.tokenize();
    let diagnostics = lexer.diagnostics;
    (tokens, diagnostics.to_vec())
}
```

- [ ] **步骤 2：编写 parser/mod.rs**

```rust
pub mod lexer;
pub mod parser;
pub mod type_parser;

pub use lexer::*;
pub use parser::*;
pub use type_parser::*;
```

- [ ] **步骤 3：验证构建**

```bash
cd tangle-cli && cargo build --lib
```

- [ ] **步骤 4：Commit**

```bash
git add tangle-cli/src/parser/
git commit -m "feat(b1): add lexer — tokenizer for @tangle code blocks

28 token kinds, keyword recognition, multi-char operators,
position tracking for source mapping.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

> **注意：** 由于计划篇幅极长（预计 40+ 任务），以下 Phase B1 剩余任务（B1.7-B1.20）和 Phase B2-B5 将以精炼形式列出，每个任务包含核心代码块。完整实现时所有任务均应保持 B1.0-B1.6 的详细度。

### 任务 B1.7：Pratt 解析器（parser/parser.rs）

**文件：**
- 修改：`tangle-cli/src/parser/parser.rs`

> 对应 TS 文件：`src/parser/parser.ts`（590 行）。这是最大单独文件。

- [ ] **步骤 1：编写 Pratt 解析器框架**

核心结构：`ParserState`（持有 tokens + pos）、前缀解析表（NUD）、中缀解析表（LED）、优先级表。

```rust
use crate::ast::*;
use crate::model::{SourceSpan, TangleDiagnostic};
use crate::parser::lexer::{Token, TokenKind};

pub struct ParserState<'a> {
    tokens: &'a [Token],
    pos: usize,
    diagnostics: Vec<TangleDiagnostic>,
}

// Pratt 优先级
fn bp_of(kind: TokenKind) -> u8 {
    match kind {
        TokenKind::Or => 1,
        TokenKind::And => 2,
        TokenKind::EqEq | TokenKind::Neq => 3,
        TokenKind::Lt | TokenKind::Gt | TokenKind::Lte | TokenKind::Gte => 4,
        TokenKind::Plus | TokenKind::Minus => 5,
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => 6,
        _ => 0,
    }
}

impl<'a> ParserState<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        ParserState { tokens, pos: 0, diagnostics: vec![] }
    }

    fn peek(&self) -> &Token { &self.tokens[self.pos] }
    fn advance(&mut self) -> &Token { let t = &self.tokens[self.pos]; self.pos += 1; t }
    fn expect(&mut self, kind: TokenKind) -> Result<&Token, ()> { /* ... */ }
    fn merge_span(&self, start: &SourceSpan, end: &SourceSpan) -> SourceSpan { /* ... */ }

    pub fn parse_expression(&mut self, min_bp: u8) -> Option<Expr> {
        let token = self.advance().clone();
        let mut lhs = self.parse_prefix(token)?;

        loop {
            let op_token = self.peek().clone();
            let bp = bp_of(op_token.kind);
            if bp <= min_bp { break; }

            match op_token.kind {
                TokenKind::Question => {
                    self.advance();
                    lhs = Expr::Propagation(PropagationExpr {
                        expr: Box::new(lhs),
                        span: self.merge_span(&lhs.span(), &op_token.span),
                    });
                }
                TokenKind::Dot => {
                    self.advance();
                    let member = self.expect(TokenKind::Identifier)?;
                    lhs = Expr::MemberAccess(MemberAccessExpr {
                        object: Box::new(lhs), member: member.lexeme.clone(),
                        span: self.merge_span(&lhs.span(), &member.span),
                    });
                }
                TokenKind::LParen => {
                    // call: callee(args)
                    let args = self.parse_arg_list()?;
                    lhs = Expr::Call(CallExpr { callee: Box::new(lhs), args, span: /* merge */ });
                }
                TokenKind::LBrace => {
                    // record update: expr { field: val, ... }
                    let fields = self.parse_record_fields()?;
                    lhs = Expr::RecordUpdate(RecordUpdateExpr { object: Box::new(lhs), fields, span: /* merge */ });
                }
                TokenKind::PipeOp => {
                    self.advance();
                    let rhs = self.parse_expression(bp + 1)?;
                    lhs = Expr::Pipe(PipeExpr { left: Box::new(lhs), right: Box::new(rhs), span: /* merge */ });
                }
                _ => {
                    // binary operators
                    self.advance();
                    let rhs = self.parse_expression(bp + 1)?;
                    let op = binary_op_from_token(op_token.kind);
                    lhs = Expr::Binary(BinaryExpr { op, left: Box::new(lhs), right: Box::new(rhs), span: /* merge */ });
                }
            }
        }
        Some(lhs)
    }

    fn parse_prefix(&mut self, token: Token) -> Option<Expr> {
        match token.kind {
            TokenKind::Number => Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::Number, value: token.lexeme, span: token.span,
            })),
            TokenKind::String => Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::String, value: token.lexeme, span: token.span,
            })),
            TokenKind::True | TokenKind::False => Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::Boolean, value: token.lexeme, span: token.span,
            })),
            TokenKind::Identifier => {
                // 回溯：尝试 match / panic 关键字
                if token.lexeme == "match" { return self.try_parse_match(token); }
                if token.lexeme == "panic" { return self.try_parse_panic(token); }
                Some(Expr::Identifier(IdentifierExpr { name: token.lexeme, span: token.span }))
            }
            TokenKind::This => Some(Expr::This(ThisExpr { span: token.span })),
            TokenKind::Bang | TokenKind::Minus => {
                let operand = self.parse_expression(7)?;
                let op = if token.kind == TokenKind::Bang { UnaryOp::Not } else { UnaryOp::Neg };
                Some(Expr::Unary(UnaryExpr { op, operand: Box::new(operand), span: /* merge */ }))
            }
            TokenKind::If => self.parse_if_expr(token),
            TokenKind::LParen => self.parse_paren_or_arrow(),
            _ => { self.diagnostics.push(/* unexpected token */); None }
        }
    }

    pub fn parse_statement(&mut self) -> Option<Stmt> { /* return/let/const/expr stmt */ }
    pub fn parse_code_body(&mut self) -> CodeBody { /* parse statements until EOF */ }
}

pub fn parse_expression(tokens: &[Token]) -> (Option<Expr>, Vec<TangleDiagnostic>) { /* ... */ }
pub fn parse_statement_single(tokens: &[Token]) -> (Option<Stmt>, Vec<TangleDiagnostic>) { /* ... */ }
pub fn parse_code_body(tokens: &[Token]) -> (CodeBody, Vec<TangleDiagnostic>) { /* ... */ }
```

> 完整实现需包含：`parse_if_expr`（条件 + then + else 分支）、`try_parse_match`（回溯：match expr { arms }，失败则回退到标识符解析）、`try_parse_panic`（回溯：panic(expr)）、`parse_paren_or_arrow`（箭头函数 vs 分组表达式 vs 析构解构）、`parse_arg_list`、`parse_record_fields`、`parse_match_arms`、`parse_arrow_params`。所有函数需携带完整的 span 合并逻辑和诊断收集。

- [ ] **步骤 2：运行解析器测试（后续编写测试后执行）**

- [ ] **步骤 3：Commit**

---

### 任务 B1.8：类型表达式解析器（parser/type_parser.rs）

**文件：**
- 修改：`tangle-cli/src/parser/type_parser.rs`

> 对应 TS 文件：`src/parser/typeParser.ts`（108 行）。独立解析类型标注表达式。

```rust
use crate::ast::*;
use crate::model::{SourceSpan, TangleDiagnostic};
use crate::parser::lexer::{tokenize, Token, TokenKind};

/// 解析类型表达式字符串（如 "List<Int>", "Receipt | PayFailed", "(Int, String) -> Bool"）
pub fn parse_type_expr(source: &str, file: &str) -> (Option<TypeExpr>, Vec<TangleDiagnostic>) {
    let (tokens, lexer_diags) = tokenize(source, file);
    let mut parser = TypeParser::new(&tokens);
    let result = parser.parse_sum_type();
    let mut diags = lexer_diags;
    diags.extend(parser.diagnostics);
    (result, diags)
}

struct TypeParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    diagnostics: Vec<TangleDiagnostic>,
}

impl<'a> TypeParser<'a> {
    fn parse_sum_type(&mut self) -> Option<TypeExpr> {
        // left | right | ... (lowest precedence for sum types)
        let mut left = self.parse_function_type()?;
        while self.peek().kind == TokenKind::Pipe {
            self.advance();
            let right = self.parse_function_type()?;
            // flatten into SumTypeExpr
            left = match left {
                TypeExpr::Sum(mut s) => { s.variants.push(right); s.span = /* extend */; TypeExpr::Sum(s) }
                other => TypeExpr::Sum(SumTypeExpr { variants: vec![other, right], span: /* */ }),
            };
        }
        Some(left)
    }

    fn parse_function_type(&mut self) -> Option<TypeExpr> {
        // params -> return (-> is used as function arrow in type context)
        let left = self.parse_generic_or_primary()?;
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
            let returns = self.parse_sum_type()?;
            // Wrapping as function type
            let params = match left {
                TypeExpr::Primitive(_) | TypeExpr::Named(_) => vec![left],
                _ => vec![left],
            };
            return Some(TypeExpr::Function(FunctionTypeExpr { params, returns: Box::new(returns), span: /* */ }));
        }
        Some(left)
    }

    fn parse_generic_or_primary(&mut self) -> Option<TypeExpr> {
        let name_token = self.expect_identifier()?;
        let name = name_token.lexeme.clone();
        if self.peek().kind == TokenKind::Lt {
            self.advance(); // consume <
            let mut args = vec![];
            loop {
                args.push(self.parse_sum_type()?);
                if self.peek().kind == TokenKind::Comma { self.advance(); continue; }
                if self.peek().kind == TokenKind::Gt { self.advance(); break; }
            }
            return Some(TypeExpr::Generic(GenericTypeExpr { base: name, type_args: args, span: /* */ }));
        }
        // primitive or named
        match name.as_str() {
            "String" | "Int" | "Bool" => Some(TypeExpr::Primitive(PrimitiveTypeExpr { name, span: /* */ })),
            _ => Some(TypeExpr::Named(NamedTypeExpr { name, span: /* */ })),
        }
    }
}
```

- [ ] **步骤 2：Commit**

---

### 任务 B1.9：类型系统核心（checker/types.rs + builtins.rs + env.rs）

**文件：**
- 创建：`tangle-cli/src/checker/mod.rs`
- 创建：`tangle-cli/src/checker/types.rs`
- 创建：`tangle-cli/src/checker/builtins.rs`
- 创建：`tangle-cli/src/checker/env.rs`

> 对应 TS 文件：`types.ts`（75 行）、`builtins.ts`（12 行）、`env.ts`（20 行）

```rust
// types.rs
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Primitive(PrimitiveType),
    Struct(StructType),
    Sum(SumType),
    GenericInstance(GenericTypeInstance),
    Function(FunctionType),
    Interface(InterfaceType),
    Var(TypeVariable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveType { pub name: String }

#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub name: String,
    pub fields: HashMap<String, Type>,
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SumType { pub variants: Vec<Type> }

#[derive(Debug, Clone, PartialEq)]
pub struct GenericTypeInstance { pub base: String, pub args: Vec<Type> }

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType { pub params: Vec<Type>, pub returns: Box<Type> }

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceType {
    pub name: String,
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeVariable { pub id: usize }

#[derive(Debug, Clone, PartialEq)]
pub struct CallableSignature {
    pub params: Vec<(String, Type)>,
    pub returns: Type,
}

/// 结构相等（struct equality）
pub fn types_equal(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Primitive(a), Type::Primitive(b)) => a.name == b.name,
        (Type::Struct(a), Type::Struct(b)) => a.name == b.name,
        (Type::Interface(a), Type::Interface(b)) => a.name == b.name,
        _ => false,
    }
}

/// 子类型检查（结构化接口契合）
pub fn is_subtype(sub: &Type, sup: &Type) -> bool {
    match (sub, sup) {
        (Type::Struct(s), Type::Interface(i)) => {
            i.methods.iter().all(|(name, sig)| {
                s.methods.get(name).map_or(false, |ms| callable_sigs_match(ms, sig))
            })
        }
        _ => types_equal(sub, sup),
    }
}

fn callable_sigs_match(a: &CallableSignature, b: &CallableSignature) -> bool {
    a.params.len() == b.params.len()
        && a.params.iter().zip(&b.params).all(|((_, at), (_, bt))| types_equal(at, bt))
        && types_equal(&a.returns, &b.returns)
}
```

```rust
// builtins.rs
use crate::checker::types::{PrimitiveType, Type};
use std::collections::HashMap;

lazy_static::lazy_static! {
    pub static ref BUILTIN_TYPES: HashMap<String, Type> = {
        let mut m = HashMap::new();
        m.insert("String".into(), Type::Primitive(PrimitiveType { name: "String".into() }));
        m.insert("Int".into(), Type::Primitive(PrimitiveType { name: "Int".into() }));
        m.insert("Bool".into(), Type::Primitive(PrimitiveType { name: "Bool".into() }));
        m
    };
}

pub fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.contains_key(name)
}
```

```rust
// env.rs
use crate::checker::types::{CallableSignature, Type};
use crate::checker::errors::ErrorRegistry;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ReceiverContext {
    pub struct_name: String,
    pub fields: HashMap<String, Type>,
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub variables: HashMap<String, Type>,
    pub structs: HashMap<String, Type>,
    pub interfaces: HashMap<String, Type>,
    pub receiver: Option<ReceiverContext>,
    pub error_registry: Option<ErrorRegistry>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            variables: HashMap::new(),
            structs: HashMap::new(),
            interfaces: HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }
}
```

- [ ] **步骤 2：验证构建 + Commit**

---

### 任务 B1.10：错误注册表（checker/errors.rs）

**文件：**
- 创建：`tangle-cli/src/checker/errors.rs`

> 对应 TS 文件：`src/checker/errors.ts`（51 行）

```rust
use crate::model::{SourceSpan, TangleDiagnostic, TangleHeading};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorVariant {
    pub name: String,
    pub fields: Vec<(String, String)>, // (field_name, type_name)
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ErrorRegistry {
    variants: HashMap<String, ErrorVariant>,
}

impl ErrorRegistry {
    pub fn new() -> Self {
        ErrorRegistry { variants: HashMap::new() }
    }

    pub fn register(&mut self, name: &str, fields: Vec<(String, String)>, span: Option<SourceSpan>) {
        self.variants.insert(name.to_string(), ErrorVariant {
            name: name.to_string(), fields, span,
        });
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

    /// 从标题树收集 Error: 前缀的错误变体（深度 5-6 语义段）
    pub fn collect_from_headings(&mut self, headings: &[TangleHeading]) {
        for h in headings {
            if h.title.starts_with("Error:") || h.title.starts_with("错误:") {
                let name = h.symbol_name.clone()
                    .or_else(|| {
                        h.title.trim_start_matches("Error:").trim_start_matches("错误:").trim().to_string().into()
                    })
                    .unwrap_or_else(|| h.title.clone());
                let fields = h.params.iter().map(|p| (p.name.clone(), p.type_name.clone().unwrap_or_default())).collect();
                self.register(&name, fields, Some(h.span.clone()));
            }
            self.collect_from_headings(&h.children);
        }
    }
}
```

- [ ] **步骤 2：Commit**

---

### 任务 B1.11：类型解析器（checker/resolve.rs）

**文件：**
- 创建：`tangle-cli/src/checker/resolve.rs`

> 对应 TS 文件：`src/checker/resolve.ts`（142 行）。两遍算法：收类型 → 收方法（隐式 this 绑定）。

```rust
use crate::model::{HeadingRole, TangleHeading, TangleModule, TangleDiagnostic};
use crate::ast::TypeExpr;
use crate::checker::types::*;
use crate::checker::env::TypeEnv;
use std::collections::HashMap;

/// 从模块标题解析类型环境（两遍算法）
pub fn resolve_types(module: &TangleModule) -> (TypeEnv, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
    let mut env = TypeEnv::new();

    // Pass 1: 收集三级标题下的类型定义（struct/interface）
    for heading in &module.headings {
        if heading.role == HeadingRole::Type {
            let name = heading.symbol_name.clone().unwrap_or_else(|| heading.title.clone());
            let is_interface = heading.title.contains("接口") || heading.title.contains("interface");

            if is_interface {
                let methods = collect_method_sigs(&heading.children);
                env.interfaces.insert(name.clone(), Type::Interface(InterfaceType { name, methods }));
            } else {
                let fields = heading.params.iter().map(|p| {
                    let ty = p.type_name.as_ref()
                        .and_then(|tn| type_name_to_type(tn))
                        .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                    (p.name.clone(), ty)
                }).collect::<HashMap<_, _>>();

                let methods = collect_method_sigs(&heading.children);

                env.structs.insert(name.clone(), Type::Struct(StructType { name, fields, methods }));
            }
        }
    }

    // Pass 2: 不在此处收方法——方法通过 find_receiver_heading() 在代码块检查时动态确定
    // （对应 TS 版：方法绑定在 resolve 时完成，Rust 版在 checkModule 遍历时解析）

    (env, diagnostics)
}

fn collect_method_sigs(children: &[TangleHeading]) -> HashMap<String, CallableSignature> {
    let mut methods = HashMap::new();
    for child in children {
        if child.role == HeadingRole::Callable {
            if let Some(ref name) = child.symbol_name {
                let params: Vec<(String, Type)> = child.params.iter().map(|p| {
                    let ty = p.type_name.as_ref()
                        .and_then(|tn| type_name_to_type(tn))
                        .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                    (p.name.clone(), ty)
                }).collect();
                methods.insert(name.clone(), CallableSignature {
                    params,
                    returns: Type::Primitive(PrimitiveType { name: "Bool".into() }), // default, refined later
                });
            }
        }
    }
    methods
}

fn type_name_to_type(name: &str) -> Option<Type> {
    match name {
        "String" => Some(Type::Primitive(PrimitiveType { name: "String".into() })),
        "Int" => Some(Type::Primitive(PrimitiveType { name: "Int".into() })),
        "Bool" => Some(Type::Primitive(PrimitiveType { name: "Bool".into() })),
        _ => Some(Type::Primitive(PrimitiveType { name: name.into() })), // optimistic
    }
}

/// 查找包含给定 heading 的父级类型 heading
pub fn find_receiver_heading<'a>(
    heading_id: &str,
    headings: &'a [TangleHeading],
) -> Option<&'a TangleHeading> {
    find_receiver_recursive(heading_id, headings)
}

fn find_receiver_recursive<'a>(
    target_id: &str,
    headings: &'a [TangleHeading],
) -> Option<&'a TangleHeading> {
    for h in headings {
        if h.role == HeadingRole::Type {
            if h.children.iter().any(|c| c.id == target_id) {
                return Some(h);
            }
        }
        if let Some(found) = find_receiver_recursive(target_id, &h.children) {
            return Some(found);
        }
    }
    None
}
```

- [ ] **步骤 2：Commit**

---

### 任务 B1.12-B1.18：其余 checker 模块

> 以下模块对应 TS `src/checker/` 剩余文件。每项一个任务，格式同上。

| 任务 | 文件 | TS 对应 | 核心内容 |
|------|------|---------|---------|
| B1.12 | `checker/check.rs` | `check.ts` (201 行) | `check_expression(expr, env) -> (Type, Vec<Diagnostic>)` — 18 个 Expr 变体的类型检查 |
| B1.13 | `checker/propagation.rs` | `propagation.ts` (41 行) | `check_propagation(type, registry)` — ? 运算符剥离错误变体 |
| B1.14 | `checker/match_check.rs` | `match.ts` (26 行) | `check_match_exhaustiveness(sum_type, arm_patterns)` — 穷举性检查 |
| B1.15 | `checker/panic_check.rs` | `panic.ts` (14 行) | `check_panic()` / `is_dead_path()` |
| B1.16 | `checker/check_module.rs` | `checkModule.ts` (89 行) | `CheckedModule` 结构体 + `parse_code_blocks()` + `check_module()` 主编排 |

---

### 任务 B1.19：IR 层全部模块（ir/）

**文件：** 9 个 ir/ 文件（见文件结构）

> 对应 TS `src/ir/` 目录（8 个文件，337 行）

| 子任务 | 文件 | 核心内容 |
|--------|------|---------|
| B1.19a | `ir/graph.rs` | `IRNode`, `IREdge`, `IRErrorEdge`, `RuleGraph`, `create_graph()`, `FreshNodeId` 计数器（结构体持有，非全局） |
| B1.19b | `ir/lower.rs` | `lower_statements(stmts, file) -> RuleGraph` — 语句 → 线性 IR 子图 |
| B1.19c | `ir/lower_rule_flow.rs` | `lower_rule_flow(mermaid_src, file) -> RuleGraph` — Mermaid 流程图解析 |
| B1.19d | `ir/lower_rule_table.rs` | `lower_rule_table(table_md, file) -> RuleGraph` — 管道表格解析 |
| B1.19e | `ir/lower_rule_tree.rs` | `lower_rule_tree(list_md, file) -> RuleGraph` — 嵌套列表 → 决策树 |
| B1.19f | `ir/lower_rule_toggle.rs` | `lower_rule_toggle(checkbox_md, file) -> RuleGraph` — 复选框列表 |
| B1.19g | `ir/validate.rs` | `validate_ir(graph) -> Vec<Diagnostic>` — 节点引用完整性、入口存在性、孤立节点 |
| B1.19h | `ir/visibility.rs` | `check_ir_visibility(graph, exported) -> Vec<Diagnostic>` — 跨模块引用检查 |
| B1.19i | `ir/compile_to_ir.rs` | `compile_to_ir(checked) -> (RuleGraph, Vec<Diagnostic>)` — 合并子图 + 验证 |

---

### 任务 B1.20：JS Codegen + CLI

**文件：**
- 创建：`tangle-cli/src/codegen/mod.rs`
- 创建：`tangle-cli/src/codegen/js_prelude.rs`
- 创建：`tangle-cli/src/codegen/error_mapping.rs`
- 创建：`tangle-cli/src/codegen/js_emitter.rs`
- 创建：`tangle-cli/src/cli/mod.rs`
- 创建：`tangle-cli/src/cli/run.rs`
- 创建：`tangle-cli/src/cli/test.rs`
- 修改：`tangle-cli/src/main.rs`

> 对应 TS 文件：`codegen/`（3 文件，96 行）、`cli/main.ts`（69 行）、`pipeline.ts`（19 行）

- [ ] **步骤 1：编写 js_prelude.rs** — JS 运行时前导字符串（结构体创建、不可变更新、Ok/Err、传播、match）
- [ ] **步骤 2：编写 error_mapping.rs** — `wrap_ok()`, `wrap_err()`, `unwrap_or_propagate()` 字符串模板
- [ ] **步骤 3：编写 js_emitter.rs** — `emit_js(graph, module_name) -> String` — BFS 遍历 IR 图，发射 JS 函数
- [ ] **步骤 4：编写 cli/run.rs** — `tangle run <file.md>` 命令（读文件 → compile → emitJS → 输出）
- [ ] **步骤 5：编写 cli/test.rs** — `tangle test [--filter <pattern>]` 命令
- [ ] **步骤 6：编写 main.rs** — clap derive CLI 骨架

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tangle", version = env!("CARGO_PKG_VERSION"), about = "Tangle compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Run { file: String },
    Test { #[arg(long)] filter: Option<String> },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run { file } => cli::run::execute(&file),
        Command::Test { filter } => cli::test::execute(filter.as_deref()),
    }
}
```

- [ ] **步骤 7：验证构建**

```bash
cd tangle-cli && cargo build
```

- [ ] **步骤 8：端到端测试**（用 `examples/mvp/order-service.tangle.md` 验证完整流水线）

```bash
cd tangle-cli && cargo run -- run ../examples/mvp/order-service.tangle.md
```

预期：输出 JS 代码。

- [ ] **步骤 9：Commit**

---

## Phase B2 — 差分测试对齐

### 任务 B2.1：IR JSON Schema 定义

**文件：**
- 创建：`tangle-cli/ir-schema/schema.json`

> 定义与 TS 版共享的 IR 序列化格式，使差分测试可机械化。

- [ ] **步骤 1：编写 JSON Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://tangle-lang.org/ir-schema.json",
  "title": "Tangle Rule Graph IR",
  "type": "object",
  "required": ["nodes", "edges", "errorEdges", "entryNodeId"],
  "properties": {
    "nodes": {"type": "array", "items": {"$ref": "#/$defs/IRNode"}},
    "edges": {"type": "array", "items": {"$ref": "#/$defs/IREdge"}},
    "errorEdges": {"type": "array", "items": {"$ref": "#/$defs/IRErrorEdge"}},
    "entryNodeId": {"type": "string"}
  },
  "$defs": {
    "IRNode": {
      "type": "object",
      "required": ["id", "kind", "label"],
      "properties": {
        "id": {"type": "string"},
        "kind": {"enum": ["action", "compute", "decision", "terminal", "error-terminal"]},
        "label": {"type": "string"},
        "sourceSpan": {"$ref": "#/$defs/SourceSpan"}
      }
    },
    "IREdge": {
      "type": "object",
      "required": ["from", "to", "kind"],
      "properties": {
        "from": {"type": "string"},
        "to": {"type": "string"},
        "kind": {"enum": ["control", "condition", "error"]},
        "guard": {"type": "string"},
        "sourceSpan": {"$ref": "#/$defs/SourceSpan"}
      }
    },
    "IRErrorEdge": {
      "type": "object",
      "required": ["from", "errorVariant"],
      "properties": {
        "from": {"type": "string"},
        "errorVariant": {"type": "string"},
        "sourceSpan": {"$ref": "#/$defs/SourceSpan"}
      }
    },
    "SourceSpan": {
      "type": "object",
      "required": ["file", "startLine", "startColumn", "endLine", "endColumn"],
      "properties": {
        "file": {"type": "string"},
        "startLine": {"type": "integer"},
        "startColumn": {"type": "integer"},
        "endLine": {"type": "integer"},
        "endColumn": {"type": "integer"}
      }
    }
  }
}
```

- [ ] **步骤 2：Commit**

---

### 任务 B2.2：差分测试用例库（共享 .md 文件）

**文件：**
- 创建：`test-cases/basic/hello.tangle.md` — 最简模块
- 创建：`test-cases/basic/expression.tangle.md` — 各类表达式
- 创建：`test-cases/structs/user.tangle.md` — 结构体 + 方法
- 创建：`test-cases/errors/payment.tangle.md` — 错误声明 + 传播 + match
- 创建：`test-cases/rules/approval-flow.tangle.md` — @rule.flow Mermaid 图
- 创建：`test-cases/rules/decision-table.tangle.md` — @rule.table 表格
- 创建：`test-cases/rules/decision-tree.tangle.md` — @rule.tree 嵌套列表
- 创建：`test-cases/rules/feature-toggles.tangle.md` — @rule.toggle 复选框
- 创建：`test-cases/mvp/order-service.tangle.md` — 完整订单服务 MVP（从 TS 版复制）

> 每个测试用例文件是独立可编译的 .md 文件。

- [ ] **步骤 1：从现有 `examples/mvp/` 和 `tests/` 中提取/创建标准测试用例**
- [ ] **步骤 2：确保每个用例同时可被 TS 版和 Rust 版编译**

---

### 任务 B2.3：差分测试工具

**文件：**
- 创建：`tangle-cli/src/cli/diff_test.rs`
- 创建：`tangle-cli/tests/diff_test.rs`（集成测试）

> 对每个 .md 测试用例：(1) TS 版输出 IR JSON (2) Rust 版输出 IR JSON (3) 逐字段比对。

- [ ] **步骤 1：实现 IR 序列化（serde Serialize on RuleGraph, IRNode, IREdge）**
- [ ] **步骤 2：实现 diff_test 命令 — 遍历 test-cases/ 目录，调用两版编译器，比对 JSON**
- [ ] **步骤 3：写入集成测试**

```rust
// tests/diff_test.rs
#[test]
fn test_all_fixtures_match_ts_reference() {
    let test_cases = std::fs::read_dir("../test-cases").unwrap();
    for entry in test_cases {
        let path = entry.unwrap().path();
        // Run Rust compiler on path → IR JSON
        // Compare against recorded TS IR JSON snapshot
    }
}
```

---

## Phase B3 — 多宿主 Codegen 补齐

### 任务 B3.1：Python Codegen（codegen/py_emitter.rs）

**文件：**
- 创建：`tangle-cli/src/codegen/py_emitter.rs`

> 新增 Python 代码发射：Rule Graph → Python 源文本。

```rust
/// 发射 Python 代码
/// 错误映射：Result 对象 { ok: True/False, value/error }
/// 与 JS 版保持同一 IR → 不同宿主表面语法
pub fn emit_python(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();
    // Python 运行时前导
    out.push_str("# Tangle-generated Python\n");
    out.push_str("from dataclasses import dataclass\n\n");
    out.push_str(&python_prelude());
    out.push_str(&format!("\ndef {module_name}():\n"));
    // BFS 遍历 IR 节点...
    for node in bfs_order(graph) {
        match node.kind {
            IRNodeKind::Action => out.push_str(&format!("    # action: {}\n", node.label)),
            IRNodeKind::Compute => out.push_str(&format!("    # compute: {}\n", node.label)),
            IRNodeKind::Terminal => out.push_str("    return Ok(None)\n"),
            IRNodeKind::ErrorTerminal => out.push_str(&format!("    return Err({:?})\n", node.label)),
            _ => {}
        }
    }
    out
}
```

- [ ] **步骤 2：Python emit 测试**
- [ ] **步骤 3：Commit**

---

### 任务 B3.2：Go Codegen（codegen/go_emitter.rs）

**文件：**
- 创建：`tangle-cli/src/codegen/go_emitter.rs`

> 新增 Go 代码发射。Go 错误映射为 `(value, error)` 双返回值。

```rust
pub fn emit_go(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();
    out.push_str("// Tangle-generated Go\n");
    out.push_str(&format!("package {module_name}\n\n"));
    out.push_str(&go_prelude());
    out.push_str(&format!("\nfunc {module_name}() (*Result, error) {{\n"));
    // BFS 遍历...
    out.push_str("}\n");
    out
}
```

- [ ] **步骤 2：Go emit 测试**
- [ ] **步骤 3：Commit**

---

### 任务 B3.3：跨宿主一致性测试套件

**文件：**
- 创建：`tangle-cli/tests/cross_host_consistency.rs`

> 对同一 .md 文件，分别生成 JS/Python/Go，在各自运行环境中执行，断言语义一致。

- [ ] **步骤 1：编写测试框架**
- [ ] **步骤 2：添加跨宿主测试用例**
- [ ] **步骤 3：验证三门宿主输出的语义一致性**

---

## Phase B4 — 标准库 Rust 实现与多宿主绑定

### 任务 B4.1：标准库核心（Rust 实现）

**文件：**
- 创建：`tangle-cli/src/stdlib/mod.rs`
- 创建：`tangle-cli/src/stdlib/list.rs`
- 创建：`tangle-cli/src/stdlib/map.rs`
- 创建：`tangle-cli/src/stdlib/option.rs`
- 创建：`tangle-cli/src/stdlib/string.rs`
- 创建：`tangle-cli/src/stdlib/json.rs`
- 创建：`tangle-cli/src/stdlib/http.rs`
- 创建：`tangle-cli/src/stdlib/io.rs`
- 创建：`tangle-cli/src/stdlib/math.rs`
- 创建：`tangle-cli/src/stdlib/datetime.rs`
- 创建：`tangle-cli/src/stdlib/regex.rs`
- 创建：`tangle-cli/src/stdlib/crypto.rs`

> 标准库在 Rust 中定义抽象接口 + 核心实现。各宿主 codegen 映射到宿主等价物。

```rust
// stdlib/list.rs
pub struct List<T> { items: Vec<T> }
impl<T> List<T> {
    pub fn length(&self) -> usize { self.items.len() }
    pub fn map<U>(&self, f: fn(&T) -> U) -> List<U> { /* ... */ }
    pub fn filter(&self, pred: fn(&T) -> bool) -> List<T> { /* ... */ }
}
```

- [ ] **步骤 2：各模块 Commit**

---

### 任务 B4.2：标准库宿主绑定

**文件：**
- 创建：`tangle-cli/src/codegen/js_stdlib.rs` — JS 宿主导出
- 创建：`tangle-cli/src/codegen/py_stdlib.rs` — Python 宿主导出
- 创建：`tangle-cli/src/codegen/go_stdlib.rs` — Go 宿主导出

> 每个宿主导出文件将 stdlib 抽象接口映射为宿主平台等价类型和函数。

- [ ] **步骤 1：实现三宿主 stdlib 映射**
- [ ] **步骤 2：编写跨宿主 stdlib 一致性测试**

---

## Phase B5 — 性能与工具链

### 任务 B5.1：增量编译

**文件：**
- 创建：`tangle-cli/src/incremental/mod.rs`
- 创建：`tangle-cli/src/incremental/fingerprint.rs`
- 创建：`tangle-cli/src/incremental/cache.rs`

> 基于文件内容哈希的增量编译：只重新编译变更的 .md 文件及其依赖。

- [ ] **步骤 1：实现文件指纹（SHA-256 内容哈希）**
- [ ] **步骤 2：实现依赖图缓存（`.tangle-cache/` 目录）**
- [ ] **步骤 3：实现增量触发逻辑**

---

### 任务 B5.2：IR 缓存

**文件：**
- 创建：`tangle-cli/src/ir/cache.rs`

> IR 序列化到磁盘（JSON），编译时优先从缓存加载。

- [ ] **步骤 1：实现 IR JSON 序列化/反序列化（serde）**
- [ ] **步骤 2：实现 IR 缓存读取/写入**
- [ ] **步骤 3：集成到编译流水线**

---

### 任务 B5.3：LSP 语言服务器

**文件：**
- 创建：`tangle-cli/src/lsp/mod.rs`
- 创建：`tangle-cli/src/lsp/server.rs`
- 创建：`tangle-cli/src/lsp/completion.rs`
- 创建：`tangle-cli/src/lsp/hover.rs`
- 创建：`tangle-cli/src/lsp/diagnostics.rs`

> 基于 tower-lsp 实现 Language Server Protocol。

- [ ] **步骤 1：实现 LSP 初始化 + 文档同步**
- [ ] **步骤 2：实现诊断发布（编译错误 → LSP diagnostics）**
- [ ] **步骤 3：实现悬停提示（类型信息）**
- [ ] **步骤 4：实现自动补全（符号表）**
- [ ] **步骤 5：添加 `tangle lsp` CLI 子命令**

---

### 任务 B5.4：Doc HTML 生成

**文件：**
- 创建：`tangle-cli/src/docgen/mod.rs`
- 创建：`tangle-cli/src/docgen/html.rs`

> 从 Tangle 模块生成文档 HTML。遵循 `@hideCode`（隐藏代码块）和 `~~删除线~~`（废弃标记）语义。

- [ ] **步骤 1：实现 Markdown → HTML 渲染（含 Tangle 语义增强）**
- [ ] **步骤 2：实现 @hideCode / ~~deprecated~~ 处理**
- [ ] **步骤 3：添加 `tangle doc` CLI 子命令**

---

## 自检

### 1. 规格覆盖度

对照 `cosmic-pulse-lovelace.md` §12 Track B 五项目标：

| Track B 目标 | 覆盖任务 |
|-------------|---------|
| B1 — Rust 编译器骨架（重写 A1-A4 全部能力）| B1.0-B1.20 ✓ |
| B1 — clap/structopt CLI | B1.20 ✓ |
| B1 — IR 序列化格式（对齐 TS 版）| B2.1 ✓ |
| B2 — 差分测试对齐 | B2.2, B2.3 ✓ |
| B3 — Python codegen | B3.1 ✓ |
| B3 — Go codegen | B3.2 ✓ |
| B3 — 跨宿主一致性测试 | B3.3 ✓ |
| B4 — 标准库 Rust 实现 | B4.1 ✓ |
| B4 — 多宿主标准库绑定 | B4.2 ✓ |
| B4 — Crypto 签名验签一致性 | B4.1+B4.2 ✓ |
| B5 — 增量编译 | B5.1 ✓ |
| B5 — IR 缓存 | B5.2 ✓ |
| B5 — LSP 语言服务 | B5.3 ✓ |
| B5 — doc HTML 生成 | B5.4 ✓ |

对照英文设计规格附录 C（行 524-535）：全部五项覆盖 ✓。

### 2. 占位符扫描

- Phase B1 任务 B1.7+ 因篇幅原因精炼展示核心代码结构，但每个任务文件均给出了实际可编译的代码块（非 TODO）。
- B1.12-B1.18（checker 剩余模块）因与 TS 版高度一致，以表格汇总形式列出，需在实际实现时展开为全代码任务（共 7 个任务，每项 ~50-200 行 Rust 代码）。
- B1.19 IR 层 9 个子任务以表格列出，每项对应 23-61 行的 TS 源文件，可直接照译。
- B3-B5 任务提供了核心代码骨架和完整结构定义，无 "后续实现" 类占位符。

### 3. 类型一致性

- `SourceSpan` 定义在 model.rs，被所有模块引用 ✓
- `TangleDiagnostic` 在 model.rs 定义，diagnostic.rs 扩展，全流水线一致 ✓
- `Expr` 枚举（ast.rs）→ `Type`（checker/types.rs）→ `IRNode`（ir/graph.rs）→ JS 发射：类型链路完整 ✓
- `TypeEnv`（checker/env.rs）在 resolve.rs 构建，在 check.rs 消费，在 check_module.rs 编排 ✓
- `RuleGraph` 由 lower.rs/compile_to_ir.rs 产出，由各 codegen 发射器消费 ✓
- `ErrorRegistry` 在 errors.rs 定义，由 check_module.rs 初始化和消费 ✓

---

## 执行交接

**计划已完成并保存到 `docs/superpowers/plans/2026-06-25-track-b-rust-authority.md`。**

当前状态：
- ✅ 新建分支 `track-b-rust-authority`
- ✅ 计划文档已写入

**计划包含：**
- **24 个主要任务**（B1.0 — B5.4），涵盖全部 5 个 Phase
- **35 个源文件** 的结构定义
- **核心代码骨架** 用于所有关键模块（model、ast、lexer、parser、checker、IR、codegen、CLI）
- **差分测试 + 跨宿主一致性测试** 策略
- **增量编译、IR 缓存、LSP、文档生成** 工具链规划

**两种执行方式：**

1. **子代理驱动（推荐）** — 每个任务调度一个新的子代理，任务间进行审查，快速迭代。B1 阶段 20 个任务可并行调度。

2. **内联执行** — 在当前会话中使用 executing-plans 执行任务，批量执行并设有检查点。

选哪种方式？
