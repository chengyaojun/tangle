import { describe, expect, it } from "vitest";
import { parseMarkdown, spanFromNode } from "../../src/index";

describe("parseMarkdown", () => {
  it("parses headings and preserves source positions", () => {
    const tree = parseMarkdown("# App\n\n### User\n");
    expect(tree.type).toBe("root");
    expect(tree.children.map((child) => child.type)).toEqual(["heading", "heading"]);

    const first = tree.children[0]!;
    expect(spanFromNode("main.md", first)).toEqual({
      file: "main.md",
      startLine: 1,
      startColumn: 1,
      endLine: 1,
      endColumn: 6
    });
  });
});
