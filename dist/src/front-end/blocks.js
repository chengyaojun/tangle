import { spanFromNode } from "./sourceMap.js";
export function collectLinks(file, root) {
    const imports = [];
    function walk(node) {
        if (node.type === "link" && node.url) {
            const alias = plainText(node).trim();
            if (alias && node.url.endsWith(".md")) {
                imports.push({ alias, target: node.url, span: spanFromNode(file, node) });
            }
        }
        for (const child of node.children ?? []) {
            walk(child);
        }
    }
    walk(root);
    return imports;
}
export function parseParamItem(text, span) {
    const match = text.match(/^`([^`]+)`:\s*(.*?)(?:\s+\(([^)]+)\))?$/);
    if (!match || !match[1] || !match[2]) {
        throw new Error(`Invalid Tangle parameter item: ${text}`);
    }
    const result = {
        name: match[1],
        description: match[2].trim(),
        span
    };
    if (match[3]) {
        result.typeName = match[3];
    }
    return result;
}
export function isTangleCodeBlock(node) {
    return node.type === "code" && node.lang === "@tangle";
}
export function plainText(node) {
    if (typeof node.value === "string") {
        return node.value;
    }
    return (node.children ?? []).map(plainText).join("");
}
