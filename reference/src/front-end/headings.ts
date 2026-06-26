import type { HeadingRole } from "../model.js";

export function headingRoleForDepth(depth: number): HeadingRole {
  switch (depth) {
    case 1:
      return "program";
    case 2:
      return "section";
    case 3:
      return "type";
    case 4:
      return "callable";
    case 5:
      return "semantic-section";
    case 6:
      return "semantic-atom";
    default:
      throw new Error(`Invalid Markdown heading depth: ${depth}`);
  }
}

export function parseHeadingText(text: string): { title: string; symbolName?: string; hasSpaces?: boolean } {
  const match = text.match(/^(.*?)\s+\(([A-Za-z_][A-Za-z0-9_]*)\)\s*$/);
  if (!match || !match[1] || !match[2]) {
    const trimmed = text.trim();
    const isAscii = /^[a-zA-Z][a-zA-Z0-9]*$/.test(trimmed);
    const hasSpaces = !isAscii && /^[a-zA-Z][a-zA-Z\s]+$/.test(trimmed);
    const result: { title: string; symbolName?: string; hasSpaces?: boolean } = { title: trimmed };
    if (hasSpaces) result.hasSpaces = true;
    return result;
  }

  return {
    title: match[1].trim(),
    symbolName: match[2]
  };
}
