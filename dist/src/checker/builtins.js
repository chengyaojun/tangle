export const builtinTypes = {
    String: { kind: "primitive", name: "String" },
    Int: { kind: "primitive", name: "Int" },
    Bool: { kind: "primitive", name: "Bool" }
};
export function isBuiltinType(name) {
    return name in builtinTypes;
}
