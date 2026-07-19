use std::collections::HashMap;

use crate::ast::*;
use crate::checker::types::*;
use crate::checker::env::TypeEnv;
use crate::checker::unify::{unify, substitute, Substitution};
use crate::model::TangleDiagnostic;

pub fn check_expression(expr: &Expr, env: &TypeEnv) -> (Type, Vec<TangleDiagnostic>) {
    let mut diags = vec![];
    let ty = match expr {
        Expr::Literal(e) => match e.literal_kind {
            LiteralKind::Number => Type::Primitive(PrimitiveType { name: "Int".into() }),
            LiteralKind::String => Type::Primitive(PrimitiveType { name: "String".into() }),
            LiteralKind::Boolean => Type::Primitive(PrimitiveType { name: "Bool".into() }),
        },
        Expr::Identifier(e) => {
            if let Some(ty) = env.variables.get(&e.name) {
                ty.clone()
            } else if let Some(ref rc) = env.receiver {
                if let Some(ty) = rc.fields.get(&e.name) {
                    ty.clone()
                } else if let Some(ty) = env.structs.get(&e.name) {
                    ty.clone()
                } else if let Some(ft) = env.functions.get(&e.name) {
                    Type::Function(ft.clone())
                } else {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_SYMBOL_NOT_FOUND".into(),
                        message: format!("Symbol '{}' not found", e.name),
                        span: e.span.clone(),
                    });
                    Type::Primitive(PrimitiveType { name: "Bool".into() })
                }
            } else if let Some(ty) = env.structs.get(&e.name) {
                ty.clone()
            } else if let Some(ft) = env.functions.get(&e.name) {
                Type::Function(ft.clone())
            } else {
                diags.push(TangleDiagnostic {
                    code: "TANGLE_SYMBOL_NOT_FOUND".into(),
                    message: format!("Symbol '{}' not found", e.name),
                    span: e.span.clone(),
                });
                Type::Primitive(PrimitiveType { name: "Bool".into() })
            }
        }
        Expr::MemberAccess(e) => {
            let (obj_ty, mut obj_diags) = check_expression(&e.object, env);
            diags.append(&mut obj_diags);
            match &obj_ty {
                Type::Struct(s) => {
                    if let Some(field_ty) = s.fields.get(&e.member) {
                        field_ty.clone()
                    } else if let Some(sig) = s.methods.get(&e.member) {
                        Type::Function(FunctionType {
                            params: sig.params.iter().map(|(_, t)| t.clone()).collect(),
                            returns: Box::new(sig.returns.clone()),
                            is_variadic: sig.is_variadic,
                        })
                    } else {
                        diags.push(TangleDiagnostic {
                            code: "TANGLE_SYMBOL_NOT_FOUND".into(),
                            message: format!("Field '{}' not found on struct '{}'", e.member, s.name),
                            span: e.span.clone(),
                        });
                        Type::Primitive(PrimitiveType { name: "Bool".into() })
                    }
                }
                _ => {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_TYPE_ERROR".into(),
                        message: "Member access on non-struct type".into(),
                        span: e.span.clone(),
                    });
                    Type::Primitive(PrimitiveType { name: "Bool".into() })
                }
            }
        }
        Expr::Call(e) => {
            let (callee_ty, mut callee_diags) = check_expression(&e.callee, env);
            diags.append(&mut callee_diags);
            let arg_types: Vec<Type> = e.args.iter().map(|arg| {
                let (ty, mut d) = check_expression(arg, env);
                diags.append(&mut d);
                ty
            }).collect();

            match &callee_ty {
                Type::Function(sig) => {
                    let expected = sig.params.len();
                    let actual = arg_types.len();
                    if sig.is_variadic {
                        if actual < expected.saturating_sub(1) {
                            diags.push(TangleDiagnostic {
                                code: "TANGLE_ARITY_MISMATCH".into(),
                                message: format!(
                                    "Expected at least {} args, got {}",
                                    expected.saturating_sub(1),
                                    actual
                                ),
                                span: e.span.clone(),
                            });
                        }
                    } else if actual != expected {
                        diags.push(TangleDiagnostic {
                            code: "TANGLE_ARITY_MISMATCH".into(),
                            message: format!("Expected {} args, got {}", expected, actual),
                            span: e.span.clone(),
                        });
                    }
                    let mut subst: Substitution = HashMap::new();
                    for (i, (arg_ty, param_ty)) in arg_types.iter().zip(&sig.params).enumerate() {
                        if let Err(msg) = unify(param_ty, arg_ty, &mut subst) {
                            diags.push(TangleDiagnostic {
                                code: "TANGLE_TYPE_ERROR".into(),
                                message: format!("Arg {} type mismatch: {}", i + 1, msg),
                                span: e.span.clone(),
                            });
                        }
                    }
                    substitute(&sig.returns, &subst)
                }
                // Non-function callee (e.g., struct constructor, unknown symbol):
                // return Any without error to avoid false positives on untyped code.
                _ => Type::Any,
            }
        }
        Expr::Binary(e) => {
            let (left_ty, mut left_diags) = check_expression(&e.left, env);
            let (right_ty, mut right_diags) = check_expression(&e.right, env);
            diags.append(&mut left_diags);
            diags.append(&mut right_diags);
            match e.op {
                BinaryOp::Eq | BinaryOp::Neq | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Lte | BinaryOp::Gte
                | BinaryOp::And | BinaryOp::Or => Type::Primitive(PrimitiveType { name: "Bool".into() }),
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                    if !types_equal(&left_ty, &right_ty) {
                        diags.push(TangleDiagnostic {
                            code: "TANGLE_TYPE_ERROR".into(),
                            message: "Type mismatch in binary operation".into(),
                            span: e.span.clone(),
                        });
                    }
                    Type::Primitive(PrimitiveType { name: "Int".into() })
                }
            }
        }
        Expr::Unary(e) => {
            let (_inner, mut inner_diags) = check_expression(&e.operand, env);
            diags.append(&mut inner_diags);
            match e.op {
                UnaryOp::Not => Type::Primitive(PrimitiveType { name: "Bool".into() }),
                UnaryOp::Neg => Type::Primitive(PrimitiveType { name: "Int".into() }),
            }
        }
        Expr::RecordUpdate(e) => {
            let (obj_ty, mut obj_diags) = check_expression(&e.object, env);
            diags.append(&mut obj_diags);
            if let Type::Struct(s) = &obj_ty {
                for field in &e.fields {
                    if !s.fields.contains_key(&field.name) {
                        diags.push(TangleDiagnostic {
                            code: "TANGLE_TYPE_ERROR".into(),
                            message: format!("Field '{}' not found on struct '{}'", field.name, s.name),
                            span: field.span.clone(),
                        });
                    }
                }
            }
            obj_ty
        }
        Expr::Pipe(e) => {
            let (_left_ty, mut left_diags) = check_expression(&e.left, env);
            diags.append(&mut left_diags);
            let (right_ty, mut right_diags) = check_expression(&e.right, env);
            diags.append(&mut right_diags);
            right_ty
        }
        Expr::This(e) => {
            match &env.receiver {
                Some(rc) => {
                    if let Some(st) = env.structs.get(&rc.struct_name) {
                        st.clone()
                    } else {
                        Type::Primitive(PrimitiveType { name: "Bool".into() })
                    }
                }
                None => {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_TYPE_ERROR".into(),
                        message: "'this' used outside of method context".into(),
                        span: e.span.clone(),
                    });
                    Type::Primitive(PrimitiveType { name: "Bool".into() })
                }
            }
        }
        Expr::If(e) => {
            let (_cond_ty, mut cond_diags) = check_expression(&e.condition, env);
            diags.append(&mut cond_diags);

            // Phase 6d: 若 condition 是 IsExpr，在 then 分支注入收窄后的 binding 类型。
            // else 分支不做负向收窄（与设计规格 §5.3 一致）。
            let then_env = if let Expr::Is(is_e) = &*e.condition {
                narrow_env_for_is(env, is_e)
            } else {
                env.clone()
            };

            let (then_ty, mut then_diags) = check_expression(&e.then_branch, &then_env);
            diags.append(&mut then_diags);
            if let Some(ref else_branch) = e.else_branch {
                let (else_ty, mut else_diags) = check_expression(else_branch, env);
                diags.append(&mut else_diags);
                crate::checker::unify::unify_pair(&then_ty, &else_ty)
                    .unwrap_or_else(|| then_ty.clone())
            } else {
                then_ty
            }
        }
        Expr::Arrow(_e) => {
            Type::Function(FunctionType {
                params: vec![],
                returns: Box::new(Type::Primitive(PrimitiveType { name: "Bool".into() })),
                is_variadic: false,
            })
        }
        Expr::Propagation(e) => {
            let (inner_ty, mut inner_diags) = check_expression(&e.expr, env);
            diags.append(&mut inner_diags);
            if let Some(ref reg) = env.error_registry {
                let (stripped, mut prop_diags) = crate::checker::propagation::check_propagation(&inner_ty, reg);
                diags.append(&mut prop_diags);
                stripped
            } else {
                inner_ty
            }
        }
        Expr::Match(e) => {
            let (matched_ty, mut match_diags) = check_expression(&e.expr, env);
            diags.append(&mut match_diags);
            let mut arm_types = vec![];
            for arm in &e.arms {
                // 构造收窄后的 arm 局部环境
                let mut arm_env = env.clone();
                if let Type::Sum(ref sum) = matched_ty {
                    if let MatchPattern::Variant { ref name, ref binding } = arm.pattern {
                        if let Some(variant_ty) = crate::checker::match_check::find_variant_by_name(sum, name) {
                            if let Some(ref bind_name) = binding {
                                let bind_ty = crate::checker::match_check::binding_type_of(variant_ty);
                                arm_env.variables.insert(bind_name.clone(), bind_ty);
                            }
                        }
                    }
                }
                let (arm_ty, mut arm_diags) = check_expression(&arm.body, &arm_env);
                diags.append(&mut arm_diags);
                arm_types.push(arm_ty);
            }
            if let Type::Sum(ref sum) = matched_ty {
                let missing = crate::checker::match_check::check_match_exhaustiveness(sum, &e.arms);
                for m in missing {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_MATCH_NOT_EXHAUSTIVE".into(),
                        message: format!("Match not exhaustive: missing variant '{}'", m),
                        span: e.span.clone(),
                    });
                }
            }
            // 返回所有 arm body 类型的统一结果（最佳努力，失败回退 Any）
            crate::checker::unify::unify_all(&arm_types)
                .unwrap_or(Type::Any)
        }
        Expr::Destructure(e) => {
            let (inner_ty, mut inner_diags) = check_expression(&e.expr, env);
            diags.append(&mut inner_diags);
            match &inner_ty {
                Type::Sum(sum) if sum.variants.len() >= 2 => {
                    Type::Primitive(PrimitiveType { name: "Bool".into() })
                }
                _ => {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_TYPE_ERROR".into(),
                        message: "Destructure requires a sum type with at least 2 variants".into(),
                        span: e.span.clone(),
                    });
                    Type::Primitive(PrimitiveType { name: "Bool".into() })
                }
            }
        }
        Expr::Panic(e) => {
            let (_msg_ty, mut msg_diags) = check_expression(&e.message, env);
            diags.append(&mut msg_diags);
            diags.push(TangleDiagnostic {
                code: "TANGLE_PANIC_REACHED".into(),
                message: "panic() call reached — this code path is unreachable after panic".into(),
                span: e.span.clone(),
            });
            Type::Primitive(PrimitiveType { name: "Bool".into() })
        }
        Expr::Is(e) => {
            let (matched_ty, mut is_diags) = check_expression(&e.expr, env);
            diags.append(&mut is_diags);

            if let Some(sum) = crate::checker::option_view::as_sum_view(&matched_ty) {
                match &e.pattern {
                    Pattern::Variant { name, .. } => {
                        if crate::checker::match_check::find_variant_by_name(&sum, name).is_none() {
                            diags.push(TangleDiagnostic {
                                code: "TANGLE_PATTERN_VARIANT_NOT_FOUND".into(),
                                message: format!(
                                    "Variant '{}' not found in type {}",
                                    name,
                                    type_display(&matched_ty)
                                ),
                                span: e.span.clone(),
                            });
                        }
                    }
                }
            } else {
                diags.push(TangleDiagnostic {
                    code: "TANGLE_PATTERN_NOT_NARROWABLE".into(),
                    message: format!("Cannot narrow type {}", type_display(&matched_ty)),
                    span: e.span.clone(),
                });
            }

            Type::Primitive(PrimitiveType { name: "Bool".into() })
        }
    };
    (ty, diags)
}

