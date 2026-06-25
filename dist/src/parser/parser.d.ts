import type { CodeBody, Expr, Stmt } from "../ast.js";
import type { Token } from "./lexer.js";
import type { TangleDiagnostic } from "../model.js";
export type ParserState = {
    tokens: Token[];
    pos: number;
    file: string;
    diagnostics: TangleDiagnostic[];
};
export declare function parseExpression(tokens: Token[]): Expr;
export declare function parseStatement(tokens: Token[]): Stmt;
export declare function parseCodeBody(tokens: Token[]): CodeBody;
