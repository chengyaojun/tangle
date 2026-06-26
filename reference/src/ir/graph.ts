import type { SourceSpan } from "../model.js";

export type IRNodeKind = "action" | "compute" | "decision" | "terminal" | "error-terminal";

export type IRNode = {
  kind: IRNodeKind;
  id: string;
  label: string;
  sourceSpan: SourceSpan;
};

export type IREdgeKind = "control" | "condition" | "error";

export type IREdge = {
  from: string;
  to: string;
  kind: IREdgeKind;
  guard?: string;
  sourceSpan: SourceSpan;
};

export type IRErrorEdge = {
  from: string;
  errorVariant: string;
  sourceSpan: SourceSpan;
};

export type RuleGraph = {
  nodes: IRNode[];
  edges: IREdge[];
  errorEdges: IRErrorEdge[];
  entryNodeId: string;
};

// Helper to create an empty graph
export function createGraph(entryNodeId?: string): RuleGraph {
  return { nodes: [], edges: [], errorEdges: [], entryNodeId: entryNodeId ?? "entry" };
}

// Helper to generate unique node IDs
let nodeCounter = 0;
export function freshNodeId(prefix?: string): string {
  return `${prefix ?? "n"}${++nodeCounter}`;
}
export function resetNodeCounter(): void { nodeCounter = 0; }
