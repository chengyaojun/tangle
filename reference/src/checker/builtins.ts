import type { PrimitiveType } from "./types.js";
import type { TypeEnv } from "./env.js";

export const builtinTypes: Record<string, PrimitiveType> = {
  String: { kind: "primitive", name: "String" },
  Int: { kind: "primitive", name: "Int" },
  Bool: { kind: "primitive", name: "Bool" }
};

export function isBuiltinType(name: string): boolean {
  return name in builtinTypes;
}

/// 注册 Err/Ok 构造器到 env.functions
export function registerBuiltins(env: TypeEnv): void {
  env.functions["Err"] = {
    params: [
      { name: "kind", type: { kind: "primitive", name: "String" } },
      { name: "msg", type: { kind: "primitive", name: "String" } },
    ],
    returns: { kind: "any" },
  };
  env.functions["Ok"] = {
    params: [{ name: "value", type: { kind: "any" } }],
    returns: { kind: "any" },
  };
}
