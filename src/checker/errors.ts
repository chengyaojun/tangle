import type { SourceSpan } from "../model.js";
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

  collectFromHeadings(headings: { title: string }[]): void {
    for (const h of headings) {
      const match = h.title.match(/^Error:\s*(.+)$/);
      if (match) {
        const rest = match[1]!.trim();
        const symMatch = rest.match(/\(([A-Za-z_][A-Za-z0-9_]*)\)/);
        this.register(symMatch ? symMatch[1]! : rest, {});
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
