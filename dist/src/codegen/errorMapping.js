// JS error result mapping strategy
export function wrapOk(expr) {
    return `Ok(${expr})`;
}
export function wrapErr(variant, expr) {
    return `Err("${variant}"${expr ? ", " + expr : ""})`;
}
export function unwrapOrPropagate(varName) {
    return `__tangle_propagate(${varName})`;
}
