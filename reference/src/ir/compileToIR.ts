import type { CheckedModule } from "../checker/checkModule.js";
import type { RuleGraph, IRNode, IREdge, IRErrorEdge, IRNodeKind } from "./graph.js";
import type { TangleDiagnostic, TangleHeading, SourceSpan } from "../model.js";
import type { Stmt, ParsedCodeBlock } from "../ast.js";
import type { RuleData } from "../front-end/compileModule.js";
import type { Type } from "../checker/types.js";
import { createGraph, freshNodeId } from "./graph.js";
import { lowerStatements } from "./lower.js";
import { validateIR } from "./validate.js";
import { lowerRuleTree } from "./ruleTree.js";
import { lowerRuleTable } from "./ruleTable.js";
import { lowerRuleFlow } from "./ruleFlow.js";
import { lowerRuleToggle } from "./ruleToggle.js";
import { typeNameToType } from "../checker/resolve.js";

// IRParam: mirror of Rust IRParam. `type` is omitted from JSON when undefined
// (matching Rust's #[serde(skip_serializing_if = "Option::is_none")]).
// `type?: Type | undefined` is needed because tsconfig enables
// `exactOptionalPropertyTypes: true`, which forbids assigning `undefined`
// to a purely optional field.
type IRParam = {
  name: string;
  type?: Type | undefined;
};

// IRFunction: mirror of Rust IRFunction (camelCase JSON serialization).
// Carries heading-defined function IR (multi-function mode).
type IRFunction = {
  name: string;
  receiver: string | null;
  params: IRParam[];
  returnType?: Type | undefined;
  nodes: IRNode[];
  edges: IREdge[];
  entryNodeId: string;
  errorEdges: IRErrorEdge[];
};

// CompiledGraph extends RuleGraph with optional functions[] (multi-function mode).
// The functions field is serialized to JSON for the CLI --emit-ir output and
// consumed by ir-diff for comparison with Rust IR.
type CompiledGraph = RuleGraph & {
  functions?: IRFunction[];
};

/// Compute the root headings of the heading tree.
/// `module.headings` is a flat list with `children` populated by
/// `buildHeadingTree` (compileModule.ts). To avoid double-processing every
/// heading (once in the top-level iteration, once via recursion through
/// `children`), we compute roots = headings that are not a child of any other
/// heading, then recurse only from roots.
function findRoots(headings: TangleHeading[]): TangleHeading[] {
  const childIds = new Set<string>();
  for (const h of headings) {
    for (const child of h.children) {
      childIds.add(child.id);
    }
  }
  return headings.filter(h => !childIds.has(h.id));
}

export function compileToIR(checked: CheckedModule): { graph: CompiledGraph; diagnostics: TangleDiagnostic[] } {
  const allDiagnostics: TangleDiagnostic[] = [...checked.diagnostics];
  let graph: CompiledGraph = createGraph("");

  // `checked.headings` is the flat list; compute roots for tree-walking
  // helpers so we don't process headings twice (once in flat iteration, once
  // via `children` recursion).
  const roots = findRoots(checked.headings);

  // Multi-function mode: a `main` Callable heading turns the module into a
  // collection of functions. @tangle blocks then live inside `functions[]`
  // only and must NOT also be merged into the top-level graph (dual-entry fix
  // A1-1). Without `main`, the fallback single-function mode merges blocks at
  // the top level. Mirror of Rust compile_to_ir.rs:23-38.
  const hasMain = hasMainCallable(roots);

  // Lower @tangle code blocks as statements (fallback mode only).
  if (!hasMain) {
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
  }

  // Lower rule blocks from headings (mirror of Rust compile_to_ir.rs:40-52).
  const ruleGraphs: RuleGraph[] = [];
  const ruleDiags: TangleDiagnostic[] = [];
  collectRuleGraphs(roots, checked.file, ruleGraphs, ruleDiags);
  allDiagnostics.push(...ruleDiags);

  for (const subGraph of ruleGraphs) {
    for (const node of subGraph.nodes) {
      graph.nodes.push(node);
    }
    for (const edge of subGraph.edges) {
      graph.edges.push(edge);
    }
    for (const errEdge of subGraph.errorEdges) {
      graph.errorEdges.push(errEdge);
    }
    if (subGraph.entryNodeId && !graph.entryNodeId) {
      graph.entryNodeId = subGraph.entryNodeId;
    }
  }

  // If no code blocks, create minimal graph
  if (graph.nodes.length === 0 && graph.entryNodeId === "") {
    graph.entryNodeId = "entry";
  }

  // Build heading-defined functions (multi-function mode only). Mirror of
  // Rust compile_to_ir.rs:77-81.
  if (hasMain) {
    const functions: IRFunction[] = [];
    collectFunctions(roots, null, checked.parsedBlocks, checked.returnTypes, functions);
    graph.functions = functions;
  }

  // Validate
  const irDiags = validateIR(graph);
  allDiagnostics.push(...irDiags);

  return { graph, diagnostics: allDiagnostics };
}

