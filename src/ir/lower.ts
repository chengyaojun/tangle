import type { Expr, Stmt } from "../ast.js";
import type { TangleDiagnostic, SourceSpan } from "../model.js";
import type { IRNode, IREdge, RuleGraph } from "./graph.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

export function lowerStatements(stmts: Stmt[], file: string): RuleGraph {
  resetNodeCounter();
  const graph = createGraph();
  const entryNode: IRNode = {
    kind: "compute", id: freshNodeId("entry"), label: "entry",
    sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
  };
  graph.nodes.push(entryNode);
  graph.entryNodeId = entryNode.id;

  let prevId = entryNode.id;
  for (const stmt of stmts) {
    const node = lowerStmt(stmt, file);
    graph.nodes.push(node);
    graph.edges.push({
      from: prevId, to: node.id, kind: "control",
      sourceSpan: node.sourceSpan
    });
    prevId = node.id;
  }

  // Terminal node
  const terminal: IRNode = {
    kind: "terminal", id: freshNodeId("end"), label: "return",
    sourceSpan: { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
  };
  graph.nodes.push(terminal);
  graph.edges.push({ from: prevId, to: terminal.id, kind: "control", sourceSpan: terminal.sourceSpan });

  return graph;
}

function lowerStmt(stmt: Stmt, file: string): IRNode {
  const span = (stmt as { span: SourceSpan }).span;
  switch (stmt.kind) {
    case "return":
      return { kind: "action", id: freshNodeId("ret"), label: "return", sourceSpan: span };
    case "let":
    case "const":
      return { kind: "compute", id: freshNodeId("bind"), label: `${stmt.kind} ${stmt.name}`, sourceSpan: span };
    case "expression":
      return { kind: "action", id: freshNodeId("expr"), label: "expression", sourceSpan: span };
    default:
      return { kind: "compute", id: freshNodeId("stmt"), label: (stmt as Stmt).kind, sourceSpan: span };
  }
}
