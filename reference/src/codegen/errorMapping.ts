// JS error result mapping strategy
export function wrapOk(expr: string): string {
  return `Ok(${expr})`;
}

export function wrapErr(variant: string, expr?: string): string {
  return `Err("${variant}"${expr ? ", " + expr : ""})`;
}

export function unwrapOrPropagate(varName: string): string {
  return `__tangle_propagate(${varName})`;
}
