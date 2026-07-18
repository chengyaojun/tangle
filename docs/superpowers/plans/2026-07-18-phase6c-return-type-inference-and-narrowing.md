# Phase 6c: 返回类型推断 + Match arm 类型收窄 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让用户函数的返回类型从 `return` 语句自动推断，通过 Match arm 类型收窄提升准确性，填充 `IRFunction.return_type`，保持 emitter 向后兼容。

**架构：** 在 `check_module` 后新增独立 pass `infer_return_types`，遍历函数体收集 return 类型并统一；改进 `check_expression` 的 Match/If 分支注入收窄类型并返回统一结果；`compile_to_ir` 读取推断结果填充 IR；TS reference 全量同步。

**技术栈：** Rust（checker/ir/codegen 模块）、TypeScript（reference/src 镜像）、PowerShell（差分测试脚本）

**规格文件：** [docs/superpowers/specs/2026-07-18-phase6c-return-type-inference-and-narrowing-design.md](file:///e:/GitProjects/tangle/docs/superpowers/specs/2026-07-18-phase6c-return-type-inference-and-narrowing-design.md)

**前置条件：** 在 `phase6c/v0.7.0` 分支的 worktree 中工作。若不存在，使用 `using-git-worktrees` 技能创建：`.worktrees/phase6c-v0.7.0` 基于 `main`。

---

## 文件结构

### Rust 端

| 文件 | 操作 | 职责 |
|------|------|------|
| `compiler/tangle-cli/src/checker/unify.rs` | 修改 | 新增 `unify_all` / `unify_pair` 共享辅助函数 |
| `compiler/tangle-cli/src/checker/match_check.rs` | 修改 | 新增 `variant_name` / `binding_type_of` / `find_variant_by_name`；扩展 `check_match_exhaustiveness` |
| `compiler/tangle-cli/src/checker/check.rs` | 修改 | `Expr::Match` arm 收窄 + 返回统一类型；`Expr::If` 统一 then/else |
| `compiler/tangle-cli/src/checker/infer_return_types.rs` | **新建** | 推断 pass：遍历函数体，收集+统一 return 类型 |
| `compiler/tangle-cli/src/checker/mod.rs` | 修改 | 注册 `infer_return_types` 模块 |
| `compiler/tangle-cli/src/checker/check_module.rs` | 修改 | `CheckedModule` 新增 `return_types` 字段；调用 `infer_return_types` |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | 修改 | `collect_functions` 读取 return_types 填充 `IRFunction.return_type` |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | 修改 | 外部签名恒为 `Result` |
| `compiler/tangle-cli/tests/v06_phase6/return_type_inference.rs` | **新建** | 集成测试：加载 3 个新 fixture 验证 returnType |
| `tests/v06_phase6/return_inference.tangle.md` | **新建** | 基础推断 fixture |
| `tests/v06_phase6/match_narrowing.tangle.md` | **新建** | Match arm 收窄 fixture |
| `tests/v06_phase6/return_conflict.tangle.md` | **新建** | 冲突回退 Any fixture |

### TS reference 端

| 文件 | 操作 | 职责 |
|------|------|------|
| `reference/src/checker/unify.ts` | 修改 | 新增 `unifyAll` / `unifyPair` |
| `reference/src/checker/match.ts` | 修改 | 扩展 `getVariantName` 支持 genericInstance；新增 `bindingTypeOf` / `findVariantByName` |
| `reference/src/checker/check.ts` | 修改 | `match` case arm 收窄 + 返回统一类型；`if` case 统一 then/else |
| `reference/src/checker/inferReturnTypes.ts` | **新建** | TS 端镜像推断算法 |
| `reference/src/checker/checkModule.ts` | 修改 | `CheckedModule` 新增 `returnTypes`；调用 `inferReturnTypes` |
| `reference/src/ir/compileToIR.ts` | 修改 | `collectFunctions` 读取 returnTypes 填充 `returnType` |
| `reference/tests/checker/inferReturnTypes.test.ts` | **新建** | TS 单元测试 |
| `reference/tests/checker/matchNarrowing.test.ts` | **新建** | TS 单元测试 |

---

## 任务 1：Rust — 新增 unify_all / unify_pair 共享辅助函数

**文件：**
- 修改：`compiler/tangle-cli/src/checker/unify.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/checker/unify.rs` 末尾的 `#[cfg(test)]` 模块中（若不存在则新增）添加测试：

```rust
#[cfg(test)]
mod unify_all_tests {
    use super::*;
    use crate::checker::types::*;

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    fn list_int() -> Type {
        Type::GenericInstance(GenericTypeInstance {
            base: "List".to_string(),
            args: vec![prim("Int")],
        })
    }

    #[test]
    fn unify_all_empty_returns_none() {
        assert!(unify_all(&[]).is_none());
    }

    #[test]
    fn unify_all_single_returns_it() {
        let types = vec![prim("Int")];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_all_same_types_succeeds() {
        let types = vec![prim("Int"), prim("Int"), prim("Int")];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_all_conflict_returns_none() {
        let types = vec![prim("Int"), prim("String")];
        assert!(unify_all(&types).is_none());
    }

    #[test]
    fn unify_all_with_any_succeeds() {
        let types = vec![prim("Int"), Type::Any];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_all_generic_instances() {
        let types = vec![list_int(), list_int()];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, list_int());
    }

    #[test]
    fn unify_pair_same_succeeds() {
        let result = unify_pair(&prim("Int"), &prim("Int")).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_pair_conflict_returns_none() {
        assert!(unify_pair(&prim("Int"), &prim("String")).is_none());
    }

    #[test]
    fn unify_pair_with_any_succeeds() {
        let result = unify_pair(&prim("Int"), &Type::Any).unwrap();
        assert_eq!(result, prim("Int"));
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --package tangle-cli unify_all_tests -- --nocapture`
预期：编译失败，报错 `unify_all` / `unify_pair` 未定义

- [ ] **步骤 3：实现 unify_all / unify_pair**

在 `compiler/tangle-cli/src/checker/unify.rs` 的 `substitute` 函数之后（第 102 行后）添加：

```rust
/// 统一类型列表：以第一个为锚点，逐个 unify。
/// 成功返回统一后的类型（含 type_var 替换）；失败返回 None。
/// 用于 return 路径类型统一、Match arm body 类型统一。
pub fn unify_all(types: &[Type]) -> Option<Type> {
    if types.is_empty() {
        return None;
    }
    let mut subst: Substitution = HashMap::new();
    let anchor = &types[0];
    for other in &types[1..] {
        if unify(anchor, other, &mut subst).is_err() {
            return None;
        }
    }
    Some(substitute(anchor, &subst))
}

/// 统一两个类型（用于 If then/else 分支统一）。
/// 成功返回统一后的类型；失败返回 None。
pub fn unify_pair(a: &Type, b: &Type) -> Option<Type> {
    let mut subst: Substitution = HashMap::new();
    match unify(a, b, &mut subst) {
        Ok(()) => Some(substitute(a, &subst)),
        Err(_) => None,
    }
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test --package tangle-cli unify_all_tests -- --nocapture`
预期：9 个测试全部 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/unify.rs
git commit -m "feat(checker): add unify_all/unify_pair shared helpers in unify.rs"
```

---

## 任务 2：Rust — 扩展 match_check.rs（variant 命名 + 穷尽性）

**文件：**
- 修改：`compiler/tangle-cli/src/checker/match_check.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/checker/match_check.rs` 末尾添加 `#[cfg(test)]` 模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{MatchArm, MatchPattern, Expr, LiteralExpr, LiteralKind, SourceSpan};
    use crate::checker::types::*;
    use std::collections::HashMap;

    fn span() -> SourceSpan {
        SourceSpan { file: "".into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 }
    }

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    fn generic(base: &str, args: Vec<Type>) -> Type {
        Type::GenericInstance(GenericTypeInstance { base: base.to_string(), args })
    }

    fn struct_type(name: &str) -> Type {
        Type::Struct(StructType { name: name.to_string(), fields: HashMap::new(), methods: HashMap::new() })
    }

    fn arm(name: &str, binding: Option<&str>) -> MatchArm {
        MatchArm {
            pattern: MatchPattern::Variant {
                name: name.to_string(),
                binding: binding.map(|s| s.to_string()),
            },
            body: Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, span: span() }),
            span: span(),
        }
    }

    fn wildcard_arm() -> MatchArm {
        MatchArm {
            pattern: MatchPattern::Wildcard,
            body: Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, span: span() }),
            span: span(),
        }
    }

    // --- variant_name tests ---

    #[test]
    fn variant_name_primitive() {
        assert_eq!(variant_name(&prim("Int")), Some("Int".to_string()));
    }

    #[test]
    fn variant_name_struct() {
        assert_eq!(variant_name(&struct_type("Order")), Some("Order".to_string()));
    }

    #[test]
    fn variant_name_generic_instance() {
        assert_eq!(variant_name(&generic("Some", vec![prim("Int")])), Some("Some".to_string()));
    }

    #[test]
    fn variant_name_sum_returns_none() {
        let sum = Type::Sum(SumType { variants: vec![] });
        assert_eq!(variant_name(&sum), None);
    }

    #[test]
    fn variant_name_any_returns_none() {
        assert_eq!(variant_name(&Type::Any), None);
    }

    // --- binding_type_of tests ---

    #[test]
    fn binding_type_of_generic_instance_returns_payload() {
        let some_int = generic("Some", vec![prim("Int")]);
        assert_eq!(binding_type_of(&some_int), prim("Int"));
    }

    #[test]
    fn binding_type_of_generic_instance_no_args_returns_any() {
        let some_empty = generic("Some", vec![]);
        assert_eq!(binding_type_of(&some_empty), Type::Any);
    }

    #[test]
    fn binding_type_of_primitive_returns_itself() {
        assert_eq!(binding_type_of(&prim("Int")), prim("Int"));
    }

    #[test]
    fn binding_type_of_struct_returns_itself() {
        let s = struct_type("Order");
        assert_eq!(binding_type_of(&s), s);
    }

    // --- find_variant_by_name tests ---

    #[test]
    fn find_variant_by_name_found() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let found = find_variant_by_name(&sum, "String");
        assert!(found.is_some());
        assert_eq!(*found.unwrap(), prim("String"));
    }

    #[test]
    fn find_variant_by_name_not_found() {
        let sum = SumType { variants: vec![prim("Int")] };
        assert!(find_variant_by_name(&sum, "Bool").is_none());
    }

    #[test]
    fn find_variant_by_name_generic_instance() {
        let sum = SumType {
            variants: vec![generic("Some", vec![prim("Int")]), prim("None")],
        };
        let found = find_variant_by_name(&sum, "Some");
        assert!(found.is_some());
    }

    // --- check_match_exhaustiveness tests ---

    #[test]
    fn exhaustiveness_all_covered() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let arms = vec![arm("Int", None), arm("String", None)];
        assert!(check_match_exhaustiveness(&sum, &arms).is_empty());
    }

    #[test]
    fn exhaustiveness_missing_variant() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let arms = vec![arm("Int", None)];
        let missing = check_match_exhaustiveness(&sum, &arms);
        assert_eq!(missing, vec!["String".to_string()]);
    }

    #[test]
    fn exhaustiveness_wildcard_covers_all() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let arms = vec![arm("Int", None), wildcard_arm()];
        assert!(check_match_exhaustiveness(&sum, &arms).is_empty());
    }

    #[test]
    fn exhaustiveness_generic_instance_variant() {
        let sum = SumType {
            variants: vec![generic("Some", vec![prim("Int")]), prim("None")],
        };
        let arms = vec![arm("Some", Some("y")), arm("None", None)];
        assert!(check_match_exhaustiveness(&sum, &arms).is_empty());
    }

    #[test]
    fn exhaustiveness_struct_variant() {
        let sum = SumType {
            variants: vec![struct_type("Order"), struct_type("User")],
        };
        let arms = vec![arm("Order", None)];
        let missing = check_match_exhaustiveness(&sum, &arms);
        assert_eq!(missing, vec!["User".to_string()]);
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --package tangle-cli match_check::tests -- --nocapture`
预期：编译失败，报错 `variant_name` / `binding_type_of` / `find_variant_by_name` 未定义

- [ ] **步骤 3：实现 variant_name / binding_type_of / find_variant_by_name**

替换 `compiler/tangle-cli/src/checker/match_check.rs` 全部内容为：

```rust
use crate::ast::{MatchArm, MatchPattern};
use crate::checker::types::{GenericTypeInstance, PrimitiveType, StructType, SumType, Type};

/// 提取 variant 名（Primitive/Struct/GenericInstance）。
/// 其他类型（Sum/Function/Var/Any/Interface）不支持作为命名 variant。
pub fn variant_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Primitive(p) => Some(p.name.clone()),
        Type::Struct(s) => Some(s.name.clone()),
        Type::GenericInstance(g) => Some(g.base.clone()),
        _ => None,
    }
}

