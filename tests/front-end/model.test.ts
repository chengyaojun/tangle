import { describe, expect, it } from "vitest";
import type { TangleHeading, TangleModule } from "../../src/index";

describe("Tangle frontend model", () => {
  it("represents a module with headings, imports, symbols, and diagnostics", () => {
    const heading: TangleHeading = {
      id: "user",
      depth: 3,
      role: "type",
      title: "User",
      symbolName: "User",
      directives: [],
      span: { file: "user.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 8 },
      children: []
    };

    const mod: TangleModule = {
      file: "user.md",
      moduleName: "user",
      imports: [],
      headings: [heading],
      symbols: [],
      diagnostics: []
    };

    expect(mod.headings[0]?.role).toBe("type");
  });
});
