import type { MarkdownNode } from "../markdown/parseMarkdown.js";
import type {
  SourceSpan,
  TangleCodeBlock,
  TangleDiagnostic,
  TangleDirective,
  TangleHeading,
  TangleModule,
  TangleParam,
  TangleSymbol
} from "../model.js";
import { parseMarkdown } from "../markdown/parseMarkdown.js";
import { collectLinks, isTangleCodeBlock, parseParamItem, plainText } from "./blocks.js";
import { headingRoleForDepth, parseHeadingText } from "./headings.js";
import { spanFromNode } from "./sourceMap.js";

export type CompileModuleInput = {
  file: string;
  source: string;
};

// ─── Rule detection (mirror of Rust determine_rule_kind / RuleData) ────────
// TangleHeading (model.ts) lacks a `rule` field (present on Rust TangleHeading).
// We attach rule data as an extra property at runtime; compileToIR.ts reads it
// via a type assertion. This avoids modifying model.ts.

export type RuleKind = "flow" | "table" | "tree" | "toggle";

export type RuleData = {
  kind: RuleKind;
  source: string;
  span: SourceSpan;
};

function isRuleHeading(title: string): boolean {
  return title.startsWith("Rule:") || title.startsWith("rule:");
}

/// Detect rule kind from body source. Priority order mirrors Rust
/// `determine_rule_kind` in compile_module.rs:
/// 1. Mermaid fenced code block → Flow
/// 2. Pipe table (2+ lines with |) → Table
/// 3. Checkbox items (- [ / * [) → Toggle
/// 4. Bullet list items (* / - ) → Tree
function determineRuleKind(source: string): RuleKind | null {
  if (source.includes("```mermaid") || source.includes("graph TD") || source.includes("graph LR")) {
    return "flow";
  }
  const pipeLines = source.split("\n").filter(l => l.includes("|"));
  if (pipeLines.length >= 2) {
    return "table";
  }
  if (source.split("\n").some(l => {
    const t = l.trimStart();
    return t.startsWith("- [") || t.startsWith("* [");
  })) {
    return "toggle";
  }
  if (source.split("\n").some(l => {
    const t = l.trimStart();
    return t.startsWith("* ") || t.startsWith("- ");
  })) {
    return "tree";
  }
  return null;
}

/// Extract the raw markdown text of the rule body from the source by using
/// body nodes' position info. Mirrors Rust's line-range tracking in
/// compile_module.rs (pending_rule_line_start / pending_rule_line_end).
function extractRuleSource(body: MarkdownNode[], source: string): string {
  let minStartLine = Infinity;
  let maxEndLine = 0;
  let found = false;
  for (const node of body) {
    if (node.position) {
      const startLine = node.position.start.line;
      const endLine = node.position.end.line;
      if (startLine < minStartLine) minStartLine = startLine;
      if (endLine > maxEndLine) maxEndLine = endLine;
      found = true;
    }
  }
  if (!found) return "";
  const lines = source.split("\n");
  return lines.slice(minStartLine - 1, maxEndLine).join("\n");
}

export function compileModule(input: CompileModuleInput): TangleModule {
  const root = parseMarkdown(input.source);
  const headings: TangleHeading[] = [];
  const diagnostics: TangleDiagnostic[] = [];

  const children = root.children ?? [];
  for (let index = 0; index < children.length; index += 1) {
    const node = children[index];
    if (node != null && (node.type !== "heading" || typeof node.depth !== "number")) {
      continue;
    }

    const nextHeadingIndex = findNextHeadingIndex(children, index + 1);
    const body = children.slice(index + 1, nextHeadingIndex);
    headings.push(buildHeading(input.file, node!, body, input.source, diagnostics));
  }

  buildHeadingTree(headings); // populates children for tree navigation
  const symbols = buildSymbols(headings);
  validateSymbolRules(headings, diagnostics);

  return {
    file: input.file,
    moduleName: moduleNameFromFile(input.file),
    imports: collectLinks(input.file, root),
    headings,
    symbols,
    diagnostics
  };
}

function buildHeadingTree(headings: TangleHeading[]): TangleHeading[] {
  const roots: TangleHeading[] = [];
  const stack: TangleHeading[] = [];

  for (const heading of headings) {
    // Pop stack until we find a parent (shallower depth)
    while (stack.length > 0 && stack[stack.length - 1]!.depth >= heading.depth) {
      stack.pop();
    }

    if (stack.length === 0) {
      roots.push(heading);
    } else {
      stack[stack.length - 1]!.children.push(heading);
    }
    stack.push(heading);
  }

  return roots;
}

