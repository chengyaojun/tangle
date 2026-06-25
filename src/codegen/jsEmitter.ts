import type { RuleGraph, IRNode } from "../ir/graph.js";
import { RUNTIME_PRELUDE } from "./prelude.js";

export function emitJS(graph: RuleGraph, moduleName?: string): string {
  const lines: string[] = [];
  lines.push(RUNTIME_PRELUDE);
  lines.push("");
  lines.push(`// Generated from ${moduleName ?? "module"}`);
  lines.push(`function ${moduleName ?? "main"}() {`);

  // Emit nodes as labeled blocks
  const nodeMap = new Map<string, IRNode>();
  for (const node of graph.nodes) {
    nodeMap.set(node.id, node);
  }

  // Simple topological emission: entry → edges → nodes
  const visited = new Set<string>();
  const queue = [graph.entryNodeId];

  while (queue.length > 0) {
    const nodeId = queue.shift()!;
    if (visited.has(nodeId)) continue;
    visited.add(nodeId);

    const node = nodeMap.get(nodeId);
    if (!node) continue;

    switch (node.kind) {
      case "action":
      case "compute":
        lines.push(`  // ${node.label}`);
        break;
      case "decision":
        lines.push(`  // decision: ${node.label}`);
        break;
      case "terminal":
        lines.push(`  return Ok(undefined);`);
        break;
      case "error-terminal":
        lines.push(`  return Err("${node.label.replace(/^错误:\s*/, "").replace(/^error:\s*/i, "")}");`);
        break;
    }

    // Follow edges
    const outgoing = graph.edges.filter(e => e.from === nodeId);
    for (const edge of outgoing) {
      if (!visited.has(edge.to)) queue.push(edge.to);
    }
  }

  lines.push("}");
  lines.push("");
  lines.push(`// Entry point`);
  lines.push(`const result = ${moduleName ?? "main"}();`);
  lines.push(`if (result && result.ok === false) {`);
  lines.push(`  console.error("Error:", result.error);`);
  lines.push(`  process.exit(1);`);
  lines.push(`}`);
  lines.push(`console.log("OK:", result);`);

  return lines.join("\n");
}
