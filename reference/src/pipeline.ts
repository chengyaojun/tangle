import { compileModule } from "./front-end/compileModule.js";
import { checkModule } from "./checker/checkModule.js";
import { compileToIR } from "./ir/compileToIR.js";
import { emitJS } from "./codegen/jsEmitter.js";
import type { TangleDiagnostic } from "./model.js";
import type { RuleGraph } from "./ir/graph.js";

export type CompileResult = {
  js: string;
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
};

export function compile(source: string, file: string): CompileResult {
  const tangleModule = compileModule({ file, source });
  const checked = checkModule(tangleModule);
  const { graph, diagnostics } = compileToIR(checked);
  const js = emitJS(graph, tangleModule.moduleName);
  return { js, graph, diagnostics };
}
