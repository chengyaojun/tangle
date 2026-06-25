// Helper to create an empty graph
export function createGraph(entryNodeId) {
    return { nodes: [], edges: [], errorEdges: [], entryNodeId: entryNodeId ?? "entry" };
}
// Helper to generate unique node IDs
let nodeCounter = 0;
export function freshNodeId(prefix) {
    return `${prefix ?? "n"}${++nodeCounter}`;
}
export function resetNodeCounter() { nodeCounter = 0; }
