import type { MarkdownNode } from "../markdown/parseMarkdown.js";
import type { SourceSpan } from "../model.js";
export declare function spanFromNode(file: string, node: MarkdownNode): SourceSpan;
