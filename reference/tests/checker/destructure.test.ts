import { describe, it, expect } from "vitest";
import { checkStmt } from "../../src/checker/checkModule.js";
import { createEnv } from "../../src/checker/env.js";
import type { Stmt, LetVariantStmt, LetRecordStmt } from "../../src/ast.js";
import type { TangleDiagnostic } from "../../src/model.js";

function span() {
  return {
    file: "test.tangle",
    startLine: 1,
    startColumn: 1,
    endLine: 1,
    endColumn: 5,
  };
}

describe("LetVariant / LetRecord via checkStmt", () => {
  it("LetVariant injects payload binding into env", () => {
    const env = createEnv();
    env.variables["x"] = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "primitive", name: "Int" }],
    };
    const stmt: LetVariantStmt = {
      kind: "letVariant",
      variantName: "Some",
      binding: "y",
      expr: { kind: "identifier", name: "x", span: span() },
      elseBranch: [],
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.length).toBe(0);
    expect(env.variables["y"]).toEqual({ kind: "primitive", name: "Int" });
  });

  it("LetVariant with null binding still runs elseBranch", () => {
    const env = createEnv();
    env.variables["x"] = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "primitive", name: "Int" }],
    };
    const stmt: LetVariantStmt = {
      kind: "letVariant",
      variantName: "Some",
      binding: null,
      expr: { kind: "identifier", name: "x", span: span() },
      elseBranch: [],
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.length).toBe(0);
  });

  it("LetVariant emits diag for unknown variant", () => {
    const env = createEnv();
    env.variables["x"] = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "primitive", name: "Int" }],
    };
    const stmt: LetVariantStmt = {
      kind: "letVariant",
      variantName: "NonExistent",
      binding: "y",
      expr: { kind: "identifier", name: "x", span: span() },
      elseBranch: [],
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.some((d) => d.code === "TANGLE_PATTERN_VARIANT_NOT_FOUND")).toBe(true);
  });

  it("LetVariant emits diag for non-sum type", () => {
    const env = createEnv();
    env.variables["x"] = { kind: "primitive", name: "Int" };
    const stmt: LetVariantStmt = {
      kind: "letVariant",
      variantName: "Some",
      binding: "y",
      expr: { kind: "identifier", name: "x", span: span() },
      elseBranch: [],
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.some((d) => d.code === "TANGLE_PATTERN_NOT_NARROWABLE")).toBe(true);
  });

  it("LetRecord injects all struct fields into env", () => {
    const env = createEnv();
    env.variables["r"] = {
      kind: "struct",
      name: "Item",
      fields: {
        name: { kind: "primitive", name: "String" },
        price: { kind: "primitive", name: "Int" },
      },
      methods: {},
    };
    const stmt: LetRecordStmt = {
      kind: "letRecord",
      fields: [["name", "n"], ["price", "p"]],
      expr: { kind: "identifier", name: "r", span: span() },
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.length).toBe(0);
    expect(env.variables["n"]).toEqual({ kind: "primitive", name: "String" });
    expect(env.variables["p"]).toEqual({ kind: "primitive", name: "Int" });
  });

  it("LetRecord emits diag for missing field", () => {
    const env = createEnv();
    env.variables["r"] = {
      kind: "struct",
      name: "Item",
      fields: {},
      methods: {},
    };
    const stmt: LetRecordStmt = {
      kind: "letRecord",
      fields: [["nonexistent", "x"]],
      expr: { kind: "identifier", name: "r", span: span() },
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.some((d) => d.code === "TANGLE_STRUCT_FIELD_NOT_FOUND")).toBe(true);
  });

  it("LetRecord emits diag for non-struct type", () => {
    const env = createEnv();
    env.variables["r"] = { kind: "primitive", name: "Int" };
    const stmt: LetRecordStmt = {
      kind: "letRecord",
      fields: [["name", "n"]],
      expr: { kind: "identifier", name: "r", span: span() },
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.some((d) => d.code === "TANGLE_DESTRUCTURE_NOT_STRUCT")).toBe(true);
  });

  it("LetRecord handles Any type by binding all to Any", () => {
    const env = createEnv();
    env.variables["r"] = { kind: "any" };
    const stmt: LetRecordStmt = {
      kind: "letRecord",
      fields: [["a", "x"], ["b", "y"]],
      expr: { kind: "identifier", name: "r", span: span() },
      span: span(),
    };
    const diags: TangleDiagnostic[] = [];
    checkStmt(stmt as Stmt, env, diags);
    expect(diags.length).toBe(0);
    expect(env.variables["x"]).toEqual({ kind: "any" });
    expect(env.variables["y"]).toEqual({ kind: "any" });
  });
});
