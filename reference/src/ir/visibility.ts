import type { RuleGraph } from "./graph.js";
import type { TangleDiagnostic } from "../model.js";

// Check that all symbols referenced in the IR are visible (exported or module-internal)
export function checkIRVisibility(graph: RuleGraph, exportedSymbols: Set<string>): TangleDiagnostic[] {
  const diags: TangleDiagnostic[] = [];
  // For now, all nodes pass visibility check since cross-module references aren't yet tracked
  // Future: check that each IRNode's label references only visible symbols
  return diags;
}
