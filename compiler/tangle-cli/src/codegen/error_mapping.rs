pub fn wrap_ok(expr: &str) -> String {
    format!("Ok({})", expr)
}

pub fn wrap_err(variant: &str, expr: Option<&str>) -> String {
    match expr {
        Some(e) => format!("Err('{}', {})", variant, e),
        None => format!("Err('{}')", variant),
    }
}

pub fn unwrap_or_propagate(var_name: &str) -> String {
    format!("__tangle_propagate({})", var_name)
}
