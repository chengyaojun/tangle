use crate::ast::{ParsedCodeBlock, Stmt};
use crate::model::{TangleDiagnostic, TangleImport, TangleModule};
use crate::checker::types::*;
use crate::checker::env::{ReceiverContext, TypeEnv};
use crate::checker::errors::ErrorRegistry;
use crate::checker::resolve::{find_receiver_heading, resolve_types};
use crate::checker::check::check_expression;
use crate::checker::option_view::as_sum_view;
use crate::checker::match_check::{find_variant_by_name, binding_type_of};
use crate::checker::check::type_display;
use crate::parser::lexer::tokenize;
use crate::parser::parser::parse_code_body;
use std::collections::HashMap;
use crate::stdlib::signatures::{stdlib_signature, stdlib_module_signatures};

#[derive(Debug, Clone)]
pub struct CheckedModule {
    pub file: String,
    pub module_name: String,
    pub imports: Vec<crate::model::TangleImport>,
    pub headings: Vec<crate::model::TangleHeading>,
    pub symbols: Vec<crate::model::TangleSymbol>,
    pub diagnostics: Vec<TangleDiagnostic>,
    pub parsed_blocks: Vec<ParsedCodeBlock>,
    pub type_env: TypeEnv,
    /// 函数 heading_id → 推断出的返回类型（Phase 6c 新增）
    pub return_types: HashMap<String, Type>,
}

/// Parse all @tangle code blocks in the module
pub fn parse_code_blocks(module: &TangleModule) -> Vec<ParsedCodeBlock> {
    let mut blocks = vec![];
    collect_code_blocks(&module.headings, &module.file, &mut blocks);
    blocks
}

fn collect_code_blocks(headings: &[crate::model::TangleHeading], file: &str, out: &mut Vec<ParsedCodeBlock>) {
    for h in headings {
        for cb in &h.code_blocks {
            let (tokens, lexer_diags) = tokenize(&cb.value, file);
            let (body, mut parse_diags) = parse_code_body(&tokens);
            let mut all_diags = lexer_diags;
            all_diags.append(&mut parse_diags);
            out.push(ParsedCodeBlock {
                heading_id: h.id.clone(),
                source: cb.value.clone(),
                body,
                diagnostics: all_diags,
            });
        }
        collect_code_blocks(&h.children, file, out);
    }
}

fn is_stdlib_import(target: &str) -> bool {
    !target.contains('/') && !target.contains('\\') && !target.starts_with('.')
}

fn resolve_stdlib_imports(imports: &[TangleImport], env: &mut TypeEnv) {
    for imp in imports {
        if !is_stdlib_import(&imp.target) { continue; }
        if stdlib_module_signatures(&imp.target).is_none() { continue; }

        let aliases: Vec<&str> = imp.alias.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let fn_import = aliases.len() > 1
            || (aliases.len() == 1 && stdlib_signature(&imp.target, aliases[0]).is_some());

        if fn_import {
            for alias in &aliases {
                if let Some(sig) = stdlib_signature(&imp.target, alias) {
                    env.variables.insert(alias.to_string(), Type::Function(FunctionType {
                        params: sig.params.iter().map(|(_, t)| t.clone()).collect(),
                        returns: Box::new(sig.returns.clone()),
                        is_variadic: sig.is_variadic,
                    }));
                }
            }
        } else {
            let methods: HashMap<String, CallableSignature> = stdlib_module_signatures(&imp.target)
                .unwrap()
                .iter()
                .map(|(name, sig)| (name.to_string(), sig.clone()))
                .collect();
            env.structs.insert(imp.alias.clone(), Type::Struct(StructType {
                name: imp.target.clone(),
                fields: HashMap::new(),
                methods,
            }));
        }
    }
}

