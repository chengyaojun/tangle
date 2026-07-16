import type { RuleGraph, IRNode, IREdge } from "./graph.js";
import type { SourceSpan, TangleDiagnostic } from "../model.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

// IRNode in graph.ts lacks sourceText/group/style (present on Rust IRNode).
// Define an extended node type carrying the extra metadata. TreeIRNode is
// structurally assignable to IRNode, so it can be pushed onto graph.nodes.
export type TreeIRNode = IRNode & {
  sourceText: string | null;
  group: string | null;
  style: string | null;
};

export interface TreeLowerResult {
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
}

/// 缩进感知的列表树节点（mirror of Rust ListNode）
interface ListNode {
  text: string;
  depth: number;
  line: number;
  children: ListNode[];
}

/// Lower a nested markdown list into a rule graph encoding DNF semantics:
/// - Multiple root branches → OR (disjunction)
/// - Chained conditions within a branch → AND (conjunction)
/// - `Action:` markers → terminal Action nodes
///
/// Faithful port of `compiler/tangle-cli/src/ir/lower_rule_tree.rs`.
export function lowerRuleTree(listMarkdown: string, file: string): TreeLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const entryId = freshNodeId("tree-entry");
  const graph = createGraph(entryId);

  // Entry node: Decision kind, label "tree.entry", synthetic span (line 0 = synthetic).
  const syntheticSpan: SourceSpan = {
    file,
    startLine: 0,
    startColumn: 0,
    endLine: 0,
    endColumn: 0,
  };
  const entryNode: TreeIRNode = {
    id: entryId,
    kind: "decision",
    label: "tree.entry",
    sourceSpan: syntheticSpan,
    sourceText: null,
    group: null,
    style: null,
  };
  graph.nodes.push(entryNode);

  const roots = parseListToTree(listMarkdown);

  for (const branch of roots) {
    if (branch.children.length === 0) {
      diagnostics.push({
        code: "TANGLE_RULE_EMPTY_BRANCH",
        message: `branch '${branch.text}' has no conditions or action`,
        span: { file, startLine: branch.line, startColumn: 0, endLine: branch.line, endColumn: 0 },
      });
      continue;
    }

    const hasAction = branch.children.some(c => c.text.startsWith("Action:"));
    if (!hasAction) {
      diagnostics.push({
        code: "TANGLE_RULE_NO_ACTION",
        message: `branch '${branch.text}' has no Action: marker`,
        span: { file, startLine: branch.line, startColumn: 0, endLine: branch.line, endColumn: 0 },
      });
    }

    const conditions: ListNode[] = branch.children.filter(c => !c.text.startsWith("Action:"));

    // Chain conditions in sequence (AND semantics): entry → cond1 → cond2 → ...
    let prevId: string | null = null;
    if (conditions.length > 0) {
      const cond = conditions[0]!;
      const nodeId = freshNodeId("tree");
      const condNode: TreeIRNode = {
        id: nodeId,
        kind: "decision",
        label: cond.text,
        sourceSpan: syntheticSpan,
        sourceText: null,
        group: null,
        style: null,
      };
      graph.nodes.push(condNode);
      const edge: IREdge = {
        from: entryId,
        to: nodeId,
        kind: "condition",
        guard: cond.text,
        sourceSpan: syntheticSpan,
      };
      graph.edges.push(edge);
      prevId = nodeId;
    }

    for (let i = 1; i < conditions.length; i++) {
      const cond = conditions[i]!;
      const nodeId = freshNodeId("tree");
      const condNode: TreeIRNode = {
        id: nodeId,
        kind: "decision",
        label: cond.text,
        sourceSpan: syntheticSpan,
        sourceText: null,
        group: null,
        style: null,
      };
      graph.nodes.push(condNode);
      const edge: IREdge = {
        from: prevId!,
        to: nodeId,
        kind: "condition",
        guard: cond.text,
        sourceSpan: syntheticSpan,
      };
      graph.edges.push(edge);
      prevId = nodeId;
    }

    // Multiple Action: markers in a branch create parallel action nodes
    // (all connected from the same prevId). This is intentional for
    // multi-action semantics.
    for (const child of branch.children) {
      if (child.text.startsWith("Action:")) {
        const actionLabel = child.text.substring("Action:".length).trim();
        const actionId = freshNodeId("tree");
        const actionNode: TreeIRNode = {
          id: actionId,
          kind: "action",
          label: actionLabel,
          sourceSpan: syntheticSpan,
          sourceText: null,
          group: null,
          style: null,
        };
        graph.nodes.push(actionNode);
        const from = prevId ?? entryId;
        const edge: IREdge = {
          from,
          to: actionId,
          kind: "control",
          sourceSpan: syntheticSpan,
        };
        graph.edges.push(edge);
      }
    }
  }

  return { graph, diagnostics };
}

/// 解析嵌套列表为缩进感知的树结构。
/// 每 4 空格或 1 tab = 1 级深度。
/// Faithful port of Rust `parse_list_to_tree`.
export function parseListToTree(markdown: string): ListNode[] {
  const items: Array<{ depth: number; text: string; line: number }> = [];
  const lines = markdown.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    const trimmed = line.trimStart();
    if (!trimmed.startsWith("* ") && !trimmed.startsWith("- ")) {
      continue;
    }
    const leading = line.substring(0, line.length - trimmed.length);
    const depth = computeDepthFromStr(leading);
    const text = trimmed
      .replace(/^\* /, "")
      .replace(/^- /, "")
      .trim();
    items.push({ depth, text, line: i + 1 }); // 1-based
  }
  const idxRef: { idx: number } = { idx: 0 };
  return buildTree(items, 0, idxRef);
}

/// 计算前导空白对应的深度：4 空格或 1 tab = 1 级。
/// Faithful port of Rust `compute_depth_from_str`.
function computeDepthFromStr(leading: string): number {
  let depth = 0;
  let spaces = 0;
  for (const c of leading) {
    if (c === "\t") {
      depth++;
      spaces = 0;
    } else if (c === " ") {
      spaces++;
      if (spaces === 4) {
        depth++;
        spaces = 0;
      }
    } else {
      break;
    }
  }
  return depth;
}

/// 递归构建 ListNode 树（按缩进深度）。
/// Faithful port of Rust `build_tree`. Uses a mutable index reference to
/// walk the flat items array in a single pass.
function buildTree(
  items: Array<{ depth: number; text: string; line: number }>,
  targetDepth: number,
  idxRef: { idx: number },
): ListNode[] {
  const nodes: ListNode[] = [];
  while (idxRef.idx < items.length) {
    const item = items[idxRef.idx]!;
    if (item.depth < targetDepth) {
      break;
    }
    if (item.depth === targetDepth) {
      idxRef.idx++;
      const children = buildTree(items, targetDepth + 1, idxRef);
      nodes.push({
        text: item.text,
        depth: targetDepth,
        line: item.line,
        children,
      });
    } else {
      // depth > targetDepth：不应发生（由上层 buildTree 处理），跳过。
      idxRef.idx++;
    }
  }
  return nodes;
}
