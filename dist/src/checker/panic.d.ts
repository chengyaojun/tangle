import type { Type } from "./types.js";
import type { TangleDiagnostic } from "../model.js";
export declare function checkPanic(): [Type, TangleDiagnostic[]];
export declare function isDeadPath(diagnostics: TangleDiagnostic[]): boolean;
