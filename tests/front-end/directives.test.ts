import { describe, expect, it } from "vitest";
import { parseDirectiveLine, compileModule } from "../../src/index";

describe("zero-directive language", () => {
  it("rejects all @-prefixed text as unknown", () => {
    expect(() => parseDirectiveLine("@anything", {
      file: "m.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 10
    })).toThrow("Unknown Tangle directive");
  });

  it("treats @-text in paragraphs as ordinary prose", () => {
    const mod = compileModule({
      file: "ok.md",
      source: `# OK
普通段落中有 @something 不会报错。
`
    });
    expect(mod.diagnostics.filter(d => d.code === "TANGLE_INVALID_DIRECTIVE_POSITION")).toHaveLength(0);
  });
});
