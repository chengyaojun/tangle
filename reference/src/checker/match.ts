import type { Type } from "./types.js";

/// 提取 variant 名（Primitive/Struct/Interface/GenericInstance）。
/// 其他类型不支持作为命名 variant。
export function getVariantName(t: Type): string | null {
  if (t.kind === "struct") return t.name;
  if (t.kind === "primitive") return t.name;
  if (t.kind === "interface") return t.name;
  if (t.kind === "genericInstance") return t.base;
  return null;
}

/// 提取 binding 类型。
/// GenericInstance 返回 args[0]（payload）；其他返回 variant 类型本身。
export function bindingTypeOf(variantType: Type): Type {
  if (variantType.kind === "genericInstance") {
    return variantType.args[0] ?? { kind: "any" };
  }
  return variantType;
}

/// 在 Sum 的 variants 中按名查找。
export function findVariantByName(sumType: Type, name: string): Type | null {
  if (sumType.kind !== "sum") return null;
  for (const v of sumType.variants) {
    const vname = getVariantName(v);
    if (vname === name) return v;
  }
  return null;
}

/// Check match exhaustiveness. Returns list of uncovered variant names.
/// 支持 Primitive/Struct/Interface/GenericInstance variant。
export function checkMatchExhaustiveness(sumType: Type, armPatterns: string[]): string[] {
  if (sumType.kind !== "sum") return [];
  const variantNames = sumType.variants
    .map((v) => getVariantName(v))
    .filter((n): n is string => n !== null);
  const covered = new Set<string>();
  let hasWildcard = false;
  for (const pattern of armPatterns) {
    if (pattern === "_") {
      hasWildcard = true;
      break;
    }
    covered.add(pattern);
  }
  if (hasWildcard) return [];
  return variantNames.filter((n) => !covered.has(n));
}
