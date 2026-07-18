use std::collections::HashMap;
use crate::ast::Stmt;
use crate::checker::check_module::CheckedModule;
use crate::checker::check::check_expression;
use crate::checker::env::{ReceiverContext, TypeEnv};
use crate::checker::resolve::find_receiver_heading;
use crate::checker::types::*;
use crate::checker::unify::unify_all;
use crate::model::{HeadingRole, TangleHeading};

/// 为模块中所有 Callable heading 推断返回类型。
/// 返回 heading_id → Type 映射。
pub fn infer_return_types(checked: &CheckedModule) -> HashMap<String, Type> {
    let mut result = HashMap::new();
    collect(&checked.headings, checked, &mut result);
    result
}

fn collect(headings: &[TangleHeading], checked: &CheckedModule, out: &mut HashMap<String, Type>) {
    for h in headings {
        if h.role == HeadingRole::Callable && !h.code_blocks.is_empty() {
            if let Some(ty) = infer_function_return_type(h, checked) {
                out.insert(h.id.clone(), ty);
            }
        }
        collect(&h.children, checked, out);
    }
}

fn infer_function_return_type(
    heading: &TangleHeading,
    checked: &CheckedModule,
) -> Option<Type> {
    // 1. 构造函数体的类型环境。
    //    注意：param_type_of 使用 type_name_to_type（比 check_module 现有硬编码 match 更准确），
    //    能正确解析泛型类型如 List<Int>。这是有意为之的改进，未来 check_module 也应统一使用。
    let mut env = checked.type_env.clone();
    setup_receiver_and_params(heading, checked, &mut env);

    // 2. 遍历该 heading 的所有 @tangle blocks，收集 return 类型
    let mut return_types: Vec<Type> = vec![];
    for block in &checked.parsed_blocks {
        if block.heading_id != heading.id {
            continue;
        }
        let mut block_env = env.clone();
        for stmt in &block.body.statements {
            match stmt {
                Stmt::Let(s) => {
                    let (ty, _) = check_expression(&s.value, &block_env);
                    block_env.variables.insert(s.name.clone(), ty);
                }
                Stmt::Const(s) => {
                    let (ty, _) = check_expression(&s.value, &block_env);
                    block_env.variables.insert(s.name.clone(), ty);
                }
                Stmt::Return(s) => {
                    if let Some(ref value) = s.value {
                        let (ty, _) = check_expression(value, &block_env);
                        return_types.push(ty);
                    }
                    // `return;`（无值）不贡献类型
                }
                Stmt::Expression(_) => {}
            }
        }
    }

    // 3. 统一所有 return 类型
    if return_types.is_empty() {
        None
    } else {
        Some(unify_all(&return_types).unwrap_or(Type::Any))
    }
}

/// 构造函数体的类型环境：设置 receiver、注入 heading params。
fn setup_receiver_and_params(
    heading: &TangleHeading,
    checked: &CheckedModule,
    env: &mut TypeEnv,
) {
    if let Some(parent) = find_receiver_heading(&heading.id, &checked.headings) {
        let struct_name = parent
            .symbol_name
            .clone()
            .unwrap_or_else(|| parent.title.clone());
        let fields = parent
            .params
            .iter()
            .map(|p| (p.name.clone(), param_type_of(&p.type_name)))
            .collect();
        env.receiver = Some(ReceiverContext { struct_name, fields });
    }
    for p in &heading.params {
        env.variables.insert(p.name.clone(), param_type_of(&p.type_name));
    }
}

