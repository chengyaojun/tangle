import type { TangleModule, TangleHeading } from "../model.js";
import type { Type } from "./types.js";
import type { TypeEnv } from "./env.js";
import type { StructType, InterfaceType, CallableSignature } from "./types.js";
import { createEnv } from "./env.js";
import { parseTypeExpr } from "../parser/typeParser.js";
import { builtinTypes } from "./builtins.js";
import type { TypeExpr } from "../ast.js";

export function resolveTypes(module: TangleModule): TypeEnv {
  const env = createEnv();

  // Pass 1: collect type headings (depth 3)
  for (const heading of module.headings) {
    if (heading.role !== "type") continue;

    const isInterface =
      heading.title.includes("(接口)") ||
      heading.title.includes("(interface)");
    const typeName = extractTypeName(heading.title);

    if (isInterface) {
      env.interfaces[typeName] = {
        kind: "interface",
        name: typeName,
        methods: {},
      };
    } else {
      const struct: StructType = {
        kind: "struct",
        name: typeName,
        fields: {},
        methods: {},
      };

      for (const param of heading.params ?? []) {
        if (param.typeName) {
          try {
            const te = parseTypeExpr(param.typeName, param.span.file);
            struct.fields[param.name] = typeExprToType(te);
          } catch {
            // skip unresolvable type
          }
        }
      }

      env.structs[typeName] = struct;
    }
  }

  // Pass 2: collect callable headings as methods (implicit binding via heading tree)
  for (const heading of module.headings) {
    if (heading.role !== "callable") continue;

    const parentHeading = findReceiverHeading(heading, module.headings);
    if (!parentHeading) continue;

    const receiver = extractTypeName(parentHeading.title);
    const signature = buildCallableSignature(heading);

    if (env.structs[receiver]) {
      const methodName = extractMethodName(heading);
      env.structs[receiver]!.methods[methodName] = signature;
    } else if (env.interfaces[receiver]) {
      const methodName = extractMethodName(heading);
      env.interfaces[receiver]!.methods[methodName] = signature;
    }
  }

  return env;
}

function extractTypeName(title: string): string {
  return title.replace(/\s*\(.*\)\s*$/, "").trim();
}

export function findReceiverHeading(heading: TangleHeading, allHeadings: TangleHeading[]): TangleHeading | null {
  function findInTree(root: TangleHeading, target: TangleHeading): TangleHeading | null {
    if (root.children.includes(target)) return root;
    for (const child of root.children) {
      const found = findInTree(child, target);
      if (found) return found;
    }
    return null;
  }

  for (const root of allHeadings) {
    if (root === heading) return null;
    const found = findInTree(root, heading);
    if (found) return found;
  }
  return null;
}

function extractMethodName(heading: TangleHeading): string {
  return heading.symbolName ?? heading.title;
}

function buildCallableSignature(heading: TangleHeading): CallableSignature {
  const params: { name: string; type: Type }[] = [];
  for (const param of heading.params ?? []) {
    let type: Type;
    if (param.typeName) {
      try {
        type = typeExprToType(parseTypeExpr(param.typeName, param.span.file));
      } catch {
        type = { kind: "primitive", name: "String" };
      }
    } else {
      type = { kind: "primitive", name: "String" };
    }
    params.push({ name: param.name, type });
  }
  return { params, returns: { kind: "any" } };
}

export function typeExprToType(te: TypeExpr): Type {
  switch (te.kind) {
    case "primitiveType": {
      const builtin = builtinTypes[te.name];
      if (builtin) return builtin;
      return { kind: "primitive", name: te.name };
    }
    case "namedType":
      return { kind: "struct", name: te.name, fields: {}, methods: {} };
    case "sumType":
      return { kind: "sum", variants: te.variants.map(typeExprToType) };
    case "genericType":
      return {
        kind: "genericInstance",
        base: te.base,
        args: te.typeArgs.map(typeExprToType),
      };
    case "functionType":
      return {
        kind: "function",
        params: te.params.map(typeExprToType),
        returns: typeExprToType(te.returns),
      };
  }
}