/// Main type-checking orchestrator
pub fn check_module(module: TangleModule) -> CheckedModule {
    let mut diagnostics = module.diagnostics.clone();
    let parsed_blocks = parse_code_blocks(&module);
    let (base_env, mut resolve_diags) = resolve_types(&module);
    diagnostics.append(&mut resolve_diags);

    let mut error_registry = ErrorRegistry::new();
    error_registry.collect_from_headings(&module.headings);

    let mut type_env = base_env;
    type_env.error_registry = Some(error_registry);

    // Resolve stdlib imports: [fmt](fmt) -> inject struct type
    resolve_stdlib_imports(&module.imports, &mut type_env);

    // Type-check each code block
    for block in &parsed_blocks {
        let mut block_diags = block.diagnostics.clone();
        diagnostics.append(&mut block_diags);

        // Find receiver context (implicit this binding)
        let receiver = find_receiver_heading(&block.heading_id, &module.headings)
            .map(|parent_heading| {
                let struct_name = parent_heading.symbol_name.clone()
                    .unwrap_or_else(|| parent_heading.title.clone());
                let fields: std::collections::HashMap<String, Type> = parent_heading.params.iter()
                    .map(|p| {
                        let ty = p.type_name.as_ref()
                            .map(|tn| crate::checker::resolve::type_name_to_type(tn)
                                .unwrap_or_else(|| Type::Primitive(PrimitiveType { name: tn.clone() })))
                            .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                        (p.name.clone(), ty)
                    }).collect();
                ReceiverContext { struct_name, fields }
            });

        let mut block_env = type_env.clone();
        block_env.receiver = receiver;

        // Inject current heading's params into local scope so method bodies
        // can reference their declared parameters.
        // Find the heading that owns this code block by heading_id.
        if let Some(owner) = find_heading_by_id(&block.heading_id, &module.headings) {
            for p in &owner.params {
                let ty = p.type_name.as_ref()
                    .map(|tn| crate::checker::resolve::type_name_to_type(tn)
                        .unwrap_or_else(|| Type::Primitive(PrimitiveType { name: tn.clone() })))
                    .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                block_env.variables.insert(p.name.clone(), ty);
            }
        }

        for stmt in &block.body.statements {
            check_stmt(stmt, &mut block_env, &mut diagnostics);
        }
    }

    let mut checked = CheckedModule {
        file: module.file,
        module_name: module.module_name,
        imports: module.imports,
        headings: module.headings,
        symbols: module.symbols,
        diagnostics,
        parsed_blocks,
        type_env,
        return_types: HashMap::new(),
    };
    checked.return_types = crate::checker::infer_return_types::infer_return_types(&checked);
    checked
}

/// Check a single statement, mutating env and diags accordingly.
fn check_stmt(stmt: &Stmt, env: &mut TypeEnv, diags: &mut Vec<TangleDiagnostic>) {
    match stmt {
        Stmt::Let(s) => {
            let (val_ty, mut val_diags) = check_expression(&s.value, env);
            diags.append(&mut val_diags);
            env.variables.insert(s.name.clone(), val_ty);
        }
        Stmt::Const(s) => {
            let (val_ty, mut val_diags) = check_expression(&s.value, env);
            diags.append(&mut val_diags);
            env.variables.insert(s.name.clone(), val_ty);
        }
        Stmt::Return(s) => {
            if let Some(ref value) = s.value {
                let (_, mut val_diags) = check_expression(value, env);
                diags.append(&mut val_diags);
            }
        }
        Stmt::Expression(s) => {
            let (_, mut expr_diags) = check_expression(&s.expr, env);
            diags.append(&mut expr_diags);
        }
        Stmt::LetVariant(s) => {
            let (matched_ty, mut expr_diags) = check_expression(&s.expr, env);
            diags.append(&mut expr_diags);

            if let Some(sum) = as_sum_view(&matched_ty) {
                if let Some(variant_ty) = find_variant_by_name(&sum, &s.variant_name) {
                    if let Some(ref bind_name) = s.binding {
                        let bind_ty = binding_type_of(variant_ty);
                        env.variables.insert(bind_name.clone(), bind_ty);
                    }
                    for stmt in &s.else_branch {
                        check_stmt(stmt, env, diags);
                    }
                } else {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_PATTERN_VARIANT_NOT_FOUND".into(),
                        message: format!(
                            "Variant '{}' not found in type {}",
                            s.variant_name,
                            type_display(&matched_ty)
                        ),
                        span: s.span.clone(),
                    });
                }
            } else {
                diags.push(TangleDiagnostic {
                    code: "TANGLE_PATTERN_NOT_NARROWABLE".into(),
                    message: format!(
                        "Cannot destructure type {}",
                        type_display(&matched_ty)
                    ),
                    span: s.span.clone(),
                });
            }
        }
        Stmt::LetRecord(s) => {
            let (matched_ty, mut d) = check_expression(&s.expr, env);
            diags.append(&mut d);

            match matched_ty {
                Type::Struct(ref struct_ty) => {
                    for (field_name, local_var) in &s.fields {
                        if let Some(field_ty) = struct_ty.fields.get(field_name) {
                            env.variables.insert(local_var.clone(), field_ty.clone());
                        } else {
                            diags.push(TangleDiagnostic {
                                code: "TANGLE_STRUCT_FIELD_NOT_FOUND".into(),
                                message: format!("Struct {} has no field '{}'", struct_ty.name, field_name),
                                span: s.span.clone(),
                            });
                        }
                    }
                }
                Type::Any => {
                    for (_, local_var) in &s.fields {
                        env.variables.insert(local_var.clone(), Type::Any);
                    }
                }
                other => {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_DESTRUCTURE_NOT_STRUCT".into(),
                        message: format!("Cannot destructure {} as record (expected struct)", type_display(&other)),
                        span: s.span.clone(),
                    });
                }
            }
        }
    }
}

