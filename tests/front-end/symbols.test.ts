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

  it("rejects exported semantic micro headings", () => {
    const mod = compileModule({
      file: "user.md",
      source: `# UserModule

##### 前置条件
@export
`
    });

    expect(mod.diagnostics).toEqual([
      expect.objectContaining({ code: "TANGLE_INVALID_EXPORT_LEVEL" })
    ]);
  });
});
