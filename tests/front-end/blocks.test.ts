import { describe, expect, it } from "vitest";
import { parseMarkdown, collectLinks, parseParamItem, isTangleCodeBlock } from "../../src/index";

describe("front-end blocks", () => {
  it("collects markdown links as imports", () => {
    const tree = parseMarkdown("## 依赖\n\n[Math](./math.md)\n");
    const imports = collectLinks("main.md", tree);
    expect(imports).toEqual([
      expect.objectContaining({ alias: "Math", target: "./math.md" })
    ]);
  });

  it("parses list items as named params", () => {
    const param = parseParamItem("\`email\`: 邮箱 (String)", {
      file: "user.md",
      startLine: 2,
      startColumn: 1,
      endLine: 2,
      endColumn: 28
    });

    expect(param).toMatchObject({
      name: "email",
      description: "邮箱",
      typeName: "String"
    });
  });

  it("recognizes @tangle code blocks", () => {
    expect(isTangleCodeBlock({ type: "code", lang: "@tangle" })).toBe(true);
    expect(isTangleCodeBlock({ type: "code", lang: "ts" })).toBe(false);
  });
});
