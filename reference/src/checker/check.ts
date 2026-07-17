import type { Expr } from "../ast.js";
import type { TangleDiagnostic } from "../model.js";
import type { Type, StructType } from "./types.js";
import type { TypeEnv } from "./env.js";
import { typesEqual } from "./types.js";
import { checkMatchExhaustiveness } from "./match.js";

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
      for (const arg of expr.args) {
        const [, argDiags] = checkExpression(arg, env);
        diags.push(...argDiags);
      }
      if (calleeType.kind === "function") {
        return [calleeType.returns, diags];
      }
      return [{ kind: "primitive", name: "Bool" }, diags];
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
        const [, elseDiags] = checkExpression(expr.elseBranch, env);
        diags.push(...elseDiags);
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
      // Type-check each arm body
      let resultType: Type = { kind: "primitive", name: "Bool" };
      for (const arm of expr.arms) {
        const [armType, armDiags] = checkExpression(arm.body, env);
        diags.push(...armDiags);
        resultType = armType; // last arm type wins (simplified)
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
