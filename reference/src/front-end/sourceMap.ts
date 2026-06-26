import type { MarkdownNode } from "../markdown/parseMarkdown.js";
import type { SourceSpan } from "../model.js";

export function spanFromNode(file: string, node: MarkdownNode): SourceSpan {
  const position = node.position;
  if (!position) {
    return { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 };
  }

  return {
    file,
    startLine: position.start.line,
    startColumn: position.start.column,
    endLine: position.end.line,
    endColumn: position.end.column
  };
}
