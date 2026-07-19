import type { TangleModule, TangleDiagnostic } from "../model.js";
import type { ParsedCodeBlock, Stmt } from "../ast.js";
import type { TypeEnv } from "./env.js";
import type { Type } from "./types.js";
import { tokenize } from "../parser/lexer.js";
import { parseCodeBody } from "../parser/parser.js";
import { resolveTypes, findReceiverHeading, typeExprToType } from "./resolve.js";
import { checkExpression } from "./check.js";
import { createEnv } from "./env.js";
import { ErrorRegistry } from "./errors.js";
import { parseTypeExpr } from "../parser/typeParser.js";
import { registerBuiltins } from "./builtins.js";
import { inferReturnTypes } from "./inferReturnTypes.js";
import { asSumView, resolveStructInEnv } from "./optionView.js";
import { findVariantByName, bindingTypeOf } from "./match.js";

export type CheckedModule = TangleModule & {
  parsedBlocks: ParsedCodeBlock[];
  typeEnv: TypeEnv;
  returnTypes: Map<string, Type>;
};

export function parseCodeBlocks(module: TangleModule): ParsedCodeBlock[] {
  const parsed: ParsedCodeBlock[] = [];
  for (const heading of module.headings) {
    for (const block of heading.codeBlocks ?? []) {
      const tokens = tokenize(block.value, block.span.file);
      const body = parseCodeBody(tokens);
      parsed.push({
        headingId: heading.id,
        source: block.value,
        body,
        diagnostics: []
      });
    }
  }
  return parsed;
}

export function checkModule(module: TangleModule): CheckedModule {
  const parsedBlocks = parseCodeBlocks(module);
  const env = resolveTypes(module);
  const allDiagnostics: TangleDiagnostic[] = [...module.diagnostics];

  const errorRegistry = new ErrorRegistry();
  errorRegistry.collectFromHeadings(module.headings);

  for (const parsed of parsedBlocks) {
    const heading = module.headings.find(h => h.id === parsed.headingId);
    if (!heading) continue;

    const parentHeading = findReceiverHeading(heading, module.headings);
    const receiverName = parentHeading ? parentHeading.title.replace(/\s*\(.*\)\s*$/, "").trim() : null;
    const checkEnv = createEnv();
    checkEnv.structs = env.structs;
    checkEnv.interfaces = env.interfaces;
    registerBuiltins(checkEnv);
    // Attach errorRegistry to env for propagation checking
    checkEnv.errorRegistry = errorRegistry;

    if (receiverName) {
      const struct = env.structs[receiverName];
      if (struct) {
        checkEnv.receiver = { structName: receiverName, fields: struct.fields };
      }
    }

    // Add method params as variables (resolve type from typeName annotation)
    for (const param of heading.params ?? []) {
      if (param.typeName) {
        try {
          const te = parseTypeExpr(param.typeName, param.span.file);
          const paramType = typeExprToType(te);
          // Phase 6d: 使用 resolveStructInEnv 补全结构体字段（镜像 Rust 修复）
          // 背景：typeExprToType 解析 `Item` 时返回空字段外壳 Struct { name: "Item", fields: {} }。
          // 真实带字段的定义位于 env.structs。此处统一通过 resolveStructInEnv 补全，
          // 修复 examples/account.tangle.md 风格的“方法参数结构体字段为空”的 bug。
          checkEnv.variables[param.name] = resolveStructInEnv(paramType, env);
        } catch {
          checkEnv.variables[param.name] = { kind: "any" };
        }
      } else {
        checkEnv.variables[param.name] = { kind: "any" };
      }
    }

    for (const stmt of parsed.body.statements) {
      checkStmt(stmt, checkEnv, allDiagnostics);
    }
  }

  const returnTypes = inferReturnTypes({
    headings: module.headings,
    parsedBlocks,
    typeEnv: env,
  });

  return {
    ...module,
    parsedBlocks,
    typeEnv: env,
    returnTypes,
    diagnostics: allDiagnostics
  };
}

/// Phase 6d: 抽取的语句检查函数。
/// - 处理原有 expression / return / let / const
/// - 新增 letVariant / letRecord（refutable let / record destructuring）
/// 镜像 Rust crate::checker::check_module::check_stmt。
export function checkStmt(stmt: Stmt, env: TypeEnv, diags: TangleDiagnostic[]): void {
  switch (stmt.kind) {
    case "expression": {
      const [, d] = checkExpression(stmt.expr, env);
      diags.push(...d);
      break;
    }
    case "return": {
      if (stmt.value) {
        const [, d] = checkExpression(stmt.value, env);
        diags.push(...d);
      }
      break;
    }
    case "let":
    case "const": {
      const [type, d] = checkExpression(stmt.value, env);
      diags.push(...d);
      if (d.length === 0) {
        env.variables[stmt.name] = type;
      }
      break;
    }
    case "letVariant": {
      // Phase 6d: `let Variant(binding) = expr else { ... }`
      const [matchedTy, d] = checkExpression(stmt.expr, env);
      diags.push(...d);
      const sum = asSumView(matchedTy);
      if (sum) {
        const variantTy = findVariantByName(sum, stmt.variantName);
        if (variantTy) {
          if (stmt.binding) {
            const rawTy = bindingTypeOf(variantTy);
            const bindTy = resolveStructInEnv(rawTy, env);
            env.variables[stmt.binding] = bindTy;
          }
          for (const s of stmt.elseBranch) checkStmt(s, env, diags);
        } else {
          diags.push({
            code: "TANGLE_PATTERN_VARIANT_NOT_FOUND",
            message: `Variant '${stmt.variantName}' not found in type`,
            span: stmt.span,
          });
        }
      } else {
        diags.push({
          code: "TANGLE_PATTERN_NOT_NARROWABLE",
          message: "Cannot destructure type",
          span: stmt.span,
        });
      }
      break;
    }
    case "letRecord": {
      // Phase 6d: `let { field: local, ... } = expr`
      const [matchedTy, d] = checkExpression(stmt.expr, env);
      diags.push(...d);
      switch (matchedTy.kind) {
        case "struct":
          for (const [field, local] of stmt.fields) {
            const fieldTy = matchedTy.fields[field];
            if (fieldTy) {
              env.variables[local] = fieldTy;
            } else {
              diags.push({
                code: "TANGLE_STRUCT_FIELD_NOT_FOUND",
                message: `Struct ${matchedTy.name} has no field '${field}'`,
                span: stmt.span,
              });
            }
          }
          break;
        case "any":
          for (const [, local] of stmt.fields) {
            env.variables[local] = { kind: "any" };
          }
          break;
        default:
          diags.push({
            code: "TANGLE_DESTRUCTURE_NOT_STRUCT",
            message: "Cannot destructure as record (expected struct)",
            span: stmt.span,
          });
      }
      break;
    }
  }
}
