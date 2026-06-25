import type { TangleModule } from "../model.js";
import type { ParsedCodeBlock } from "../ast.js";
import type { TypeEnv } from "./env.js";
export type CheckedModule = TangleModule & {
    parsedBlocks: ParsedCodeBlock[];
    typeEnv: TypeEnv;
};
export declare function parseCodeBlocks(module: TangleModule): ParsedCodeBlock[];
export declare function checkModule(module: TangleModule): CheckedModule;
