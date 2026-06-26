import { describe, expect, it } from "vitest";
import { lowerRuleToggle } from "../../src/index";

describe("lowerRuleToggle", () => {
  it("parses checkbox list into IR", () => {
    const graph = lowerRuleToggle(`- [x] \`enable_new_ui\`: 启用新 UI
- [ ] \`enable_crypto\`: 开启加密支付`, "test.md");
    expect(graph.nodes.length).toBe(2);
    expect(graph.nodes[0]!.label).toContain("true");
    expect(graph.nodes[1]!.label).toContain("false");
  });
});
