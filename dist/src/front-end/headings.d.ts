import type { HeadingRole } from "../model.js";
export declare function headingRoleForDepth(depth: number): HeadingRole;
export declare function parseHeadingText(text: string): {
    title: string;
    symbolName?: string;
};
