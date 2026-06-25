import type { SourceSpan } from "../model.js";
export type TokenKind = "number" | "string" | "true" | "false" | "identifier" | "return" | "let" | "const" | "if" | "else" | "this" | "with" | "dot" | "comma" | "colon" | "semicolon" | "lparen" | "rparen" | "lbrace" | "rbrace" | "lbracket" | "rbracket" | "plus" | "minus" | "star" | "slash" | "percent" | "eq" | "eqeq" | "neq" | "lt" | "gt" | "lte" | "gte" | "and" | "or" | "bang" | "pipe" | "arrow" | "fatArrow" | "error" | "question" | "eof";
export type Token = {
    kind: TokenKind;
    value: string;
    span: SourceSpan;
};
export declare function tokenize(source: string, file: string): Token[];
