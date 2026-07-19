export type {
  DirectiveKind,
  HeadingRole,
  SourceSpan,
  SymbolKind,
  TangleCodeBlock,
  TangleDiagnostic,
  TangleDirective,
  TangleHeading,
  TangleImport,
  TangleModule,
  TangleParam,
  TangleSymbol
} from "./model.js";

export { collectLinks, isTangleCodeBlock, parseParamItem, plainText } from "./front-end/blocks.js";
export { compileModule } from "./front-end/compileModule.js";
export type { CompileModuleInput } from "./front-end/compileModule.js";
export { parseDirectiveLine } from "./front-end/directives.js";
export { headingRoleForDepth, parseHeadingText } from "./front-end/headings.js";
export { spanFromNode } from "./front-end/sourceMap.js";
export { parseMarkdown } from "./markdown/parseMarkdown.js";
export { tokenize } from "./parser/lexer.js";
export type { Token, TokenKind } from "./parser/lexer.js";
export { parseCodeBody, parseExpression, parseStatement } from "./parser/parser.js";
export type { ParserState } from "./parser/parser.js";
export { parseTypeExpr } from "./parser/typeParser.js";

export type {
  ArrowExpr,
  ArrowParam,
  BinaryExpr,
  BinaryOp,
  CallExpr,
  CodeBody,
  ConstStmt,
  DestructureExpr,
  Expr,
  ExpressionStmt,
  FunctionTypeExpr,
  GenericTypeExpr,
  IdentifierExpr,
  IfExpr,
  IsExpr,
  LetRecordStmt,
  LetStmt,
  LetVariantStmt,
  LiteralExpr,
  MatchArm,
  MatchExpr,
  MatchPattern,
  MemberAccessExpr,
  NamedTypeExpr,
  PanicExpr,
  ParsedCodeBlock,
  Pattern,
  PrimitiveTypeExpr,
  PropagationExpr,
  ReturnStmt,
  Stmt,
  SumTypeExpr,
  ThisExpr,
  TypeExpr,
  UnaryExpr,
  UnaryOp,
  PipeExpr,
  RecordField,
  RecordUpdateExpr
} from "./ast.js";

// Checker types
export type { CallableSignature, FunctionType_ as FunctionType, GenericTypeInstance, InterfaceType, PrimitiveType, StructType, SumType_ as SumType, Type, TypeVariable } from "./checker/types.js";
export { isSubtype, typesEqual } from "./checker/types.js";

// Builtins
export { builtinTypes, isBuiltinType } from "./checker/builtins.js";

// Env
export { createEnv } from "./checker/env.js";
export type { ReceiverContext, TypeEnv } from "./checker/env.js";

// Type resolution
export { resolveTypes } from "./checker/resolve.js";

// Core type checker
export { checkExpression } from "./checker/check.js";

// Error registry
export { ErrorRegistry } from "./checker/errors.js";
export type { ErrorVariant } from "./checker/errors.js";

// Propagation
export { checkPropagation } from "./checker/propagation.js";

// Match exhaustiveness
export { checkMatchExhaustiveness } from "./checker/match.js";

// Panic
export { checkPanic, isDeadPath } from "./checker/panic.js";

// Pipeline integration
export { checkModule, parseCodeBlocks } from "./checker/checkModule.js";
export type { CheckedModule } from "./checker/checkModule.js";

// IR
export type { IREdge, IREdgeKind, IRErrorEdge, IRNode, IRNodeKind, RuleGraph } from "./ir/graph.js";
export { createGraph, freshNodeId, resetNodeCounter } from "./ir/graph.js";
export { lowerStatements } from "./ir/lower.js";
export { lowerRuleFlow } from "./ir/ruleFlow.js";
export type { FlowLowerResult } from "./ir/ruleFlow.js";
export { lowerRuleTable } from "./ir/ruleTable.js";
export type { TableLowerResult } from "./ir/ruleTable.js";
export { lowerRuleTree } from "./ir/ruleTree.js";
export { lowerRuleToggle } from "./ir/ruleToggle.js";
export type { ToggleLowerResult } from "./ir/ruleToggle.js";
export type { TreeLowerResult, TreeIRNode } from "./ir/ruleTree.js";
export { checkIRVisibility } from "./ir/visibility.js";
export { validateIR } from "./ir/validate.js";
export { compileToIR } from "./ir/compileToIR.js";

// Codegen
export { emitJS } from "./codegen/jsEmitter.js";
export { wrapOk, wrapErr, unwrapOrPropagate } from "./codegen/errorMapping.js";

// Pipeline
export { compile } from "./pipeline.js";
export type { CompileResult } from "./pipeline.js";
