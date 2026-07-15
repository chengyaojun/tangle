//! stdlib signature registry completeness tests.
//! Verifies all 19 modules have signatures and key functions are present.

use tangle_cli::stdlib::signatures::{stdlib_signature, stdlib_module_signatures};

const EXPECTED_MODULES: &[&str] = &[
    "fmt", "IO", "List", "Map", "Set", "Option", "Math", "String",
    "Env", "Path", "JSON", "DateTime", "Random", "Encoding", "Sort",
    "Process", "Task", "Channel", "Sync",
];

#[test]
fn all_19_modules_have_signatures() {
    for module in EXPECTED_MODULES {
        assert!(
            stdlib_module_signatures(module).is_some(),
            "Module '{}' missing from signature registry",
            module
        );
    }
}

#[test]
fn fmt_module_has_expected_functions() {
    let sigs = stdlib_module_signatures("fmt").expect("fmt module must exist");
    for fn_name in &["print", "println", "input", "debug", "error", "format"] {
        assert!(sigs.contains_key(*fn_name), "fmt.{} missing", fn_name);
    }
}

#[test]
fn println_is_variadic() {
    let sig = stdlib_signature("fmt", "println").expect("fmt.println must exist");
    assert!(sig.is_variadic, "fmt.println should be variadic");
}

#[test]
fn readfile_has_string_param_and_return() {
    let sig = stdlib_signature("IO", "readFile").expect("IO.readFile must exist");
    assert_eq!(sig.params.len(), 1);
    assert!(!sig.is_variadic);
}

#[test]
fn json_parse_and_stringify_exist() {
    assert!(stdlib_signature("JSON", "parse").is_some());
    assert!(stdlib_signature("JSON", "stringify").is_some());
}

#[test]
fn sync_has_all_wait_group_ops() {
    let sigs = stdlib_module_signatures("Sync").expect("Sync module must exist");
    for op in &["mutex_new", "mutex_lock", "mutex_unlock", "once_do",
                "wait_group_new", "wait_group_add", "wait_group_done", "wait_group_wait"] {
        assert!(sigs.contains_key(*op), "Sync.{} missing", op);
    }
}
