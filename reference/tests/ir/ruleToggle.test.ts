import { describe, it, expect } from "vitest";
import { lowerRuleToggle } from "../../src/ir/ruleToggle.js";

describe("lowerRuleToggle", () => {
  it("ruleToggle_lower_basic_checkbox: basic checkbox lowering", () => {
    const md = "- [x] `enable_flag`: Enable the feature";
    const result = lowerRuleToggle(md, "test.tangle");

    expect(result.graph.nodes.length).toBe(2); // entry + 1 toggle
    expect(result.graph.nodes[0].kind).toBe("compute");
    expect(result.graph.nodes[0].label).toBe("toggle.entry");
    expect(result.graph.nodes[1].label).toBe("enable_flag = true");
    expect(result.graph.edges.length).toBe(1);
    expect(result.graph.edges[0].kind).toBe("control");
  });

  it("ruleToggle_lower_with_group_style: group/style 附加", () => {
    const md = `<!-- group: UI -->
<!-- style: highlight -->
- [x] \`enable_ui\`: Enable UI`;
    const result = lowerRuleToggle(md, "test.tangle");

    expect(result.graph.nodes[1].group).toBe("UI");
    expect(result.graph.nodes[1].style).toBe("highlight");
  });

  it("ruleToggle_lower_uppercase_x: [X] 大写支持", () => {
    const md = "- [X] `flag`: desc";
    const result = lowerRuleToggle(md, "test.tangle");

    expect(result.graph.nodes[1].label).toBe("flag = true");
  });

  it("ruleToggle_lower_colon_name: colon 名称提取", () => {
    const md = "- [x] flag_name: true";
    const result = lowerRuleToggle(md, "test.tangle");

    expect(result.graph.nodes[1].label).toBe("flag_name = true");
  });

  it("ruleToggle_lower_missing_name_diagnostic: 缺名称发诊断", () => {
    const md = "- [x] no name here";
    const result = lowerRuleToggle(md, "test.tangle");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_TOGGLE_MISSING_NAME")).toBe(true);
    expect(result.graph.nodes[1].label).toContain("toggle_0");
  });

  it("ruleToggle_lower_malformed_diagnostic: 畸形发诊断", () => {
    const md = "- [y] `flag`: desc";
    const result = lowerRuleToggle(md, "test.tangle");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_TOGGLE_MALFORMED")).toBe(true);
  });
});
