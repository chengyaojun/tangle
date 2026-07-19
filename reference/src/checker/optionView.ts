import type { Type } from "./types.js";
import type { TypeEnv } from "./env.js";

/// 把已知类型识别为 Sum 视图。
/// 当前仅识别 `Option<T>`；`Result<T,E>` 推迟到 Phase 6e。
/// 镜像 Rust crate::checker::option_view::as_sum_view。
export function asSumView(ty: Type): Type | null {
  switch (ty.kind) {
    case "sum":
      return ty;
    case "genericInstance":
      if (ty.base === "Option") {
        const inner = ty.args[0] ?? { kind: "any" as const };
        return {
          kind: "sum",
          variants: [
            { kind: "genericInstance", base: "Some", args: [inner] },
            { kind: "struct", name: "None", fields: {}, methods: {} },
          ],
        };
      }
      return null;
    default:
      return null;
  }
}

/// 在 env.structs 中查找同名结构体，若有则返回带字段的完整定义。
/// 镜像 Rust crate::checker::option_view::resolve_struct_in_env。
///
/// 背景：typeExprToType 解析 `Option<Item>` 时会生成空字段的外壳
/// `Struct { name: "Item", fields: {} }`。真实带字段的定义位于 env.structs。
/// 此函数用于在 binding 类型注入前补全结构体定义。
export function resolveStructInEnv(ty: Type, env: TypeEnv): Type {
  if (ty.kind === "struct") {
    const full = env.structs[ty.name];
    if (full && full.kind === "struct") {
      return full;
    }
    return ty;
  }
  return ty;
}