/// 为 `if x is Pattern` 的 then 分支构造收窄后的环境。
///
/// 仅注入 binding（如 `y`），不改变 `x` 本身的类型。这与设计规格 §5.3 一致，
/// 避免 flow-sensitive 类型重写。若 variant 不存在或类型不可收窄，则静默回退
/// （相关诊断已由 `Expr::Is` 分支产生，不在此重复）。
fn narrow_env_for_is(env: &TypeEnv, is_e: &IsExpr) -> TypeEnv {
    let mut narrowed = env.clone();
    let (matched_ty, _) = check_expression(&is_e.expr, env);
    if let Some(sum) = crate::checker::option_view::as_sum_view(&matched_ty) {
        if let Pattern::Variant { name, binding: Some(binding) } = &is_e.pattern {
            if let Some(variant_ty) = crate::checker::match_check::find_variant_by_name(&sum, name) {
                let bind_ty = crate::checker::match_check::binding_type_of(variant_ty);
                narrowed.variables.insert(binding.clone(), bind_ty);
            }
        }
    }
    narrowed
}

/// 类型显示辅助（用于诊断消息）。
fn type_display(ty: &Type) -> String {
    match ty {
        Type::Primitive(p) => p.name.clone(),
        Type::Struct(s) => s.name.clone(),
        Type::Interface(i) => i.name.clone(),
        Type::GenericInstance(g) => {
            let args: Vec<String> = g.args.iter().map(type_display).collect();
            format!("{}<{}>", g.base, args.join(", "))
        }
        Type::Sum(_) => "Sum".to_string(),
        Type::Function(_) => "Function".to_string(),
        Type::Var(_) => "Var".to_string(),
        Type::Any => "Any".to_string(),
    }
}

