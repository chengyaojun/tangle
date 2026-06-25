import { compileModule } from "./front-end/compileModule.js";
import { checkModule } from "./checker/checkModule.js";
import { compileToIR } from "./ir/compileToIR.js";
import { emitJS } from "./codegen/jsEmitter.js";
export function compile(source, file) {
    const tangleModule = compileModule({ file, source });
    const checked = checkModule(tangleModule);
    const { graph, diagnostics } = compileToIR(checked);
    const js = emitJS(graph, tangleModule.moduleName);
    return { js, diagnostics };
}
