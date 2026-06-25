import { describe, expect, it } from "vitest";
import { compileModule } from "../../src/index";

describe("symbol diagnostics", () => {
  it("detects main function as entry point", () => {
    const mod = compileModule({
      file: "main.md",
      source: `#### main
`
    });
    const entry = mod.symbols.find(s => s.kind === "entry");
    expect(entry).toBeDefined();
    expect(entry?.name).toBe("main");
  });

  it("reports duplicate main functions", () => {
    const mod = compileModule({
      file: "main.md",
      source: `#### main

#### main
`
    });
    expect(mod.diagnostics).toEqual([
      expect.objectContaining({ code: "TANGLE_DUPLICATE_ENTRY" })
    ]);
  });

  it("marks underscore-prefixed symbols as private", () => {
    const mod = compileModule({
      file: "test.md",
      source: `### _InternalHelper

#### _reset
`
    });
    const privType = mod.symbols.find(s => s.name === "_InternalHelper");
    expect(privType?.exported).toBe(false);
  });
});
