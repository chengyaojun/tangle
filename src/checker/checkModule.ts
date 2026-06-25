import type { TangleModule, TangleDiagnostic } from "../model.js";
import type { ParsedCodeBlock } from "../ast.js";
import type { TypeEnv } from "./env.js";
import { tokenize } from "../parser/lexer.js";
import { parseCodeBody } from "../parser/parser.js";
import { resolveTypes } from "./resolve.js";
import { checkExpression } from "./check.js";
import { createEnv } from "./env.js";
import { ErrorRegistry } from "./errors.js";

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
  // Collect @error directives from all headings
  for (const heading of module.headings) {
    errorRegistry.collectFromDirectives(heading.directives);
  }

  for (const parsed of parsedBlocks) {
    const heading = module.headings.find(h => h.id === parsed.headingId);
    if (!heading) continue;

    const receiverName = extractReceiver(heading.title);
    const checkEnv = createEnv();
    checkEnv.structs = env.structs;
    checkEnv.interfaces = env.interfaces;
    // Attach errorRegistry to env for propagation checking
    checkEnv.errorRegistry = errorRegistry;

    if (receiverName) {
      const struct = env.structs[receiverName];
      if (struct) {
        checkEnv.receiver = { structName: receiverName, fields: struct.fields };
      }
    }

    // Add method params as variables
    for (const param of heading.params ?? []) {
      // Use default type for params without type annotations
      checkEnv.variables[param.name] = { kind: "primitive", name: "String" };
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

function extractReceiver(title: string): string | null {
  const match = title.match(/^(\w+)\s*->/);
  return match?.[1] ?? null;
}
