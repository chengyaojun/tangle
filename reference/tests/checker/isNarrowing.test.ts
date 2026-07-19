import { describe, it, expect } from "vitest";
import { checkExpression } from "../../src/checker/check.js";
import { createEnv } from "../../src/checker/env.js";
import type { Expr, IfExpr, IsExpr } from "../../src/ast.js";

function span() {
  return {
    file: "test.tangle",
    startLine: 1,
    startColumn: 1,
    endLine: 1,
    endColumn: 5,
  };
}

describe("IsExpr narrowing", () => {
  it("returns Bool type for valid pattern with no diagnostics", () => {
    const env = createEnv();
    env.variables["x"] = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "primitive", name: "Int" }],
    };
    const isExpr: IsExpr = {
      kind: "is",
      expr: { kind: "identifier", name: "x", span: span() },
      pattern: { kind: "variant", name: "Some", binding: "y", span: span() },
      span: span(),
    };
    const [ty, diags] = checkExpression(isExpr, env);
    expect(ty).toEqual({ kind: "primitive", name: "Bool" });
    expect(diags.length).toBe(0);
  });

  it("emits TANGLE_PATTERN_VARIANT_NOT_FOUND for unknown variant", () => {
    const env = createEnv();
    env.variables["x"] = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "primitive", name: "Int" }],
    };
    const isExpr: IsExpr = {
      kind: "is",
      expr: { kind: "identifier", name: "x", span: span() },
      pattern: { kind: "variant", name: "NonExistent", span: span() },
      span: span(),
    };
    const [, diags] = checkExpression(isExpr, env);
    expect(diags.some((d) => d.code === "TANGLE_PATTERN_VARIANT_NOT_FOUND")).toBe(true);
  });

  it("emits TANGLE_PATTERN_NOT_NARROWABLE for non-sum type", () => {
    const env = createEnv();
    env.variables["x"] = { kind: "primitive", name: "Int" };
    const isExpr: IsExpr = {
      kind: "is",
      expr: { kind: "identifier", name: "x", span: span() },
      pattern: { kind: "variant", name: "Some", span: span() },
      span: span(),
    };
    const [, diags] = checkExpression(isExpr, env);
    expect(diags.some((d) => d.code === "TANGLE_PATTERN_NOT_NARROWABLE")).toBe(true);
  });

  it("injects binding into then-branch env when condition is IsExpr", () => {
    const env = createEnv();
    env.variables["x"] = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "primitive", name: "Int" }],
    };
    const isExpr: IsExpr = {
      kind: "is",
      expr: { kind: "identifier", name: "x", span: span() },
      pattern: { kind: "variant", name: "Some", binding: "y", span: span() },
      span: span(),
    };
    const ifExpr: IfExpr = {
      kind: "if",
      condition: isExpr,
      thenBranch: { kind: "identifier", name: "y", span: span() },
      elseBranch: { kind: "literal", literalKind: "number", value: 0, span: span() },
      span: span(),
    };
    const [ty, diags] = checkExpression(ifExpr, env);
    // Then-branch yields Int (binding y: Int injected), else yields Int → unified Int
    expect(diags.length).toBe(0);
    expect(ty).toEqual({ kind: "primitive", name: "Int" });
  });

  it("resolveStructInEnv applied to struct payload binding", () => {
    // When Option<Item> is matched and Item has fields in env.structs,
    // the binding should be the full struct (with fields), not the empty shell.
    const env = createEnv();
    env.structs["Item"] = {
      kind: "struct",
      name: "Item",
      fields: { name: { kind: "primitive", name: "String" } },
      methods: {},
    };
    env.variables["opt"] = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "struct", name: "Item", fields: {}, methods: {} }],
    };
    const isExpr: IsExpr = {
      kind: "is",
      expr: { kind: "identifier", name: "opt", span: span() },
      pattern: { kind: "variant", name: "Some", binding: "item", span: span() },
      span: span(),
    };
    const ifExpr: IfExpr = {
      kind: "if",
      condition: isExpr,
      thenBranch: {
        kind: "memberAccess",
        object: { kind: "identifier", name: "item", span: span() },
        member: "name",
        span: span(),
      },
      elseBranch: { kind: "literal", literalKind: "string", value: "", span: span() },
      span: span(),
    };
    const [, diags] = checkExpression(ifExpr as Expr, env);
    // Should not emit TANGLE_TYPE_UNKNOWN_FIELD for `item.name`
    expect(diags.some((d) => d.code === "TANGLE_TYPE_UNKNOWN_FIELD")).toBe(false);
  });
});
