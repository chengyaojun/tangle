import type { RuleGraph, IRNode, IREdge, IREdgeKind, IRNodeKind } from "./graph.js";
import type { SourceSpan, TangleDiagnostic } from "../model.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

// IRNode in graph.ts lacks sourceText/group/style (present on Rust IRNode).
// Define an extended node type carrying the extra metadata. FlowIRNode is
// structurally assignable to IRNode, so it can be pushed onto graph.nodes.
export type FlowIRNode = IRNode & {
  sourceText: string | null;
  group: string | null;
  style: string | null;
};

// IREdge in graph.ts lacks priority/style (present on Rust IREdge), and
// IREdgeKind lacks Dashed/Thick/Crossed (present on Rust IREdgeKind).
// Define an extended edge type with the extra fields and a wider kind.
export type FlowIREdgeKind = IREdgeKind | "dashed" | "thick" | "crossed";

export type FlowIREdge = Omit<IREdge, "kind"> & {
  kind: FlowIREdgeKind;
  priority: number | null;
  style: string | null;
};

// RuleGraph in graph.ts types nodes as IRNode[] and edges as IREdge[].
// For ruleFlow we need the extended element types to carry group/style/priority
// and the wider edge kind. FlowRuleGraph keeps the other RuleGraph fields.
export type FlowRuleGraph = Omit<RuleGraph, "nodes" | "edges"> & {
  nodes: FlowIRNode[];
  edges: FlowIREdge[];
};

export interface FlowLowerResult {
  graph: FlowRuleGraph;
  diagnostics: TangleDiagnostic[];
}

/// Mermaid node shape, used to infer `IRNodeKind`. Mirror of Rust `NodeShape`.
type NodeShape = "rect" | "rounded" | "diamond" | "circle";