/// Recursively collect rule subgraphs from headings. Mirror of Rust
/// `collect_rule_graphs` in compile_to_ir.rs:101-123. Reads the `rule` field
/// attached by compileModule.ts (TangleHeading has no `rule` field in
/// model.ts, so we access it via type assertion).
function collectRuleGraphs(
  headings: TangleHeading[],
  file: string,
  out: RuleGraph[],
  diagnostics: TangleDiagnostic[],
): void {
  for (const h of headings) {
    const rule = (h as TangleHeading & { rule?: RuleData }).rule;
    if (rule) {
      const result = lowerRuleByKind(rule, file);
      out.push(result.graph);
      diagnostics.push(...result.diagnostics);
    }
    collectRuleGraphs(h.children, file, out, diagnostics);
  }
}

/// Dispatch to the correct rule lower function based on kind. Returns the
/// sub-graph and diagnostics. Mirror of Rust match in collect_rule_graphs.
function lowerRuleByKind(
  rule: RuleData,
  file: string,
): { graph: RuleGraph; diagnostics: TangleDiagnostic[] } {
  switch (rule.kind) {
    case "flow": {
      const result = lowerRuleFlow(rule.source, file);
      // FlowRuleGraph has wider edge kind (dashed/thick/crossed) and extra
      // fields (priority/style). Cast to RuleGraph — ir-diff normalizes
      // extra fields away during comparison.
      return { graph: result.graph as unknown as RuleGraph, diagnostics: result.diagnostics };
    }
    case "table": {
      const result = lowerRuleTable(rule.source, file);
      return { graph: result.graph, diagnostics: result.diagnostics };
    }
    case "tree": {
      const result = lowerRuleTree(rule.source, file);
      return { graph: result.graph, diagnostics: result.diagnostics };
    }
    case "toggle": {
      const result = lowerRuleToggle(rule.source, file);
      return { graph: result.graph, diagnostics: result.diagnostics };
    }
  }
}

/// Check whether the module has a `main` Callable heading that owns `@tangle`
/// code blocks. Mirror of Rust `has_main_callable` in compile_to_ir.rs:227-240.
/// Note: Rust's `parse_heading_text` always sets `symbol_name` (even for plain
/// `#### main`), but TS's version only sets `symbolName` for the `(symbolName)`
/// pattern. We fall back to `title` to match Rust's behavior (same fallback
/// used by `buildSymbols` in compileModule.ts).
function hasMainCallable(headings: TangleHeading[]): boolean {
  for (const h of headings) {
    const name = h.symbolName ?? h.title;
    if (
      h.role === "callable"
      && (h.codeBlocks ?? []).length > 0
      && name === "main"
    ) {
      return true;
    }
    if (hasMainCallable(h.children)) {
      return true;
    }
  }
  return false;
}

