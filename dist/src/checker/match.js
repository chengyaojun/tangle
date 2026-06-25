// checkMatchExhaustiveness: returns list of uncovered variant names
export function checkMatchExhaustiveness(sumType, armPatterns) {
    if (sumType.kind !== "sum")
        return [];
    const variantNames = sumType.variants.map((v) => getVariantName(v)).filter(Boolean);
    const covered = new Set();
    let hasWildcard = false;
    for (const pattern of armPatterns) {
        if (pattern === "_") {
            hasWildcard = true;
            break;
        }
        covered.add(pattern);
    }
    if (hasWildcard)
        return [];
    return variantNames.filter((n) => !covered.has(n));
}
function getVariantName(t) {
    if (t.kind === "struct")
        return t.name;
    if (t.kind === "primitive")
        return t.name;
    if (t.kind === "interface")
        return t.name;
    return null;
}
