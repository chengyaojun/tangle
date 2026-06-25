import type { Type } from "./types.js";
import type { TangleDiagnostic } from "../model.js";
import type { ErrorRegistry } from "./errors.js";
export declare function checkPropagation(type: Type, errorRegistry: ErrorRegistry): [Type, TangleDiagnostic[]];
