import type { Type } from "./types.js";
import type { TangleDiagnostic } from "../model.js";
import type { ErrorRegistry } from "./errors.js";

// checkPropagation: strips error variants from a sum type on ?
// Returns [okType, diagnostics]
export function checkPropagation(type: Type, errorRegistry: ErrorRegistry): [Type, TangleDiagnostic[]] {
  const diags: TangleDiagnostic[] = [];
  if (type.kind === "sum") {
    const oks: Type[] = [];
    for (const v of type.variants) {
      const name = getVariantName(v);
      if (name && errorRegistry.isError(name)) {
        // error variant — stripped
      } else {
        oks.push(v);
      }
    }
    if (oks.length === 0) {
      diags.push({
        code: "TANGLE_TYPE_ALL_ERROR",
        message: "All variants are errors — nothing to unwrap",
        span: { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 },
      });
      return [{ kind: "primitive", name: "Bool" }, diags];
    }
    if (oks.length === 1) return [oks[0]!, diags];
    // Multiple ok variants: return the sum of oks
    return [{ kind: "sum", variants: oks }, diags];
  }
  // Not a sum type — ? has no error to propagate
  return [type, diags];
}

function getVariantName(t: Type): string | null {
  if (t.kind === "struct") return t.name;
  if (t.kind === "primitive") return t.name;
  if (t.kind === "interface") return t.name;
  return null;
}
