import { describe, expect, it } from "vitest";
import { emitJS, createGraph } from "../../src/index";

describe("emitJS", () => {
  it("emits a minimal graph as a function", () => {
    const graph = createGraph("entry");
    graph.nodes.push({ kind: "terminal", id: "entry", label: "end", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    const js = emitJS(graph, "test");
    expect(js).toContain("function test");
    expect(js).toContain("Ok(undefined)");
  });

  it("includes runtime prelude", () => {
    const graph = createGraph("entry");
    graph.nodes.push({ kind: "terminal", id: "entry", label: "end", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    const js = emitJS(graph, "test");
    expect(js).toContain("__tangle_struct");
    expect(js).toContain("__tangle_update");
    expect(js).toContain("Ok");
    expect(js).toContain("Err");
  });

  it("emits error terminal as Err return", () => {
    const graph = createGraph("e1");
    graph.nodes.push({ kind: "error-terminal", id: "e1", label: "错误: PayFailed", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    const js = emitJS(graph, "test");
    expect(js).toContain("PayFailed");
    expect(js).toContain('Err("PayFailed"');
  });
});
