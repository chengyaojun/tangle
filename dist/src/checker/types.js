export function typesEqual(a, b) {
    if (a.kind !== b.kind)
        return false;
    if (a.kind === "primitive" && b.kind === "primitive")
        return a.name === b.name;
    if (a.kind === "struct" && b.kind === "struct")
        return a.name === b.name;
    if (a.kind === "interface" && b.kind === "interface")
        return a.name === b.name;
    return false;
}
export function isSubtype(value, target) {
    if (target.kind === "interface" && value.kind === "struct") {
        return Object.entries(target.methods).every(([name, sig]) => {
            const ms = value.methods[name];
            if (!ms)
                return false;
            if (ms.params.length !== sig.params.length)
                return false;
            return ms.params.every((p, i) => typesEqual(p.type, sig.params[i].type))
                && typesEqual(ms.returns, sig.returns);
        });
    }
    return typesEqual(value, target);
}
