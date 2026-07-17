import type { Type } from "./types.js";

/// 类型变量替换表：TypeVarId → 实际类型
export type Substitution = Map<number, Type>;

/// 统一 expected 类型与 actual 类型，更新 subst。
/// 成功返回 null；失败返回冲突描述。
export function unify(expected: Type, actual: Type, subst: Substitution): string | null {
  // Any 总是成功（双向）
  if (expected.kind === "any" || actual.kind === "any") return null;

  // 类型变量（expected 侧）
  if (expected.kind === "var") {
    const existing = subst.get(expected.id);
    if (existing) {
      return unify(existing, actual, subst);
    }
    subst.set(expected.id, actual);
    return null;
  }
  // 类型变量（actual 侧）
  if (actual.kind === "var") {
    const existing = subst.get(actual.id);
    if (existing) {
      return unify(expected, existing, subst);
    }
    subst.set(actual.id, expected);
    return null;
  }

  // 泛型实例
  if (expected.kind === "genericInstance" && actual.kind === "genericInstance") {
    if (expected.base !== actual.base) {
      return `Expected ${expected.base}, got ${actual.base}`;
    }
    if (expected.args.length !== actual.args.length) {
      return "Generic arity mismatch";
    }
    for (let i = 0; i < expected.args.length; i++) {
      const err = unify(expected.args[i]!, actual.args[i]!, subst);
      if (err) return err;
    }
    return null;
  }

  // 基本类型
  if (expected.kind === "primitive" && actual.kind === "primitive") {
    return expected.name === actual.name ? null : `Expected ${expected.name}, got ${actual.name}`;
  }

  // 结构体
  if (expected.kind === "struct" && actual.kind === "struct") {
    return expected.name === actual.name ? null : `Expected ${expected.name}, got ${actual.name}`;
  }

  // 函数类型
  if (expected.kind === "function" && actual.kind === "function") {
    if (expected.params.length !== actual.params.length) {
      return "Function arity mismatch";
    }
    for (let i = 0; i < expected.params.length; i++) {
      const err = unify(expected.params[i]!, actual.params[i]!, subst);
      if (err) return err;
    }
    return unify(expected.returns, actual.returns, subst);
  }

  return `Type mismatch: ${expected.kind} vs ${actual.kind}`;
}

/// 用 subst 替换类型中的 TypeVariable（递归）
export function substitute(ty: Type, subst: Substitution): Type {
  switch (ty.kind) {
    case "var":
      return subst.get(ty.id) ?? ty;
    case "genericInstance":
      return {
        kind: "genericInstance",
        base: ty.base,
        args: ty.args.map(a => substitute(a, subst)),
      };
    case "function":
      return {
        kind: "function",
        params: ty.params.map(p => substitute(p, subst)),
        returns: substitute(ty.returns, subst),
      };
    default:
      return ty;
  }
}
