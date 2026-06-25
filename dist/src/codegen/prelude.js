// JS runtime prelude — helper functions emitted alongside generated code
export const RUNTIME_PRELUDE = `
// Tangle Runtime Prelude
function __tangle_struct(fields) { return Object.freeze({ ...fields }); }
function __tangle_with(obj, updates) { return Object.freeze({ ...obj, ...updates }); }
function Ok(value) { return { ok: true, value }; }
function Err(variant, value) { return { ok: false, error: { variant, value } }; }
function __tangle_propagate(result) { if (!result.ok) return result; return result.value; }
function __tangle_match(result, handlers) {
  if (result.ok) {
    if (handlers._) return handlers._(result.value);
    throw new Error("Unexpected Ok in match");
  }
  const h = handlers[result.error.variant];
  if (h) return h(result.error.value);
  if (handlers._) return handlers._(result);
  throw new Error("Non-exhaustive match: " + result.error.variant);
}
`.trim();
