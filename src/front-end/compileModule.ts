import type { MarkdownNode } from "../markdown/parseMarkdown.js";
import type {
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
import { parseDirectiveLine } from "./directives.js";
import { headingRoleForDepth, parseHeadingText } from "./headings.js";
import { spanFromNode } from "./sourceMap.js";

export type CompileModuleInput = {
  file: string;
  source: string;
};

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
    headings.push(buildHeading(input.file, node!, body, diagnostics));
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
  diagnostics: TangleDiagnostic[]
): TangleHeading {
  const parsed = parseHeadingText(plainText(node));
  const directives: TangleDirective[] = [];
  const params: TangleParam[] = [];
  const codeBlocks: TangleCodeBlock[] = [];

  for (const child of body) {
    if (child.type === "paragraph") {
      const text = plainText(child).trim();
      if (text.startsWith("@")) {
        try {
          directives.push(parseDirectiveLine(text, spanFromNode(file, child)));
        } catch (error) {
          diagnostics.push({
            code: "TANGLE_UNKNOWN_DIRECTIVE",
            message: error instanceof Error ? error.message : String(error),
            span: spanFromNode(file, child)
          });
        }
      } else if (/\s@[A-Za-z]/.test(text)) {
        diagnostics.push({
          code: "TANGLE_INVALID_DIRECTIVE_POSITION",
          message: "Tangle directives must appear directly under a heading or directly above their target block",
          span: spanFromNode(file, child)
        });
      }
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
  return heading;
}

function buildSymbols(headings: TangleHeading[]): TangleSymbol[] {
  return headings.map((heading) => {
    const exported = heading.directives.some((directive) => directive.kind === "export" || directive.kind === "entry");
    const name = heading.symbolName ?? heading.title;

    if (heading.role === "type") {
      return { name, kind: "type", exported, headingId: heading.id, span: heading.span };
    }

    if (heading.role === "callable") {
      return { name, kind: "callable", exported, headingId: heading.id, span: heading.span };
    }

    if (heading.role === "program" && heading.directives.some((directive) => directive.kind === "entry")) {
      return { name, kind: "entry", exported: true, headingId: heading.id, span: heading.span };
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
    heading.directives.some((directive) => directive.kind === "entry")
  );

  if (entryHeadings.length > 1) {
    diagnostics.push({
      code: "TANGLE_DUPLICATE_ENTRY",
      message: "A Tangle program must declare exactly one @entry",
      span: entryHeadings[1]!.span
    });
  }

  for (const heading of headings) {
    const exported = heading.directives.some((directive) => directive.kind === "export");
    const exportable = heading.role === "type" || heading.role === "callable";
    if (exported && !exportable) {
      diagnostics.push({
        code: "TANGLE_INVALID_EXPORT_LEVEL",
        message: "@export is only valid on type and callable headings",
        span: heading.span
      });
    }
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
