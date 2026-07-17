use tangle_cli::checker::types::*;
use tangle_cli::checker::unify::*;
use tangle_cli::stdlib::signatures::stdlib_module_signatures;

// === unify 算法测试 ===

#[test]
fn unify_binds_type_variable() {
    let mut subst = Substitution::new();
    let result = unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst);
    assert!(result.is_ok());
    assert_eq!(subst.get(&0), Some(&Type::Primitive(PrimitiveType { name: "Int".into() })));
}

#[test]
fn unify_type_variable_consistent() {
    let mut subst = Substitution::new();
    unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst).unwrap();
    // 再次统一相同类型应成功
    let result = unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst);
    assert!(result.is_ok());
}

#[test]
fn unify_type_variable_conflict() {
    let mut subst = Substitution::new();
    unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst).unwrap();
    // 统一不同类型应失败
    let result = unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "String".into() }), &mut subst);
    assert!(result.is_err());
}

#[test]
fn unify_nested_generic() {
    let mut subst = Substitution::new();
    let expected = generic("List", vec![type_var(0)]);
    let actual = generic("List", vec![Type::Primitive(PrimitiveType { name: "Int".into() })]);
    let result = unify(&expected, &actual, &mut subst);
    assert!(result.is_ok());
    assert_eq!(subst.get(&0), Some(&Type::Primitive(PrimitiveType { name: "Int".into() })));
}

#[test]
fn unify_function_type() {
    let mut subst = Substitution::new();
    let expected = Type::Function(FunctionType {
        params: vec![type_var(0)],
        returns: Box::new(type_var(1)),
        is_variadic: false,
    });
    let actual = Type::Function(FunctionType {
        params: vec![Type::Primitive(PrimitiveType { name: "Int".into() })],
        returns: Box::new(Type::Primitive(PrimitiveType { name: "String".into() })),
        is_variadic: false,
    });
    let result = unify(&expected, &actual, &mut subst);
    assert!(result.is_ok());
    assert_eq!(subst.get(&0), Some(&Type::Primitive(PrimitiveType { name: "Int".into() })));
    assert_eq!(subst.get(&1), Some(&Type::Primitive(PrimitiveType { name: "String".into() })));
}

#[test]
fn unify_any_always_succeeds() {
    let mut subst = Substitution::new();
    let result = unify(&Type::Any, &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst);
    assert!(result.is_ok());
    assert!(subst.is_empty()); // Any 不绑定变量
}

// === substitute 测试 ===

#[test]
fn substitute_replaces_type_variable() {
    let mut subst = Substitution::new();
    subst.insert(0, Type::Primitive(PrimitiveType { name: "Int".into() }));
    let ty = type_var(0);
    let result = substitute(&ty, &subst);
    assert_eq!(result, Type::Primitive(PrimitiveType { name: "Int".into() }));
}

#[test]
fn substitute_recursive_generic() {
    let mut subst = Substitution::new();
    subst.insert(0, Type::Primitive(PrimitiveType { name: "Int".into() }));
    let ty = generic("List", vec![type_var(0)]);
    let result = substitute(&ty, &subst);
    match result {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List");
            assert_eq!(g.args[0], Type::Primitive(PrimitiveType { name: "Int".into() }));
        }
        _ => panic!("Expected GenericInstance"),
    }
}

// === stdlib 泛型签名测试 ===

#[test]
fn list_map_returns_generic_list() {
    let list_mod = stdlib_module_signatures("List").expect("List module exists");
    let map_sig = list_mod.get("map").expect("map function exists");
    match &map_sig.returns {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List");
            assert_eq!(g.args.len(), 1);
        }
        _ => panic!("Expected GenericInstance for List.map return type"),
    }
}

#[test]
fn map_get_returns_type_variable() {
    let map_mod = stdlib_module_signatures("Map").expect("Map module exists");
    let get_sig = map_mod.get("get").expect("get function exists");
    // 返回类型应为 type_var(1)（V）
    assert!(matches!(get_sig.returns, Type::Var(_)));
}

#[test]
fn option_some_returns_generic_option() {
    let opt_mod = stdlib_module_signatures("Option").expect("Option module exists");
    let some_sig = opt_mod.get("Some").expect("Some function exists");
    match &some_sig.returns {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "Option");
            assert_eq!(g.args.len(), 1);
        }
        _ => panic!("Expected GenericInstance for Option.Some return type"),
    }
}
