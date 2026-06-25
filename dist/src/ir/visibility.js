// Check that all symbols referenced in the IR are visible (exported or module-internal)
export function checkIRVisibility(graph, exportedSymbols) {
    const diags = [];
    // For now, all nodes pass visibility check since cross-module references aren't yet tracked
    // Future: check that each IRNode's label references only visible symbols
    return diags;
}
