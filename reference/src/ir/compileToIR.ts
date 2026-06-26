import type { CheckedModule } from "../checker/checkModule.js";
import type { RuleGraph } from "./graph.js";
import type { TangleDiagnostic } from "../model.js";
import { createGraph } from "./graph.js";
import { lowerStatements } from "./lower.js";
import { validateIR } from "./validate.js";

export function compileToIR(checked: CheckedModule): { graph: RuleGraph; diagnostics: TangleDiagnostic[] } {
  const allDiagnostics: TangleDiagnostic[] = [...checked.diagnostics];
  let graph = createGraph("");

  // Lower all parsed code blocks to IR and merge
  for (const parsed of checked.parsedBlocks) {
    const subGraph = lowerStatements(parsed.body.statements, checked.file);
    // Merge subGraph into main graph
    for (const node of subGraph.nodes) {
      if (!graph.nodes.find(n => n.id === node.id)) {
        graph.nodes.push(node);
      }
    }
    for (const edge of subGraph.edges) {
      graph.edges.push(edge);
    }
    if (subGraph.entryNodeId && !graph.entryNodeId) {
      graph.entryNodeId = subGraph.entryNodeId;
    }
  }

  // If no code blocks, create minimal graph
  if (graph.nodes.length === 0 && graph.entryNodeId === "") {
    graph.entryNodeId = "entry";
  }

  // Validate
  const irDiags = validateIR(graph);
  allDiagnostics.push(...irDiags);

  return { graph, diagnostics: allDiagnostics };
}
