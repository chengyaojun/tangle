import type { PrimitiveType, Type, CallableSignature } from "./types.js";
import { typeVar, generic } from "./types.js";
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

/// stdlib 泛型签名（与 Rust signatures.rs 对齐）
export const stdlibGenericSignatures: Record<string, Record<string, CallableSignature>> = {
  List: {
    length: { params: [{ name: "list", type: generic("List", [typeVar(0)]) }], returns: { kind: "primitive", name: "Int" }, is_variadic: false },
    map: {
      params: [
        { name: "list", type: generic("List", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [typeVar(0)], returns: typeVar(1) } },
      ],
      returns: generic("List", [typeVar(1)]),
      is_variadic: false,
    },
    filter: {
      params: [
        { name: "list", type: generic("List", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [typeVar(0)], returns: { kind: "primitive", name: "Bool" } } },
      ],
      returns: generic("List", [typeVar(0)]),
      is_variadic: false,
    },
    push: {
      params: [{ name: "list", type: generic("List", [typeVar(0)]) }, { name: "item", type: typeVar(0) }],
      returns: generic("List", [typeVar(0)]),
      is_variadic: false,
    },
    get: {
      params: [{ name: "list", type: generic("List", [typeVar(0)]) }, { name: "index", type: { kind: "primitive", name: "Int" } }],
      returns: typeVar(0),
      is_variadic: false,
    },
  },
  Map: {
    get: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }, { name: "key", type: typeVar(0) }],
      returns: typeVar(1),
      is_variadic: false,
    },
    set: {
      params: [
        { name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) },
        { name: "key", type: typeVar(0) },
        { name: "value", type: typeVar(1) },
      ],
      returns: generic("Map", [typeVar(0), typeVar(1)]),
      is_variadic: false,
    },
    has: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }, { name: "key", type: typeVar(0) }],
      returns: { kind: "primitive", name: "Bool" },
      is_variadic: false,
    },
    keys: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }],
      returns: generic("List", [typeVar(0)]),
      is_variadic: false,
    },
    values: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }],
      returns: generic("List", [typeVar(1)]),
      is_variadic: false,
    },
    delete: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }, { name: "key", type: typeVar(0) }],
      returns: generic("Map", [typeVar(0), typeVar(1)]),
      is_variadic: false,
    },
  },
  Set: {
    add: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }, { name: "value", type: typeVar(0) }],
      returns: generic("Set", [typeVar(0)]),
      is_variadic: false,
    },
    remove: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }, { name: "value", type: typeVar(0) }],
      returns: generic("Set", [typeVar(0)]),
      is_variadic: false,
    },
    contains: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }, { name: "value", type: typeVar(0) }],
      returns: { kind: "primitive", name: "Bool" },
      is_variadic: false,
    },
    size: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }],
      returns: { kind: "primitive", name: "Int" },
      is_variadic: false,
    },
    union: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }, { name: "other", type: generic("Set", [typeVar(0)]) }],
      returns: generic("Set", [typeVar(0)]),
      is_variadic: false,
    },
    intersection: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }, { name: "other", type: generic("Set", [typeVar(0)]) }],
      returns: generic("Set", [typeVar(0)]),
      is_variadic: false,
    },
    difference: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }, { name: "other", type: generic("Set", [typeVar(0)]) }],
      returns: generic("Set", [typeVar(0)]),
      is_variadic: false,
    },
    to_list: {
      params: [{ name: "set", type: generic("Set", [typeVar(0)]) }],
      returns: generic("List", [typeVar(0)]),
      is_variadic: false,
    },
  },
  Option: {
    Some: { params: [{ name: "value", type: typeVar(0) }], returns: generic("Option", [typeVar(0)]), is_variadic: false },
    // None 无参可推导 T，返回 Any 避免悬空类型变量（与 Rust 对齐）
    None: { params: [], returns: { kind: "any" }, is_variadic: false },
    unwrap: { params: [{ name: "opt", type: generic("Option", [typeVar(0)]) }], returns: typeVar(0), is_variadic: false },
    is_some: { params: [{ name: "opt", type: generic("Option", [typeVar(0)]) }], returns: { kind: "primitive", name: "Bool" }, is_variadic: false },
    is_none: { params: [{ name: "opt", type: generic("Option", [typeVar(0)]) }], returns: { kind: "primitive", name: "Bool" }, is_variadic: false },
    map: {
      params: [
        { name: "opt", type: generic("Option", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [typeVar(0)], returns: typeVar(1) } },
      ],
      returns: generic("Option", [typeVar(1)]),
      is_variadic: false,
    },
    or_else: {
      params: [
        { name: "opt", type: generic("Option", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [], returns: generic("Option", [typeVar(0)]) } },
      ],
      returns: generic("Option", [typeVar(0)]),
      is_variadic: false,
    },
  },
};
