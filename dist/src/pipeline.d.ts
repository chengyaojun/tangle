import type { TangleDiagnostic } from "./model.js";
export type CompileResult = {
    js: string;
    diagnostics: TangleDiagnostic[];
};
export declare function compile(source: string, file: string): CompileResult;
