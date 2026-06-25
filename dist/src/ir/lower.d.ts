import type { Stmt } from "../ast.js";
import type { RuleGraph } from "./graph.js";
export declare function lowerStatements(stmts: Stmt[], file: string): RuleGraph;
