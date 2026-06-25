import { describe, expect, it } from "vitest";
import { checkPropagation, ErrorRegistry } from "../../src/index";

describe("checkPropagation", () => {
  it("strips error variant from sum type on ?", () => {
    const reg = new ErrorRegistry();
    reg.register("PayFailed", {});
    const sumType = {
      kind: "sum" as const,
      variants: [
        { kind: "struct" as const, name: "Receipt", fields: {}, methods: {} },
        { kind: "struct" as const, name: "PayFailed", fields: {}, methods: {} },
      ],
    };
    const [type, diags] = checkPropagation(sumType, reg);
    expect(diags).toEqual([]);
    expect(type).toMatchObject({ kind: "struct", name: "Receipt" });
  });

  it("returns original type for non-sum types", () => {
    const reg = new ErrorRegistry();
    const [type] = checkPropagation({ kind: "primitive", name: "Int" }, reg);
    expect(type).toEqual({ kind: "primitive", name: "Int" });
  });
});