#[cfg(test)]
mod if_expr_tests {
    use super::*;
    use crate::model::SourceSpan;

    fn span() -> SourceSpan {
        SourceSpan { file: "".into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 }
    }

    fn num_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, value: "0".to_string(), span: span() })
    }

    fn str_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::String, value: "".to_string(), span: span() })
    }

    fn bool_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Boolean, value: "true".to_string(), span: span() })
    }

    fn if_expr(then: Expr, else_: Option<Expr>) -> Expr {
        Expr::If(IfExpr {
            condition: Box::new(bool_expr()),
            then_branch: Box::new(then),
            else_branch: else_.map(Box::new),
            span: span(),
        })
    }

    fn empty_env() -> TypeEnv {
        TypeEnv {
            variables: std::collections::HashMap::new(),
            structs: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
            interfaces: std::collections::HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }

    #[test]
    fn if_without_else_returns_then_type() {
        let env = empty_env();
        let (ty, _) = check_expression(&if_expr(num_expr(), None), &env);
        match ty {
            Type::Primitive(p) => assert_eq!(p.name, "Int"),
            other => panic!("expected Int, got {:?}", other),
        }
    }

    #[test]
    fn if_with_same_then_else_returns_type() {
        let env = empty_env();
        let (ty, _) = check_expression(&if_expr(num_expr(), Some(num_expr())), &env);
        match ty {
            Type::Primitive(p) => assert_eq!(p.name, "Int"),
            other => panic!("expected Int, got {:?}", other),
        }
    }

    #[test]
    fn if_with_conflict_then_else_returns_then_type() {
        // 冲突时回退到 then 类型（best-effort）
        let env = empty_env();
        let (ty, _) = check_expression(&if_expr(num_expr(), Some(str_expr())), &env);
        match ty {
            Type::Primitive(p) => assert_eq!(p.name, "Int"),
            other => panic!("expected Int (then fallback), got {:?}", other),
        }
    }
}

