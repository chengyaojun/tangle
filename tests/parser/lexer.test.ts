import { describe, expect, it } from "vitest";
import { tokenize } from "../../src/index";

describe("lexer", () => {
  it("tokenizes numbers", () => {
    const tokens = tokenize("42", "test.md");
    expect(tokens).toHaveLength(2); // number + eof
    expect(tokens[0]).toMatchObject({ kind: "number", value: "42" });
  });

  it("tokenizes strings", () => {
    const tokens = tokenize('"hello"', "test.md");
    expect(tokens[0]).toMatchObject({ kind: "string", value: '"hello"' });
  });

  it("tokenizes boolean literals", () => {
    expect(tokenize("true", "test.md")[0]).toMatchObject({ kind: "true", value: "true" });
    expect(tokenize("false", "test.md")[0]).toMatchObject({ kind: "false", value: "false" });
  });

  it("tokenizes keywords", () => {
    const tokens = tokenize("return let const if else this with", "test.md");
    expect(tokens[0]!.kind).toBe("return");
    expect(tokens[1]!.kind).toBe("let");
    expect(tokens[2]!.kind).toBe("const");
    expect(tokens[3]!.kind).toBe("if");
    expect(tokens[4]!.kind).toBe("else");
    expect(tokens[5]!.kind).toBe("this");
    expect(tokens[6]!.kind).toBe("with");
  });

  it("tokenizes identifiers", () => {
    const tokens = tokenize("foo bar123 _private", "test.md");
    expect(tokens[0]!).toMatchObject({ kind: "identifier", value: "foo" });
    expect(tokens[1]!).toMatchObject({ kind: "identifier", value: "bar123" });
    expect(tokens[2]!).toMatchObject({ kind: "identifier", value: "_private" });
  });

  it("tokenizes operators", () => {
    const tokens = tokenize("+ - * / % == != < > <= >= && || ! => ->", "test.md");
    expect(tokens.map(t => t.kind)).toEqual([
      "plus", "minus", "star", "slash", "percent",
      "eqeq", "neq", "lt", "gt", "lte", "gte",
      "and", "or", "bang", "fatArrow", "arrow",
      "eof"
    ]);
  });

  it("tokenizes delimiters", () => {
    const tokens = tokenize(". , : ; ( ) { } [ ] |", "test.md");
    expect(tokens.map(t => t.kind)).toEqual([
      "dot", "comma", "colon", "semicolon",
      "lparen", "rparen", "lbrace", "rbrace", "lbracket", "rbracket",
      "pipe", "eof"
    ]);
  });

  it("tokenizes a complete code snippet", () => {
    const tokens = tokenize("return this with { is_active: true }", "test.md");
    expect(tokens.map(t => t.kind)).toEqual([
      "return", "this", "with", "lbrace", "identifier", "colon", "true", "rbrace", "eof"
    ]);
  });

  it("skips whitespace and attaches source spans", () => {
    const tokens = tokenize("  x\n  42", "test.md");
    const x = tokens[0]!;
    expect(x.kind).toBe("identifier");
    expect(x.span.startLine).toBe(1);
    expect(x.span.startColumn).toBe(3);
    const num = tokens[1]!;
    expect(num.kind).toBe("number");
    expect(num.span.startLine).toBe(2);
  });

  it("handles empty input", () => {
    const tokens = tokenize("", "test.md");
    expect(tokens).toHaveLength(1);
    expect(tokens[0]!.kind).toBe("eof");
  });

  it("handles decimal numbers and boundary case", () => {
    // Decimal number
    const tokens1 = tokenize("3.14", "test.md");
    expect(tokens1[0]).toMatchObject({ kind: "number", value: "3.14" });

    // Boundary: 42. should be number 42 then dot
    const tokens2 = tokenize("42.", "test.md");
    expect(tokens2.map(t => t.kind)).toEqual(["number", "dot", "eof"]);
    expect(tokens2[0]!.value).toBe("42");
  });

  it("verifies multi-char operator spans", () => {
    const tokens = tokenize("==", "test.md");
    expect(tokens[0]!.kind).toBe("eqeq");
    expect(tokens[0]!.span.startColumn).toBe(1);
    expect(tokens[0]!.span.endColumn).toBe(3);
  });

  it("emits error token for unterminated strings", () => {
    const tokens = tokenize('"unclosed', "test.md");
    expect(tokens[0]!.kind).toBe("error");
    expect(tokens[0]!.value).toBe("Unterminated string literal");
  });

  it("emits error token for unknown characters", () => {
    const tokens = tokenize("@", "test.md");
    expect(tokens[0]!.kind).toBe("error");
    expect(tokens[0]!.value).toContain("Unexpected character");
  });
});
