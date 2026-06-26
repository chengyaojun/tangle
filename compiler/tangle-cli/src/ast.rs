use crate::model::SourceSpan;

// ============================================================
// 表达式 (15 variants)
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
// 语句 (4 variants)
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
// 类型表达式 (用于类型标注解析, 5 variants)
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
// 解析后的代码块
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCodeBlock {
    pub heading_id: String,
    pub source: String,
    pub body: CodeBody,
    pub diagnostics: Vec<crate::model::TangleDiagnostic>,
}
