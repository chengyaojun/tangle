import { describe, expect, it } from "vitest";
import type { Expr, LiteralExpr, IdentifierExpr, Stmt, ReturnStmt, CodeBody, MatchExpr, PropagationExpr, DestructureExpr, PanicExpr } from "../../src/index";
import { parseCodeBody, parseExpression, parseStatement, tokenize } from "../../src/index";

describe("Code AST types", () => {
  it("supports literal expressions", () => {
    const lit: LiteralExpr = {
      kind: "literal",
      literalKind: "number",
      value: 42,
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 3 }
    };
    expect(lit.kind).toBe("literal");
    expect(lit.value).toBe(42);
  });

  it("supports identifier expressions", () => {
    const id: IdentifierExpr = {
      kind: "identifier",
      name: "x",
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 2 }
    };
    expect(id.name).toBe("x");
  });

  it("defines discriminated union Expr", () => {
    const expr: Expr = {
      kind: "literal",
      literalKind: "boolean",
      value: true,
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 5 }
    };
    if (expr.kind === "literal") {
      expect(expr.literalKind).toBe("boolean");
    }
  });

  it("defines CodeBody with statements", () => {
    const body: CodeBody = {
      kind: "codeBody",
      statements: [],
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
    };
    expect(body.statements).toEqual([]);
  });
});

describe("parseExpression", () => {
  it("parses number literals", () => {
    const expr = parseExpression(tokenize("42", "test.md"));
    expect(expr).toMatchObject({ kind: "literal", literalKind: "number", value: 42 });
  });

  it("parses string literals", () => {
    const expr = parseExpression(tokenize('"hello"', "test.md"));
    expect(expr).toMatchObject({ kind: "literal", literalKind: "string", value: "hello" });
  });

  it("parses boolean literals", () => {
    expect(parseExpression(tokenize("true", "test.md"))).toMatchObject({
      kind: "literal", literalKind: "boolean", value: true
    });
    expect(parseExpression(tokenize("false", "test.md"))).toMatchObject({
      kind: "literal", literalKind: "boolean", value: false
    });
  });

  it("parses identifiers", () => {
    const expr = parseExpression(tokenize("userName", "test.md"));
    expect(expr).toMatchObject({ kind: "identifier", name: "userName" });
  });

  it("parses member access chains", () => {
    const expr = parseExpression(tokenize("this.is_active", "test.md"));
    expect(expr).toMatchObject({
      kind: "memberAccess",
      object: { kind: "this" },
      member: "is_active"
    });
  });

  it("parses function calls", () => {
    const expr = parseExpression(tokenize("notify(user, msg)", "test.md"));
    expect(expr).toMatchObject({
      kind: "call",
      callee: { kind: "identifier", name: "notify" },
      args: [
        { kind: "identifier", name: "user" },
        { kind: "identifier", name: "msg" }
      ]
    });
  });

  it("parses binary operators with correct precedence", () => {
    const expr = parseExpression(tokenize("1 + 2 * 3", "test.md"));
    // should parse as 1 + (2 * 3), not (1 + 2) * 3
    expect(expr).toMatchObject({
      kind: "binary",
      op: "+",
      left: { kind: "literal", value: 1 },
      right: {
        kind: "binary",
        op: "*",
        left: { kind: "literal", value: 2 },
        right: { kind: "literal", value: 3 }
      }
    });
  });

  it("parses unary operators", () => {
    const expr = parseExpression(tokenize("!ready", "test.md"));
    expect(expr).toMatchObject({
      kind: "unary",
      op: "!",
      operand: { kind: "identifier", name: "ready" }
    });
  });

  it("parses this keyword", () => {
    const expr = parseExpression(tokenize("this", "test.md"));
    expect(expr).toMatchObject({ kind: "this" });
  });

  it("parses record-update expressions", () => {
    const expr = parseExpression(tokenize("user { is_active: true }", "test.md"));
    expect(expr).toMatchObject({
      kind: "recordUpdate",
      object: { kind: "identifier", name: "user" },
      fields: [
        { name: "is_active", value: { kind: "literal", literalKind: "boolean", value: true } }
      ]
    });
  });

  it("parses if expressions", () => {
    const expr = parseExpression(tokenize("if (x > 0) 1 else 0", "test.md"));
    expect(expr).toMatchObject({
      kind: "if",
      condition: { kind: "binary", op: ">" },
      thenBranch: { kind: "literal", value: 1 },
      elseBranch: { kind: "literal", value: 0 }
    });
  });

  it("parses arrow function expressions", () => {
    const expr = parseExpression(tokenize("(x, y) -> x + y", "test.md"));
    expect(expr).toMatchObject({
      kind: "arrow",
      params: [{ name: "x" }, { name: "y" }],
      body: { kind: "binary", op: "+" }
    });
  });

  it("parses parenthesized expressions", () => {
    const expr = parseExpression(tokenize("(1 + 2) * 3", "test.md"));
    expect(expr).toMatchObject({
      kind: "binary",
      op: "*",
      left: { kind: "binary", op: "+" },
      right: { kind: "literal", value: 3 }
    });
  });
});

