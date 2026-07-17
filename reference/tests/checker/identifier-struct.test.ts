import { describe, it, expect } from "vitest";
import { checkExpression } from "../../src/checker/check.js";
import { createEnv } from "../../src/checker/env.js";
import type { StructType } from "../../src/checker/types.js";

describe("identifier resolves struct", () => {
  it("returns struct type when identifier matches env.structs", () => {
    const env = createEnv();
    const orderStruct: StructType = {
      kind: "struct",
      name: "Order",
      fields: { id: { kind: "primitive", name: "String" } },
      methods: {},
    };
    env.structs["Order"] = orderStruct;

    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Order", span: { file: "test.tangle", startLine: 1, startColumn: 1, endLine: 1, endColumn: 5 } },
      env
    );

    expect(diags).toHaveLength(0);
    expect(type.kind).toBe("struct");
    if (type.kind === "struct") {
      expect(type.name).toBe("Order");
    }
  });

  it("returns undefined variable diagnostic for unknown identifier", () => {
    const env = createEnv();
    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Unknown", span: { file: "test.tangle", startLine: 1, startColumn: 1, endLine: 1, endColumn: 7 } },
      env
    );
    expect(diags.some(d => d.code === "TANGLE_TYPE_UNDEFINED_VARIABLE")).toBe(true);
  });
});
