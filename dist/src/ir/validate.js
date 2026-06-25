// Validate structural integrity of a RuleGraph
export function validateIR(graph) {
    const diags = [];
    const nodeIds = new Set(graph.nodes.map(n => n.id));
    // Check entryNodeId exists
    if (!nodeIds.has(graph.entryNodeId)) {
        diags.push({
            code: "TANGLE_IR_MISSING_ENTRY",
            message: `Entry node '${graph.entryNodeId}' not found in graph nodes`,
            span: { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
        });
    }
    // Check all edges reference existing nodes
    for (const edge of graph.edges) {
        if (!nodeIds.has(edge.from)) {
            diags.push({
                code: "TANGLE_IR_INVALID_EDGE",
                message: `Edge from '${edge.from}' references unknown node`,
                span: edge.sourceSpan
            });
        }
        if (!nodeIds.has(edge.to)) {
            diags.push({
                code: "TANGLE_IR_INVALID_EDGE",
                message: `Edge to '${edge.to}' references unknown node`,
                span: edge.sourceSpan
            });
        }
    }
    // Check error edges reference existing nodes
    for (const errEdge of graph.errorEdges) {
        if (!nodeIds.has(errEdge.from)) {
            diags.push({
                code: "TANGLE_IR_INVALID_ERROR_EDGE",
                message: `Error edge from '${errEdge.from}' references unknown node`,
                span: errEdge.sourceSpan
            });
        }
    }
    // Check no isolated nodes (every node except terminals should have at least one outgoing edge)
    const nodesWithOutgoing = new Set(graph.edges.map(e => e.from));
    for (const node of graph.nodes) {
        if (node.kind !== "terminal" && node.kind !== "error-terminal" && !nodesWithOutgoing.has(node.id) && node.id !== graph.entryNodeId) {
            diags.push({
                code: "TANGLE_IR_ISOLATED_NODE",
                message: `Node '${node.id}' has no outgoing edges`,
                span: node.sourceSpan
            });
        }
    }
    return diags;
}
