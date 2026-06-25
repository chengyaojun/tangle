export type MarkdownNode = {
    type: string;
    children?: MarkdownNode[];
    value?: string;
    depth?: number;
    lang?: string;
    url?: string;
    position?: {
        start: {
            line: number;
            column: number;
        };
        end: {
            line: number;
            column: number;
        };
    };
};
export type MarkdownRoot = MarkdownNode & {
    type: "root";
    children: MarkdownNode[];
};
export declare function parseMarkdown(source: string): MarkdownRoot;
