import type { TangleModule } from "../model.js";
export type CompileModuleInput = {
    file: string;
    source: string;
};
export declare function compileModule(input: CompileModuleInput): TangleModule;
