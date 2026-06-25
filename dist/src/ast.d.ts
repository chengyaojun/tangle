import type { SourceSpan, TangleDiagnostic } from "./model.js";
export type Expr = LiteralExpr | IdentifierExpr | MemberAccessExpr | CallExpr | BinaryExpr | UnaryExpr | WithUpdateExpr | ThisExpr | IfExpr | ArrowExpr | PropagationExpr | MatchExpr | DestructureExpr | PanicExpr;
export type LiteralExpr = {
    kind: "literal";
    literalKind: "number" | "string" | "boolean";
    value: number | string | boolean;
    span: SourceSpan;
};
export type IdentifierExpr = {
    kind: "identifier";
    name: string;
    span: SourceSpan;
};
export type MemberAccessExpr = {
    kind: "memberAccess";
    object: Expr;
    member: string;
    span: SourceSpan;
};
export type CallExpr = {
    kind: "call";
    callee: Expr;
    args: Expr[];
    span: SourceSpan;
};
export type BinaryExpr = {
    kind: "binary";
    op: BinaryOp;
    left: Expr;
    right: Expr;
    span: SourceSpan;
};
export type BinaryOp = "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||";
export type UnaryExpr = {
    kind: "unary";
    op: UnaryOp;
    operand: Expr;
    span: SourceSpan;
};
export type UnaryOp = "!" | "-";
export type WithUpdateExpr = {
    kind: "withUpdate";
    object: Expr;
    fields: WithField[];
    span: SourceSpan;
};
export type WithField = {
    name: string;
    value: Expr;
    span: SourceSpan;
};
export type ThisExpr = {
    kind: "this";
    span: SourceSpan;
};
export type IfExpr = {
    kind: "if";
    condition: Expr;
    thenBranch: Expr;
    elseBranch?: Expr;
    span: SourceSpan;
};
export type ArrowExpr = {
    kind: "arrow";
    params: ArrowParam[];
    body: Expr;
    span: SourceSpan;
};
export type ArrowParam = {
    name: string;
    typeAnnotation?: TypeExpr;
    span: SourceSpan;
};
export type Stmt = ReturnStmt | LetStmt | ConstStmt | ExpressionStmt;
export type ReturnStmt = {
    kind: "return";
    value?: Expr;
    span: SourceSpan;
};
export type LetStmt = {
    kind: "let";
    name: string;
    typeAnnotation?: TypeExpr;
    value: Expr;
    span: SourceSpan;
};
export type ConstStmt = {
    kind: "const";
    name: string;
    typeAnnotation?: TypeExpr;
    value: Expr;
    span: SourceSpan;
};
export type ExpressionStmt = {
    kind: "expression";
    expr: Expr;
    span: SourceSpan;
};
export type CodeBody = {
    kind: "codeBody";
    statements: Stmt[];
    span: SourceSpan;
};
export type TypeExpr = PrimitiveTypeExpr | SumTypeExpr | GenericTypeExpr | FunctionTypeExpr | NamedTypeExpr;
export type PrimitiveTypeExpr = {
    kind: "primitiveType";
    name: "String" | "Int" | "Bool";
    span: SourceSpan;
};
export type SumTypeExpr = {
    kind: "sumType";
    variants: TypeExpr[];
    span: SourceSpan;
};
export type GenericTypeExpr = {
    kind: "genericType";
    base: string;
    typeArgs: TypeExpr[];
    span: SourceSpan;
};
export type FunctionTypeExpr = {
    kind: "functionType";
    params: TypeExpr[];
    returns: TypeExpr;
    span: SourceSpan;
};
export type NamedTypeExpr = {
    kind: "namedType";
    name: string;
    span: SourceSpan;
};
export type PropagationExpr = {
    kind: "propagation";
    expr: Expr;
    span: SourceSpan;
};
export type MatchArm = {
    pattern: MatchPattern;
    body: Expr;
    span: SourceSpan;
};
export type MatchPattern = {
    kind: "variantPattern";
    name: string;
    binding?: string;
    span: SourceSpan;
} | {
    kind: "wildcardPattern";
    span: SourceSpan;
};
export type MatchExpr = {
    kind: "match";
    expr: Expr;
    arms: MatchArm[];
    span: SourceSpan;
};
export type DestructureExpr = {
    kind: "destructure";
    okName: string;
    errName: string;
    expr: Expr;
    span: SourceSpan;
};
export type PanicExpr = {
    kind: "panic";
    message: Expr;
    span: SourceSpan;
};
export type ParsedCodeBlock = {
    headingId: string;
    source: string;
    body: CodeBody;
    diagnostics: TangleDiagnostic[];
};
