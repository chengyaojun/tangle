import type { MarkdownNode, MarkdownRoot } from "../markdown/parseMarkdown.js";
import type { SourceSpan, TangleImport, TangleParam } from "../model.js";
import { spanFromNode } from "./sourceMap.js";

export function collectLinks(file: string, root: MarkdownRoot): TangleImport[] {
  const imports: TangleImport[] = [];

  function walk(node: MarkdownNode): void {
    if (node.type === "link" && node.url) {
      const alias = plainText(node).trim();
      if (alias && node.url.endsWith(".md")) {
        imports.push({ alias, target: node.url, span: spanFromNode(file, node) });
      }
    }

    for (const child of node.children ?? []) {
      walk(child);
    }
  }

  walk(root);
  return imports;
}

export function parseParamItem(text: string, span: SourceSpan): TangleParam {
  // Phase 6c: 修正正则以支持 `name`: (Type) 形式（无 description，仅有类型注解）。
  // 原正则要求 `\s+` 前置空格，但 `: (Config)` 中 `:\s*` 已消耗空格，导致
  // `(Config)` 被当作 description。改为 `\s*` 并允许 description 为空，与
  // Rust parse_param_item 行为一致（Rust 用 rfind('(')/rfind(')') 提取最后
  // 括号对作为 typeName，不要求 description 非空）。
  const match = text.match(/^`([^`]+)`:\s*(.*?)(?:\s*\(([^)]+)\))?$/);
  if (!match || !match[1]) {
    throw new Error(`Invalid Tangle parameter item: ${text}`);
  }

  const result: TangleParam = {
    name: match[1],
    description: (match[2] ?? "").trim(),
    span
  };
  if (match[3]) {
    result.typeName = match[3];
  }
  return result;
}

export function isTangleCodeBlock(node: Pick<MarkdownNode, "type" | "lang">): boolean {
  return node.type === "code" && node.lang === "@tangle";
}

export function plainText(node: MarkdownNode): string {
  if (typeof node.value === "string") {
    return node.value;
  }

  return (node.children ?? []).map(plainText).join("");
}
