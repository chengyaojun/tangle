import type { BinaryOp, CodeBody, Expr, Stmt, TypeExpr, WithField, ArrowParam } from "../ast.js";
import type { Token } from "./lexer.js";
import type { TangleDiagnostic, SourceSpan } from "../model.js";

// ─── Parser State ──────────────────────────────────────

export type ParserState = {
  tokens: Token[];
  pos: number;
  file: string;
  diagnostics: TangleDiagnostic[];
};

// ─── Precedence Table ──────────────────────────────────

// Higher number = tighter binding
const PREC: Record<string, number> = {
  "||": 1, "&&": 2,
  "==": 3, "!=": 3,
  "<": 4, ">": 4, "<=": 4, ">=": 4,
  "+": 5, "-": 5,
  "*": 6, "/": 6, "%": 6,
};

// ─── Main Entry ────────────────────────────────────────

export function parseExpression(tokens: Token[]): Expr {
  const state = createState(tokens);
  const expr = parseExpr(state, 0);
  return expr;
}

function createState(tokens: Token[]): ParserState {
  return { tokens, pos: 0, file: tokens[0]?.span.file ?? "", diagnostics: [] };
}

function peek(state: ParserState): Token {
  return state.tokens[state.pos]!;
}

function advance(state: ParserState): Token {
  const t = state.tokens[state.pos]!;
  state.pos += 1;
  return t;
}

// ─── Pratt Parser Core ─────────────────────────────────

function parseExpr(state: ParserState, minPrec: number): Expr {
  let left = parsePrefix(state);

  while (true) {
    const token = peek(state);
    // Stop at tokens that cannot be infix operators
    if (
      token.kind === "eof" ||
      token.kind === "semicolon" ||
      token.kind === "rparen" ||
      token.kind === "rbrace" ||
      token.kind === "rbracket" ||
      token.kind === "comma" ||
      token.kind === "colon" ||
      token.kind === "fatArrow" ||
      token.kind === "else" ||
      token.kind === "return" ||
      token.kind === "let" ||
      token.kind === "const"
    ) {
      break;
    }

    const prec = infixPrecedence(token);
    if (prec < minPrec) break;

    left = parseInfix(state, left, token);
  }

  return left;
}

function infixPrecedence(token: Token): number {
  // postfix propagation (?) has highest precedence
  if (token.kind === "question") return 10;
  // member access (dot) and calls (lparen)
  if (token.kind === "dot" || token.kind === "lparen") return 9;
  // with-update binds loosely
  if (token.kind === "with") return 2;
  // Look up binary operators via token value (e.g. "+" for kind "plus", "==" for kind "eqeq")
  const opPrec = PREC[token.value];
  if (opPrec !== undefined) return opPrec;
  // Not an infix token
  return -1;
}

// ─── Prefix Parsers ────────────────────────────────────

