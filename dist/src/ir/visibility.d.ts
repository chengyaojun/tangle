import type { RuleGraph } from "./graph.js";
import type { TangleDiagnostic } from "../model.js";
export declare function checkIRVisibility(graph: RuleGraph, exportedSymbols: Set<string>): TangleDiagnostic[];
