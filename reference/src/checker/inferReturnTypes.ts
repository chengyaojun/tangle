import type { TangleHeading, TangleParam } from "../model.js";
import type { Type } from "./types.js";
import type { TypeEnv } from "./env.js";
import type { ParsedCodeBlock } from "../ast.js";
import { checkExpression } from "./check.js";
import { findReceiverHeading, typeNameToType } from "./resolve.js";
import { unifyAll } from "./unify.js";

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
    for (const stmt of block.body.statements) {
      if (stmt.kind === "let" || stmt.kind === "const") {
        const [ty] = checkExpression(stmt.value, blockEnv);
        blockEnv.variables[stmt.name] = ty;
      } else if (stmt.kind === "return" && stmt.value) {
        const [ty] = checkExpression(stmt.value, blockEnv);
        returnTypes.push(ty);
      }
    }
  }

  // 3. 统一所有 return 类型
  if (returnTypes.length === 0) return null;
  return unifyAll(returnTypes) ?? { kind: "any" };
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