/// Lower a Mermaid `graph TD` source into a rule graph.
/// Faithful port of `compiler/tangle-cli/src/ir/lower_rule_flow.rs`.
///
/// Recognized syntax:
/// - Node declarations: `A[Label]` (rect), `A(Label)` (rounded),
///   `A{Label}` (diamond), `A((Label))` (circle)
/// - Edges: `-->` (control), `-.->` (dashed), `==>` (thick), `--x` (crossed)
/// - Guarded edges: `A -->|guard| B`
/// - Subgraphs: `subgraph Name ... end` → node.group
/// - Styling: `classDef`, `class`, `style`, `linkStyle`
export function lowerRuleFlow(mermaidSource: string, file: string): FlowLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const graph = createGraph() as unknown as FlowRuleGraph;

  // Synthetic span (line 0 marks synthetic). Rust uses source_span: None;
  // TS requires a SourceSpan, so we substitute a synthetic one.
  const syntheticSpan: SourceSpan = {
    file,
    startLine: 0,
    startColumn: 0,
    endLine: 0,
    endColumn: 0,
  };

  const nodeMap = new Map<string, string>(); // mermaid id → ir id
  const nodes: FlowIRNode[] = [];
  const edges: FlowIREdge[] = [];
  let entryId: string | null = null; // first declared node's ir id

  const subgraphStack: string[] = [];
  const edgeStyles = new Map<number, string>(); // edge index → style def
  const nodeStyles = new Map<string, string>(); // mermaid id → style def
  const classDefs = new Map<string, string>(); // class name → style def
  const classAssignments = new Map<string, string>(); // mermaid id → class name

  function registerNode(
    mermaidId: string,
    label: string,
    isError: boolean,
    group: string | null,
    shape: NodeShape,
  ): void {
    if (nodeMap.has(mermaidId)) return;
    const nodeId = freshNodeId("flow");
    const kind = shapeToKind(shape, isError);
    nodeMap.set(mermaidId, nodeId);
    if (entryId === null) {
      entryId = nodeId;
    }
    const node: FlowIRNode = {
      id: nodeId,
      kind,
      label,
      sourceSpan: syntheticSpan,
      sourceText: null,
      group,
      style: null,
    };
    nodes.push(node);
  }

  const rawLines = mermaidSource.split("\n");
  for (const rawLine of rawLines) {
    const line = rawLine.trim();

    // Skip empty lines, graph declarations, and fence markers.
    if (
      line === ""
      || line.startsWith("graph ")
      || line.startsWith("graph\t")
      || line.startsWith("```")
    ) {
      continue;
    }

    // subgraph start: push first whitespace-delimited token.
    if (line.startsWith("subgraph ")) {
      const rest = line.substring("subgraph ".length);
      const name = rest.trim().split(/\s+/)[0] ?? "";
      subgraphStack.push(name);
      continue;
    }

    // subgraph end
    if (line === "end") {
      subgraphStack.pop();
      continue;
    }

    // classDef <name> <style-def>
    if (line.startsWith("classDef ")) {
      const rest = line.substring("classDef ".length);
      const sp = rest.search(/\s/);
      if (sp !== -1) {
        const className = rest.substring(0, sp);
        const styleDef = rest.substring(sp).trim();
        classDefs.set(className, styleDef);
      }
      continue;
    }

    // class assignment: "class A,B className"
    if (line.startsWith("class ")) {
      const rest = line.substring("class ".length);
      const sp = rest.search(/\s/);
      if (sp !== -1) {
        const nodeIds = rest.substring(0, sp).split(",").map(s => s.trim());
        const className = rest.substring(sp).trim();
        for (const nid of nodeIds) {
          classAssignments.set(nid, className);
        }
      }
      continue;
    }

    // style <nodeId> <style-def>
    if (line.startsWith("style ")) {
      const rest = line.substring("style ".length);
      const sp = rest.search(/\s/);
      if (sp !== -1) {
        const nodeId = rest.substring(0, sp);
        const styleDef = rest.substring(sp).trim();
        nodeStyles.set(nodeId, styleDef);
      }
      continue;
    }

    // linkStyle <idx> <style-def>
    if (line.startsWith("linkStyle ")) {
      const rest = line.substring("linkStyle ".length);
      const sp = rest.search(/\s/);
      if (sp !== -1) {
        const idxStr = rest.substring(0, sp);
        // Mirror Rust parse::<usize>(): only accept digit strings.
        if (/^\d+$/.test(idxStr)) {
          const idx = parseInt(idxStr, 10);
          const styleDef = rest.substring(sp).trim();
          edgeStyles.set(idx, styleDef);
        }
      }
      continue;
    }

    // Current group = top of subgraph stack (innermost).
    const currentGroup = subgraphStack.length > 0
      ? subgraphStack[subgraphStack.length - 1]!
      : null;

    // Try standalone node declaration: A[Label] / A(Label) / A{Label} / A((Label))
    const decl = parseNodeDecl(line);
    if (decl !== null) {
      registerNode(decl.mermaidId, decl.label, decl.isError, currentGroup, decl.shape);
      continue;
    }

    // Try edge: may contain inline node declarations.
    const edgeParts = parseEdgePartsV2(line);
    if (edgeParts !== null) {
      const { fromPart, guard, toPart, edgeKind } = edgeParts;

      // Extract and register inline nodes from declarations.
      const fromInline = extractInlineNode(fromPart);
      if (fromInline !== null) {
        // from_part nodes are never error terminals (mirror of Rust).
        registerNode(fromInline.id, fromInline.label, false, currentGroup, fromInline.shape);
      }
      const toInline = extractInlineNode(toPart);
      if (toInline !== null) {
        const isError = toInline.label.toLowerCase().startsWith("error:")
          || toInline.label.startsWith("错误:");
        registerNode(toInline.id, toInline.label, isError, currentGroup, toInline.shape);
      }

      // Resolve edge endpoints (strip labels if present).
      const fromMermaidId = extractNodeId(fromPart);
      const toMermaidId = extractNodeId(toPart);
      const fromIrId = nodeMap.get(fromMermaidId);
      const toIrId = nodeMap.get(toMermaidId);

      if (fromIrId !== undefined && toIrId !== undefined) {
        // A guard present always makes the edge a Condition; otherwise
        // propagate the parsed arrow kind (control/dashed/thick/crossed).
        const kind: FlowIREdgeKind = guard !== null ? "condition" : edgeKind;
        // With exactOptionalPropertyTypes, guard must be omitted (not set to
        // undefined) when absent. Conditional spread mirrors ruleTable pattern.
        const edge: FlowIREdge = {
          from: fromIrId,
          to: toIrId,
          kind,
          ...(guard !== null ? { guard } : {}),
          sourceSpan: syntheticSpan,
          priority: null,
          style: null,
        };
        edges.push(edge);
      }
    }
  }

  // Apply node styles (by mermaid id).
  for (const [mermaidId, style] of nodeStyles) {
    const irId = nodeMap.get(mermaidId);
    if (irId !== undefined) {
      const node = nodes.find(n => n.id === irId);
      if (node !== undefined) {
        node.style = style;
      }
    }
  }
  // Apply class assignments — use class_defs resolved style text so that
  // node.style carries the parsed style (e.g. "fill:#ff0,stroke:#f00") rather
  // than the raw class name. Falls back to the class name if undefined.
  for (const [mermaidId, className] of classAssignments) {
    const irId = nodeMap.get(mermaidId);
    if (irId !== undefined) {
      const node = nodes.find(n => n.id === irId);
      if (node !== undefined) {
        const style = classDefs.get(className) ?? className;
        node.style = style;
      }
    }
  }
  // Apply edge styles (by index).
  for (const [idx, style] of edgeStyles) {
    if (idx < edges.length) {
      const edge = edges[idx];
      if (edge !== undefined) {
        edge.style = style;
      }
    }
  }

  // Entry detection (mirror of Rust):
  // 1. First node with no incoming edges.
  // 2. Fallback to entry_id (first declared node).
  // 3. Fallback: create an "empty" Terminal node.
  const hasIncoming = new Set<string>();
  for (const e of edges) {
    hasIncoming.add(e.to);
  }

  let entryNodeId: string;
  const noIncomingNode = nodes.find(n => !hasIncoming.has(n.id));
  if (noIncomingNode !== undefined) {
    entryNodeId = noIncomingNode.id;
  } else if (entryId !== null) {
    entryNodeId = entryId;
  } else {
    const id = freshNodeId("flow");
    const emptyNode: FlowIRNode = {
      id,
      kind: "terminal",
      label: "empty",
      sourceSpan: syntheticSpan,
      sourceText: null,
      group: null,
      style: null,
    };
    nodes.push(emptyNode);
    entryNodeId = id;
  }

  graph.nodes = nodes;
  graph.edges = edges;
  graph.entryNodeId = entryNodeId;

  return { graph, diagnostics };
}

