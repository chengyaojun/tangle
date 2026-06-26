pub const RUNTIME_PRELUDE: &str = r#"
// Tangle Runtime Prelude
function __tangle_struct(obj) { return Object.freeze(obj); }
function __tangle_update(obj, updates) { return Object.freeze(Object.assign({}, obj, updates)); }
function Ok(value) { return { ok: true, value: value }; }
function Err(variant, value) { return { ok: false, error: variant, value: value }; }
function __tangle_propagate(result) { if (!result.ok) return result; return result.value; }
function __tangle_match(value, patterns) {
    for (const [pattern, handler] of patterns) {
        if (pattern === '_' || pattern === value.error) return handler(value);
    }
    throw new Error('Match not exhaustive: ' + value.error);
}
"#;
