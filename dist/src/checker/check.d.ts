import type { Expr } from "../ast.js";
import type { TangleDiagnostic } from "../model.js";
import type { Type } from "./types.js";
import type { TypeEnv } from "./env.js";
export declare function checkExpression(expr: Expr, env: TypeEnv): [Type, TangleDiagnostic[]];
