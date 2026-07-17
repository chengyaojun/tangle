import { describe, it, expect } from "vitest";
import { checkExpression } from "../../src/checker/check.js";
import { createEnv } from "../../src/checker/env.js";
import { registerBuiltins } from "../../src/checker/builtins.js";
import type { StructType } from "../../src/checker/types.js";
import type { Expr } from "../../src/ast.js";

const span = { file: "t", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 };

describe("Err/Ok constructors", () => {
  it("resolves Err as function type", () => {
    const env = createEnv();
    registerBuiltins(env);
    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Err", span },
      env
    );
    expect(diags).toHaveLength(0);
    expect(type.kind).toBe("function");
  });

  it("resolves Ok as function type", () => {
    const env = createEnv();
    registerBuiltins(env);
    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Ok", span },
      env
    );
    expect(diags).toHaveLength(0);
    expect(type.kind).toBe("function");
  });

  it("calling Err returns Any type", () => {
    const env = createEnv();
    registerBuiltins(env);
    const callExpr: Expr = {
      kind: "call",
      callee: { kind: "identifier", name: "Err", span },
      args: [
        { kind: "literal", literalKind: "string", value: "NotFound", span },
        { kind: "literal", literalKind: "string", value: "not found", span },
      ],
      span,
    };
    const [type] = checkExpression(callExpr, env);
    expect(type.kind).toBe("any");
  });

  it("struct constructor call returns struct type", () => {
    const env = createEnv();
    const orderStruct: StructType = {
      kind: "struct",
      name: "Order",
      fields: { id: { kind: "primitive", name: "String" } },
      methods: {},
    };
    env.structs["Order"] = orderStruct;
    const callExpr: Expr = {
      kind: "call",
      callee: { kind: "identifier", name: "Order", span },
      args: [],
      span,
    };
    const [type] = checkExpression(callExpr, env);
    expect(type.kind).toBe("struct");
    if (type.kind === "struct") {
      expect(type.name).toBe("Order");
    }
  });
});
