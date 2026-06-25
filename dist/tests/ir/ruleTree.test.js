import { describe, expect, it } from "vitest";
import { lowerRuleTree } from "../../src/index";
describe("lowerRuleTree", () => {
    it("parses markdown list into decision tree IR", () => {
        const graph = lowerRuleTree(`* 收入门槛：user.income >= 10000
* 信用良好：user.credit_score > 700
* 资产证明：user.has_house == true`, "test.md");
        expect(graph.nodes.length).toBe(3);
        expect(graph.edges.length).toBe(3);
    });
});
