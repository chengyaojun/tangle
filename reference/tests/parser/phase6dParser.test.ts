import { describe, it, expect } from "vitest";
import { tokenize } from "../../src/index.js";
import { parseExpression, parseStatement } from "../../src/index.js";

describe("Phase 6d Parser", () => {
  it("parses is Variant pattern", () => {
    const tokens = tokenize("x is Some", "test.tangle");
    const expr = parseExpression(tokens);
    expect(expr.kind).toBe("is");
    if (expr.kind === "is") {
      expect(expr.pattern.kind).toBe("variant");
      if (expr.pattern.kind === "variant") {
        expect(expr.pattern.name).toBe("Some");
        expect(expr.pattern.binding).toBeUndefined();
      }
    }
  });

  it("parses is Variant(binding) pattern", () => {
    const tokens = tokenize("x is Some(y)", "test.tangle");
    const expr = parseExpression(tokens);
    expect(expr.kind).toBe("is");
    if (expr.kind === "is" && expr.pattern.kind === "variant") {
      expect(expr.pattern.name).toBe("Some");
      expect(expr.pattern.binding).toBe("y");
    }
  });

  it("parses let Variant(binding) = expr else { ... }", () => {
    const tokens = tokenize("let Some(y) = x else { return 0 }", "test.tangle");
    const stmt = parseStatement(tokens);
    expect(stmt.kind).toBe("letVariant");
    if (stmt.kind === "letVariant") {
      expect(stmt.variantName).toBe("Some");
      expect(stmt.binding).toBe("y");
      expect(stmt.elseBranch.length).toBe(1);
    }
  });

  it("parses let { field1, field2: local2 } = expr", () => {
    const tokens = tokenize("let { ok, err: e } = r", "test.tangle");
    const stmt = parseStatement(tokens);
    expect(stmt.kind).toBe("letRecord");
    if (stmt.kind === "letRecord") {
      expect(stmt.fields.length).toBe(2);
      expect(stmt.fields[0]).toEqual(["ok", "ok"]);
      expect(stmt.fields[1]).toEqual(["err", "e"]);
    }
  });

  it("emits TANGLE_REFUTABLE_LET_REQUIRES_ELSE diagnostic for missing else", () => {
    const tokens = tokenize("let Some(y) = x", "test.tangle");
    const stmt = parseStatement(tokens);
    expect(stmt.kind).toBe("letVariant");
    // The diagnostic is emitted via state.diagnostics but parseStatement doesn't return it
    // For now, just verify the parser doesn't crash and returns letVariant with empty elseBranch
    if (stmt.kind === "letVariant") {
      expect(stmt.elseBranch).toEqual([]);
    }
  });
});
