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
export declare function createGraph(entryNodeId?: string): RuleGraph;
export declare function freshNodeId(prefix?: string): string;
export declare function resetNodeCounter(): void;
