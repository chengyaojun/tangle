import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";
export function lowerStatements(stmts, file) {
    resetNodeCounter();
    const graph = createGraph();
    const entryNode = {
        kind: "compute", id: freshNodeId("entry"), label: "entry",
        sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
    };
    graph.nodes.push(entryNode);
    graph.entryNodeId = entryNode.id;
    let prevId = entryNode.id;
    for (const stmt of stmts) {
        const node = lowerStmt(stmt, file);
        graph.nodes.push(node);
        graph.edges.push({
            from: prevId, to: node.id, kind: "control",
            sourceSpan: node.sourceSpan
        });
        prevId = node.id;
    }
    // Terminal node
    const terminal = {
        kind: "terminal", id: freshNodeId("end"), label: "return",
        sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
    };
    graph.nodes.push(terminal);
    graph.edges.push({ from: prevId, to: terminal.id, kind: "control", sourceSpan: terminal.sourceSpan });
    return graph;
}
function lowerStmt(stmt, file) {
    const span = stmt.span;
    switch (stmt.kind) {
        case "return":
            return { kind: "action", id: freshNodeId("ret"), label: "return", sourceSpan: span };
        case "let":
        case "const":
            return { kind: "compute", id: freshNodeId("bind"), label: `${stmt.kind} ${stmt.name}`, sourceSpan: span };
        case "expression":
            return { kind: "action", id: freshNodeId("expr"), label: "expression", sourceSpan: span };
        default:
            return { kind: "compute", id: freshNodeId("stmt"), label: stmt.kind, sourceSpan: span };
    }
}
