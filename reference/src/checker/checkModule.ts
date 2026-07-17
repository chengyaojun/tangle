import type { TangleModule, TangleDiagnostic } from "../model.js";
import type { ParsedCodeBlock } from "../ast.js";
import type { TypeEnv } from "./env.js";
import { tokenize } from "../parser/lexer.js";
import { parseCodeBody } from "../parser/parser.js";
import { resolveTypes, findReceiverHeading, typeExprToType } from "./resolve.js";
import { checkExpression } from "./check.js";
import { createEnv } from "./env.js";
import { ErrorRegistry } from "./errors.js";
import { parseTypeExpr } from "../parser/typeParser.js";
import { registerBuiltins } from "./builtins.js";

export type CheckedModule = TangleModule & {
  parsedBlocks: ParsedCodeBlock[];
  typeEnv: TypeEnv;
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
          let paramType = typeExprToType(te);
          // If it's a struct name, look up full definition from env (with fields/methods)
          if (paramType.kind === "struct") {
            const fullStruct = env.structs[paramType.name];
            if (fullStruct) paramType = fullStruct;
          }
          checkEnv.variables[param.name] = paramType;
        } catch {
          checkEnv.variables[param.name] = { kind: "any" };
        }
      } else {
        checkEnv.variables[param.name] = { kind: "any" };
      }
    }

    for (const stmt of parsed.body.statements) {
      if (stmt.kind === "expression") {
        const [, diags] = checkExpression(stmt.expr, checkEnv);
        allDiagnostics.push(...diags);
      } else if (stmt.kind === "return" && stmt.value) {
        const [, diags] = checkExpression(stmt.value, checkEnv);
        allDiagnostics.push(...diags);
      } else if (stmt.kind === "let" || stmt.kind === "const") {
        const [type, diags] = checkExpression(stmt.value, checkEnv);
        allDiagnostics.push(...diags);
        if (diags.length === 0) {
          checkEnv.variables[stmt.name] = type;
        }
      }
    }
  }

  return {
    ...module,
    parsedBlocks,
    typeEnv: env,
    diagnostics: allDiagnostics
  };
}
