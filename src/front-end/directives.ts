import type { DirectiveKind, SourceSpan, TangleDirective } from "../model.js";

const SIMPLE_DIRECTIVES: Record<string, DirectiveKind> = {
  "@hideCode": "hideCode",
  "@rule.table": "rule.table",
  "@rule.tree": "rule.tree",
  "@rule.toggle": "rule.toggle",
  "@rule.flow": "rule.flow"
};

export function parseDirectiveLine(rawLine: string, span: SourceSpan): TangleDirective {
  const raw = rawLine.trim();

  const simple = SIMPLE_DIRECTIVES[raw];
  if (simple) {
    return { kind: simple, raw, span };
  }

  const deprecated = raw.match(/^@deprecated\((.*)\)$/);
  if (deprecated && deprecated[1]) {
    return { kind: "deprecated", raw, args: deprecated[1], span };
  }

  const test = raw.match(/^@test\((.*)\)$/);
  if (test && test[1]) {
    return { kind: "test", raw, args: test[1], span };
  }

  const error = raw.match(/^@error\s+([A-Za-z_][A-Za-z0-9_]*)(?:\((.*)\))?$/);
  if (error && error[1]) {
    const result: TangleDirective = { kind: "error", raw, name: error[1], span };
    if (error[2]) {
      result.args = error[2];
    }
    return result;
  }

  throw new Error(`Unknown Tangle directive: ${raw}`);
}
