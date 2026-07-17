//! Phase 6b: IRParam carries type annotations from Tangle source.
//!
//! Verifies that `collect_functions` populates `IRParam.type_` from heading
//! parameter annotations like `* \`items\`: ... (List<Int>)`. The fixture
//! `tests/v06_phase6/generics.tangle.md` defines `process(items, threshold)`
//! under `### ItemProcessor` and a free `main()` function.

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::checker::types::Type;
use tangle_cli::ir::graph::IRParam;

fn fixture_path(group: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(group)
        .join(name)
}

#[test]
fn test_params_carry_types_from_fixture() {
    let path = fixture_path("v06_phase6", "generics.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    // Multi-function mode: fixture has `#### main` → functions[] populated.
    assert!(
        !graph.functions.is_empty(),
        "fixture should produce functions[] (multi-function mode)"
    );

    // Find the `process` function (method of ItemProcessor).
    let process = graph.functions.iter().find(|f| f.name == "process")
        .expect("fixture should define a `process` function");

    assert_eq!(
        process.params.len(),
        2,
        "process should have 2 params (items, threshold), got {:?}",
        process.params.iter().map(|p| &p.name).collect::<Vec<_>>()
    );

    // First param `items` should carry type List<Int>.
    let items_param: &IRParam = &process.params[0];
    assert_eq!(items_param.name, "items");
    let ty = items_param.type_.as_ref().expect("items should have a type");
    match ty {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List", "items type base should be List");
            assert_eq!(g.args.len(), 1, "List should have 1 type arg");
        }
        other => panic!("expected GenericInstance for items, got {:?}", other),
    }

    // Second param `threshold` should carry type Int.
    let threshold_param = &process.params[1];
    assert_eq!(threshold_param.name, "threshold");
    match threshold_param.type_.as_ref().unwrap() {
        Type::Primitive(p) => assert_eq!(p.name, "Int", "threshold should be Primitive Int"),
        other => panic!("expected Primitive Int for threshold, got {:?}", other),
    }

    // `main` should have no params.
    let main = graph.functions.iter().find(|f| f.name == "main")
        .expect("fixture should define a `main` function");
    assert!(
        main.params.is_empty(),
        "main should have no params, got {:?}",
        main.params.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_ir_param_json_serialization_shape() {
    // Verify the JSON shape: params serialize as [{name, type?}, ...].
    let param_with_type = IRParam {
        name: "items".into(),
        type_: Some(Type::GenericInstance(
            tangle_cli::checker::types::GenericTypeInstance {
                base: "List".into(),
                args: vec![Type::Primitive(tangle_cli::checker::types::PrimitiveType {
                    name: "Int".into(),
                })],
            },
        )),
    };
    let json = serde_json::to_value(&param_with_type).unwrap();
    assert_eq!(json["name"], "items");
    assert_eq!(json["type"]["kind"], "genericInstance");
    assert_eq!(json["type"]["base"], "List");

    let param_without_type = IRParam {
        name: "x".into(),
        type_: None,
    };
    let json = serde_json::to_value(&param_without_type).unwrap();
    assert_eq!(json["name"], "x");
    assert!(
        json.get("type").is_none(),
        "type field should be omitted when None"
    );
}
