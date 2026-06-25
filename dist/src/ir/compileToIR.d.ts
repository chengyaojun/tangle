import type { CheckedModule } from "../checker/checkModule.js";
import type { RuleGraph } from "./graph.js";
import type { TangleDiagnostic } from "../model.js";
export declare function compileToIR(checked: CheckedModule): {
    graph: RuleGraph;
    diagnostics: TangleDiagnostic[];
};
