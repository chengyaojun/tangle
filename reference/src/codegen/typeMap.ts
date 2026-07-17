import type { Type } from "../checker/types.js";

/**
 * 将 Tangle Type 映射为 Python 类型注解字符串。
 * undefined 表示无注解（emitter 省略 `: ...`）。
 * 镜像 Rust compiler/tangle-cli/src/codegen/type_map.rs:tangle_type_to_py。
 */
export function tangleTypeToPy(ty: Type): string | undefined {
  switch (ty.kind) {
    case "any":
      return undefined;
    case "primitive":
      switch (ty.name as string) {
        case "Int": return "int";
        case "String": return "str";
        case "Bool": return "bool";
        case "Float": return "float";
        default: return ty.name;
      }
    case "struct":
      return ty.name;
    case "interface":
      return ty.name;
    case "genericInstance":
      switch (ty.base) {
        case "List": return `List[${innerPy(ty.args[0]!)}]`;
        case "Map": return `Dict[${innerPy(ty.args[0]!)}, ${innerPy(ty.args[1]!)}]`;
        case "Option": return `Optional[${innerPy(ty.args[0]!)}]`;
        case "Set": return `Set[${innerPy(ty.args[0]!)}]`;
        default: return `${ty.base}[${ty.args.map(innerPy).join(", ")}]`;
      }
    case "var":
      return undefined;
    case "function":
      return "Callable";
    case "sum": {
      const parts = ty.variants.map(tangleTypeToPy).filter((p): p is string => p !== undefined);
      return parts.length === 0 ? undefined : `Union[${parts.join(", ")}]`;
    }
  }
}

function innerPy(ty: Type): string {
  return tangleTypeToPy(ty) ?? "Any";
}

/**
 * 将 Tangle Type 映射为 Go 类型字符串。
 * Go 必须有返回类型，无注解时返回 "any"。
 * 镜像 Rust compiler/tangle-cli/src/codegen/type_map.rs:tangle_type_to_go。
 */
export function tangleTypeToGo(ty: Type): string {
  switch (ty.kind) {
    case "any":
      return "any";
    case "primitive":
      switch (ty.name as string) {
        case "Int": return "int";
        case "String": return "string";
        case "Bool": return "bool";
        case "Float": return "float64";
        default: return ty.name;
      }
    case "struct":
      return ty.name;
    case "interface":
      return ty.name;
    case "genericInstance":
      switch (ty.base) {
        case "List": return `[]${innerGo(ty.args[0]!)}`;
        case "Map": return `map[${innerGo(ty.args[0]!)}]${innerGo(ty.args[1]!)}`;
        case "Option": return `*${innerGo(ty.args[0]!)}`;
        case "Set": return `map[${innerGo(ty.args[0]!)}]struct{}`;
        default: return `any /* ${ty.base} */`;
      }
    case "var":
      return "any";
    case "function":
      return "func()";
    case "sum":
      return ty.variants[0] ? innerGo(ty.variants[0]) : "any";
  }
}

function innerGo(ty: Type): string {
  return tangleTypeToGo(ty);
}
