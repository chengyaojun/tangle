import type { RuleGraph, IRNode, IREdge } from "./graph.js";
import type { SourceSpan, TangleDiagnostic } from "../model.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

// IRNode in graph.ts lacks sourceText/group/style (present on Rust IRNode).
// Define an extended node type carrying the extra metadata. TableIRNode is
// structurally assignable to IRNode, so it can be pushed onto graph.nodes.
export type TableIRNode = IRNode & {
  sourceText: string | null;
  group: string | null;
  style: string | null;
};

// IREdge in graph.ts lacks priority/style (present on Rust IREdge).
export type TableIREdge = IREdge & {
  priority: number | null;
  style: string | null;
};

export interface TableLowerResult {
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
}

/// Detect if two rows overlap (both can match the same input). `-` = wildcard.
/// Faithful port of Rust `rows_overlap`.
export function rowsOverlap(rowA: string[], rowB: string[]): boolean {
  // Rust uses zip, which stops at the shorter. We mirror that.
  const len = Math.min(rowA.length, rowB.length);
  for (let i = 0; i < len; i++) {
    const a = rowA[i]!;
    const b = rowB[i]!;
    if (!(a === "-" || b === "-" || a === b)) {
      return false;
    }
  }
  return true;
}

/// Lower a markdown decision table into a rule graph.
/// Faithful port of `compiler/tangle-cli/src/ir/lower_rule_table.rs`.
export function lowerRuleTable(tableMarkdown: string, file: string): TableLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const entryId = freshNodeId("table-entry");
  const graph = createGraph(entryId);

  // Entry node: Decision kind, label "table.entry", synthetic span (line 0).
  const syntheticSpan: SourceSpan = {
    file,
    startLine: 0,
    startColumn: 0,
    endLine: 0,
    endColumn: 0,
  };
  const entryNode: TableIRNode = {
    id: entryId,
    kind: "decision",
    label: "table.entry",
    sourceSpan: syntheticSpan,
    sourceText: null,
    group: null,
    style: null,
  };
  graph.nodes.push(entryNode);

  // Filter lines: must contain '|' AND not be separator-only (only |, -, :, space).
  // 1-based line numbers (mirror of Rust line_no + 1).
  const lines: Array<{ lineNo: number; text: string }> = [];
  const rawLines = tableMarkdown.split("\n");
  for (let i = 0; i < rawLines.length; i++) {
    const line = rawLines[i]!;
    if (!line.includes("|")) continue;
    const trimmed = line.trim();
    if (trimmed.length === 0) continue;
    let allSeparator = true;
    for (const c of trimmed) {
      if (c !== "|" && c !== "-" && c !== ":" && c !== " ") {
        allSeparator = false;
        break;
      }
    }
    if (allSeparator) continue;
    lines.push({ lineNo: i + 1, text: line });
  }

  if (lines.length < 2) {
    return { graph, diagnostics };
  }

  // Parse header
  const headers = splitTableRow(lines[0]!.text);
  if (headers.length === 0) {
    return { graph, diagnostics };
  }

  const conditionCount = Math.max(0, headers.length - 1);

  // Parse data rows: (line_no, conds) and parallel actions.
  const parsedRows: Array<{ lineNo: number; conds: string[] }> = [];
  const parsedActions: string[] = [];
  for (let i = 1; i < lines.length; i++) {
    const { lineNo, text } = lines[i]!;
    const cells = splitTableRow(text);
    if (cells.length < 2) continue;

    const take = Math.min(conditionCount, Math.max(0, cells.length - 1));
    const conds: string[] = cells.slice(0, take).map(c => c.trim());
    while (conds.length < conditionCount) {
      conds.push("-");
    }
    parsedRows.push({ lineNo, conds });
    parsedActions.push(cells[cells.length - 1]!);
  }

  // Overlap detection: for each pair (i, j) with i < j.
  for (let i = 0; i < parsedRows.length; i++) {
    for (let j = i + 1; j < parsedRows.length; j++) {
      const rowI = parsedRows[i]!;
      const rowJ = parsedRows[j]!;
      if (!rowsOverlap(rowI.conds, rowJ.conds)) continue;

      if (condsEqual(rowI.conds, rowJ.conds)) {
        diagnostics.push({
          code: "TANGLE_RULE_DUPLICATE",
          message: `rows ${i + 1} and ${j + 1} are identical`,
          span: {
            file,
            startLine: rowJ.lineNo,
            startColumn: 0,
            endLine: rowJ.lineNo,
            endColumn: 0,
          },
        });
      } else {
        // Check if row i covers row j (j unreachable). Mirror of Rust:
        // i_covers_j = rowI.conds.iter().zip(rowJ.conds.iter())
        //   .all(|(a, b)| a == "-" || a == b)
        const iCoversJ = iCovers(rowI.conds, rowJ.conds);
        if (iCoversJ) {
          diagnostics.push({
            code: "TANGLE_RULE_UNREACHABLE",
            message: `row ${j + 1} is unreachable (covered by row ${i + 1})`,
            span: {
              file,
              startLine: rowJ.lineNo,
              startColumn: 0,
              endLine: rowJ.lineNo,
              endColumn: 0,
            },
          });
        } else {
          diagnostics.push({
            code: "TANGLE_RULE_OVERLAP",
            message: `rows ${i + 1} and ${j + 1} overlap; row ${i + 1} wins by priority`,
            span: {
              file,
              startLine: rowJ.lineNo,
              startColumn: 0,
              endLine: rowJ.lineNo,
              endColumn: 0,
            },
          });
        }
      }
    }
  }

  // Generate IR nodes and edges. Priority = row index (row order wins).
  for (let rowIdx = 0; rowIdx < parsedRows.length; rowIdx++) {
    const { conds } = parsedRows[rowIdx]!;
    const action = parsedActions[rowIdx]!;

    const conditions: string[] = [];
    for (let i = 0; i < conds.length; i++) {
      const condVal = conds[i]!;
      if (condVal !== "" && condVal !== "-") {
        const colName = headers[i]?.trim() ?? "?";
        conditions.push(`${colName} = ${condVal}`);
      }
    }

    const nodeId = freshNodeId("table-action");
    const actionNode: TableIRNode = {
      id: nodeId,
      kind: "action",
      label: action,
      sourceSpan: syntheticSpan,
      sourceText: null,
      group: null,
      style: null,
    };
    graph.nodes.push(actionNode);

    if (conditions.length === 0) {
      const edge: TableIREdge = {
        from: entryId,
        to: nodeId,
        kind: "condition",
        sourceSpan: syntheticSpan,
        priority: rowIdx,
        style: null,
      };
      graph.edges.push(edge);
    } else {
      const edge: TableIREdge = {
        from: entryId,
        to: nodeId,
        kind: "condition",
        guard: conditions.join(" AND "),
        sourceSpan: syntheticSpan,
        priority: rowIdx,
        style: null,
      };
      graph.edges.push(edge);
    }
  }

  return { graph, diagnostics };
}

function splitTableRow(line: string): string[] {
  return line
    .split("|")
    .map(c => c.trim())
    .filter(c => c !== "");
}

function condsEqual(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

/// Mirror of Rust `i_covers_j`: rowI covers rowJ if for every column,
/// rowI's value is "-" (wildcard) OR equals rowJ's value. Uses zip semantics
/// (stops at shorter length, .all() on empty → true).
function iCovers(rowI: string[], rowJ: string[]): boolean {
  const len = Math.min(rowI.length, rowJ.length);
  for (let k = 0; k < len; k++) {
    const a = rowI[k]!;
    const b = rowJ[k]!;
    if (a !== "-" && a !== b) {
      return false;
    }
  }
  return true;
}
