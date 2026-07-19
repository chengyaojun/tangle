import type { SourceSpan, TangleDiagnostic } from "./model.js";

// ─── Expressions ───────────────────────────────────────

export type Expr =
  | LiteralExpr
  | IdentifierExpr
  | MemberAccessExpr
  | CallExpr
  | BinaryExpr
  | UnaryExpr
  | RecordUpdateExpr
  | PipeExpr
  | ThisExpr
  | IfExpr
  | ArrowExpr
  | PropagationExpr
  | MatchExpr
  | DestructureExpr
  | PanicExpr
  | IsExpr;

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

export type BinaryOp =
  | "+" | "-" | "*" | "/" | "%"
  | "==" | "!=" | "<" | ">" | "<=" | ">="
  | "&&" | "||";

export type UnaryExpr = {
  kind: "unary";
  op: UnaryOp;
  operand: Expr;
  span: SourceSpan;
};

export type UnaryOp = "!" | "-";

export type RecordUpdateExpr = {
  kind: "recordUpdate";
  object: Expr;
  fields: RecordField[];
  span: SourceSpan;
};

export type RecordField = {
  name: string;
  value: Expr;
  span: SourceSpan;
};

export type PipeExpr = {
  kind: "pipe";
  left: Expr;
  right: Expr;
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

// ─── Statements ────────────────────────────────────────

export type Stmt =
  | ReturnStmt
  | LetStmt
  | ConstStmt
  | ExpressionStmt
  | LetVariantStmt
  | LetRecordStmt;

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

// ─── Code Block AST ────────────────────────────────────

export type CodeBody = {
  kind: "codeBody";
  statements: Stmt[];
  span: SourceSpan;
};

// ─── Type Expressions (parsed from type annotations) ───

export type TypeExpr =
  | PrimitiveTypeExpr
  | SumTypeExpr
  | GenericTypeExpr
  | FunctionTypeExpr
  | NamedTypeExpr;

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

// ─── Error Handling Expressions ────────────────────────

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

export type MatchPattern =
  | { kind: "variantPattern"; name: string; binding?: string; span: SourceSpan }
  | { kind: "wildcardPattern"; span: SourceSpan };

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

// ─── Patterns (Phase 6d) ──────────────────────────────

export type Pattern =
  | { kind: "variant"; name: string; binding?: string; span: SourceSpan }
  | { kind: "wildcard"; span: SourceSpan };

// ─── IsExpr (Phase 6d type narrowing) ─────────────────

export type IsExpr = {
  kind: "is";
  expr: Expr;
  pattern: Pattern;
  span: SourceSpan;
};

// ─── Refutable let / Record destructuring (Phase 6d) ──

export type LetVariantStmt = {
  kind: "letVariant";
  variantName: string;
  binding: string | null;
  expr: Expr;
  elseBranch: Stmt[];
  span: SourceSpan;
};

export type LetRecordStmt = {
  kind: "letRecord";
  fields: [string, string][];  // [fieldName, localVar]
  expr: Expr;
  span: SourceSpan;
};

// ─── Parser result ─────────────────────────────────────

export type ParsedCodeBlock = {
  headingId: string;
  source: string;
  body: CodeBody;
  diagnostics: TangleDiagnostic[];
};
