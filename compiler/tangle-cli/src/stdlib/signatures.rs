//! Static signature registry for all 19 stdlib modules.
//! Replaces the dummy signatures in stdlib_ops() with real type information.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::checker::types::{CallableSignature, FunctionType, PrimitiveType, Type, generic, type_var};

// --- type helpers ---

fn prim(name: &str) -> Type {
    Type::Primitive(PrimitiveType { name: name.into() })
}

fn str_t() -> Type { prim("String") }
fn int_t() -> Type { prim("Int") }
fn bool_t() -> Type { prim("Bool") }
fn void_t() -> Type { prim("Void") }
fn any_t() -> Type { Type::Any }

// --- signature helpers ---

fn sig_fixed(params: &[(&str, Type)], returns: Type) -> CallableSignature {
    CallableSignature {
        params: params.iter().map(|(n, t)| (n.to_string(), t.clone())).collect(),
        returns,
        is_variadic: false,
    }
}

fn sig_variadic(params: &[(&str, Type)], returns: Type) -> CallableSignature {
    CallableSignature {
        params: params.iter().map(|(n, t)| (n.to_string(), t.clone())).collect(),
        returns,
        is_variadic: true,
    }
}

// --- registry ---

static STDLIB_SIGNATURES: LazyLock<HashMap<&'static str, HashMap<&'static str, CallableSignature>>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();

        m.insert("fmt", module(&[
            ("print", sig_variadic(&[("args", any_t())], void_t())),
            ("println", sig_variadic(&[("args", any_t())], void_t())),
            ("input", sig_fixed(&[("prompt", str_t())], str_t())),
            ("debug", sig_variadic(&[("args", any_t())], void_t())),
            ("error", sig_variadic(&[("args", any_t())], void_t())),
            ("format", sig_variadic(&[("s", str_t()), ("args", any_t())], str_t())),
        ]));

        m.insert("IO", module(&[
            ("readFile", sig_fixed(&[("path", str_t())], str_t())),
            ("writeFile", sig_fixed(&[("path", str_t()), ("data", str_t())], void_t())),
            ("exists", sig_fixed(&[("path", str_t())], bool_t())),
            ("stat", sig_fixed(&[("path", str_t())], any_t())),
            ("mkdir", sig_fixed(&[("path", str_t())], void_t())),
            ("read_dir", sig_fixed(&[("path", str_t())], any_t())),
            ("remove", sig_fixed(&[("path", str_t())], void_t())),
            ("rename", sig_fixed(&[("from", str_t()), ("to", str_t())], void_t())),
            ("copy", sig_fixed(&[("from", str_t()), ("to", str_t())], void_t())),
            ("chmod", sig_fixed(&[("path", str_t()), ("mode", int_t())], void_t())),
            ("size", sig_fixed(&[("path", str_t())], int_t())),
            ("is_dir", sig_fixed(&[("path", str_t())], bool_t())),
            ("is_file", sig_fixed(&[("path", str_t())], bool_t())),
        ]));

        m.insert("List", module(&[
            ("length", sig_fixed(&[("list", generic("List", vec![type_var(0)]))], int_t())),
            ("map", sig_fixed(&[
                ("list", generic("List", vec![type_var(0)])),
                ("fn", Type::Function(FunctionType {
                    params: vec![type_var(0)],
                    returns: Box::new(type_var(1)),
                    is_variadic: false,
                })),
            ], generic("List", vec![type_var(1)]))),
            ("filter", sig_fixed(&[
                ("list", generic("List", vec![type_var(0)])),
                ("fn", Type::Function(FunctionType {
                    params: vec![type_var(0)],
                    returns: Box::new(bool_t()),
                    is_variadic: false,
                })),
            ], generic("List", vec![type_var(0)]))),
            ("push", sig_fixed(&[
                ("list", generic("List", vec![type_var(0)])),
                ("item", type_var(0)),
            ], generic("List", vec![type_var(0)]))),
            ("get", sig_fixed(&[
                ("list", generic("List", vec![type_var(0)])),
                ("index", int_t()),
            ], type_var(0))),
        ]));

        m.insert("Map", module(&[
            ("get", sig_fixed(&[
                ("map", generic("Map", vec![type_var(0), type_var(1)])),
                ("key", type_var(0)),
            ], type_var(1))),
            ("set", sig_fixed(&[
                ("map", generic("Map", vec![type_var(0), type_var(1)])),
                ("key", type_var(0)),
                ("value", type_var(1)),
            ], generic("Map", vec![type_var(0), type_var(1)]))),
            ("has", sig_fixed(&[
                ("map", generic("Map", vec![type_var(0), type_var(1)])),
                ("key", type_var(0)),
            ], bool_t())),
            ("keys", sig_fixed(&[
                ("map", generic("Map", vec![type_var(0), type_var(1)])),
            ], generic("List", vec![type_var(0)]))),
            ("values", sig_fixed(&[
                ("map", generic("Map", vec![type_var(0), type_var(1)])),
            ], generic("List", vec![type_var(1)]))),
            ("delete", sig_fixed(&[
                ("map", generic("Map", vec![type_var(0), type_var(1)])),
                ("key", type_var(0)),
            ], generic("Map", vec![type_var(0), type_var(1)]))),
        ]));

        m.insert("Set", module(&[
            ("add", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
                ("value", type_var(0)),
            ], generic("Set", vec![type_var(0)]))),
            ("remove", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
                ("value", type_var(0)),
            ], generic("Set", vec![type_var(0)]))),
            ("contains", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
                ("value", type_var(0)),
            ], bool_t())),
            ("size", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
            ], int_t())),
            ("union", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
                ("other", generic("Set", vec![type_var(0)])),
            ], generic("Set", vec![type_var(0)]))),
            ("intersection", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
                ("other", generic("Set", vec![type_var(0)])),
            ], generic("Set", vec![type_var(0)]))),
            ("difference", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
                ("other", generic("Set", vec![type_var(0)])),
            ], generic("Set", vec![type_var(0)]))),
            ("to_list", sig_fixed(&[
                ("set", generic("Set", vec![type_var(0)])),
            ], generic("List", vec![type_var(0)]))),
        ]));

        m.insert("Option", module(&[
            ("Some", sig_fixed(&[
                ("value", type_var(0)),
            ], generic("Option", vec![type_var(0)]))),
            // None has no args to infer T from; return Any to avoid unbound type var.
            ("None", sig_fixed(&[], any_t())),
            ("unwrap", sig_fixed(&[
                ("opt", generic("Option", vec![type_var(0)])),
            ], type_var(0))),
            ("is_some", sig_fixed(&[
                ("opt", generic("Option", vec![type_var(0)])),
            ], bool_t())),
            ("is_none", sig_fixed(&[
                ("opt", generic("Option", vec![type_var(0)])),
            ], bool_t())),
            ("map", sig_fixed(&[
                ("opt", generic("Option", vec![type_var(0)])),
                ("fn", Type::Function(FunctionType {
                    params: vec![type_var(0)],
                    returns: Box::new(type_var(1)),
                    is_variadic: false,
                })),
            ], generic("Option", vec![type_var(1)]))),
            ("or_else", sig_fixed(&[
                ("opt", generic("Option", vec![type_var(0)])),
                ("fn", Type::Function(FunctionType {
                    params: vec![],
                    returns: Box::new(generic("Option", vec![type_var(0)])),
                    is_variadic: false,
                })),
            ], generic("Option", vec![type_var(0)]))),
        ]));

        m.insert("Math", module(&[
            ("abs", sig_fixed(&[("n", int_t())], int_t())),
            ("min", sig_fixed(&[("a", int_t()), ("b", int_t())], int_t())),
            ("max", sig_fixed(&[("a", int_t()), ("b", int_t())], int_t())),
            ("floor", sig_fixed(&[("n", any_t())], any_t())),
            ("ceil", sig_fixed(&[("n", any_t())], any_t())),
            ("round", sig_fixed(&[("n", any_t())], any_t())),
            ("sqrt", sig_fixed(&[("n", any_t())], any_t())),
            ("pow", sig_fixed(&[("base", any_t()), ("exp", any_t())], any_t())),
        ]));

        m.insert("String", module(&[
            ("length", sig_fixed(&[("s", str_t())], int_t())),
            ("concat", sig_fixed(&[("a", str_t()), ("b", str_t())], str_t())),
            ("split", sig_fixed(&[("s", str_t()), ("sep", str_t())], any_t())),
            ("replace", sig_fixed(&[("s", str_t()), ("from", str_t()), ("to", str_t())], str_t())),
            ("to_upper", sig_fixed(&[("s", str_t())], str_t())),
            ("to_lower", sig_fixed(&[("s", str_t())], str_t())),
            ("trim", sig_fixed(&[("s", str_t())], str_t())),
            ("contains", sig_fixed(&[("s", str_t()), ("sub", str_t())], bool_t())),
        ]));

        m.insert("Env", module(&[
            ("get", sig_fixed(&[("key", str_t())], str_t())),
            ("set", sig_fixed(&[("key", str_t()), ("value", str_t())], void_t())),
            ("remove", sig_fixed(&[("key", str_t())], void_t())),
            ("args", sig_fixed(&[], any_t())),
            ("current_dir", sig_fixed(&[], str_t())),
            ("exit", sig_fixed(&[("code", int_t())], void_t())),
        ]));

        m.insert("Path", module(&[
            ("join", sig_variadic(&[("parts", str_t())], str_t())),
            ("basename", sig_fixed(&[("path", str_t())], str_t())),
            ("dirname", sig_fixed(&[("path", str_t())], str_t())),
            ("extension", sig_fixed(&[("path", str_t())], str_t())),
            ("is_absolute", sig_fixed(&[("path", str_t())], bool_t())),
            ("normalize", sig_fixed(&[("path", str_t())], str_t())),
            ("relative", sig_fixed(&[("from", str_t()), ("to", str_t())], str_t())),
            ("split", sig_fixed(&[("path", str_t())], any_t())),
        ]));

        m.insert("JSON", module(&[
            ("parse", sig_fixed(&[("s", str_t())], any_t())),
            ("stringify", sig_fixed(&[("value", any_t())], str_t())),
        ]));

        m.insert("DateTime", module(&[
            ("now", sig_fixed(&[], any_t())),
            ("format", sig_fixed(&[("date", any_t()), ("format", str_t())], str_t())),
            ("timestamp", sig_fixed(&[("date", any_t())], int_t())),
        ]));

        m.insert("Random", module(&[
            ("int", sig_fixed(&[], int_t())),
            ("int_range", sig_fixed(&[("lo", int_t()), ("hi", int_t())], int_t())),
            ("float", sig_fixed(&[], any_t())),
            ("bool", sig_fixed(&[], bool_t())),
            ("bytes", sig_fixed(&[("n", int_t())], any_t())),
            ("shuffle", sig_fixed(&[("arr", any_t())], any_t())),
            ("choice", sig_fixed(&[("arr", any_t())], any_t())),
        ]));

        m.insert("Encoding", module(&[
            ("hex_encode", sig_fixed(&[("data", any_t())], str_t())),
            ("hex_decode", sig_fixed(&[("s", str_t())], any_t())),
            ("base64_encode", sig_fixed(&[("data", any_t())], str_t())),
            ("base64_decode", sig_fixed(&[("s", str_t())], any_t())),
            ("url_encode", sig_fixed(&[("s", str_t())], str_t())),
            ("url_decode", sig_fixed(&[("s", str_t())], str_t())),
        ]));

        m.insert("Sort", module(&[
            ("asc", sig_fixed(&[("arr", any_t())], any_t())),
            ("desc", sig_fixed(&[("arr", any_t())], any_t())),
            ("by_key_asc", sig_fixed(&[("arr", any_t()), ("fn", any_t())], any_t())),
            ("by_key_desc", sig_fixed(&[("arr", any_t()), ("fn", any_t())], any_t())),
            ("is_sorted", sig_fixed(&[("arr", any_t())], bool_t())),
            ("min", sig_fixed(&[("arr", any_t())], any_t())),
            ("max", sig_fixed(&[("arr", any_t())], any_t())),
        ]));

        m.insert("Process", module(&[
            ("run", sig_fixed(&[("cmd", str_t()), ("args", any_t())], any_t())),
            ("exec", sig_fixed(&[("cmd", str_t())], str_t())),
            ("spawn", sig_fixed(&[("cmd", str_t()), ("args", any_t())], any_t())),
            ("exit", sig_fixed(&[("code", int_t())], void_t())),
            ("pid", sig_fixed(&[], int_t())),
            ("args", sig_fixed(&[], any_t())),
            ("stdout", sig_fixed(&[], any_t())),
            ("stderr", sig_fixed(&[], any_t())),
            ("status", sig_fixed(&[], int_t())),
        ]));

        m.insert("Task", module(&[
            ("spawn", sig_fixed(&[("fn", any_t())], any_t())),
            ("await", sig_fixed(&[("task", any_t())], any_t())),
            ("sleep", sig_fixed(&[("ms", int_t())], void_t())),
            ("join", sig_variadic(&[("tasks", any_t())], any_t())),
            ("parallel", sig_fixed(&[("fns", any_t())], any_t())),
            ("race", sig_fixed(&[("fns", any_t())], any_t())),
            ("all", sig_fixed(&[("fns", any_t())], any_t())),
            ("timeout", sig_fixed(&[("task", any_t()), ("ms", int_t())], any_t())),
        ]));

        m.insert("Channel", module(&[
            ("new", sig_fixed(&[("cap", int_t())], any_t())),
            ("send", sig_fixed(&[("ch", any_t()), ("value", any_t())], void_t())),
            ("recv", sig_fixed(&[("ch", any_t())], any_t())),
            ("close", sig_fixed(&[("ch", any_t())], void_t())),
            ("len", sig_fixed(&[("ch", any_t())], int_t())),
            ("cap", sig_fixed(&[("ch", any_t())], int_t())),
            ("select", sig_fixed(&[("chs", any_t())], any_t())),
            ("try_send", sig_fixed(&[("ch", any_t()), ("value", any_t())], bool_t())),
            ("try_recv", sig_fixed(&[("ch", any_t())], any_t())),
        ]));

        m.insert("Sync", module(&[
            ("mutex_new", sig_fixed(&[], any_t())),
            ("mutex_lock", sig_fixed(&[("m", any_t())], void_t())),
            ("mutex_unlock", sig_fixed(&[("m", any_t())], void_t())),
            ("once_do", sig_fixed(&[("fn", any_t())], any_t())),
            ("wait_group_new", sig_fixed(&[], any_t())),
            ("wait_group_add", sig_fixed(&[("wg", any_t()), ("n", int_t())], void_t())),
            ("wait_group_done", sig_fixed(&[("wg", any_t())], void_t())),
            ("wait_group_wait", sig_fixed(&[("wg", any_t())], void_t())),
        ]));

        m
    });

fn module(fns: &[(&'static str, CallableSignature)]) -> HashMap<&'static str, CallableSignature> {
    fns.iter().map(|(name, sig)| (*name, sig.clone())).collect()
}

/// Look up a single function signature by module and function name.
pub fn stdlib_signature(module: &str, function: &str) -> Option<&'static CallableSignature> {
    STDLIB_SIGNATURES.get(module).and_then(|m| m.get(function))
}

/// Get all function signatures for a module.
pub fn stdlib_module_signatures(module: &str) -> Option<&'static HashMap<&'static str, CallableSignature>> {
    STDLIB_SIGNATURES.get(module)
}

/// List all module names in the registry.
pub fn stdlib_modules() -> Vec<&'static str> {
    STDLIB_SIGNATURES.keys().copied().collect()
}
