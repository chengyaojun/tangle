import { describe, expect, it } from "vitest";
import { lowerRuleFlow } from "../../src/index";
describe("lowerRuleFlow", () => {
    it("parses Mermaid nodes into IR nodes", () => {
        const graph = lowerRuleFlow(`graph TD
    A[Start]
    B(Process)`, "test.md");
        expect(graph.nodes).toHaveLength(2);
        expect(graph.nodes[0].label).toBe("Start");
        expect(graph.nodes[1].label).toBe("Process");
    });
    it("parses edges into IR edges", () => {
        const graph = lowerRuleFlow(`graph TD
    A[Start]
    B[End]
    A --> B`, "test.md");
        expect(graph.edges).toHaveLength(1);
        expect(graph.edges[0].from).toBe("A");
        expect(graph.edges[0].to).toBe("B");
    });
    it("parses guarded edges", () => {
        const graph = lowerRuleFlow(`graph TD
    A[New]
    B[Done]
    A -->|success| B`, "test.md");
        expect(graph.edges[0].kind).toBe("condition");
        expect(graph.edges[0].guard).toBe("success");
    });
    it("recognizes error terminals", () => {
        const graph = lowerRuleFlow(`graph TD
    A[Start]
    F(错误: PayFailed)`, "test.md");
        const errNode = graph.nodes.find(n => n.kind === "error-terminal");
        expect(errNode).toBeDefined();
    });
});
