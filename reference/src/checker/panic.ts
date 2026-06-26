import type { Type } from "./types.js";
import type { TangleDiagnostic } from "../model.js";

// Panic always returns bottom type (never returns normally)
export function checkPanic(): [Type, TangleDiagnostic[]] {
  // Bottom type — panic never returns
  return [{ kind: "primitive", name: "Bool" }, []];
}

// Check if a code path after panic is dead
export function isDeadPath(diagnostics: TangleDiagnostic[]): boolean {
  return diagnostics.some((d) => d.code === "TANGLE_PANIC_REACHED");
}