/// 提取 binding 类型。
/// GenericInstance 返回 args[0]（payload）；其他返回 variant 类型本身。
pub fn binding_type_of(variant_ty: &Type) -> Type {
    match variant_ty {
        Type::GenericInstance(g) => g.args.first().cloned().unwrap_or(Type::Any),
        other => other.clone(),
    }
}

/// 在 Sum 的 variants 中按名查找。
pub fn find_variant_by_name<'a>(sum: &'a SumType, name: &str) -> Option<&'a Type> {
    sum.variants
        .iter()
        .find(|v| variant_name(v).as_deref() == Some(name))
}

/// Check match exhaustiveness. Returns list of missing variant names.
/// 支持 Primitive/Struct/GenericInstance variant。
pub fn check_match_exhaustiveness(sum: &SumType, arms: &[MatchArm]) -> Vec<String> {
    let has_wildcard = arms
        .iter()
        .any(|a| matches!(a.pattern, MatchPattern::Wildcard));
    if has_wildcard {
        return vec![];
    }

    let mut missing = vec![];
    for variant in &sum.variants {
        if let Some(name) = variant_name(variant) {
            let covered = arms.iter().any(|a| match &a.pattern {
                MatchPattern::Variant { name: pn, .. } => pn == &name,
                _ => false,
            });
            if !covered {
                missing.push(name);
            }
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    // （粘贴步骤 1 中的测试代码）
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test --package tangle-cli match_check::tests -- --nocapture`
预期：所有测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/match_check.rs
git commit -m "feat(checker): extend match_check with variant_name/binding_type_of/find_variant_by_name"
```

---

## 任务 3：Rust — 改进 check.rs 的 Expr::If 分支

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check.rs:199-209`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/checker/check.rs` 的 `#[cfg(test)]` 模块中（若不存在则新增）添加测试：

```rust
#[cfg(test)]
mod if_expr_tests {
    use super::*;
    use crate::ast::*;
    use crate::checker::env::TypeEnv;
    use crate::checker::types::*;

    fn span() -> SourceSpan {
        SourceSpan { file: "".into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 }
    }

    fn num_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, span: span() })
    }

    fn str_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::String, span: span() })
    }

    fn bool_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Boolean, span: span() })
    }

    fn if_expr(then: Expr, else_: Option<Expr>) -> Expr {
        Expr::If(IfExpr {
            condition: bool_expr(),
            then_branch: then,
            else_branch: else_,
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
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --package tangle-cli if_expr_tests -- --nocapture`
预期：`if_with_same_then_else_returns_type` 可能通过（当前返回 then_ty），但 `if_with_conflict_then_else_returns_then_type` 行为需验证。实际上当前代码也返回 then_ty，所以测试可能通过。关键是确认后续 Match 改动不破坏。

注意：当前 If 实现已返回 then_ty，这些测试可能直接通过。此任务的主要价值是为后续 Match 改动建立测试基线，并引入 `unify_pair` 使用。如果测试已通过，仍继续实现以引入 `unify_pair` 调用（为一致性和未来扩展）。

- [ ] **步骤 3：修改 Expr::If 分支使用 unify_pair**

修改 `compiler/tangle-cli/src/checker/check.rs:199-209`。找到：

```rust
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
```

替换为：

```rust
        Expr::If(e) => {
            let (_cond_ty, mut cond_diags) = check_expression(&e.condition, env);
            diags.append(&mut cond_diags);
            let (then_ty, mut then_diags) = check_expression(&e.then_branch, env);
            diags.append(&mut then_diags);
            let ty = if let Some(ref else_branch) = e.else_branch {
                let (else_ty, mut else_diags) = check_expression(else_branch, env);
                diags.append(&mut else_diags);
                crate::checker::unify::unify_pair(&then_ty, &else_ty)
                    .unwrap_or_else(|| then_ty.clone())
            } else {
                then_ty
            };
            ty
        }
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test --package tangle-cli if_expr_tests -- --nocapture`
预期：3 个测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/check.rs
git commit -m "feat(checker): If expression unifies then/else types via unify_pair"
```

---

## 任务 4：Rust — 改进 check.rs 的 Expr::Match 分支（arm 收窄）

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check.rs:228-246`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/checker/check.rs` 的 `#[cfg(test)]` 模块中添加测试：

```rust
#[cfg(test)]
mod match_narrowing_tests {
    use super::*;
    use crate::ast::*;
    use crate::checker::env::TypeEnv;
    use crate::checker::types::*;
    use std::collections::HashMap;

    fn span() -> SourceSpan {
        SourceSpan { file: "".into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 }
    }

    fn num_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, span: span() })
    }

    fn ident_expr(name: &str) -> Expr {
        Expr::Identifier(IdentifierExpr { name: name.to_string(), span: span() })
    }

    fn bool_expr() -> Expr {
        Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Boolean, span: span() })
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
            Type::Primitive(p) => assert_eq!(p.name, "Int", "expected Int, got {:?}", ty),
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
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --package tangle-cli match_narrowing_tests -- --nocapture`
预期：`match_narrows_generic_instance_binding` 和 `match_returns_unified_arm_types` FAIL（当前返回 Bool，不收窄）

- [ ] **步骤 3：实现 Match arm 收窄**

修改 `compiler/tangle-cli/src/checker/check.rs:228-246`。找到：

```rust
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
```

替换为：

```rust
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
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test --package tangle-cli match_narrowing_tests -- --nocapture`
预期：4 个测试 PASS

- [ ] **步骤 5：运行全量回归测试确保无破坏**

运行：`cargo test --package tangle-cli`
预期：所有现有测试 PASS（注意：可能有测试断言 Match 返回 Bool，需检查并更新）

若有测试因 Match 返回类型从 Bool 变为实际类型而失败，更新这些测试的断言。

- [ ] **步骤 6：Commit**

```bash
git add compiler/tangle-cli/src/checker/check.rs
git commit -m "feat(checker): Match expression narrows arm bindings and unifies arm types"
```

---

## 任务 5：Rust — 新建 infer_return_types.rs 模块

**文件：**
- 创建：`compiler/tangle-cli/src/checker/infer_return_types.rs`
- 修改：`compiler/tangle-cli/src/checker/mod.rs`

- [ ] **步骤 1：编写失败的单元测试**

创建 `compiler/tangle-cli/src/checker/infer_return_types.rs`，先写测试框架：

```rust
use std::collections::HashMap;
use crate::ast::{ParsedCodeBlock, Stmt};
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
    // 1. 构造与 check_module 一致的 block_env
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
        .as_ref()
        .and_then(crate::checker::resolve::type_name_to_type)
        .unwrap_or(Type::Any)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    #[test]
    fn unify_all_empty_returns_none() {
        // 直接测试 infer_function_return_type 在无 return 时返回 None
        // （通过构造空 CheckedModule，较复杂；此处用 unify_all 间接验证）
        assert!(unify_all(&[]).is_none());
    }

    // 完整的集成测试在 tests/v06_phase6/return_type_inference.rs 中
    // 此处仅验证核心逻辑的存在性
    #[test]
    fn infer_return_types_function_exists() {
        // 验证函数可被调用（编译通过）
        let checked = CheckedModule {
            file: "".into(),
            module_name: "".into(),
            imports: vec![],
            headings: vec![],
            symbols: vec![],
            diagnostics: vec![],
            parsed_blocks: vec![],
            type_env: TypeEnv {
                variables: HashMap::new(),
                structs: HashMap::new(),
                functions: HashMap::new(),
                interfaces: HashMap::new(),
                receiver: None,
                error_registry: None,
            },
            return_types: HashMap::new(),
        };
        let result = infer_return_types(&checked);
        assert!(result.is_empty());
    }
}
```

注意：`CheckedModule` 需要 `return_types` 字段（任务 6 添加）。此步骤会编译失败，这是预期的——TDD 流程。

- [ ] **步骤 2：注册模块**

修改 `compiler/tangle-cli/src/checker/mod.rs`，在 `pub mod unify;` 后添加：

```rust
pub mod infer_return_types;
```

并在 `pub use unify::*;` 后添加：

```rust
pub use infer_return_types::*;
```

- [ ] **步骤 3：运行测试验证失败**

运行：`cargo test --package tangle-cli infer_return_types`
预期：编译失败，报错 `CheckedModule` 没有 `return_types` 字段

- [ ] **步骤 4：临时注释掉测试，先完成任务 6 再回来**

由于 `CheckedModule` 需要 `return_types` 字段（任务 6 添加），此任务的测试暂时无法编译。继续任务 6 添加字段后，此任务的测试会自动通过。

跳过步骤 4-5，直接进入任务 6。此任务的代码已就位，待任务 6 完成后统一验证。

- [ ] **步骤 5：（延迟到任务 6 后）运行测试验证通过**

任务 6 完成后运行：`cargo test --package tangle-cli infer_return_types`
预期：2 个测试 PASS

- [ ] **步骤 6：（延迟到任务 6 后）Commit**

任务 6 完成后统一 commit。

---

## 任务 6：Rust — 扩展 CheckedModule + 集成 infer_return_types + compile_to_ir + go_emitter

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check_module.rs`
- 修改：`compiler/tangle-cli/src/ir/compile_to_ir.rs`
- 修改：`compiler/tangle-cli/src/codegen/go_emitter.rs`

- [ ] **步骤 1：扩展 CheckedModule 结构体**

修改 `compiler/tangle-cli/src/checker/check_module.rs:13-23`。找到：

```rust
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
}
```

替换为：

```rust
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
```

- [ ] **步骤 2：在 check_module 末尾调用 infer_return_types**

修改 `compiler/tangle-cli/src/checker/check_module.rs:175-185`。找到：

```rust
    CheckedModule {
        file: module.file,
        module_name: module.module_name,
        imports: module.imports,
        headings: module.headings,
        symbols: module.symbols,
        diagnostics,
        parsed_blocks,
        type_env,
    }
}
```

替换为：

```rust
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
```

- [ ] **步骤 3：更新 compile_to_ir 的 collect_functions**

修改 `compiler/tangle-cli/src/ir/compile_to_ir.rs:130-169`。找到 `collect_functions` 函数签名：

```rust
fn collect_functions(
    headings: &[TangleHeading],
    parent: Option<&TangleHeading>,
    parsed_blocks: &[ParsedCodeBlock],
    id_gen: &mut FreshNodeId,
    out: &mut Vec<IRFunction>,
) {
```

替换为：

```rust
fn collect_functions(
    headings: &[TangleHeading],
    parent: Option<&TangleHeading>,
    parsed_blocks: &[ParsedCodeBlock],
    id_gen: &mut FreshNodeId,
    return_types: &std::collections::HashMap<String, Type>,
    out: &mut Vec<IRFunction>,
) {
```

找到 `return_type: None,`（第 159 行）替换为：

```rust
                    return_type: return_types.get(&h.id).cloned(),
```

找到递归调用（第 167 行）：

```rust
        collect_functions(&h.children, Some(h), parsed_blocks, id_gen, out);
```

替换为：

```rust
        collect_functions(&h.children, Some(h), parsed_blocks, id_gen, return_types, out);
```

- [ ] **步骤 4：更新 compile_to_ir 主函数中的调用**

修改 `compiler/tangle-cli/src/ir/compile_to_ir.rs:78-82`。找到：

```rust
    if has_main {
        let mut functions: Vec<IRFunction> = vec![];
        collect_functions(&checked.headings, None, &checked.parsed_blocks, &mut id_gen, &mut functions);
        graph.functions = functions;
    }
```

替换为：

```rust
    if has_main {
        let mut functions: Vec<IRFunction> = vec![];
        collect_functions(&checked.headings, None, &checked.parsed_blocks, &mut id_gen, &checked.return_types, &mut functions);
        graph.functions = functions;
    }
```

- [ ] **步骤 5：添加必要的 use 语句**

检查 `compiler/tangle-cli/src/ir/compile_to_ir.rs` 顶部是否已导入 `Type`。若无，在现有 use 语句后添加：

```rust
use crate::checker::types::Type;
```

- [ ] **步骤 6：修复 go_emitter.rs 隐患**

修改 `compiler/tangle-cli/src/codegen/go_emitter.rs:270-271`。找到：

```rust
        // 返回类型：return_type 为 None 时用 Result（现有约定），Phase 6c 实现返回推导后可覆盖
        let ret_ty = func.return_type.as_ref().map(tangle_type_to_go).unwrap_or_else(|| "Result".into());
```

替换为：

```rust
        // Phase 6c 设计 A：外部签名恒为 Result，return_type 仅作 IR 元数据
        let ret_ty = "Result";
```

- [ ] **步骤 7：运行任务 5 的测试验证通过**

运行：`cargo test --package tangle-cli infer_return_types`
预期：2 个测试 PASS

- [ ] **步骤 8：运行全量编译检查**

运行：`cargo build --package tangle-cli`
预期：编译成功，零错误

- [ ] **步骤 9：运行全量测试**

运行：`cargo test --workspace`
预期：所有测试 PASS（若有测试因 `CheckedModule` 新增字段或 `collect_functions` 签名变更而失败，更新这些测试）

常见失败点：
- 其他地方构造 `CheckedModule { ... }` 未包含 `return_types` 字段 → 添加 `return_types: HashMap::new()`
- 其他地方调用 `collect_functions(...)` 未传入 `return_types` → 传入 `&HashMap::new()` 或 `&checked.return_types`

- [ ] **步骤 10：Commit**

```bash
git add compiler/tangle-cli/src/checker/check_module.rs
git add compiler/tangle-cli/src/checker/infer_return_types.rs
git add compiler/tangle-cli/src/checker/mod.rs
git add compiler/tangle-cli/src/ir/compile_to_ir.rs
git add compiler/tangle-cli/src/codegen/go_emitter.rs
git commit -m "feat(checker,ir,codegen): integrate return type inference into CheckedModule and IR"
```

---

## 任务 7：Rust — 创建 fixtures + 集成测试

**文件：**
- 创建：`tests/v06_phase6/return_inference.tangle.md`
- 创建：`tests/v06_phase6/match_narrowing.tangle.md`
- 创建：`tests/v06_phase6/return_conflict.tangle.md`
- 创建：`compiler/tangle-cli/tests/v06_phase6/return_type_inference.rs`

- [ ] **步骤 1：创建 return_inference.tangle.md fixture**

写入 `tests/v06_phase6/return_inference.tangle.md`：

```markdown
# ReturnInferenceTest

### ItemProcessor

#### process
* `items`: List of items (List<Int>)

```@tangle
return items
```

#### main

```@tangle
return 0
```
```

- [ ] **步骤 2：创建 match_narrowing.tangle.md fixture**

写入 `tests/v06_phase6/match_narrowing.tangle.md`：

```markdown
# MatchNarrowingTest

### process
* `input`: Optional value (Some<Int> | None)

```@tangle
match input {
  Some(y) => return y
  None => return 0
}
```

### main

```@tangle
return 0
```
```

- [ ] **步骤 3：创建 return_conflict.tangle.md fixture**

写入 `tests/v06_phase6/return_conflict.tangle.md`：

```markdown
# ReturnConflictTest

### process
* `input`: Input value (Int | String)

```@tangle
match input {
  Int(x) => return x
  String(s) => return s
}
```

### main

```@tangle
return 0
```
```

- [ ] **步骤 4：创建集成测试文件**

写入 `compiler/tangle-cli/tests/v06_phase6/return_type_inference.rs`：

```rust
//! Phase 6c: Return type inference + Match arm narrowing integration tests.
//!
//! Verifies that `IRFunction.return_type` is populated correctly from
//! return statements and Match arm narrowing.

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::checker::types::Type;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join("v06_phase6")
        .join(name)
}

// --- Fixture 1: return_inference.tangle.md ---

#[test]
fn test_return_inference_process_returns_list_int() {
    let path = fixture_path("return_inference.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should have return_type");

    match ty {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List", "process return type base should be List");
            assert_eq!(g.args.len(), 1, "List should have 1 type arg");
            match &g.args[0] {
                Type::Primitive(p) => assert_eq!(p.name, "Int", "List arg should be Int"),
                other => panic!("expected Int, got {:?}", other),
            }
        }
        other => panic!("expected GenericInstance List<Int>, got {:?}", other),
    }
}

#[test]
fn test_return_inference_main_returns_int() {
    let path = fixture_path("return_inference.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let main = graph
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("fixture should define main");

    let ty = main
        .return_type
        .as_ref()
        .expect("main should have return_type");

    match ty {
        Type::Primitive(p) => assert_eq!(p.name, "Int", "main return type should be Int"),
        other => panic!("expected Int, got {:?}", other),
    }
}

// --- Fixture 2: match_narrowing.tangle.md ---

#[test]
fn test_match_narrowing_process_returns_int() {
    let path = fixture_path("match_narrowing.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should have return_type");

    match ty {
        Type::Primitive(p) => assert_eq!(p.name, "Int", "process return type should be Int"),
        other => panic!("expected Int (from narrowed match arms), got {:?}", other),
    }
}

// --- Fixture 3: return_conflict.tangle.md ---

#[test]
fn test_return_conflict_process_returns_any() {
    let path = fixture_path("return_conflict.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should have return_type");

    assert!(
        matches!(ty, Type::Any),
        "expected Any (conflict Int vs String), got {:?}",
        ty
    );
}

// --- Existing fixture: generics.tangle.md (Phase 6b) should now have return_type ---

#[test]
fn test_generics_fixture_process_returns_list_int() {
    let path = fixture_path("generics.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should now have return_type (Phase 6c)");

    match ty {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List", "generics fixture process should return List<Int>");
        }
        other => panic!("expected GenericInstance, got {:?}", other),
    }
}
```

- [ ] **步骤 5：运行集成测试验证通过**

运行：`cargo test --package tangle-cli --test return_type_inference -- --nocapture`
预期：5 个测试 PASS

若失败，检查：
- fixture 语法是否正确（Sum type `Some<Int> | None` 是否被 type_parser 支持）
- `run_collecting_ir` 是否暴露在 `tangle_cli::audit_support`

- [ ] **步骤 6：运行 clippy 检查**

运行：`cargo clippy --package tangle-cli --all-targets -- -D warnings`
预期：零警告

- [ ] **步骤 7：Commit**

```bash
git add tests/v06_phase6/return_inference.tangle.md
git add tests/v06_phase6/match_narrowing.tangle.md
git add tests/v06_phase6/return_conflict.tangle.md
git add compiler/tangle-cli/tests/v06_phase6/return_type_inference.rs
git commit -m "test(phase6c): add return type inference + match narrowing fixtures and integration tests"
```

---

## 任务 8：TS — 同步 check.ts（Match/If 改进）+ match.ts

**文件：**
- 修改：`reference/src/checker/match.ts`
- 修改：`reference/src/checker/check.ts`
- 修改：`reference/src/checker/unify.ts`

- [ ] **步骤 1：扩展 match.ts（新增 bindingTypeOf / findVariantByName，扩展 getVariantName）**

读取当前 `reference/src/checker/match.ts`（已在上下文中）。替换全部内容为：

```typescript
import type { Type } from "./types.js";

/// 提取 variant 名（Primitive/Struct/Interface/GenericInstance）。
/// 其他类型不支持作为命名 variant。
export function getVariantName(t: Type): string | null {
  if (t.kind === "struct") return t.name;
  if (t.kind === "primitive") return t.name;
  if (t.kind === "interface") return t.name;
  if (t.kind === "genericInstance") return t.base;
  return null;
}

/// 提取 binding 类型。
/// GenericInstance 返回 args[0]（payload）；其他返回 variant 类型本身。
export function bindingTypeOf(variantType: Type): Type {
  if (variantType.kind === "genericInstance") {
    return variantType.args[0] ?? { kind: "any" };
  }
  return variantType;
}

/// 在 Sum 的 variants 中按名查找。
export function findVariantByName(sumType: Type, name: string): Type | null {
  if (sumType.kind !== "sum") return null;
  for (const v of sumType.variants) {
    const vname = getVariantName(v);
    if (vname === name) return v;
  }
  return null;
}

/// Check match exhaustiveness. Returns list of uncovered variant names.
/// 支持 Primitive/Struct/Interface/GenericInstance variant。
export function checkMatchExhaustiveness(sumType: Type, armPatterns: string[]): string[] {
  if (sumType.kind !== "sum") return [];
  const variantNames = sumType.variants
    .map((v) => getVariantName(v))
    .filter((n): n is string => n !== null);
  const covered = new Set<string>();
  let hasWildcard = false;
  for (const pattern of armPatterns) {
    if (pattern === "_") {
      hasWildcard = true;
      break;
    }
    covered.add(pattern);
  }
  if (hasWildcard) return [];
  return variantNames.filter((n) => !covered.has(n));
}
```

- [ ] **步骤 2：扩展 unify.ts（新增 unifyAll / unifyPair）**

读取当前 `reference/src/checker/unify.ts` 末尾，在文件末尾添加：

```typescript
/// 统一类型列表：以第一个为锚点，逐个 unify。
/// 成功返回统一后的类型（含 type_var 替换）；失败返回 null。
/// 用于 return 路径类型统一、Match arm body 类型统一。
export function unifyAll(types: Type[]): Type | null {
  if (types.length === 0) return null;
  const subst: Substitution = new Map();
  const anchor = types[0]!;
  for (let i = 1; i < types.length; i++) {
    const result = unify(anchor, types[i]!, subst);
    if (!result.ok) return null;
  }
  return substitute(anchor, subst);
}

/// 统一两个类型（用于 If then/else 分支统一）。
/// 成功返回统一后的类型；失败返回 null。
export function unifyPair(a: Type, b: Type): Type | null {
  const subst: Substitution = new Map();
  const result = unify(a, b, subst);
  if (!result.ok) return null;
  return substitute(a, subst);
}
```

注意：需确认 `reference/src/checker/unify.ts` 中 `unify` 和 `substitute` 的签名。若 `unify` 返回 `Result` 类型而非 `{ ok: boolean }`，调整代码匹配实际签名。

- [ ] **步骤 3：修改 check.ts 的 If case**

读取 `reference/src/checker/check.ts:139-149`（已在上下文中）。找到 `case "if":` 块：

```typescript
    case "if": {
      const [, condDiags] = checkExpression(expr.condition, env);
      diags.push(...condDiags);
      const [thenType, thenDiags] = checkExpression(expr.thenBranch, env);
      diags.push(...thenDiags);
      if (expr.elseBranch) {
        const [, elseDiags] = checkExpression(expr.elseBranch, env);
        diags.push(...elseDiags);
      }
      return [thenType, diags];
    }
```

替换为：

```typescript
    case "if": {
      const [, condDiags] = checkExpression(expr.condition, env);
      diags.push(...condDiags);
      const [thenType, thenDiags] = checkExpression(expr.thenBranch, env);
      diags.push(...thenDiags);
      if (expr.elseBranch) {
        const [elseType, elseDiags] = checkExpression(expr.elseBranch, env);
        diags.push(...elseDiags);
        const unified = unifyPair(thenType, elseType);
        return [unified ?? thenType, diags];
      }
      return [thenType, diags];
    }
```

在 `check.ts` 顶部添加导入（若尚未导入）：

```typescript
import { unifyPair } from "./unify.js";
```

- [ ] **步骤 4：修改 check.ts 的 Match case**

找到 `case "match":` 块（约第 172-197 行）：

```typescript
    case "match": {
      const [matchType, matchDiags] = checkExpression(expr.expr, env);
      diags.push(...matchDiags);
      // Type-check each arm body
      let resultType: Type = { kind: "primitive", name: "Bool" };
      for (const arm of expr.arms) {
        const [armType, armDiags] = checkExpression(arm.body, env);
        diags.push(...armDiags);
        resultType = armType; // last arm type wins (simplified)
      }
      // Check exhaustiveness
      const patternNames = expr.arms.map((a) =>
        a.pattern.kind === "variantPattern" ? a.pattern.name : "_"
      );
      if (matchType.kind === "sum") {
        const missing = checkMatchExhaustiveness(matchType, patternNames);
        for (const m of missing) {
          diags.push({
            code: "TANGLE_TYPE_MATCH_NOT_EXHAUSTIVE",
            message: `Missing match arm for variant: ${m}`,
            span: expr.span,
          });
        }
      }
      return [resultType, diags];
    }
```

替换为：

```typescript
    case "match": {
      const [matchType, matchDiags] = checkExpression(expr.expr, env);
      diags.push(...matchDiags);
      // Type-check each arm body with narrowed binding type
      const armTypes: Type[] = [];
      for (const arm of expr.arms) {
        // 构造收窄后的 arm 局部环境
        const armEnv: TypeEnv = {
          ...env,
          variables: { ...env.variables },
        };
        if (matchType.kind === "sum" && arm.pattern.kind === "variantPattern") {
          const variant = findVariantByName(matchType, arm.pattern.name);
          if (variant && arm.pattern.binding) {
            const bindType = bindingTypeOf(variant);
            armEnv.variables[arm.pattern.binding] = bindType;
          }
        }
        const [armType, armDiags] = checkExpression(arm.body, armEnv);
        diags.push(...armDiags);
        armTypes.push(armType);
      }
      // Check exhaustiveness
      const patternNames = expr.arms.map((a) =>
        a.pattern.kind === "variantPattern" ? a.pattern.name : "_"
      );
      if (matchType.kind === "sum") {
        const missing = checkMatchExhaustiveness(matchType, patternNames);
        for (const m of missing) {
          diags.push({
            code: "TANGLE_TYPE_MATCH_NOT_EXHAUSTIVE",
            message: `Missing match arm for variant: ${m}`,
            span: expr.span,
          });
        }
      }
      // 返回所有 arm body 类型的统一结果（最佳努力，失败回退 Any）
      const resultType = unifyAll(armTypes) ?? { kind: "any" as const };
      return [resultType, diags];
    }
```

在 `check.ts` 顶部添加导入：

```typescript
import { findVariantByName, bindingTypeOf } from "./match.js";
import { unifyAll } from "./unify.js";
```

注意：需确认 `TypeEnv` 类型是否已导入，以及 `env.variables` 的结构。若 `TypeEnv` 不可 spread，改用 `cloneEnv(env)` 辅助函数。

- [ ] **步骤 5：运行 TS 编译检查**

运行：`cd reference && npm run build`
预期：编译成功，零类型错误

常见错误：
- `Property 'binding' does not exist on variantPattern` → 检查 `MatchPattern` 类型定义，确认 binding 字段存在
- `Type 'TypeEnv' is not assignable` → 环境克隆方式需调整

- [ ] **步骤 6：运行 TS 现有测试**

运行：`cd reference && npm test`
预期：所有现有测试 PASS（可能有测试断言 Match 返回 Bool，需更新）

- [ ] **步骤 7：Commit**

```bash
git add reference/src/checker/match.ts
git add reference/src/checker/check.ts
git add reference/src/checker/unify.ts
git commit -m "feat(reference): sync Match arm narrowing + If unify to TS"
```

---

## 任务 9：TS — 新建 inferReturnTypes.ts + 集成到 checkModule.ts + compileToIR.ts

**文件：**
- 创建：`reference/src/checker/inferReturnTypes.ts`
- 修改：`reference/src/checker/checkModule.ts`
- 修改：`reference/src/ir/compileToIR.ts`

- [ ] **步骤 1：创建 inferReturnTypes.ts**

写入 `reference/src/checker/inferReturnTypes.ts`：

```typescript
import type { TangleHeading, TangleParam } from "../model.js";
import type { Type, TypeEnv } from "./types.js";
import type { ParsedCodeBlock } from "../ast.js";
import { checkExpression } from "./check.js";
import { findReceiverHeading, typeNameToType } from "./resolve.js";
import { unifyAll } from "./unify.js";

export interface ReturnInferenceInput {
  headings: TangleHeading[];
  parsedBlocks: ParsedCodeBlock[];
  typeEnv: TypeEnv;
}

/// 为模块中所有 Callable heading 推断返回类型。
/// 返回 heading_id → Type 映射。
export function inferReturnTypes(input: ReturnInferenceInput): Map<string, Type> {
  const result = new Map<string, Type>();
  collect(input.headings, input, result);
  return result;
}

function collect(headings: TangleHeading[], input: ReturnInferenceInput, out: Map<string, Type>): void {
  for (const h of headings) {
    if (h.role === "callable" && (h.codeBlocks ?? []).length > 0) {
      const ty = inferFunctionReturnType(h, input);
      if (ty) {
        out.set(h.id, ty);
      }
    }
    collect(h.children ?? [], input, out);
  }
}

function inferFunctionReturnType(
  heading: TangleHeading,
  input: ReturnInferenceInput,
): Type | null {
  // 1. 构造与 checkModule 一致的 env
  const env: TypeEnv = {
    ...input.typeEnv,
    variables: { ...input.typeEnv.variables },
  };
  setupReceiverAndParams(heading, input, env);

  // 2. 遍历该 heading 的所有 @tangle blocks，收集 return 类型
  const returnTypes: Type[] = [];
  for (const block of input.parsedBlocks) {
    if (block.headingId !== heading.id) continue;
    const blockEnv: TypeEnv = {
      ...env,
      variables: { ...env.variables },
    };
    for (const stmt of block.body.statements) {
      if (stmt.kind === "let" || stmt.kind === "const") {
        const [ty] = checkExpression(stmt.value, blockEnv);
        blockEnv.variables[stmt.name] = ty;
      } else if (stmt.kind === "return" && stmt.value) {
        const [ty] = checkExpression(stmt.value, blockEnv);
        returnTypes.push(ty);
      }
    }
  }

  // 3. 统一所有 return 类型
  if (returnTypes.length === 0) return null;
  return unifyAll(returnTypes) ?? { kind: "any" };
}

function setupReceiverAndParams(
  heading: TangleHeading,
  input: ReturnInferenceInput,
  env: TypeEnv,
): void {
  const parent = findReceiverHeading(heading, input.headings);
  if (parent) {
    const structName = parent.symbolName ?? parent.title;
    const fields = new Map<string, Type>();
    for (const p of parent.params ?? []) {
      fields.set(p.name, paramTypeOf(p));
    }
    env.receiver = { structName, fields };
  }
  for (const p of heading.params ?? []) {
    env.variables[p.name] = paramTypeOf(p);
  }
}

function paramTypeOf(p: TangleParam): Type {
  if (!p.typeName) return { kind: "any" };
  const ty = typeNameToType(p.typeName);
  return ty ?? { kind: "any" };
}
```

注意：
- 需确认 `TypeEnv` 的 `variables` 是 `Map` 还是 `Record`；若是 `Record`，改用 `{ ...env.variables }`
- 需确认 `typeNameToType` 是否在 `resolve.ts` 中导出；若名为 `typeExprToType`，调整导入
- 需确认 `TangleParam` 类型名；可能是 `TangleHeadingParam`

- [ ] **步骤 2：修改 checkModule.ts 集成 inferReturnTypes**

读取 `reference/src/checker/checkModule.ts`（已在上下文中）。找到 `CheckedModule` 类型定义（第 13-16 行）：

```typescript
export type CheckedModule = TangleModule & {
  parsedBlocks: ParsedCodeBlock[];
  typeEnv: TypeEnv;
};
```

替换为：

```typescript
export type CheckedModule = TangleModule & {
  parsedBlocks: ParsedCodeBlock[];
  typeEnv: TypeEnv;
  returnTypes: Map<string, Type>;
};
```

在顶部添加导入：

```typescript
import type { Type } from "./types.js";
import { inferReturnTypes } from "./inferReturnTypes.js";
```

找到 `checkModule` 函数末尾（约第 100-106 行）：

```typescript
  return {
    ...module,
    parsedBlocks,
    typeEnv: env,
    diagnostics: allDiagnostics
  };
}
```

替换为：

```typescript
  const returnTypes = inferReturnTypes({
    headings: module.headings,
    parsedBlocks,
    typeEnv: env,
  });

  return {
    ...module,
    parsedBlocks,
    typeEnv: env,
    returnTypes,
    diagnostics: allDiagnostics
  };
}
```

- [ ] **步骤 3：修改 compileToIR.ts 读取 returnTypes**

读取 `reference/src/ir/compileToIR.ts:216-244`（已在上下文中）。找到 `collectFunctions` 函数签名：

```typescript
function collectFunctions(
  headings: TangleHeading[],
  parent: TangleHeading | null,
  parsedBlocks: ParsedCodeBlock[],
  out: IRFunction[],
): void {
```

替换为：

```typescript
function collectFunctions(
  headings: TangleHeading[],
  parent: TangleHeading | null,
  parsedBlocks: ParsedCodeBlock[],
  returnTypes: Map<string, Type>,
  out: IRFunction[],
): void {
```

找到 `returnType: undefined,`（第 240 行）替换为：

```typescript
      returnType: returnTypes.get(h.id) ?? undefined,
```

找到递归调用（第 242 行）：

```typescript
    collectFunctions(h.children, h, parsedBlocks, out);
```

替换为：

```typescript
    collectFunctions(h.children, h, parsedBlocks, returnTypes, out);
```

找到 `compileToIR` 主函数中调用 `collectFunctions` 的地方，添加 `returnTypes` 参数。查找类似：

```typescript
    collectFunctions(checked.headings, null, checked.parsedBlocks, functions);
```

替换为：

```typescript
    collectFunctions(checked.headings, null, checked.parsedBlocks, checked.returnTypes, functions);
```

在 `compileToIR.ts` 顶部添加导入：

```typescript
import type { Type } from "../checker/types.js";
```

- [ ] **步骤 4：运行 TS 编译检查**

运行：`cd reference && npm run build`
预期：编译成功

- [ ] **步骤 5：运行 TS 现有测试**

运行：`cd reference && npm test`
预期：所有现有测试 PASS

- [ ] **步骤 6：Commit**

```bash
git add reference/src/checker/inferReturnTypes.ts
git add reference/src/checker/checkModule.ts
git add reference/src/ir/compileToIR.ts
git commit -m "feat(reference): add inferReturnTypes pass and integrate into checkModule/compileToIR"
```

---

## 任务 10：TS — 单元测试

**文件：**
- 创建：`reference/tests/checker/inferReturnTypes.test.ts`
- 创建：`reference/tests/checker/matchNarrowing.test.ts`

- [ ] **步骤 1：创建 inferReturnTypes.test.ts**

写入 `reference/tests/checker/inferReturnTypes.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { unifyAll, unifyPair } from "../../src/checker/unify.js";
import type { Type } from "../../src/checker/types.js";

function prim(name: string): Type {
  return { kind: "primitive", name };
}

function listInt(): Type {
  return { kind: "genericInstance", base: "List", args: [prim("Int")] };
}

describe("unifyAll", () => {
  it("returns null for empty array", () => {
    expect(unifyAll([])).toBeNull();
  });

  it("returns single type", () => {
    expect(unifyAll([prim("Int")])).toEqual(prim("Int"));
  });

  it("unifies same types", () => {
    expect(unifyAll([prim("Int"), prim("Int"), prim("Int")])).toEqual(prim("Int"));
  });

  it("returns null on conflict", () => {
    expect(unifyAll([prim("Int"), prim("String")])).toBeNull();
  });

  it("unifies with Any", () => {
    expect(unifyAll([prim("Int"), { kind: "any" }])).toEqual(prim("Int"));
  });

  it("unifies generic instances", () => {
    expect(unifyAll([listInt(), listInt()])).toEqual(listInt());
  });
});

describe("unifyPair", () => {
  it("unifies same types", () => {
    expect(unifyPair(prim("Int"), prim("Int"))).toEqual(prim("Int"));
  });

  it("returns null on conflict", () => {
    expect(unifyPair(prim("Int"), prim("String"))).toBeNull();
  });

  it("unifies with Any", () => {
    expect(unifyPair(prim("Int"), { kind: "any" })).toEqual(prim("Int"));
  });
});
```

- [ ] **步骤 2：创建 matchNarrowing.test.ts**

写入 `reference/tests/checker/matchNarrowing.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import {
  getVariantName,
  bindingTypeOf,
  findVariantByName,
  checkMatchExhaustiveness,
} from "../../src/checker/match.js";
import type { Type } from "../../src/checker/types.js";

function prim(name: string): Type {
  return { kind: "primitive", name };
}

function generic(base: string, args: Type[]): Type {
  return { kind: "genericInstance", base, args };
}

function structType(name: string): Type {
  return { kind: "struct", name, fields: new Map(), methods: new Map() };
}

describe("getVariantName", () => {
  it("returns name for primitive", () => {
    expect(getVariantName(prim("Int"))).toBe("Int");
  });

  it("returns name for struct", () => {
    expect(getVariantName(structType("Order"))).toBe("Order");
  });

  it("returns base for genericInstance", () => {
    expect(getVariantName(generic("Some", [prim("Int")]))).toBe("Some");
  });

  it("returns null for sum", () => {
    expect(getVariantName({ kind: "sum", variants: [] })).toBeNull();
  });
});

describe("bindingTypeOf", () => {
  it("returns payload for genericInstance", () => {
    expect(bindingTypeOf(generic("Some", [prim("Int")]))).toEqual(prim("Int"));
  });

  it("returns any for genericInstance without args", () => {
    expect(bindingTypeOf(generic("Some", []))).toEqual({ kind: "any" });
  });

  it("returns itself for primitive", () => {
    expect(bindingTypeOf(prim("Int"))).toEqual(prim("Int"));
  });
});

describe("findVariantByName", () => {
  it("finds variant by name", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(findVariantByName(sum, "String")).toEqual(prim("String"));
  });

  it("returns null when not found", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int")] };
    expect(findVariantByName(sum, "Bool")).toBeNull();
  });
});

