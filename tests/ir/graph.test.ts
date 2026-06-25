import { describe, expect, it } from "vitest";
import type { IRNode, IREdge, IRErrorEdge, RuleGraph } from "../../src/index";
import { createGraph, freshNodeId, resetNodeCounter } from "../../src/index";

describe("IR data structures", () => {
  it("defines IRNode variants", () => {
    const kinds = ["action", "compute", "decision", "terminal", "error-terminal"] as const;
    kinds.forEach(k => {
      const node: IRNode = { kind: k, id: "n", label: k, sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } };
      expect(node.kind).toBe(k);
    });
  });

  it("defines IREdge with guard", () => {
    const edge: IREdge = { from: "n1", to: "n2", kind: "condition", guard: "x > 0", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 10 } };
    expect(edge.guard).toBe("x > 0");
  });

  it("defines IRErrorEdge", () => {
    const errEdge: IRErrorEdge = { from: "n1", errorVariant: "PayFailed", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 10 } };
    expect(errEdge.errorVariant).toBe("PayFailed");
  });

  it("assembles a minimal RuleGraph", () => {
    const graph = createGraph("n1");
    expect(graph.nodes).toEqual([]);
    expect(graph.entryNodeId).toBe("n1");
  });

  it("generates unique node IDs", () => {
    resetNodeCounter();
    expect(freshNodeId()).toBe("n1");
    expect(freshNodeId("act")).toBe("act2");
  });
});
