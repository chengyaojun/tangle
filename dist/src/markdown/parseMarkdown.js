import { unified } from "unified";
import remarkParse from "remark-parse";
export function parseMarkdown(source) {
    return unified().use(remarkParse).parse(source);
}
