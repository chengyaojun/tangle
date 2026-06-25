import type { PrimitiveType } from "./types.js";

export const builtinTypes: Record<string, PrimitiveType> = {
  String: { kind: "primitive", name: "String" },
  Int: { kind: "primitive", name: "Int" },
  Bool: { kind: "primitive", name: "Bool" }
};

export function isBuiltinType(name: string): boolean {
  return name in builtinTypes;
}