function parsePrefix(state: ParserState): Expr {
  const token = peek(state);

  // Unary: ! and -
  if (token.kind === "bang" || token.kind === "minus") {
    const opToken = advance(state);
    const op: "!" | "-" = opToken.kind === "bang" ? "!" : "-";
    const operand = parseExpr(state, 7); // high precedence for unary
    return { kind: "unary", op, operand, span: mergeSpan(opToken.span, operand.span) };
  }

  // if expression
  if (token.kind === "if") {
    const ifToken = advance(state);
    // expect lparen, parse condition, expect rparen
    if (peek(state).kind === "lparen") advance(state);
    const condition = parseExpr(state, 0);
    if (peek(state).kind === "rparen") advance(state);
    const thenBranch = parseExpr(state, 0);
    let elseBranch: Expr | undefined;
    if (peek(state).kind === "else") {
      advance(state);
      elseBranch = parseExpr(state, 0);
    }
    const endSpan = elseBranch ? elseBranch.span : thenBranch.span;
    if (elseBranch) {
      return {
        kind: "if",
        condition,
        thenBranch,
        elseBranch,
        span: mergeSpan(ifToken.span, endSpan),
      };
    }
    return {
      kind: "if",
      condition,
      thenBranch,
      span: mergeSpan(ifToken.span, endSpan),
    };
  }

  // match expression: match expr { arm1, arm2, ... }
  if (token.kind === "identifier" && token.value === "match") {
    const savedPos = state.pos;
    const matchToken = advance(state);
    const matchedExpr = parseExpr(state, 0);
    if (peek(state).kind === "lbrace") {
      advance(state); // {
      const arms: import("../ast.js").MatchArm[] = [];
      while (peek(state).kind !== "rbrace" && peek(state).kind !== "eof") {
        const patternToken = peek(state);
        let pattern: import("../ast.js").MatchPattern;
        if (patternToken.kind === "identifier") {
          if (patternToken.value === "_") {
            advance(state);
            pattern = { kind: "wildcardPattern", span: patternToken.span };
          } else {
            const name = patternToken.value;
            advance(state);
            let binding: string | undefined;
            if (peek(state).kind === "lparen" && name[0] === name[0]?.toUpperCase()) {
              advance(state); // (
              const bindToken = peek(state);
              if (bindToken.kind === "identifier") {
                binding = bindToken.value;
                advance(state);
              }
              if (peek(state).kind === "rparen") advance(state); // )
            }
            pattern = binding !== undefined
              ? { kind: "variantPattern", name, binding, span: patternToken.span }
              : { kind: "variantPattern", name, span: patternToken.span };
          }
        } else {
          state.diagnostics.push({
            code: "TANGLE_PARSE_EXPECTED_PATTERN",
            message: "Expected match pattern",
            span: patternToken.span,
          });
          break;
        }
        if (peek(state).kind === "fatArrow") advance(state); // =>
        const body = parseExpr(state, 0);
        arms.push({ pattern, body, span: pattern.span });
        if (peek(state).kind === "comma") advance(state);
      }
      const rbrace = peek(state);
      if (rbrace.kind === "rbrace") advance(state);
      return { kind: "match", expr: matchedExpr, arms, span: mergeSpan(matchToken.span, rbrace.span) };
    }
    // Not a match expression — backtrack and fall through to identifier
    state.pos = savedPos;
  }

  // panic expression: panic("message")
  if (token.kind === "identifier" && token.value === "panic") {
    const savedPos = state.pos;
    const panicToken = advance(state);
    if (peek(state).kind === "lparen") {
      advance(state); // (
      const message = parseExpr(state, 0);
      const rparen = peek(state);
      if (rparen.kind === "rparen") advance(state);
      return { kind: "panic", message, span: mergeSpan(panicToken.span, rparen.span) };
    }
    // Not a panic call — backtrack
    state.pos = savedPos;
  }

  return parsePrimary(state);
}

function parsePrimary(state: ParserState): Expr {
  const token = advance(state);

  // Number literal
  if (token.kind === "number") {
    const num = Number(token.value);
    return { kind: "literal", literalKind: "number", value: num, span: token.span };
  }

  // String literal
  if (token.kind === "string") {
    const str = token.value.slice(1, -1); // strip quotes
    return { kind: "literal", literalKind: "string", value: str, span: token.span };
  }

  // Boolean literals
  if (token.kind === "true") {
    return { kind: "literal", literalKind: "boolean", value: true, span: token.span };
  }
  if (token.kind === "false") {
    return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
  }

  // this
  if (token.kind === "this") {
    return { kind: "this", span: token.span };
  }

  // Identifier
  if (token.kind === "identifier") {
    return { kind: "identifier", name: token.value, span: token.span };
  }

  // Parenthesized expression or arrow function params
  if (token.kind === "lparen") {
    return parseGroupOrArrow(state, token);
  }

  // Standalone braces not valid at expression level
  if (token.kind === "lbrace") {
    state.diagnostics.push({
      code: "TANGLE_PARSE_UNEXPECTED_TOKEN",
      message: "Unexpected { — use 'with { }' for struct updates",
      span: token.span,
    });
    return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
  }

  // Error token from lexer
  if (token.kind === "error") {
    state.diagnostics.push({
      code: "TANGLE_PARSE_LEXER_ERROR",
      message: token.value,
      span: token.span,
    });
    return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
  }

  state.diagnostics.push({
    code: "TANGLE_PARSE_UNEXPECTED_TOKEN",
    message: `Unexpected token: ${token.kind}`,
    span: token.span,
  });
  return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
}