/// Mirror of Rust `shape_to_kind`. Diamond → decision, Circle → terminal,
/// Rect/Rounded → action. Error labels override to error-terminal.
function shapeToKind(shape: NodeShape, isError: boolean): IRNodeKind {
  if (isError) return "error-terminal";
  switch (shape) {
    case "diamond":
      return "decision";
    case "circle":
      return "terminal";
    default:
      return "action"; // rect, rounded
  }
}

/// Find the first char that is not ASCII alphanumeric and not underscore.
/// Returns -1 if no such char exists (entire string is an identifier).
/// Mirror of Rust `find(|c: char| !c.is_ascii_alphanumeric() && c != '_')`.
function findIdEnd(s: string): number {
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    const isDigit = c >= 48 && c <= 57; // 0-9
    const isUpper = c >= 65 && c <= 90; // A-Z
    const isLower = c >= 97 && c <= 122; // a-z
    const isUnderscore = c === 95; // _
    if (!isDigit && !isUpper && !isLower && !isUnderscore) {
      return i;
    }
  }
  return -1;
}

/// Parse standalone node declaration: A[Label] / A(Label) / A{Label} / A((Label)).
/// Returns null if the line is an edge line (contains an edge operator) or
/// doesn't match any shape. Mirror of Rust `parse_node_decl`.
function parseNodeDecl(
  line: string,
): { mermaidId: string; label: string; isError: boolean; shape: NodeShape } | null {
  const trimmed = line.trim();
  const idEnd = findIdEnd(trimmed);
  if (idEnd === -1) return null;
  const mermaidId = trimmed.substring(0, idEnd);
  const rest = trimmed.substring(idEnd).trimStart();

  // If any edge operator is present, this is an edge line, not a standalone node.
  if (
    trimmed.includes("-->")
    || trimmed.includes("-.->")
    || trimmed.includes("==>")
    || trimmed.includes("--x")
  ) {
    return null;
  }

  // Circle: ((Label))
  if (rest.startsWith("((") && rest.endsWith("))")) {
    const label = rest.substring(2, rest.length - 2).trim();
    const isError = label.toLowerCase().startsWith("error:") || label.startsWith("错误:");
    return { mermaidId, label, isError, shape: "circle" };
  }
  // Rect: [Label]
  if (rest.startsWith("[") && rest.endsWith("]")) {
    const label = rest.substring(1, rest.length - 1).trim();
    const isError = label.toLowerCase().startsWith("error:") || label.startsWith("错误:");
    return { mermaidId, label, isError, shape: "rect" };
  }
  // Rounded: (Label)
  if (rest.startsWith("(") && rest.endsWith(")")) {
    const label = rest.substring(1, rest.length - 1).trim();
    const isError = label.toLowerCase().startsWith("error:") || label.startsWith("错误:");
    return { mermaidId, label, isError, shape: "rounded" };
  }
  // Diamond: {Label}
  if (rest.startsWith("{") && rest.endsWith("}")) {
    const label = rest.substring(1, rest.length - 1).trim();
    const isError = label.toLowerCase().startsWith("error:") || label.startsWith("错误:");
    return { mermaidId, label, isError, shape: "diamond" };
  }

  return null;
}

