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

  it("parses list items with type only and no description", () => {
    // Phase 6c: 支持 `name`: (Type) 形式（无 description），与 Rust parse_param_item 一致
    const param = parseParamItem("\`config\`: (Config)", {
      file: "order.md",
      startLine: 4,
      startColumn: 1,
      endLine: 4,
      endColumn: 20
    });

    expect(param).toMatchObject({
      name: "config",
      description: "",
      typeName: "Config"
    });
  });

  it("recognizes @tangle code blocks", () => {
    expect(isTangleCodeBlock({ type: "code", lang: "@tangle" })).toBe(true);
    expect(isTangleCodeBlock({ type: "code", lang: "ts" })).toBe(false);
  });
});