function parseGroupOrArrow(state: ParserState, lparen: Token): Expr {
  // Check for empty parens
  if (peek(state).kind === "rparen") {
    advance(state); // )
    state.diagnostics.push({
      code: "TANGLE_PARSE_EMPTY_PARENS",
      message: "Empty parentheses are not valid",
      span: lparen.span,
    });
    return { kind: "literal", literalKind: "boolean", value: false, span: lparen.span };
  }

  // Parse first expression or param name
  const first = parseExpr(state, 0);

  // If followed by comma, this is either a tuple/params or we collect more
  if (peek(state).kind === "comma") {
    // Collect all params/expressions
    const items: Expr[] = [first];
    while (peek(state).kind === "comma") {
      advance(state); // ,
      items.push(parseExpr(state, 0));
    }
    if (peek(state).kind !== "rparen") {
      state.diagnostics.push({
        code: "TANGLE_PARSE_EXPECTED_RPAREN",
        message: "Expected )",
        span: peek(state).span,
      });
    } else {
      advance(state); // )
    }

    // Arrow function?
    if (peek(state).kind === "fatArrow") {
      advance(state); // =>
      const params: ArrowParam[] = items.map((item) => {
        if (item.kind === "identifier") {
          return { name: item.name, span: item.span };
        }
        return { name: "param", span: item.span };
      });
      const body = parseExpr(state, 0);
      return {
        kind: "arrow",
        params,
        body,
        span: mergeSpan(lparen.span, body.span),
      };
    }

    // Not an arrow — just return the first expression (tuples not yet supported)
    return first;
  }

  // Expect rparen
  if (peek(state).kind === "rparen") {
    advance(state); // )
  } else {
    state.diagnostics.push({
      code: "TANGLE_PARSE_EXPECTED_RPAREN",
      message: "Expected ')'",
      span: peek(state).span,
    });
  }

  // Arrow function with single param?
  if (peek(state).kind === "fatArrow") {
    advance(state); // =>
    const params: ArrowParam[] =
      first.kind === "identifier"
        ? [{ name: first.name, span: first.span }]
        : [{ name: "param", span: first.span }];
    const body = parseExpr(state, 0);
    return {
      kind: "arrow",
      params,
      body,
      span: mergeSpan(lparen.span, body.span),
    };
  }

  return first;
}

// ─── Infix Parsers ─────────────────────────────────────

function parseInfix(state: ParserState, left: Expr, token: Token): Expr {
  // Propagation: expr?
  if (token.kind === "question") {
    advance(state);
    return { kind: "propagation", expr: left, span: mergeSpan(left.span, token.span) };
  }

  // Member access: obj.member
  if (token.kind === "dot") {
    advance(state); // .
    const memberToken = peek(state);
    if (memberToken.kind === "identifier") {
      advance(state);
      return {
        kind: "memberAccess",
        object: left,
        member: memberToken.value,
        span: mergeSpan(left.span, memberToken.span),
      };
    }
    state.diagnostics.push({
      code: "TANGLE_PARSE_EXPECTED_IDENTIFIER",
      message: "Expected property name after '.'",
      span: token.span,
    });
    return left;
  }

  // Function call: callee(args)
  if (token.kind === "lparen") {
    advance(state); // (
    const args: Expr[] = [];
    while (peek(state).kind !== "rparen" && peek(state).kind !== "eof") {
      args.push(parseExpr(state, 0));
      if (peek(state).kind === "comma") advance(state);
    }
    const rparen = peek(state);
    if (rparen.kind === "rparen") advance(state);
    return {
      kind: "call",
      callee: left,
      args,
      span: mergeSpan(left.span, rparen.span),
    };
  }

  // With update: expr with { fields }
  if (token.kind === "with") {
    advance(state); // with
    const fields: WithField[] = [];
    if (peek(state).kind === "lbrace") advance(state); // {
    while (peek(state).kind !== "rbrace" && peek(state).kind !== "eof") {
      const fieldName = peek(state);
      if (fieldName.kind !== "identifier") break;
      advance(state);
      if (peek(state).kind === "colon") advance(state); // :
      const fieldValue = parseExpr(state, 0);
      fields.push({
        name: fieldName.value,
        value: fieldValue,
        span: fieldName.span,
      });
      if (peek(state).kind === "comma") advance(state);
    }
    const rbrace = peek(state);
    if (rbrace.kind === "rbrace") advance(state);
    return {
      kind: "withUpdate",
      object: left,
      fields,
      span: mergeSpan(left.span, rbrace.span),
    };
  }

  // Binary operators (via Pratt)
  const op = token.value;
  advance(state);
  const prec = infixPrecedence(token);
  const right = parseExpr(state, prec + 1); // left-associative

  return {
    kind: "binary",
    op: op as BinaryOp,
    left,
    right,
    span: mergeSpan(left.span, right.span),
  };
}