/// Extract node ID from a part like "A" or "A[Label]" → "A".
/// Mirror of Rust `extract_node_id`.
function extractNodeId(part: string): string {
  const trimmed = part.trim();
  const pos = findIdEnd(trimmed);
  if (pos === -1) return trimmed;
  return trimmed.substring(0, pos);
}

/// Extract inline node: "A[Label]" → (A, Label, rect), "A" → null (bare ID).
/// The shape is determined by the delimiter immediately following the ID, so a
/// label containing `{` or `((` substrings does not skew classification.
/// Mirror of Rust `extract_inline_node`.
function extractInlineNode(
  part: string,
): { id: string; label: string; shape: NodeShape } | null {
  const trimmed = part.trim();
  const idEnd = findIdEnd(trimmed);
  if (idEnd === -1) return null;
  const id = trimmed.substring(0, idEnd);
  const rest = trimmed.substring(idEnd).trimStart();

  let open: string;
  let close: string;
  let shape: NodeShape;

  if (rest.startsWith("((")) {
    open = "((";
    close = "))";
    shape = "circle";
  } else if (rest.startsWith("[")) {
    open = "[";
    close = "]";
    shape = "rect";
  } else if (rest.startsWith("(")) {
    open = "(";
    close = ")";
    shape = "rounded";
  } else if (rest.startsWith("{")) {
    open = "{";
    close = "}";
    shape = "diamond";
  } else {
    return null;
  }

  const closePos = rest.indexOf(close);
  if (closePos === -1) return null;
  const innerStart = open.length;
  const label = rest.substring(innerStart, closePos).trim();
  return { id, label, shape };
}

/// Parse edge with multi-edge-type support.
/// Recognizes four arrow operators and returns the matching edge kind:
///   `-.->` → dashed, `==>` → thick, `--x` → crossed, `-->` → control.
/// Returns `{ fromPart, guard, toPart, edgeKind }` or null if no operator.
///
/// Order matters: `-.->` must be checked before `-->` because `indexOf("-->")`
/// would otherwise match the trailing `-->` substring inside `-.->`.
/// Mirror of Rust `parse_edge_parts_v2`.
function parseEdgePartsV2(
  line: string,
): { fromPart: string; guard: string | null; toPart: string; edgeKind: FlowIREdgeKind } | null {
  const trimmed = line.trim();

  // Match by priority: longer / ambiguous operators first.
  let arrowPos: number;
  let arrowLen: number;
  let edgeKind: FlowIREdgeKind;

  const dashedPos = trimmed.indexOf("-.->");
  const thickPos = trimmed.indexOf("==>");
  const crossedPos = trimmed.indexOf("--x");
  const solidPos = trimmed.indexOf("-->");

  if (dashedPos !== -1) {
    arrowPos = dashedPos;
    arrowLen = 4;
    edgeKind = "dashed";
  } else if (thickPos !== -1) {
    arrowPos = thickPos;
    arrowLen = 3;
    edgeKind = "thick";
  } else if (crossedPos !== -1) {
    arrowPos = crossedPos;
    arrowLen = 3;
    edgeKind = "crossed";
  } else if (solidPos !== -1) {
    arrowPos = solidPos;
    arrowLen = 3;
    edgeKind = "control";
  } else {
    return null;
  }

  const fromPart = trimmed.substring(0, arrowPos).trim();
  const afterArrow = trimmed.substring(arrowPos + arrowLen).trim();

  // Look for |guard|.
  const pipeStart = afterArrow.indexOf("|");
  if (pipeStart !== -1) {
    const pipeEnd = afterArrow.indexOf("|", pipeStart + 1);
    if (pipeEnd !== -1) {
      const guard = afterArrow.substring(pipeStart + 1, pipeEnd).trim();
      const toPart = afterArrow.substring(pipeEnd + 1).trim();
      return { fromPart, guard, toPart, edgeKind };
    }
  }
  const toPart = afterArrow.trim();
  return { fromPart, guard: null, toPart, edgeKind };
}
