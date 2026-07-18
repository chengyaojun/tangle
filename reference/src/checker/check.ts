import type { Expr } from "../ast.js";
import type { TangleDiagnostic } from "../model.js";
import type { Type, StructType } from "./types.js";
import type { TypeEnv } from "./env.js";
import { typesEqual } from "./types.js";
import { unify, substitute, type Substitution, unifyAll, unifyPair } from "./unify.js";
import { checkMatchExhaustiveness, findVariantByName, bindingTypeOf } from "./match.js";

export function checkExpression(expr: Expr, env: TypeEnv): [Type, TangleDiagnostic[]] {
  const diags: TangleDiagnostic[] = [];

  switch (expr.kind) {
    case "literal": {
      const map: Record<string, Type> = {
        number: { kind: "primitive", name: "Int" },
        string: { kind: "primitive", name: "String" },
        boolean: { kind: "primitive", name: "Bool" }
      };
      return [map[expr.literalKind] ?? { kind: "primitive", name: "String" }, diags];
    }

    case "identifier": {
      if (env.variables[expr.name]) return [env.variables[expr.name]!, diags];
      if (env.receiver?.fields[expr.name]) return [env.receiver.fields[expr.name]!, diags];
      if (env.structs[expr.name]) return [env.structs[expr.name]!, diags];
      if (env.functions[expr.name]) {
        const fn = env.functions[expr.name]!;
        return [{ kind: "function", params: fn.params.map(p => p.type), returns: fn.returns }, diags];
      }
      if (["String", "Int", "Bool"].includes(expr.name)) {
        return [{ kind: "primitive", name: expr.name as "String" | "Int" | "Bool" }, diags];
      }
      diags.push({ code: "TANGLE_TYPE_UNDEFINED_VARIABLE", message: `Undefined variable: ${expr.name}`, span: expr.span });
      return [{ kind: "primitive", name: "String" }, diags];
    }

    case "this": {
      if (!env.receiver) {
        diags.push({ code: "TANGLE_TYPE_THIS_OUTSIDE_METHOD", message: "this can only be used inside a method", span: expr.span });
        return [{ kind: "primitive", name: "String" }, diags];
      }
      const recv: StructType = { kind: "struct", name: env.receiver.structName, fields: env.receiver.fields, methods: {} };
      return [recv, diags];
    }

    case "memberAccess": {
      const [objType, objDiags] = checkExpression(expr.object, env);
      diags.push(...objDiags);
      if (objType.kind === "any") {
        return [{ kind: "any" }, diags];
      }
      if (objType.kind === "struct" || objType.kind === "interface") {
        if (objType.kind === "struct" && objType.fields[expr.member]) {
          return [objType.fields[expr.member]!, diags];
        }
        if (objType.methods[expr.member]) {
          const sig = objType.methods[expr.member]!;
          return [{ kind: "function", params: sig.params.map(p => p.type), returns: sig.returns }, diags];
        }
      }
      diags.push({ code: "TANGLE_TYPE_UNKNOWN_FIELD", message: `Unknown member: ${expr.member}`, span: expr.span });
      return [{ kind: "primitive", name: "String" }, diags];
    }

    case "call": {
      const [calleeType, calleeDiags] = checkExpression(expr.callee, env);
      diags.push(...calleeDiags);
      const argTypes: Type[] = [];
      for (const arg of expr.args) {
        const [argType, argDiags] = checkExpression(arg, env);
        diags.push(...argDiags);
        argTypes.push(argType);
      }
      if (calleeType.kind === "function") {
        // 泛型推导：unify 参数类型，substitute 返回类型
        const subst: Substitution = new Map();
        for (let i = 0; i < calleeType.params.length && i < argTypes.length; i++) {
          const err = unify(calleeType.params[i]!, argTypes[i]!, subst);
          if (err) {
            diags.push({ code: "TANGLE_TYPE_ERROR", message: `Arg ${i + 1} type mismatch: ${err}`, span: expr.span });
          }
        }
        return [substitute(calleeType.returns, subst), diags];
      }
      if (calleeType.kind === "struct") {
        return [calleeType, diags];
      }
      return [{ kind: "any" }, diags];
    }

    case "binary": {
      const [leftType, leftDiags] = checkExpression(expr.left, env);
      const [rightType, rightDiags] = checkExpression(expr.right, env);
      diags.push(...leftDiags, ...rightDiags);
      if (["+", "-", "*", "/", "%"].includes(expr.op)) {
        if (!typesEqual(leftType, rightType)) {
          diags.push({ code: "TANGLE_TYPE_MISMATCH", message: `Operator ${expr.op} requires matching types`, span: expr.span });
        }
        return [{ kind: "primitive", name: "Int" }, diags];
      }
      return [{ kind: "primitive", name: "Bool" }, diags];
    }

    case "unary": {
      const [, operandDiags] = checkExpression(expr.operand, env);
      diags.push(...operandDiags);
      return [expr.op === "!" ? { kind: "primitive", name: "Bool" } : { kind: "primitive", name: "Int" }, diags];
    }

    case "recordUpdate": {
      const [objType, objDiags] = checkExpression(expr.object, env);
      diags.push(...objDiags);
      if (objType.kind === "any") {
        return [{ kind: "any" }, diags];
      }
      if (objType.kind === "struct") {
        for (const field of expr.fields) {
          if (!(field.name in objType.fields)) {
            diags.push({ code: "TANGLE_TYPE_UNKNOWN_FIELD", message: `Unknown field: ${field.name} on struct ${objType.name}`, span: field.span });
          } else {
            const [, valDiag] = checkExpression(field.value, env);
            diags.push(...valDiag);
          }
        }
        return [objType, diags];
      }
      diags.push({ code: "TANGLE_TYPE_NOT_STRUCT", message: "record update requires a struct type", span: expr.span });
      return [objType, diags];
    }

    case "pipe": {
      const [, leftDiags] = checkExpression(expr.left, env);
      diags.push(...leftDiags);
      const [rightType, rightDiags] = checkExpression(expr.right, env);
      diags.push(...rightDiags);
      return [rightType, diags];
    }

    case "if": {
      const [, condDiags] = checkExpression(expr.condition, env);
      diags.push(...condDiags);
      const [thenType, thenDiags] = checkExpression(expr.thenBranch, env);
      diags.push(...thenDiags);
      if (expr.elseBranch) {
        const [elseType, elseDiags] = checkExpression(expr.elseBranch, env);
        diags.push(...elseDiags);
        const unified = unifyPair(thenType, elseType);
        return [unified ?? thenType, diags];
      }
      return [thenType, diags];
    }

    case "arrow": {
      return [{ kind: "function", params: [], returns: { kind: "primitive", name: "Bool" } }, diags];
    }

    case "propagation": {
      const [innerType, innerDiags] = checkExpression(expr.expr, env);
      diags.push(...innerDiags);
      // Use a simple default: strip error variants
      if (innerType.kind === "sum") {
        // For now, simple heuristic: return first non-error variant
        // Full ErrorRegistry integration comes in checkModule.ts
        for (const v of innerType.variants) {
          const name = (v as { kind: string; name?: string }).name;
          if (name && !env.errorRegistry?.isError(name)) {
            return [v, diags];
          }
        }
      }
      return [innerType, diags];
    }

    case "match": {
      const [matchType, matchDiags] = checkExpression(expr.expr, env);
      diags.push(...matchDiags);
      // Type-check each arm body with narrowed binding type
      const armTypes: Type[] = [];
      for (const arm of expr.arms) {
        // 构造收窄后的 arm 局部环境
        const armEnv: TypeEnv = {
          ...env,
          variables: { ...env.variables },
        };
        if (matchType.kind === "sum" && arm.pattern.kind === "variantPattern") {
          const variant = findVariantByName(matchType, arm.pattern.name);
          if (variant && arm.pattern.binding) {
            const bindType = bindingTypeOf(variant);
            armEnv.variables[arm.pattern.binding] = bindType;
          }
        }
        const [armType, armDiags] = checkExpression(arm.body, armEnv);
        diags.push(...armDiags);
        armTypes.push(armType);
      }
      // Check exhaustiveness
      const patternNames = expr.arms.map((a) =>
        a.pattern.kind === "variantPattern" ? a.pattern.name : "_"
      );
      if (matchType.kind === "sum") {
        const missing = checkMatchExhaustiveness(matchType, patternNames);
        for (const m of missing) {
          diags.push({
            code: "TANGLE_TYPE_MATCH_NOT_EXHAUSTIVE",
            message: `Missing match arm for variant: ${m}`,
            span: expr.span,
          });
        }
      }
      // 返回所有 arm body 类型的统一结果（最佳努力，失败回退 Any）
      const resultType = unifyAll(armTypes) ?? { kind: "any" as const };
      return [resultType, diags];
    }

    case "panic": {
      const [, msgDiags] = checkExpression(expr.message, env);
      diags.push(...msgDiags);
      diags.push({
        code: "TANGLE_PANIC_REACHED",
        message: "panic: unrecoverable error",
        span: expr.span,
      });
      return [{ kind: "primitive", name: "Bool" }, diags];
    }

    case "destructure": {
      const [innerType, innerDiags] = checkExpression(expr.expr, env);
      diags.push(...innerDiags);
      if (innerType.kind !== "sum") {
        diags.push({
          code: "TANGLE_TYPE_NOT_SUM",
          message: "Destructure requires a sum type",
          span: expr.span,
        });
      }
      return [{ kind: "primitive", name: "Bool" }, diags];
    }

    default:
      return [{ kind: "primitive", name: "String" }, diags];
  }
}
