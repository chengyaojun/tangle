import type { RuleGraph } from "./graph.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

export function lowerRuleTable(tableMarkdown: string, file: string): RuleGraph {
  resetNodeCounter();
  const graph = createGraph();
  const lines = tableMarkdown.trim().split("\n").filter(l => l.includes("|"));
  if (lines.length < 2) return graph;

  const headers = lines[0]!.split("|").map(h => h.trim()).filter(Boolean);
  const actionCol = headers[headers.length - 1]!;
  const condCols = headers.slice(0, -1);

  const entryId = freshNodeId("entry");
  graph.entryNodeId = entryId;

  for (let i = 1; i < lines.length; i++) {
    const cells = lines[i]!.split("|").map(c => c.trim()).filter(Boolean);
    if (cells.length < 2) continue;
    const action = cells[cells.length - 1]!;
    const conds = cells.slice(0, -1);

    const actionNodeId = freshNodeId("action");
    const guardStr = conds.map((c, j) => c !== "-" && c !== "" ? `${condCols[j]} = ${c}` : null).filter(Boolean).join(" AND ");
    graph.nodes.push({ kind: "action", id: actionNodeId, label: action, sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } });
    graph.edges.push({
      from: entryId, to: actionNodeId, kind: "condition",
      ...(guardStr ? { guard: guardStr } : {}),
      sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
    });
  }

  return graph;
}
