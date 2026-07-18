import { describe, it, expect } from "vitest";
import {
  getVariantName,
  bindingTypeOf,
  findVariantByName,
  checkMatchExhaustiveness,
} from "../../src/checker/match.js";
import type { Type } from "../../src/checker/types.js";

function prim(name: "String" | "Int" | "Bool"): Type {
  return { kind: "primitive", name };
}

function generic(base: string, args: Type[]): Type {
  return { kind: "genericInstance", base, args };
}

function structType(name: string): Type {
  return { kind: "struct", name, fields: {}, methods: {} };
}

describe("getVariantName", () => {
  it("returns name for primitive", () => {
    expect(getVariantName(prim("Int"))).toBe("Int");
  });

  it("returns name for struct", () => {
    expect(getVariantName(structType("Order"))).toBe("Order");
  });

  it("returns base for genericInstance", () => {
    expect(getVariantName(generic("Some", [prim("Int")]))).toBe("Some");
  });

  it("returns null for sum", () => {
    expect(getVariantName({ kind: "sum", variants: [] })).toBeNull();
  });
});

describe("bindingTypeOf", () => {
  it("returns payload for genericInstance", () => {
    expect(bindingTypeOf(generic("Some", [prim("Int")]))).toEqual(prim("Int"));
  });

  it("returns any for genericInstance without args", () => {
    expect(bindingTypeOf(generic("Some", []))).toEqual({ kind: "any" });
  });

  it("returns itself for primitive", () => {
    expect(bindingTypeOf(prim("Int"))).toEqual(prim("Int"));
  });
});

describe("findVariantByName", () => {
  it("finds variant by name", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(findVariantByName(sum, "String")).toEqual(prim("String"));
  });

  it("returns null when not found", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int")] };
    expect(findVariantByName(sum, "Bool")).toBeNull();
  });
});

describe("checkMatchExhaustiveness", () => {
  it("all covered", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(checkMatchExhaustiveness(sum, ["Int", "String"])).toEqual([]);
  });

  it("missing variant", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(checkMatchExhaustiveness(sum, ["Int"])).toEqual(["String"]);
  });

  it("wildcard covers all", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(checkMatchExhaustiveness(sum, ["Int", "_"])).toEqual([]);
  });

  it("genericInstance variant", () => {
    // 用 generic("None", []) 表示无 payload 的 None variant
    // （prim("None") 不合法，因 primitive.name 限定为 String|Int|Bool）
    const sum: Type = {
      kind: "sum",
      variants: [generic("Some", [prim("Int")]), generic("None", [])],
    };
    expect(checkMatchExhaustiveness(sum, ["Some", "None"])).toEqual([]);
  });
});
