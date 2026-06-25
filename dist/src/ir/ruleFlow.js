import { createGraph, resetNodeCounter } from "./graph.js";
// Parse a Mermaid graph TD string into IR nodes and edges
export function lowerRuleFlow(mermaidSource, file) {
    resetNodeCounter();
    const graph = createGraph();
    const nodeMap = new Map();
    const lines = mermaidSource.split("\n").map(l => l.trim()).filter(l => l.length > 0 && !l.startsWith("graph"));
    for (const line of lines) {
        // Node declaration: X[Label] or X(Label)
        const nodeMatch = line.match(/^\s*(\w+)\s*[\[\(](.+?)[\]\)]\s*$/);
        if (nodeMatch) {
            const id = nodeMatch[1];
            const label = nodeMatch[2];
            let kind = "action";
            if (label.startsWith("错误:") || label.startsWith("error:")) {
                kind = "error-terminal";
            }
            const node = { kind, id, label, sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } };
            nodeMap.set(id, node);
            graph.nodes.push(node);
            continue;
        }
        // Edge: A -->|guard| B or A --> B
        const edgeMatch = line.match(/^\s*(\w+)\s*-->\s*(?:\|(.+?)\|)?\s*(\w+)\s*$/);
        if (edgeMatch) {
            const from = edgeMatch[1];
            const guard = edgeMatch[2];
            const to = edgeMatch[3];
            graph.edges.push({
                from, to,
                kind: guard ? "condition" : "control",
                ...(guard ? { guard } : {}),
                sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
            });
        }
    }
    if (graph.nodes.length > 0) {
        graph.entryNodeId = graph.nodes[0].id;
    }
    return graph;
}