describe("parseStatement", () => {
  it("parses return statements", () => {
    const stmt = parseStatement(tokenize("return 42", "test.md"));
    expect(stmt).toMatchObject({
      kind: "return",
      value: { kind: "literal", value: 42 }
    });
  });

  it("parses bare return", () => {
    const stmt = parseStatement(tokenize("return", "test.md"));
    expect(stmt).toMatchObject({ kind: "return" });
    expect((stmt as { kind: "return"; value?: unknown }).value).toBeUndefined();
  });

  it("parses let bindings", () => {
    const stmt = parseStatement(tokenize("let x = 5", "test.md"));
    expect(stmt).toMatchObject({
      kind: "let",
      name: "x",
      value: { kind: "literal", value: 5 }
    });
  });

  it("parses const bindings", () => {
    const stmt = parseStatement(tokenize("const pi = 3.14", "test.md"));
    expect(stmt).toMatchObject({
      kind: "const",
      name: "pi",
      value: { kind: "literal", value: 3.14 }
    });
  });

  it("parses expression statements", () => {
    const stmt = parseStatement(tokenize("notify(user)", "test.md"));
    expect(stmt).toMatchObject({
      kind: "expression",
      expr: { kind: "call", callee: { kind: "identifier", name: "notify" } }
    });
  });
});

describe("parseCodeBody", () => {
  it("parses multiple statements", () => {
    const body = parseCodeBody(tokenize(
      "let a = 1\nreturn this { is_active: true }",
      "test.md"
    ));
    expect(body.statements).toHaveLength(2);
    expect(body.statements[0]!.kind).toBe("let");
    expect(body.statements[1]!.kind).toBe("return");
  });

  it("semicolons separate statements", () => {
    const body = parseCodeBody(tokenize("let a = 1; let b = 2", "test.md"));
    expect(body.statements).toHaveLength(2);
  });

  it("empty input produces empty statement list", () => {
    const body = parseCodeBody(tokenize("", "test.md"));
    expect(body.statements).toHaveLength(0);
  });
});

describe("error handling AST types", () => {
  it("defines PropagationExpr for ? operator", () => {
    const pe: PropagationExpr = {
      kind: "propagation",
      expr: { kind: "identifier", name: "result", span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 7 } },
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 8 }
    };
    expect(pe.kind).toBe("propagation");
  });

  it("defines MatchExpr with arms", () => {
    const me: MatchExpr = {
      kind: "match",
      expr: { kind: "identifier", name: "result", span: { file: "t.md", startLine: 1, startColumn: 7, endLine: 1, endColumn: 13 } },
      arms: [],
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 5, endColumn: 1 }
    };
    expect(me.kind).toBe("match");
  });

  it("defines DestructureExpr", () => {
    const de: DestructureExpr = {
      kind: "destructure",
      okName: "receipt",
      errName: "err",
      expr: { kind: "identifier", name: "confirm", span: { file: "t.md", startLine: 1, startColumn: 2, endLine: 1, endColumn: 9 } },
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 30 }
    };
    expect(de.okName).toBe("receipt");
  });

  it("defines PanicExpr", () => {
    const pe: PanicExpr = {
      kind: "panic",
      message: { kind: "literal", literalKind: "string", value: "unrecoverable", span: { file: "t.md", startLine: 1, startColumn: 8, endLine: 1, endColumn: 24 } },
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 25 }
    };
    expect(pe.kind).toBe("panic");
  });
});

describe("error handling parser", () => {
  it("parses propagation operator ?", () => {
    const expr = parseExpression(tokenize("result?", "test.md"));
    expect(expr).toMatchObject({ kind: "propagation" });
    const propExpr = expr as { kind: "propagation"; expr: { kind: string; name: string } };
    expect(propExpr.expr.kind).toBe("identifier");
    expect(propExpr.expr.name).toBe("result");
  });

  it("parses panic expressions", () => {
    const expr = parseExpression(tokenize('panic("unrecoverable")', "test.md"));
    expect(expr).toMatchObject({ kind: "panic" });
  });
});