#[cfg(test)]
mod match_narrowing_tests {
    use super::*;
    use crate::model::SourceSpan;
    use std::collections::HashMap;

    fn span() -> SourceSpan {
        SourceSpan { file: "".into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 }
    }

    fn num_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, value: "0".to_string(), span: span() })
    }

    fn ident_expr(name: &str) -> Expr {
        Expr::Identifier(IdentifierExpr { name: name.to_string(), span: span() })
    }

    fn arm(name: &str, binding: Option<&str>, body: Expr) -> MatchArm {
        MatchArm {
            pattern: MatchPattern::Variant {
                name: name.to_string(),
                binding: binding.map(|s| s.to_string()),
            },
            body,
            span: span(),
        }
    }

    fn match_expr(scrutinee: Expr, arms: Vec<MatchArm>) -> Expr {
        Expr::Match(MatchExpr {
            expr: Box::new(scrutinee),
            arms,
            span: span(),
        })
    }

    fn env_with_var(name: &str, ty: Type) -> TypeEnv {
        let mut vars = HashMap::new();
        vars.insert(name.to_string(), ty);
        TypeEnv {
            variables: vars,
            structs: HashMap::new(),
            functions: HashMap::new(),
            interfaces: HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    #[test]
    fn match_narrows_generic_instance_binding() {
        // match x { Some(y) => return y, None => return 0 }
        // x: Some<Int> | None, y should be narrowed to Int
        let sum_ty = Type::Sum(SumType {
            variants: vec![
                Type::GenericInstance(GenericTypeInstance {
                    base: "Some".to_string(),
                    args: vec![prim("Int")],
                }),
                prim("None"),
            ],
        });
        let env = env_with_var("x", sum_ty);
        let m = match_expr(
            ident_expr("x"),
            vec![
                arm("Some", Some("y"), ident_expr("y")),
                arm("None", None, num_expr()),
            ],
        );
        let (ty, diags) = check_expression(&m, &env);
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
        // Both arms return Int → unified to Int
        match ty {
            Type::Primitive(ref p) => assert_eq!(p.name, "Int", "expected Int, got {:?}", ty),
            other => panic!("expected Int, got {:?}", other),
        }
    }

    #[test]
    fn match_narrows_primitive_binding() {
        // match x { Int(y) => return y, String(s) => return s }
        // x: Int | String, y: Int, s: String → conflict → Any
        let sum_ty = Type::Sum(SumType {
            variants: vec![prim("Int"), prim("String")],
        });
        let env = env_with_var("x", sum_ty);
        let m = match_expr(
            ident_expr("x"),
            vec![
                arm("Int", Some("y"), ident_expr("y")),
                arm("String", Some("s"), ident_expr("s")),
            ],
        );
        let (ty, _diags) = check_expression(&m, &env);
        // Int vs String conflict → Any
        assert!(matches!(ty, Type::Any), "expected Any on conflict, got {:?}", ty);
    }

    #[test]
    fn match_no_narrowing_for_non_sum() {
        // match x { _ => return 0 } where x is Int (not Sum)
        let env = env_with_var("x", prim("Int"));
        let m = match_expr(
            ident_expr("x"),
            vec![MatchArm {
                pattern: MatchPattern::Wildcard,
                body: num_expr(),
                span: span(),
            }],
        );
        let (ty, _diags) = check_expression(&m, &env);
        match ty {
            Type::Primitive(p) => assert_eq!(p.name, "Int"),
            other => panic!("expected Int, got {:?}", other),
        }
    }

    #[test]
    fn match_returns_unified_arm_types() {
        // match x { Some(y) => return y, None => return 0 }
        // Both arms Int → unified Int (not Bool)
        let sum_ty = Type::Sum(SumType {
            variants: vec![
                Type::GenericInstance(GenericTypeInstance {
                    base: "Some".to_string(),
                    args: vec![prim("Int")],
                }),
                prim("None"),
            ],
        });
        let env = env_with_var("x", sum_ty);
        let m = match_expr(
            ident_expr("x"),
            vec![
                arm("Some", Some("y"), ident_expr("y")),
                arm("None", None, num_expr()),
            ],
        );
        let (ty, _) = check_expression(&m, &env);
        // Should NOT be Bool (old behavior); should be Int (unified)
        assert!(
            !matches!(ty, Type::Primitive(PrimitiveType { name }) if name == "Bool"),
            "should not return Bool (old behavior)"
        );
    }
}

#[cfg(test)]
mod phase6d_is_tests {
    use super::*;
    use crate::model::SourceSpan;
    use std::collections::HashMap;

    fn span() -> SourceSpan {
        SourceSpan { file: "".into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 }
    }

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    fn int_t() -> Type {
        prim("Int")
    }

    fn option_int() -> Type {
        Type::GenericInstance(GenericTypeInstance {
            base: "Option".to_string(),
            args: vec![int_t()],
        })
    }

    fn ident_expr(name: &str) -> Expr {
        Expr::Identifier(IdentifierExpr { name: name.to_string(), span: span() })
    }

    fn num_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, value: "0".to_string(), span: span() })
    }

    fn is_expr(target: Expr, name: &str, binding: Option<&str>) -> Expr {
        Expr::Is(IsExpr {
            expr: Box::new(target),
            pattern: Pattern::Variant {
                name: name.to_string(),
                binding: binding.map(|s| s.to_string()),
            },
            span: span(),
        })
    }

    fn env_with_var(name: &str, ty: Type) -> TypeEnv {
        let mut vars = HashMap::new();
        vars.insert(name.to_string(), ty);
        TypeEnv {
            variables: vars,
            structs: HashMap::new(),
            functions: HashMap::new(),
            interfaces: HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }

    #[test]
    fn is_expr_returns_bool_type() {
        // x: Option<Int>, x is Some(y) → Bool, no diagnostics
        let env = env_with_var("x", option_int());
        let e = is_expr(ident_expr("x"), "Some", Some("y"));
        let (ty, diags) = check_expression(&e, &env);
        assert!(
            matches!(ty, Type::Primitive(PrimitiveType { ref name }) if name == "Bool"),
            "expected Bool, got {:?}", ty
        );
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    }

    #[test]
    fn is_expr_emits_diag_for_unknown_variant() {
        // x: Option<Int>, x is NonExistent → TANGLE_PATTERN_VARIANT_NOT_FOUND
        let env = env_with_var("x", option_int());
        let e = is_expr(ident_expr("x"), "NonExistent", None);
        let (_ty, diags) = check_expression(&e, &env);
        assert!(
            diags.iter().any(|d| d.code == "TANGLE_PATTERN_VARIANT_NOT_FOUND"),
            "expected TANGLE_PATTERN_VARIANT_NOT_FOUND diagnostic, got: {:?}", diags
        );
    }

    #[test]
    fn is_expr_emits_diag_for_non_sum_type() {
        // x: Int (not Sum/Option), x is Some → TANGLE_PATTERN_NOT_NARROWABLE
        let env = env_with_var("x", int_t());
        let e = is_expr(ident_expr("x"), "Some", None);
        let (_ty, diags) = check_expression(&e, &env);
        assert!(
            diags.iter().any(|d| d.code == "TANGLE_PATTERN_NOT_NARROWABLE"),
            "expected TANGLE_PATTERN_NOT_NARROWABLE diagnostic, got: {:?}", diags
        );
    }

    #[test]
    fn if_with_is_injects_binding_in_then() {
        // if (x is Some(y)) { y } else { 0 }
        // 验证 then 分支中 y 被收窄为 Int，不报 SYMBOL_NOT_FOUND for 'y'
        let env = env_with_var("x", option_int());
        let cond = is_expr(ident_expr("x"), "Some", Some("y"));
        let if_e = Expr::If(IfExpr {
            condition: Box::new(cond),
            then_branch: Box::new(ident_expr("y")),
            else_branch: Some(Box::new(num_expr())),
            span: span(),
        });
        let (_ty, diags) = check_expression(&if_e, &env);
        assert!(
            !diags.iter().any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND" && d.message.contains("'y'")),
            "y should be narrowed in then-branch, but got: {:?}", diags
        );
    }
}
