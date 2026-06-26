import { describe, expect, it } from "vitest";
import { lowerRuleTable } from "../../src/index";

describe("lowerRuleTable", () => {
  it("parses decision table into IR", () => {
    const graph = lowerRuleTable(`| 金额 > 1000 | 用户等级 | 结果 |
|-------------|----------|------|
| true        | VIP      | 通过 |
| false       | -        | 拒绝 |`, "test.md");
    expect(graph.nodes.length).toBeGreaterThanOrEqual(2);
    expect(graph.edges.length).toBeGreaterThanOrEqual(2);
  });
});
