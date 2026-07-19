import type { TangleHeading, TangleParam } from "../model.js";
import type { Type } from "./types.js";
import type { TypeEnv } from "./env.js";
import type { ParsedCodeBlock, Stmt } from "../ast.js";
import { checkExpression } from "./check.js";
import { findReceiverHeading, typeNameToType } from "./resolve.js";
import { unifyAll } from "./unify.js";
import { asSumView, resolveStructInEnv } from "./optionView.js";
import { bindingTypeOf, findVariantByName } from "./match.js";

export interface ReturnInferenceInput {
  headings: TangleHeading[];
  parsedBlocks: ParsedCodeBlock[];
  typeEnv: TypeEnv;
}

/// 为模块中所有 Callable heading 推断返回类型。
/// 返回 heading_id → Type 映射。
export function inferReturnTypes(input: ReturnInferenceInput): Map<string, Type> {
  const result = new Map<string, Type>();
  collect(input.headings, input, result);
  return result;
}

function collect(headings: TangleHeading[], input: ReturnInferenceInput, out: Map<string, Type>): void {
  for (const h of headings) {
    if (h.role === "callable" && (h.codeBlocks ?? []).length > 0) {
      const ty = inferFunctionReturnType(h, input);
      if (ty) {
        out.set(h.id, ty);
      }
    }
    collect(h.children, input, out);
  }
}

function inferFunctionReturnType(
  heading: TangleHeading,
  input: ReturnInferenceInput,
): Type | null {
  // 1. 构造与 checkModule 一致的 env（克隆 typeEnv 以避免污染）
  const env: TypeEnv = {
    ...input.typeEnv,
    variables: { ...input.typeEnv.variables },
  };
  setupReceiverAndParams(heading, input, env);

  // 2. 遍历该 heading 的所有 @tangle blocks，收集 return 类型
  const returnTypes: Type[] = [];
  for (const block of input.parsedBlocks) {
    if (block.headingId !== heading.id) continue;
    const blockEnv: TypeEnv = {
      ...env,
      variables: { ...env.variables },
    };
    collectReturnsFromStmts(block.body.statements, blockEnv, returnTypes);
  }

  // 3. 统一所有 return 类型
  if (returnTypes.length === 0) return null;
  return unifyAll(returnTypes) ?? { kind: "any" };
}

/// 遍历语句列表，在类型环境中累积 binding 类型，并收集所有 `return` 表达式的类型。
/// Mirrors Rust's collect_returns_from_stmts in infer_return_types.rs.
/// 处理 `let` / `const` / `return` / `expression` 以及 Phase 6d 新增的
/// `letVariant`（refutable 变体解构）与 `letRecord`（record 解构）。
/// `letVariant` 的 else 分支会被递归处理，以便收集其中的 `return` 类型。
function collectReturnsFromStmts(
  stmts: Stmt[],
  env: TypeEnv,
  returnTypes: Type[],
): void {
  for (const stmt of stmts) {
    switch (stmt.kind) {
      case "let":
      case "const": {
        const [ty] = checkExpression(stmt.value, env);
        env.variables[stmt.name] = ty;
        break;
      }
      case "return": {
        if (stmt.value) {
          const [ty] = checkExpression(stmt.value, env);
          returnTypes.push(ty);
        }
        break;
      }
      case "expression":
        break;
      case "letVariant": {
        const [matchedTy] = checkExpression(stmt.expr, env);
        const sum = asSumView(matchedTy);
        if (sum) {
          const variantTy = findVariantByName(sum, stmt.variantName);
          if (variantTy) {
            if (stmt.binding) {
              const rawTy = bindingTypeOf(variantTy);
              const bindTy = resolveStructInEnv(rawTy, env);
              env.variables[stmt.binding] = bindTy;
            }
            // Recurse into else branch to collect its return types
            collectReturnsFromStmts(stmt.elseBranch, env, returnTypes);
          }
        }
        break;
      }
      case "letRecord": {
        const [matchedTy] = checkExpression(stmt.expr, env);
        if (matchedTy.kind === "struct") {
          for (const [fieldName, localVar] of stmt.fields) {
            const fieldTy = matchedTy.fields[fieldName];
            if (fieldTy) {
              env.variables[localVar] = fieldTy;
            }
          }
        } else if (matchedTy.kind === "any") {
          for (const [, localVar] of stmt.fields) {
            env.variables[localVar] = { kind: "any" };
          }
        }
        break;
      }
    }
  }
}

function setupReceiverAndParams(
  heading: TangleHeading,
  input: ReturnInferenceInput,
  env: TypeEnv,
): void {
  const parent = findReceiverHeading(heading, input.headings);
  if (parent) {
    // 与 checkModule 一致：strip (接口)/(interface) 等后缀
    const structName = parent.title.replace(/\s*\(.*\)\s*$/, "").trim();
    // 优先使用 typeEnv 中已解析的完整 struct（含 fields/methods）
    const fullStruct = input.typeEnv.structs[structName];
    if (fullStruct) {
      env.receiver = { structName, fields: { ...fullStruct.fields } };
    } else {
      // 回退：从 parent.params 构造字段
      const fields: Record<string, Type> = {};
      for (const p of parent.params ?? []) {
        fields[p.name] = paramTypeOf(p);
      }
      env.receiver = { structName, fields };
    }
  }
  for (const p of heading.params ?? []) {
    env.variables[p.name] = paramTypeOf(p);
  }
}

function paramTypeOf(p: TangleParam): Type {
  if (!p.typeName) return { kind: "any" };
  const ty = typeNameToType(p.typeName);
  return ty ?? { kind: "any" };
}
