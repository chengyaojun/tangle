import { describe, it, expect } from "vitest";
import { tangleTypeToPy, tangleTypeToGo } from "../../src/codegen/typeMap.js";

describe("tangleTypeToPy", () => {
  it("maps primitives", () => {
    expect(tangleTypeToPy({ kind: "primitive", name: "Int" })).toBe("int");
    expect(tangleTypeToPy({ kind: "primitive", name: "String" })).toBe("str");
    expect(tangleTypeToPy({ kind: "primitive", name: "Bool" })).toBe("bool");
    expect(tangleTypeToPy({ kind: "primitive", name: "Float" } as any)).toBe("float");
  });

  it("returns undefined for Any", () => {
    expect(tangleTypeToPy({ kind: "any" })).toBeUndefined();
  });

  it("maps List<Int>", () => {
    const ty = { kind: "genericInstance", base: "List", args: [{ kind: "primitive", name: "Int" }] };
    expect(tangleTypeToPy(ty as any)).toBe("List[int]");
  });

  it("maps Map<String,Int>", () => {
    const ty = { kind: "genericInstance", base: "Map", args: [{ kind: "primitive", name: "String" }, { kind: "primitive", name: "Int" }] };
    expect(tangleTypeToPy(ty as any)).toBe("Dict[str, int]");
  });

  it("maps Option<String>", () => {
    const ty = { kind: "genericInstance", base: "Option", args: [{ kind: "primitive", name: "String" }] };
    expect(tangleTypeToPy(ty as any)).toBe("Optional[str]");
  });
});

describe("tangleTypeToGo", () => {
  it("maps primitives", () => {
    expect(tangleTypeToGo({ kind: "primitive", name: "Int" })).toBe("int");
    expect(tangleTypeToGo({ kind: "primitive", name: "String" })).toBe("string");
    expect(tangleTypeToGo({ kind: "primitive", name: "Bool" })).toBe("bool");
    expect(tangleTypeToGo({ kind: "primitive", name: "Float" } as any)).toBe("float64");
  });

  it("returns any for Any", () => {
    expect(tangleTypeToGo({ kind: "any" })).toBe("any");
  });

  it("maps List<Int>", () => {
    const ty = { kind: "genericInstance", base: "List", args: [{ kind: "primitive", name: "Int" }] };
    expect(tangleTypeToGo(ty as any)).toBe("[]int");
  });

  it("maps Map<String,Int>", () => {
    const ty = { kind: "genericInstance", base: "Map", args: [{ kind: "primitive", name: "String" }, { kind: "primitive", name: "Int" }] };
    expect(tangleTypeToGo(ty as any)).toBe("map[string]int");
  });

  it("maps Option<String> to pointer", () => {
    const ty = { kind: "genericInstance", base: "Option", args: [{ kind: "primitive", name: "String" }] };
    expect(tangleTypeToGo(ty as any)).toBe("*string");
  });
});