/// Find a heading by id, recursively searching the heading tree.
fn find_heading_by_id<'a>(target_id: &str, headings: &'a [crate::model::TangleHeading]) -> Option<&'a crate::model::TangleHeading> {
    for h in headings {
        if h.id == target_id {
            return Some(h);
        }
        if let Some(found) = find_heading_by_id(target_id, &h.children) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod phase6d_let_variant_tests {
    use super::*;
    use crate::ast::{Expr, IdentifierExpr, LetVariantStmt, LiteralExpr, LiteralKind, ReturnStmt};
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

    fn let_variant_stmt(
        variant: &str,
        binding: Option<&str>,
        target: Expr,
        else_branch: Vec<Stmt>,
    ) -> Stmt {
        Stmt::LetVariant(LetVariantStmt {
            variant_name: variant.to_string(),
            binding: binding.map(|s| s.to_string()),
            expr: Box::new(target),
            else_branch,
            span: span(),
        })
    }

    #[test]
    fn let_variant_injects_binding_into_env() {
        // x: Option<Int>, `let Some(y) = x else { return 0 }`
        // 期望: y: Int 已注入 env，无诊断
        let mut env = env_with_var("x", option_int());
        let mut diags: Vec<TangleDiagnostic> = Vec::new();
        let else_branch = vec![Stmt::Return(ReturnStmt { value: Some(num_expr()), span: span() })];
        let stmt = let_variant_stmt("Some", Some("y"), ident_expr("x"), else_branch);
        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
        let y_ty = env.variables.get("y").expect("expected y to be injected into env");
        assert!(
            matches!(y_ty, Type::Primitive(PrimitiveType { ref name }) if name == "Int"),
            "expected y: Int, got {:?}", y_ty
        );
    }

    #[test]
    fn let_variant_emits_diag_for_non_sum() {
        // x: Int (not Sum/Option), `let Some(y) = x`
        // 期望: TANGLE_PATTERN_NOT_NARROWABLE 诊断
        let mut env = env_with_var("x", int_t());
        let mut diags: Vec<TangleDiagnostic> = Vec::new();
        let stmt = let_variant_stmt("Some", Some("y"), ident_expr("x"), vec![]);
        check_stmt(&stmt, &mut env, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "TANGLE_PATTERN_NOT_NARROWABLE"),
            "expected TANGLE_PATTERN_NOT_NARROWABLE diagnostic, got: {:?}", diags
        );
    }

    #[test]
    fn let_variant_emits_diag_for_unknown_variant() {
        // x: Option<Int>, `let NonExistent = x`
        // 期望: TANGLE_PATTERN_VARIANT_NOT_FOUND 诊断
        let mut env = env_with_var("x", option_int());
        let mut diags: Vec<TangleDiagnostic> = Vec::new();
        let stmt = let_variant_stmt("NonExistent", None, ident_expr("x"), vec![]);
        check_stmt(&stmt, &mut env, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "TANGLE_PATTERN_VARIANT_NOT_FOUND"),
            "expected TANGLE_PATTERN_VARIANT_NOT_FOUND diagnostic, got: {:?}", diags
        );
    }
}

#[cfg(test)]
mod phase6d_let_record_tests {
    use super::*;
    use crate::ast::{Expr, IdentifierExpr, LetRecordStmt};
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

    fn string_t() -> Type {
        prim("String")
    }