describe("checkMatchExhaustiveness", () => {
  it("all covered", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(checkMatchExhaustiveness(sum, ["Int", "String"])).toEqual([]);
  });

  it("missing variant", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(checkMatchExhaustiveness(sum, ["Int"])).toEqual(["String"]);
  });

  it("wildcard covers all", () => {
    const sum: Type = { kind: "sum", variants: [prim("Int"), prim("String")] };
    expect(checkMatchExhaustiveness(sum, ["Int", "_"])).toEqual([]);
  });

  it("genericInstance variant", () => {
    const sum: Type = {
      kind: "sum",
      variants: [generic("Some", [prim("Int")]), prim("None")],
    };
    expect(checkMatchExhaustiveness(sum, ["Some", "None"])).toEqual([]);
  });
});
```

- [ ] **步骤 3：运行 TS 测试验证通过**

运行：`cd reference && npm test`
预期：所有测试 PASS（含新测试）

- [ ] **步骤 4：Commit**

```bash
git add reference/tests/checker/inferReturnTypes.test.ts
git add reference/tests/checker/matchNarrowing.test.ts
git commit -m "test(reference): add TS unit tests for return type inference and match narrowing"
```

---

## 任务 11：出口闸门验证

- [ ] **步骤 1：Rust 全量测试**

运行：`cargo test --workspace`
预期：全绿

若有失败，根据错误信息修复。

- [ ] **步骤 2：Rust clippy 检查**

运行：`cargo clippy --workspace --all-targets -- -D warnings`
预期：零警告

- [ ] **步骤 3：TS 编译检查**

运行：`cd reference && npm run build`
预期：零类型错误

- [ ] **步骤 4：TS 测试**

运行：`cd reference && npm test`
预期：全绿

- [ ] **步骤 5：差分测试**

运行：`pwsh tests/audit/diff-ir.ps1`
预期：**15 MATCH + 0 SKIPPED + 0 DIFF**

若出现 DIFF：
- 检查 Rust/TS 是否在某个 fixture 上推断出不同类型
- 用 `cargo run -- build <fixture> --emit-ir` 和 `cd reference && node dist/cli/main.js build <fixture> --emit-ir` 对比输出
- 定位不一致点并修复

若出现 SKIPPED：
- 检查 fixture 语法是否被 TS 端正确解析
- 检查是否触发 TS fatal exit

- [ ] **步骤 6：审计脚本**

运行：`pwsh tests/audit/run-audit.ps1`
预期：0 failing

- [ ] **步骤 7：手动验证 IR JSON**

运行：`cargo run -- build tests/v06_phase6/match_narrowing.tangle.md --emit-ir`
预期：输出 JSON 中 `process` 函数包含 `"returnType":{"kind":"primitive","name":"Int"}`

- [ ] **步骤 8：回归测试确认**

确认以下回归测试通过：
- Phase 4（tests/v04_phase4/）
- Phase 5（tests/v05_phase5/）
- Phase 6a（compiler/tangle-cli/tests/ 中相关文件）
- Phase 6b（compiler/tangle-cli/tests/v06_phase6/）

- [ ] **步骤 9：Commit 闸门验证结果（可选）**

若所有闸门通过，可创建一个标记 commit：

```bash
git commit --allow-empty -m "chore(phase6c): exit gate verified — 15 MATCH + 0 SKIPPED + 0 DIFF"
```

- [ ] **步骤 10：打 tag v0.7.0（等待用户批准）**

```bash
git tag v0.7.0
```

**不要 push tag 到 remote。** 等待用户批准后再 push。

---

## 完成后

1. 通知用户所有出口闸门已通过
2. 提供 tag push 命令：`git push origin v0.7.0`
3. 询问是否需要合并到 main（若在 worktree 中工作）
4. 更新 project_memory.md 记录 Phase 6c 完成状态
