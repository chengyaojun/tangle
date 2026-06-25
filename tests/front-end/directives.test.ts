import { describe, expect, it } from "vitest";
import { compileModule, parseDirectiveLine } from "../../src/index";

describe("parseDirectiveLine", () => {
  it("parses simple directives", () => {
    const directive = parseDirectiveLine("@export", {
      file: "main.md",
      startLine: 2,
      startColumn: 1,
      endLine: 2,
      endColumn: 8
    });

    expect(directive).toMatchObject({ kind: "export", raw: "@export" });
  });

  it("parses error directives with names and args", () => {
    const directive = parseDirectiveLine('@error PayFailed("支付失败", code: Int)', {
      file: "pay.md",
      startLine: 3,
      startColumn: 1,
      endLine: 3,
      endColumn: 39
    });

    expect(directive).toMatchObject({
      kind: "error",
      name: "PayFailed",
      args: '"支付失败", code: Int'
    });
  });

  it("rejects unknown directives", () => {
    expect(() =>
      parseDirectiveLine("@unknown", {
        file: "main.md",
        startLine: 1,
        startColumn: 1,
        endLine: 1,
        endColumn: 9
      })
    ).toThrow("Unknown Tangle directive");
  });
});

describe("directive placement", () => {
  it("reports directives embedded in ordinary paragraphs", () => {
    const mod = compileModule({
      file: "bad.md",
      source: `# Bad

这是一段普通说明，里面出现 @export 是非法的。
`
    });

    expect(mod.diagnostics).toEqual([
      expect.objectContaining({
        code: "TANGLE_INVALID_DIRECTIVE_POSITION",
        message: "Tangle directives must appear directly under a heading or directly above their target block"
      })
    ]);
  });
});