    fn ident_expr(name: &str) -> Expr {
        Expr::Identifier(IdentifierExpr { name: name.to_string(), span: span() })
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

    fn item_struct() -> Type {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), string_t());
        fields.insert("price".to_string(), int_t());
        Type::Struct(StructType {
            name: "Item".to_string(),
            fields,
            methods: HashMap::new(),
        })
    }

    fn let_record_stmt(fields: Vec<(&str, &str)>, target: Expr) -> Stmt {
        Stmt::LetRecord(LetRecordStmt {
            fields: fields
                .into_iter()
                .map(|(f, v)| (f.to_string(), v.to_string()))
                .collect(),
            expr: Box::new(target),
            span: span(),
        })
    }

    #[test]
    fn let_record_injects_all_fields() {
        // r: Item { name: String, price: Int }, `let { name: n, price: p } = r`
        // 期望: n: String, p: Int 已注入 env，无诊断
        let mut env = env_with_var("r", item_struct());
        let mut diags: Vec<TangleDiagnostic> = Vec::new();
        let stmt = let_record_stmt(
            vec![("name", "n"), ("price", "p")],
            ident_expr("r"),
        );
        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
        let n_ty = env.variables.get("n").expect("expected n to be injected into env");
        assert!(
            matches!(n_ty, Type::Primitive(PrimitiveType { ref name }) if name == "String"),
            "expected n: String, got {:?}", n_ty
        );
        let p_ty = env.variables.get("p").expect("expected p to be injected into env");
        assert!(
            matches!(p_ty, Type::Primitive(PrimitiveType { ref name }) if name == "Int"),
            "expected p: Int, got {:?}", p_ty
        );
    }

    #[test]
    fn let_record_emits_diag_for_missing_field() {
        // r: Item { name, price }, `let { nonexistent: x } = r`
        // 期望: TANGLE_STRUCT_FIELD_NOT_FOUND 诊断
        let mut env = env_with_var("r", item_struct());
        let mut diags: Vec<TangleDiagnostic> = Vec::new();
        let stmt = let_record_stmt(
            vec![("nonexistent", "x")],
            ident_expr("r"),
        );
        check_stmt(&stmt, &mut env, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "TANGLE_STRUCT_FIELD_NOT_FOUND"),
            "expected TANGLE_STRUCT_FIELD_NOT_FOUND diagnostic, got: {:?}", diags
        );
    }

    #[test]
    fn let_record_emits_diag_for_non_struct() {
        // r: Int (not Struct), `let { x: y } = r`
        // 期望: TANGLE_DESTRUCTURE_NOT_STRUCT 诊断
        let mut env = env_with_var("r", int_t());
        let mut diags: Vec<TangleDiagnostic> = Vec::new();
        let stmt = let_record_stmt(
            vec![("x", "y")],
            ident_expr("r"),
        );
        check_stmt(&stmt, &mut env, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "TANGLE_DESTRUCTURE_NOT_STRUCT"),
            "expected TANGLE_DESTRUCTURE_NOT_STRUCT diagnostic, got: {:?}", diags
        );
    }

    #[test]
    fn let_record_with_any_binds_all_to_any() {
        // r: Any, `let { ok: o } = r`
        // 期望: o: Any，无诊断
        let mut env = env_with_var("r", Type::Any);
        let mut diags: Vec<TangleDiagnostic> = Vec::new();
        let stmt = let_record_stmt(
            vec![("ok", "o")],
            ident_expr("r"),
        );
        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
        let o_ty = env.variables.get("o").expect("expected o to be injected into env");
        assert!(
            matches!(*o_ty, Type::Any),
            "expected o: Any, got {:?}", o_ty
        );
    }
}

#[cfg(test)]
mod phase6d_param_type_tests {
    use super::*;

    #[test]
    fn param_type_injection_handles_option_int() {
        // 验证 `* \`opt\`: Optional value (Option<Int>)` 参数被注入为 GenericInstance
        // 而非 Primitive("Option<Int>")。当参数类型被错误地塞进 Primitive.name 时，
        // 在 method body 中使用 `let Some(x) = opt else { return 0 }` 会触发
        // TANGLE_PATTERN_NOT_NARROWABLE 诊断（因为 Primitive 不是 Sum/Option）。
        let src = r#"# TestParam

### Processor
* `opt`: Optional value (Option<Int>)

#### process
* `opt`: Optional value (Option<Int>)

```@tangle
let Some(x) = opt else { return 0 }
return x
```

#### main

```@tangle
return 0
```
"#;
        use crate::frontend::compile_module::{compile_module, CompileModuleInput};
        let module = compile_module(CompileModuleInput {
            file: "test.tangle.md".to_string(),
            source: src.to_string(),
        });
        let checked = check_module(module);
        // 不应有 TANGLE_PATTERN_NOT_NARROWABLE 诊断（来自 LetVariant 收窄失败），
        // 也不应有 TANGLE_SYMBOL_NOT_FOUND（来自 binding 未注入）。
        let has_narrowable_error = checked.diagnostics.iter()
            .any(|d| d.code == "TANGLE_PATTERN_NOT_NARROWABLE");
        assert!(
            !has_narrowable_error,
            "Option<Int> should be recognized as GenericInstance, got diagnostics: {:?}",
            checked.diagnostics
        );
        let has_symbol_error = checked.diagnostics.iter()
            .any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND" && d.message.contains("'x'"));
        assert!(
            !has_symbol_error,
            "x should be injected via let-variant narrowing, got diagnostics: {:?}",
            checked.diagnostics
        );
    }
}
