import { describe, expect, it } from "vitest";
import { validateIR, createGraph } from "../../src/index";

describe("validateIR", () => {
  it("accepts valid graph", () => {
    const graph = createGraph("n1");
    graph.nodes.push({ kind: "terminal", id: "n1", label: "end", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    const diags = validateIR(graph);
    expect(diags).toEqual([]);
  });

  it("reports missing entry node", () => {
    const graph = createGraph("missing");
    const diags = validateIR(graph);
    expect(diags.some(d => d.code === "TANGLE_IR_MISSING_ENTRY")).toBe(true);
  });

  it("reports invalid edge references", () => {
    const graph = createGraph("n1");
    graph.nodes.push({ kind: "terminal", id: "n1", label: "end", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    graph.edges.push({ from: "n1", to: "nope", kind: "control", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    const diags = validateIR(graph);
    expect(diags.some(d => d.code === "TANGLE_IR_INVALID_EDGE")).toBe(true);
  });
});
