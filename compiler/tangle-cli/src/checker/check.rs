use crate::ast::*;
use crate::checker::types::*;
use crate::checker::env::TypeEnv;
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
            let (_callee_ty, mut callee_diags) = check_expression(&e.callee, env);
            diags.append(&mut callee_diags);
            for arg in &e.args {
                let (_, mut arg_diags) = check_expression(arg, env);
                diags.append(&mut arg_diags);
            }
            Type::Primitive(PrimitiveType { name: "Bool".into() })
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
            let (then_ty, mut then_diags) = check_expression(&e.then_branch, env);
            diags.append(&mut then_diags);
            if let Some(ref else_branch) = e.else_branch {
                let (_else_ty, mut else_diags) = check_expression(else_branch, env);
                diags.append(&mut else_diags);
            }
            then_ty
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
            for arm in &e.arms {
                let (_, mut arm_diags) = check_expression(&arm.body, env);
                diags.append(&mut arm_diags);
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
            Type::Primitive(PrimitiveType { name: "Bool".into() })
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
    };
    (ty, diags)
}
