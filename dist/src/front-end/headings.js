export function headingRoleForDepth(depth) {
    switch (depth) {
        case 1:
            return "program";
        case 2:
            return "section";
        case 3:
            return "type";
        case 4:
            return "callable";
        case 5:
            return "semantic-section";
        case 6:
            return "semantic-atom";
        default:
            throw new Error(`Invalid Markdown heading depth: ${depth}`);
    }
}
export function parseHeadingText(text) {
    const match = text.match(/^(.*?)\s+\(([A-Za-z_][A-Za-z0-9_]*)\)\s*$/);
    if (!match || !match[1] || !match[2]) {
        return { title: text.trim() };
    }
    return {
        title: match[1].trim(),
        symbolName: match[2]
    };
}
