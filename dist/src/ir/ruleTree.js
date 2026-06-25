import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";
// Parse a tree-like nested list into IR. Simple approach: each line is a node, indentation determines parent/child.
export function lowerRuleTree(listMarkdown, file) {
    resetNodeCounter();
    const graph = createGraph();
    const lines = listMarkdown.trim().split("\n").filter(l => l.match(/^\s*[\*\-]\s/));
    let prevId = freshNodeId("entry");
    graph.entryNodeId = prevId;
    for (const line of lines) {
        const content = line.replace(/^\s*[\*\-]\s+/, "").trim();
        const nodeId = freshNodeId("tree");
        graph.nodes.push({ kind: "decision", id: nodeId, label: content, sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
        graph.edges.push({ from: prevId, to: nodeId, kind: "condition", guard: content, sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
        prevId = nodeId;
    }
    return graph;
}
