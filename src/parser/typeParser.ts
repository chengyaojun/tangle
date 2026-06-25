import type { TypeExpr } from "../ast.js";
import { tokenize } from "./lexer.js";
import type { Token } from "./lexer.js";

type ParseState = { tokens: Token[]; pos: number };

export function parseTypeExpr(source: string, file: string): TypeExpr {
  const tokens = tokenize(source, file);
  // Remove EOF token for cleaner parsing
  const state: ParseState = { tokens: tokens.slice(0, -1), pos: 0 };
  return parseSumType(state);
}

function peek(s: ParseState): Token {
  return s.tokens[s.pos] ?? { kind: "eof", value: "", span: { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } };
}

function advance(s: ParseState): Token {
  return s.tokens[s.pos++]!;
}

// Sum type: Type | Type | ...
function parseSumType(s: ParseState): TypeExpr {
  let left = parseFunctionType(s);
  while (peek(s).kind === "pipe") {
    advance(s); // |
    const right = parseFunctionType(s);
    if (left.kind === "sumType") {
      left.variants.push(right);
    } else {
      left = { kind: "sumType", variants: [left, right], span: mergeTypeSpan(left, right) };
    }
  }
  return left;
}

// Function type: (Params) => Return  OR  Type => Return
function parseFunctionType(s: ParseState): TypeExpr {
  if (peek(s).kind === "lparen") {
    advance(s); // (
    const params: TypeExpr[] = [];
    while (peek(s).kind !== "rparen" && peek(s).kind !== "eof") {
      params.push(parseSumType(s));
      if (peek(s).kind === "comma") advance(s);
    }
    if (peek(s).kind === "rparen") advance(s); // )

    if (peek(s).kind === "fatArrow") {
      advance(s); // ->
      const returns = parseSumType(s);
      const span = params.length > 0
        ? { ...params[0]!.span, endLine: returns.span.endLine, endColumn: returns.span.endColumn }
        : returns.span;
      return { kind: "functionType", params, returns, span };
    }
    // Just parenthesized type
    if (params.length === 1) return params[0]!;
    return { kind: "namedType", name: "void", span: { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } };
  }

  let left = parseGenericType(s);
  if (peek(s).kind === "fatArrow") {
    advance(s);
    const returns = parseSumType(s);
    return { kind: "functionType", params: [left], returns, span: mergeTypeSpan(left, returns) };
  }
  return left;
}

// Generic: Name<Args>
function parseGenericType(s: ParseState): TypeExpr {
  const left = parsePrimaryType(s);
  if (peek(s).kind === "lt") {
    advance(s); // <
    const args: TypeExpr[] = [];
    while (peek(s).kind !== "gt" && peek(s).kind !== "eof") {
      args.push(parseSumType(s));
      if (peek(s).kind === "comma") advance(s);
    }
    if (peek(s).kind === "gt") advance(s); // >
    const baseName = left.kind === "namedType" ? left.name : left.kind === "primitiveType" ? left.name : "";
    return { kind: "genericType", base: baseName, typeArgs: args, span: left.span };
  }
  return left;
}

// Primary: String | Int | Bool | Identifier
function parsePrimaryType(s: ParseState): TypeExpr {
  const token = advance(s);
  if (token.kind === "identifier") {
    if (["String", "Int", "Bool"].includes(token.value)) {
      return { kind: "primitiveType", name: token.value as "String" | "Int" | "Bool", span: token.span };
    }
    return { kind: "namedType", name: token.value, span: token.span };
  }
  return { kind: "namedType", name: "unknown", span: token.span };
}

function mergeTypeSpan(a: TypeExpr, b: TypeExpr): TypeExpr["span"] {
  return {
    file: a.span.file,
    startLine: a.span.startLine,
    startColumn: a.span.startColumn,
    endLine: b.span.endLine,
    endColumn: b.span.endColumn
  };
}
