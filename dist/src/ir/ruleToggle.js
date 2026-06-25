import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";
export function lowerRuleToggle(checkboxMarkdown, file) {
    resetNodeCounter();
    const graph = createGraph();
    const lines = checkboxMarkdown.trim().split("\n").filter(l => l.match(/^\s*-\s*\[/));
    const entryId = freshNodeId("toggle-entry");
    graph.entryNodeId = entryId;
    for (const line of lines) {
        const checked = line.includes("[x]");
        const labelMatch = line.match(/\`(\w+)\`:\s*(.+)/);
        const name = labelMatch?.[1] ?? "flag";
        const desc = labelMatch?.[2] ?? line.replace(/^\s*-\s*\[.\]\s*/, "");
        const nodeId = freshNodeId("toggle");
        graph.nodes.push({ kind: "compute", id: nodeId, label: `${name} = ${checked}`, sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
        graph.edges.push({ from: entryId, to: nodeId, kind: "control", sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    }
    return graph;
}
