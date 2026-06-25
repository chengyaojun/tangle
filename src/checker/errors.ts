import type { TangleDirective, SourceSpan } from "../model.js";
import type { Type } from "./types.js";

export type ErrorVariant = {
  name: string;
  fields: Record<string, Type>;
  span: SourceSpan;
};

export class ErrorRegistry {
  private variants: Map<string, ErrorVariant> = new Map();

  register(name: string, fields: Record<string, Type>, span?: SourceSpan): void {
    this.variants.set(name, {
      name,
      fields,
      span: span ?? { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 },
    });
  }

  lookup(name: string): ErrorVariant | undefined {
    return this.variants.get(name);
  }

  isError(name: string): boolean {
    return this.variants.has(name);
  }

  collectFromDirectives(directives: TangleDirective[]): void {
    for (const d of directives) {
      if (d.kind === "error" && d.name) {
        const fields: Record<string, Type> = {};
        if (d.args) {
          const parts = d.args.split(",").map((s) => s.trim());
          for (const part of parts) {
            const colonIdx = part.lastIndexOf(":");
            if (colonIdx > 0) {
              const fieldName = part
                .slice(0, colonIdx)
                .trim()
                .replace(/^["']|["']$/g, "");
              const typeName = part.slice(colonIdx + 1).trim();
              fields[fieldName] = typeNameToPrimitive(typeName);
            }
          }
        }
        this.register(d.name, fields, d.span);
      }
    }
  }

  allVariants(): ErrorVariant[] {
    return Array.from(this.variants.values());
  }
}

function typeNameToPrimitive(name: string): Type {
  if (name === "String" || name === "Int" || name === "Bool") {
    return { kind: "primitive", name } as Type;
  }
  return { kind: "struct", name, fields: {}, methods: {} } as Type;
}
