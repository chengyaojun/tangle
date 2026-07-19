import type { Expr, IsExpr } from "../ast.js";
import type { TangleDiagnostic } from "../model.js";
import type { Type, StructType } from "./types.js";
import type { TypeEnv } from "./env.js";
import { typesEqual } from "./types.js";
import { unify, substitute, type Substitution, unifyAll, unifyPair } from "./unify.js";
import { checkMatchExhaustiveness, findVariantByName, bindingTypeOf } from "./match.js";
import { asSumView, resolveStructInEnv } from "./optionView.js";

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
      // Phase 6d: 若 condition 是 IsExpr，在 then 分支注入收窄后的 binding 类型
      const thenEnv: TypeEnv = expr.condition.kind === "is"
        ? narrowEnvForIs(env, expr.condition)
        : env;
      const [thenType, thenDiags] = checkExpression(expr.thenBranch, thenEnv);
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
      // Phase 6d: 使用 asSumView 将 Option<T> 等 genericInstance 视作 Sum 进行收窄
      const sumView = asSumView(matchType);
      // Type-check each arm body with narrowed binding type
      const armTypes: Type[] = [];
      for (const arm of expr.arms) {
        // 构造收窄后的 arm 局部环境
        const armEnv: TypeEnv = {
          ...env,
          variables: { ...env.variables },
        };
        if (sumView && arm.pattern.kind === "variantPattern") {
          const variant = findVariantByName(sumView, arm.pattern.name);
          if (variant && arm.pattern.binding) {
            const rawTy = bindingTypeOf(variant);
            const bindTy = resolveStructInEnv(rawTy, env);
            armEnv.variables[arm.pattern.binding] = bindTy;
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
      if (sumView) {
        const missing = checkMatchExhaustiveness(sumView, patternNames);
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

    case "is": {
      // Phase 6d: IsExpr 类型收窄 —— `expr is Pattern`
      // 返回 Bool；若 scrutinee 不是 sum-compatible 类型则报 NOT_NARROWABLE；
      // 若 pattern 命名 variant 不存在则报 VARIANT_NOT_FOUND。
      const [matchedTy, innerDiags] = checkExpression(expr.expr, env);
      diags.push(...innerDiags);
      const resultTy: Type = { kind: "primitive", name: "Bool" };
      const sum = asSumView(matchedTy);
      if (sum) {
        if (expr.pattern.kind === "variant") {
          const variantTy = findVariantByName(sum, expr.pattern.name);
          if (!variantTy) {
            diags.push({
              code: "TANGLE_PATTERN_VARIANT_NOT_FOUND",
              message: `Variant '${expr.pattern.name}' not found in type`,
              span: expr.span,
            });
          }
        }
        // wildcard pattern 总是合法
      } else {
        diags.push({
          code: "TANGLE_PATTERN_NOT_NARROWABLE",
          message: "Cannot narrow type",
          span: expr.span,
        });
      }
      return [resultTy, diags];
    }

    default:
      return [{ kind: "primitive", name: "String" }, diags];
  }
}

/// Phase 6d: 为 IsExpr 构造收窄后的局部环境。
/// 若 IsExpr 形如 `x is Some(y)` 且 x 类型可视为 Sum（含 Option<T>），
/// 在 then 分支将 `y` 注入为 Some payload 类型（经 resolveStructInEnv 补全字段）。
function narrowEnvForIs(env: TypeEnv, isExpr: IsExpr): TypeEnv {
  const narrowed: TypeEnv = {
    ...env,
    variables: { ...env.variables },
  };
  const [matchedTy] = checkExpression(isExpr.expr, env);
  const sum = asSumView(matchedTy);
  if (sum && isExpr.pattern.kind === "variant" && isExpr.pattern.binding) {
    const variantTy = findVariantByName(sum, isExpr.pattern.name);
    if (variantTy) {
      const rawTy = bindingTypeOf(variantTy);
      const bindTy = resolveStructInEnv(rawTy, env);
      narrowed.variables[isExpr.pattern.binding] = bindTy;
    }
  }
  return narrowed;
}