/// 用 type_name_to_type 解析参数类型（比 check_module 现有硬编码 match 更准确）
fn param_type_of(type_name: &Option<String>) -> Type {
    type_name
        .as_deref()
        .and_then(crate::checker::resolve::type_name_to_type)
        .unwrap_or(Type::Any)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{
        CodeBody, Expr, ExpressionStmt, LiteralExpr, LiteralKind, ParsedCodeBlock, ReturnStmt,
    };
    use crate::model::{HeadingRole, SourceSpan, TangleCodeBlock, TangleHeading};

    fn span() -> SourceSpan {
        SourceSpan {
            file: "test.md".into(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        }
    }

    fn int_expr(n: &str) -> Expr {
        Expr::Literal(LiteralExpr {
            literal_kind: LiteralKind::Number,
            value: n.to_string(),
            span: span(),
        })
    }

    fn str_expr(s: &str) -> Expr {
        Expr::Literal(LiteralExpr {
            literal_kind: LiteralKind::String,
            value: s.to_string(),
            span: span(),
        })
    }

    fn return_stmt(value: Expr) -> Stmt {
        Stmt::Return(ReturnStmt {
            value: Some(value),
            span: span(),
        })
    }

    /// 构造一个 Callable heading，带一个非空 code_blocks 占位条目
    /// （collect 的入口条件要求 !code_blocks.is_empty()）
    fn callable_heading(id: &str, title: &str) -> TangleHeading {
        TangleHeading {
            id: id.to_string(),
            depth: 4,
            role: HeadingRole::Callable,
            title: title.to_string(),
            symbol_name: Some(title.to_string()),
            directives: vec![],
            params: vec![],
            code_blocks: vec![TangleCodeBlock {
                language: "tangle".into(),
                value: String::new(),
                span: span(),
            }],
            rule: None,
            span: span(),
            children: vec![],
        }
    }

    fn parsed_block(heading_id: &str, statements: Vec<Stmt>) -> ParsedCodeBlock {
        ParsedCodeBlock {
            heading_id: heading_id.to_string(),
            source: String::new(),
            body: CodeBody {
                statements,
                span: span(),
            },
            diagnostics: vec![],
        }
    }

    fn empty_type_env() -> TypeEnv {
        TypeEnv {
            variables: HashMap::new(),
            structs: HashMap::new(),
            functions: HashMap::new(),
            interfaces: HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }

    fn checked_with(heading: TangleHeading, block: ParsedCodeBlock) -> CheckedModule {
        CheckedModule {
            file: "test.md".into(),
            module_name: "test".into(),
            imports: vec![],
            headings: vec![heading],
            symbols: vec![],
            diagnostics: vec![],
            parsed_blocks: vec![block],
            type_env: empty_type_env(),
            return_types: HashMap::new(),
        }
    }

    // 完整的集成测试将在 Task 7 中添加到 tests/v06_phase6/return_type_inference.rs。
    // 此处单元测试聚焦核心推断逻辑：多 return 统一、冲突回退、无 return 路径。

    #[test]
    fn infer_return_types_function_exists() {
        // 空模块：无 heading，应返回空 map
        let checked = CheckedModule {
            file: "".into(),
            module_name: "".into(),
            imports: vec![],
            headings: vec![],
            symbols: vec![],
            diagnostics: vec![],
            parsed_blocks: vec![],
            type_env: empty_type_env(),
            return_types: HashMap::new(),
        };
        let result = infer_return_types(&checked);
        assert!(result.is_empty());
    }

    #[test]
    fn infer_function_return_type_unifies_multiple_int_returns() {
        // 一个 Callable heading + 多个 Int return → 应统一为 Int
        let heading = callable_heading("h1", "main");
        let block = parsed_block(
            "h1",
            vec![return_stmt(int_expr("0")), return_stmt(int_expr("1"))],
        );
        let checked = checked_with(heading, block);

        let result = infer_return_types(&checked);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("h1"),
            Some(&Type::Primitive(PrimitiveType { name: "Int".into() })),
            "两个 Int return 应统一为 Int"
        );
    }

    #[test]
    fn infer_function_return_type_conflict_falls_back_to_any() {
        // 一个 Callable heading + Int 与 String return → unify_all 失败 → 回退到 Type::Any
        let heading = callable_heading("h2", "main");
        let block = parsed_block(
            "h2",
            vec![
                return_stmt(int_expr("0")),
                return_stmt(str_expr("hello")),
            ],
        );
        let checked = checked_with(heading, block);

        let result = infer_return_types(&checked);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("h2"),
            Some(&Type::Any),
            "Int 与 String 冲突时应回退到 Type::Any"
        );
    }

    #[test]
    fn infer_function_return_type_no_returns_yields_no_entry() {
        // Callable heading 但 block 中无 return → 不进入返回类型 map
        let heading = callable_heading("h3", "main");
        let block = parsed_block(
            "h3",
            vec![Stmt::Expression(ExpressionStmt {
                expr: int_expr("42"),
                span: span(),
            })],
        );
        let checked = checked_with(heading, block);

        let result = infer_return_types(&checked);
        assert!(
            result.is_empty(),
            "无 return 的函数不应出现在返回类型 map 中"
        );
    }
}
