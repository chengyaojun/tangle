import { describe, expect, it } from "vitest";
import { lowerStatements } from "../../src/index";
import { tokenize, parseCodeBody } from "../../src/index";

describe("lowerStatements", () => {
  it("lowers a simple return to IR nodes and edges", () => {
    const tokens = tokenize("return 42", "test.md");
    const body = parseCodeBody(tokens);
    const graph = lowerStatements(body.statements, "test.md");
    expect(graph.nodes.length).toBeGreaterThanOrEqual(2); // entry + return + terminal
    expect(graph.edges.length).toBeGreaterThanOrEqual(1);
    expect(graph.entryNodeId).toBeTruthy();
  });

  it("lowers multiple statements with sequential edges", () => {
    const tokens = tokenize("let a = 1\nlet b = 2\nreturn a + b", "test.md");
    const body = parseCodeBody(tokens);
    const graph = lowerStatements(body.statements, "test.md");
    expect(graph.nodes.length).toBeGreaterThanOrEqual(3);
    expect(graph.edges.length).toBeGreaterThanOrEqual(2);
  });

  it("empty statements produce entry to terminal graph", () => {
    const tokens = tokenize("", "test.md");
    const body = parseCodeBody(tokens);
    const graph = lowerStatements(body.statements, "test.md");
    expect(graph.nodes.length).toBe(2); // entry + terminal
    expect(graph.edges.length).toBe(1); // entry to terminal
  });
});
