import { describe, expect, it } from "vitest";
import { compileModule } from "../../src/index";
import { USER_MODULE } from "../fixtures";

describe("compileModule", () => {
  it("builds a TangleModule with imports, headings, params, code blocks, and symbols", () => {
    const mod = compileModule({ file: "user.md", source: USER_MODULE });

    expect(mod.moduleName).toBe("user");
    expect(mod.imports).toEqual([
      expect.objectContaining({ alias: "Notify", target: "./notify.md" })
    ]);

    expect(mod.headings.map((heading) => [heading.depth, heading.role, heading.title])).toEqual([
      [1, "program", "\u7528\u6237\u4E2D\u5FC3"],
      [2, "section", "\u4F9D\u8D56"],
      [3, "type", "User"],
      [4, "callable", "\u6FC0\u6D3B"],
      [5, "semantic-section", "\u524D\u7F6E\u6761\u4EF6"],
      [6, "semantic-atom", "\u90AE\u7BB1\u5B58\u5728"]
    ]);

    // buildSymbols creates symbols for all headings
    expect(mod.symbols.map((s) => [s.name, s.kind, s.exported])).toEqual([
      ["\u7528\u6237\u4E2D\u5FC3", "semantic-internal", false],
      ["\u4F9D\u8D56", "semantic-internal", false],
      ["User", "type", true],
      ["activate", "callable", true],
      ["\u524D\u7F6E\u6761\u4EF6", "semantic-internal", false],
      ["\u90AE\u7BB1\u5B58\u5728", "semantic-internal", false]
    ]);

    const userHeading = mod.headings.find((heading) => heading.title === "User");
    expect(userHeading?.params).toEqual([
      expect.objectContaining({ name: "id", typeName: "Int" }),
      expect.objectContaining({ name: "email", typeName: "String" })
    ]);

    const callable = mod.headings.find((heading) => heading.symbolName === "activate");
    expect(callable?.codeBlocks).toEqual([
      expect.objectContaining({ language: "tangle", value: "return this { is_active: true }" })
    ]);
  });
});
