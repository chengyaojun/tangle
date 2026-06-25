import type { MarkdownNode, MarkdownRoot } from "../markdown/parseMarkdown.js";
import type { SourceSpan, TangleImport, TangleParam } from "../model.js";
export declare function collectLinks(file: string, root: MarkdownRoot): TangleImport[];
export declare function parseParamItem(text: string, span: SourceSpan): TangleParam;
export declare function isTangleCodeBlock(node: Pick<MarkdownNode, "type" | "lang">): boolean;
export declare function plainText(node: MarkdownNode): string;
