import { describe, expect, it } from "vitest";
import { parseTypeExpr } from "../../src/index";

describe("parseTypeExpr", () => {
  it("parses primitive types", () => {
    expect(parseTypeExpr("String", "test.md")).toMatchObject({ kind: "primitiveType", name: "String" });
    expect(parseTypeExpr("Int", "test.md")).toMatchObject({ kind: "primitiveType", name: "Int" });
    expect(parseTypeExpr("Bool", "test.md")).toMatchObject({ kind: "primitiveType", name: "Bool" });
  });

  it("parses named type references", () => {
    expect(parseTypeExpr("User", "test.md")).toMatchObject({ kind: "namedType", name: "User" });
  });

  it("parses sum types with pipe", () => {
    const t = parseTypeExpr("Receipt | PayFailed | Timeout", "test.md");
    expect(t).toMatchObject({
      kind: "sumType",
      variants: [
        { kind: "namedType", name: "Receipt" },
        { kind: "namedType", name: "PayFailed" },
        { kind: "namedType", name: "Timeout" }
      ]
    });
  });

  it("parses generic types", () => {
    expect(parseTypeExpr("List<Int>", "test.md")).toMatchObject({
      kind: "genericType",
      base: "List",
      typeArgs: [{ kind: "primitiveType", name: "Int" }]
    });
  });

  it("parses nested generics", () => {
    const t = parseTypeExpr("Option<List<Int>>", "test.md");
    expect(t).toMatchObject({
      kind: "genericType",
      base: "Option",
      typeArgs: [{
        kind: "genericType",
        base: "List",
        typeArgs: [{ kind: "primitiveType", name: "Int" }]
      }]
    });
  });

  it("parses function types", () => {
    expect(parseTypeExpr("(Int, String) -> Bool", "test.md")).toMatchObject({
      kind: "functionType",
      params: [
        { kind: "primitiveType", name: "Int" },
        { kind: "primitiveType", name: "String" }
      ],
      returns: { kind: "primitiveType", name: "Bool" }
    });
  });
});