function buildHeading(
  file: string,
  node: MarkdownNode,
  body: MarkdownNode[],
  source: string,
  diagnostics: TangleDiagnostic[]
): TangleHeading {
  const parsed = parseHeadingText(plainText(node));
  const directives: TangleDirective[] = [];
  const params: TangleParam[] = [];
  const codeBlocks: TangleCodeBlock[] = [];

  for (const child of body) {
    if (child.type === "paragraph") {
      // @-directives have been eliminated; all @-text is ordinary prose
    }

    if (child.type === "list") {
      for (const item of child.children ?? []) {
        const firstInlineCode = findInlineCode(item);
        if (firstInlineCode) {
          const textWithBackticks = "\`" + (firstInlineCode.value ?? "") + "\`" +
            plainText(item).slice((firstInlineCode.value ?? "").length);
          params.push(parseParamItem(textWithBackticks.trim(), spanFromNode(file, item)));
        }
      }
    }

    if (isTangleCodeBlock(child)) {
      codeBlocks.push({
        language: "tangle",
        value: (child.value ?? "").trim(),
        span: spanFromNode(file, child)
      });
    }
  }

  // Validate heading casing
  const candidateName = parsed.symbolName ?? parsed.title;
  const isAsciiOnly = /^[a-zA-Z][a-zA-Z0-9]*$/.test(candidateName);

  if (parsed.hasSpaces) {
    diagnostics.push({
      code: "TANGLE_HEADING_MULTI_WORD",
      message: `Heading "${parsed.title}" contains spaces. Use camelCase (e.g. "clearAll") or add an explicit parenthesized identifier "(clear)".`,
      span: spanFromNode(file, node)
    });
  }

  if (isAsciiOnly) {
    const depth = node.depth ?? 1;
    const firstChar = candidateName[0]!;
    if (depth >= 1 && depth <= 3) {
      // PascalCase: must start with uppercase
      if (firstChar < 'A' || firstChar > 'Z') {
        diagnostics.push({
          code: "TANGLE_INVALID_HEADING_CASE",
          message: `Heading "${parsed.title}" (depth ${depth}): symbol "${candidateName}" must use PascalCase (start with uppercase).`,
          span: spanFromNode(file, node)
        });
      }
    } else if (depth >= 4 && depth <= 6) {
      // camelCase: must start with lowercase
      if (firstChar < 'a' || firstChar > 'z') {
        diagnostics.push({
          code: "TANGLE_INVALID_HEADING_CASE",
          message: `Heading "${parsed.title}" (depth ${depth}): symbol "${candidateName}" must use camelCase (start with lowercase).`,
          span: spanFromNode(file, node)
        });
      }
    }
  }

  const heading: TangleHeading = {
    id: stableHeadingId(parsed.symbolName ?? parsed.title),
    depth: node.depth ?? 1,
    role: headingRoleForDepth(node.depth ?? 1),
    title: parsed.title,
    directives,
    params,
    codeBlocks,
    span: spanFromNode(file, node),
    children: []
  };
  if (parsed.symbolName) {
    heading.symbolName = parsed.symbolName;
  }

  // Extract rule data if this is a rule heading (mirror of Rust compile_module.rs).
  // TangleHeading has no `rule` field, so we attach it as an extra property.
  if (isRuleHeading(parsed.title)) {
    const ruleSource = extractRuleSource(body, source);
    const kind = determineRuleKind(ruleSource);
    if (kind) {
      (heading as TangleHeading & { rule?: RuleData }).rule = {
        kind,
        source: ruleSource,
        span: heading.span,
      };
    }
  }

  return heading;
}

function buildSymbols(headings: TangleHeading[]): TangleSymbol[] {
  return headings.map((heading) => {
    const name = heading.symbolName ?? heading.title;
    const isPrivate = name.startsWith('_');
    const exported = isPrivate ? false
      : heading.role === "type" || heading.role === "callable";

    if (heading.role === "type") {
      return { name, kind: "type", exported, headingId: heading.id, span: heading.span };
    }

    if (name === "main" && heading.role === "callable") {
      return { name, kind: "entry", exported: true, headingId: heading.id, span: heading.span };
    }

    if (heading.role === "callable") {
      return { name, kind: "callable", exported, headingId: heading.id, span: heading.span };
    }

    return { name, kind: "semantic-internal", exported: false, headingId: heading.id, span: heading.span };
  });
}

function findNextHeadingIndex(nodes: MarkdownNode[], start: number): number {
  const next = nodes.findIndex((node, offset) => offset >= start && node.type === "heading");
  return next === -1 ? nodes.length : next;
}

function moduleNameFromFile(file: string): string {
  return file.replace(/\\/g, "/").split("/").pop()?.replace(/\.md$/, "") ?? file;
}

function stableHeadingId(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^\p{L}\p{N}_-]/gu, "");
}

function validateSymbolRules(headings: TangleHeading[], diagnostics: TangleDiagnostic[]): void {
  const entryHeadings = headings.filter((heading) =>
    heading.symbolName === "main" || heading.title === "main"
  );

  if (entryHeadings.length > 1) {
    diagnostics.push({
      code: "TANGLE_DUPLICATE_ENTRY",
      message: "A Tangle program must declare exactly one main function",
      span: entryHeadings[1]!.span
    });
  }

}

function findInlineCode(node: MarkdownNode): MarkdownNode | null {
  if (node.type === "inlineCode") {
    return node;
  }
  for (const child of node.children ?? []) {
    const found = findInlineCode(child);
    if (found) {
      return found;
    }
  }
  return null;
}
