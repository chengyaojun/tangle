// Panic always returns bottom type (never returns normally)
export function checkPanic() {
    // Bottom type — panic never returns
    return [{ kind: "primitive", name: "Bool" }, []];
}
// Check if a code path after panic is dead
export function isDeadPath(diagnostics) {
    return diagnostics.some((d) => d.code === "TANGLE_PANIC_REACHED");
}
