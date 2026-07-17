export type Type =
  | PrimitiveType
  | StructType
  | SumType_
  | GenericTypeInstance
  | FunctionType_
  | InterfaceType
  | TypeVariable
  | AnyType;

export type PrimitiveType = {
  kind: "primitive";
  name: "String" | "Int" | "Bool";
};

export type AnyType = {
  kind: "any";
};

export type StructType = {
  kind: "struct";
  name: string;
  fields: Record<string, Type>;
  methods: Record<string, CallableSignature>;
};

export type SumType_ = {
  kind: "sum";
  variants: Type[];
};

export type GenericTypeInstance = {
  kind: "genericInstance";
  base: string;
  args: Type[];
};

export type FunctionType_ = {
  kind: "function";
  params: Type[];
  returns: Type;
};

export type InterfaceType = {
  kind: "interface";
  name: string;
  methods: Record<string, CallableSignature>;
};

export type TypeVariable = {
  kind: "var";
  id: number;
};

export type CallableSignature = {
  params: { name: string; type: Type }[];
  returns: Type;
  is_variadic?: boolean;
};

export function typesEqual(a: Type, b: Type): boolean {
  if (a.kind === "any" || b.kind === "any") return true;
  if (a.kind !== b.kind) return false;
  if (a.kind === "primitive" && b.kind === "primitive") return a.name === b.name;
  if (a.kind === "struct" && b.kind === "struct") return a.name === b.name;
  if (a.kind === "interface" && b.kind === "interface") return a.name === b.name;
  return false;
}

export function isSubtype(value: Type, target: Type): boolean {
  if (target.kind === "interface" && value.kind === "struct") {
    return Object.entries(target.methods).every(([name, sig]) => {
      const ms = value.methods[name];
      if (!ms) return false;
      if (ms.params.length !== sig.params.length) return false;
      return ms.params.every((p, i) => typesEqual(p.type, sig.params[i]!.type))
        && typesEqual(ms.returns, sig.returns);
    });
  }
  return typesEqual(value, target);
}

/// 构造类型变量
export function typeVar(id: number): Type {
  return { kind: "var", id };
}

/// 构造泛型实例
export function generic(base: string, args: Type[]): Type {
  return { kind: "genericInstance", base, args };
}
