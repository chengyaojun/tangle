export class ErrorRegistry {
    variants = new Map();
    register(name, fields, span) {
        this.variants.set(name, {
            name,
            fields,
            span: span ?? { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 },
        });
    }
    lookup(name) {
        return this.variants.get(name);
    }
    isError(name) {
        return this.variants.has(name);
    }
    collectFromDirectives(directives) {
        for (const d of directives) {
            if (d.kind === "error" && d.name) {
                const fields = {};
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
    allVariants() {
        return Array.from(this.variants.values());
    }
}
function typeNameToPrimitive(name) {
    if (name === "String" || name === "Int" || name === "Bool") {
        return { kind: "primitive", name };
    }
    return { kind: "struct", name, fields: {}, methods: {} };
}
