import { describe, expect, it } from "vitest";
import { compileModule } from "../../src/index";

describe("symbol diagnostics", () => {
  it("allows exactly one @entry", () => {
    const mod = compileModule({
      file: "main.md",
      source: `# App
@entry

#### Start (start)
@entry
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