/// Walk the heading tree and build an IRFunction for each Callable heading
/// that has `@tangle` code blocks. Mirror of Rust `collect_functions` in
/// compile_to_ir.rs:129-164. `parent` determines the receiver: a Callable
/// under a Type heading becomes a method `Type.method`; `main` and free
/// callables get `receiver = null`. `returnTypes` carries the inferred
/// return types from checkModule's inferReturnTypes pass.
function collectFunctions(
  headings: TangleHeading[],
  parent: TangleHeading | null,
  parsedBlocks: ParsedCodeBlock[],
  returnTypes: Map<string, Type>,
  out: IRFunction[],
): void {
  for (const h of headings) {
    if (
      h.role === "callable"
      && (h.codeBlocks ?? []).length > 0
    ) {
      // Mirror Rust's `symbol_name` fallback: TS `parseHeadingText` only sets
      // `symbolName` for `(symbolName)` pattern; Rust always sets it. Use
      // `title` as fallback (same as `buildSymbols` in compileModule.ts).
      const name = h.symbolName ?? h.title;
      const receiver = name !== "main"
        ? (parent !== null && parent.role === "type" ? (parent.symbolName ?? parent.title) : null)
        : null;
      const params = (h.params ?? []).map(p => ({
        name: p.name,
        type: p.typeName ? typeNameToType(p.typeName) : undefined,
      }));
      const blocks = parsedBlocks.filter(b => b.headingId === h.id);
      const { nodes, edges, entryNodeId, errorEdges } = lowerFunctionBody(blocks);
      out.push({ name, receiver, params, returnType: returnTypes.get(h.id) ?? undefined, nodes, edges, entryNodeId, errorEdges });
    }
    collectFunctions(h.children, h, parsedBlocks, returnTypes, out);
  }
}

/// Lower a function body from its parsed code blocks into IR nodes/edges.
/// Chains statements across multiple blocks sequentially
/// (entry → stmts → terminal). Mirror of Rust `lower_function_body` in
/// compile_to_ir.rs:168-218.
function lowerFunctionBody(blocks: ParsedCodeBlock[]): {
  nodes: IRNode[];
  edges: IREdge[];
  entryNodeId: string;
  errorEdges: IRErrorEdge[];
} {
  const syntheticSpan: SourceSpan = {
    file: "",
    startLine: 1,
    startColumn: 1,
    endLine: 1,
    endColumn: 1,
  };

  const entryId = freshNodeId("entry");
  const nodes: IRNode[] = [{
    kind: "compute",
    id: entryId,
    label: "entry",
    sourceSpan: syntheticSpan,
  }];
  const edges: IREdge[] = [];
  let prevId = entryId;

  for (const block of blocks) {
    for (const stmt of block.body.statements) {
      const { kind, label } = lowerStmtForFunction(stmt);
      const nodeId = freshNodeId("n");
      nodes.push({ kind, id: nodeId, label, sourceSpan: syntheticSpan });
      edges.push({ from: prevId, to: nodeId, kind: "control", sourceSpan: syntheticSpan });
      prevId = nodeId;
    }
  }

  const terminalId = freshNodeId("n");
  nodes.push({ kind: "terminal", id: terminalId, label: "exit", sourceSpan: syntheticSpan });
  edges.push({ from: prevId, to: terminalId, kind: "control", sourceSpan: syntheticSpan });

  return { nodes, edges, entryNodeId: entryId, errorEdges: [] };
}

/// Map a Stmt to (IRNodeKind, label) for function body lowering.
/// Mirror of Rust match in lower_function_body (compile_to_ir.rs:183-188).
function lowerStmtForFunction(stmt: Stmt): { kind: IRNodeKind; label: string } {
  switch (stmt.kind) {
    case "return":
      return { kind: "action", label: "return" };
    case "let":
      return { kind: "compute", label: `let ${stmt.name}` };
    case "const":
      return { kind: "compute", label: `const ${stmt.name}` };
    case "expression":
      return { kind: "action", label: "expr" };
  }
}
