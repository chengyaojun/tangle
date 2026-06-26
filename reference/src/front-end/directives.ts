import type { SourceSpan, TangleDirective } from "../model.js";

export function parseDirectiveLine(rawLine: string, span: SourceSpan): TangleDirective {
  throw new Error(`Unknown Tangle directive: ${rawLine.trim()}`);
}
