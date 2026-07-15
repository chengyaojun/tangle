import type { RuleGraph, IRNode, IREdge, TangleDiagnostic } from "./graph.js";
import type { SourceSpan } from "../model.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

// IRNode in graph.ts lacks sourceText/group/style (present on Rust IRNode).
// Define an extended node type carrying the extra metadata. ToggleIRNode is
// structurally assignable to IRNode, so it can be pushed onto graph.nodes.
type ToggleIRNode = IRNode & {
  sourceText: string | null;
  group: string | null;
  style: string | null;
};

export interface ToggleLowerResult {
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
}

export function lowerRuleToggle(checkboxMarkdown: string, file: string): ToggleLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const entryId = freshNodeId("toggle-entry");
  const graph = createGraph(entryId);

  // Entry node: synthetic span (line 0 marks synthetic), no group/style/text.
  const entrySpan: SourceSpan = {
    file,
    startLine: 0,
    startColumn: 0,
    endLine: 0,
    endColumn: 0,
  };
  graph.nodes.push({
    id: entryId,
    kind: "compute",
    label: "toggle.entry",
    sourceSpan: entrySpan,
    sourceText: null,
    group: null,
    style: null,
  });

  let pendingGroup: string | null = null;
  let pendingStyle: string | null = null;
  let toggleIndex = 0;

  const lines = checkboxMarkdown.split("\n");
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const line = lines[lineIdx]!;
    const lineNo = lineIdx + 1; // 1-based
    const t = line.trimStart();

    // HTML comment metadata: <!-- group: X --> or <!-- style: Y -->
    const meta = parseHtmlComment(t);
    if (meta) {
      if (meta.key === "group") pendingGroup = meta.value;
      if (meta.key === "style") pendingStyle = meta.value;
      continue;
    }

    // Skip non-checkbox lines (but clear pending metadata on other content)
    if (!t.startsWith("- [") && !t.startsWith("* [")) {
      if (t !== "" && !t.startsWith("<!--")) {
        pendingGroup = null;
        pendingStyle = null;
      }
      continue;
    }

    // Malformed: starts with `- [` or `* [` but lacks `[x]`/`[X]`/`[ ]`
    const isValid = t.includes("[x]") || t.includes("[X]") || t.includes("[ ]");
    if (!isValid) {
      diagnostics.push({
        code: "TANGLE_RULE_TOGGLE_MALFORMED",
        message: `malformed checkbox: expected [x], [X], or [ ]: ${t}`,
        span: { file, startLine: lineNo, startColumn: 0, endLine: lineNo, endColumn: 0 },
      });
      continue;
    }

    const checked = t.includes("[x]") || t.includes("[X]");
    // Strip checkbox prefix
    const rest = t
      .replace(/^-\s*\[x\]\s*/, "")
      .replace(/^-\s*\[X\]\s*/, "")
      .replace(/^-\s*\[ \]\s*/, "")
      .replace(/^\*\s*\[x\]\s*/, "")
      .replace(/^\*\s*\[X\]\s*/, "")
      .replace(/^\*\s*\[ \]\s*/, "")
      .trim();

    // Extract name: backtick > colon > null
    const extracted = extractName(rest);
    let name: string;
    if (extracted !== null) {
      name = extracted;
    } else {
      diagnostics.push({
        code: "TANGLE_RULE_TOGGLE_MISSING_NAME",
        message: `could not extract toggle name from: ${rest}`,
        span: { file, startLine: lineNo, startColumn: 0, endLine: lineNo, endColumn: 0 },
      });
      name = `toggle_${toggleIndex}`;
    }

    const nodeId = freshNodeId("toggle");
    const toggleNode: ToggleIRNode = {
      id: nodeId,
      kind: "compute",
      label: `${name} = ${checked}`,
      sourceSpan: { file, startLine: lineNo, startColumn: 0, endLine: lineNo, endColumn: 0 },
      sourceText: null,
      group: pendingGroup,
      style: pendingStyle,
    };
    graph.nodes.push(toggleNode);
    const edge: IREdge = {
      from: entryId,
      to: nodeId,
      kind: "control",
      sourceSpan: { file, startLine: lineNo, startColumn: 0, endLine: lineNo, endColumn: 0 },
    };
    graph.edges.push(edge);
    pendingGroup = null;
    pendingStyle = null;
    toggleIndex++;
  }

  return { graph, diagnostics };
}

/// Extract toggle name. Priority: backtick (`name`) > colon (name: value) > null.
function extractName(rest: string): string | null {
  // 1. Backtick: `name`: desc
  const tickStart = rest.indexOf("`");
  if (tickStart >= 0) {
    const afterTick = rest.substring(tickStart + 1);
    const tickEnd = afterTick.indexOf("`");
    if (tickEnd >= 0) {
      return afterTick.substring(0, tickEnd);
    }
  }
  // 2. Colon: name: value (name must be a valid identifier)
  const colonPos = rest.indexOf(":");
  if (colonPos >= 0) {
    const candidate = rest.substring(0, colonPos).trim();
    if (isValidIdentifier(candidate)) {
      return candidate;
    }
  }
  return null;
}

/// Valid identifier: [a-zA-Z_][a-zA-Z0-9_]*
function isValidIdentifier(s: string): boolean {
  if (s.length === 0) return false;
  const first = s.charCodeAt(0);
  if (!((first >= 65 && first <= 90) || (first >= 97 && first <= 122) || first === 95)) return false;
  for (let i = 1; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (!((c >= 65 && c <= 90) || (c >= 97 && c <= 122) || (c >= 48 && c <= 57) || c === 95)) return false;
  }
  return true;
}

interface HtmlCommentMeta {
  key: "group" | "style";
  value: string;
}

/// Parse `<!-- group: X -->` or `<!-- style: Y -->`. Returns null otherwise.
function parseHtmlComment(line: string): HtmlCommentMeta | null {
  const trimmed = line.trim();
  // Need at least "<!--x-->" (8 chars) to slice safely
  if (trimmed.length < 8) return null;
  if (!trimmed.startsWith("<!--") || !trimmed.endsWith("-->")) return null;
  const inner = trimmed.substring(4, trimmed.length - 3).trim();
  if (inner.startsWith("group:")) {
    return { key: "group", value: inner.substring(6).trim() };
  }
  if (inner.startsWith("style:")) {
    return { key: "style", value: inner.substring(6).trim() };
  }
  return null;
}