// ─── Statement Parser ──────────────────────────────────

export function parseStatement(tokens: Token[]): Stmt {
  const state = createState(tokens);
  return parseStmt(state);
}

export function parseCodeBody(tokens: Token[]): CodeBody {
  const state = createState(tokens);
  const statements: Stmt[] = [];
  while (peek(state).kind !== "eof") {
    if (peek(state).kind === "semicolon") {
      advance(state);
      continue;
    }
    statements.push(parseStmt(state));
    if (peek(state).kind === "semicolon") advance(state);
  }
  const span = tokens.length > 0
    ? {
        file: tokens[0]!.span.file,
        startLine: tokens[0]!.span.startLine,
        startColumn: tokens[0]!.span.startColumn,
        endLine: tokens[tokens.length - 1]!.span.endLine,
        endColumn: tokens[tokens.length - 1]!.span.endColumn
      }
    : { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 };
  return { kind: "codeBody", statements, span };
}

function parseStmt(state: ParserState): Stmt {
  const token = peek(state);

  // return [expr]
  if (token.kind === "return") {
    const retToken = advance(state);
    if (peek(state).kind === "eof" || peek(state).kind === "semicolon" || peek(state).kind === "rbrace") {
      return { kind: "return", span: retToken.span };
    }
    const value = parseExpr(state, 0);
    return { kind: "return", value, span: mergeSpan(retToken.span, value.span) };
  }

  // let name [: Type] = expr
  if (token.kind === "let" || token.kind === "const") {
    const kw = advance(state);
    const nameToken = peek(state);
    if (nameToken.kind !== "identifier") {
      state.diagnostics.push({
        code: "TANGLE_PARSE_EXPECTED_IDENTIFIER",
        message: `Expected variable name after ${kw.kind}`,
        span: nameToken.span
      });
      advance(state);
      return { kind: "expression", expr: { kind: "literal", literalKind: "boolean", value: false, span: kw.span }, span: kw.span };
    }
    advance(state);
    let typeAnnotation: TypeExpr | undefined;
    if (peek(state).kind === "colon") {
      advance(state); // :
      typeAnnotation = parseTypeAnnotation(state);
    }
    if (peek(state).kind === "eq") advance(state); // =
    const value = parseExpr(state, 0);
    const kind = kw.kind === "let" ? "let" : "const";
    return { kind, name: nameToken.value, typeAnnotation, value, span: mergeSpan(kw.span, value.span) } as Stmt;
  }

  // expression statement
  const expr = parseExpr(state, 0);
  return { kind: "expression", expr, span: expr.span };
}

// ─── Type Annotation Parser (inline) ─────────────────

function parseTypeAnnotation(state: ParserState): TypeExpr {
  const token = peek(state);
  if (token.kind === "identifier") {
    if (["String", "Int", "Bool"].includes(token.value)) {
      advance(state);
      return { kind: "primitiveType", name: token.value as "String" | "Int" | "Bool", span: token.span };
    }
    advance(state);
    // Check for generic <...>
    if (peek(state).kind === "lt") {
      advance(state);
      const args: TypeExpr[] = [];
      while (peek(state).kind !== "gt" && peek(state).kind !== "eof") {
        args.push(parseTypeAnnotation(state));
        if (peek(state).kind === "comma") advance(state);
      }
      if (peek(state).kind === "gt") advance(state);
      return { kind: "genericType", base: token.value, typeArgs: args, span: token.span };
    }
    return { kind: "namedType", name: token.value, span: token.span };
  }
  state.diagnostics.push({
    code: "TANGLE_PARSE_EXPECTED_TYPE",
    message: `Expected type annotation, got ${token.kind}`,
    span: token.span
  });
  advance(state);
  return { kind: "namedType", name: "unknown", span: token.span };
}

// ─── Helpers ───────────────────────────────────────────

function mergeSpan(a: SourceSpan, b: SourceSpan): SourceSpan {
  return {
    file: a.file,
    startLine: a.startLine,
    startColumn: a.startColumn,
    endLine: b.endLine,
    endColumn: b.endColumn,
  };
}
