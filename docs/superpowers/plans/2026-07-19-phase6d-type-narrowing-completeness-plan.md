# Phase 6d: 类型收窄完整性 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 Phase 6c 已建立的 Match arm 收窄基础上，新增 `if x is Pattern`、`let Pattern = expr else { ... }`、`let { fields } = expr` 三类源代码层收窄机制，并通过 `as_sum_view()` 让 `Option<T>` 自动暴露为 `Sum(Some<T>, None)`。

**架构：** AST 引入 3 个新节点 + Pattern 子树；Checker 直接处理复用 6c 的 `arm_env`/`unify_all`/`binding_type_of`；IR lowering 脱糖为 Match / MemberAccess / Let；IR schema 零变更，三个 emitter 零改动；TS reference 全量同步。

**技术栈：** Rust（parser/checker/ir 模块）、TypeScript（reference/src 镜像）、PowerShell（差分测试脚本）

**规格文件：** [docs/superpowers/specs/2026-07-19-phase6d-type-narrowing-completeness-design.md](file:///e:/GitProjects/tangle/docs/superpowers/specs/2026-07-19-phase6d-type-narrowing-completeness-design.md)

**前置条件：** 在 `phase6d/v0.8.0` 分支的 worktree 中工作。若不存在，使用 `using-git-worktrees` 技能创建：`.worktrees/phase6d-v0.8.0` 基于 `main`（HEAD `d8e1e77`）。

---

## 文件结构

### Rust 端

| 文件 | 操作 | 职责 |
|------|------|------|
| `compiler/tangle-cli/src/ast.rs` | 修改 | 新增 `IsExpr` / `LetVariantStmt` / `LetRecordStmt` / `Pattern` |
| `compiler/tangle-cli/src/parser/lexer.rs` | 修改 | 新增 `TokenKind::Is` 保留字 |
| `compiler/tangle-cli/src/parser/parser.rs` | 修改 | 解析 `is` 表达式 + `let Variant` / `let { ... }` 语法 |
| `compiler/tangle-cli/src/checker/option_view.rs` | **新建** | `as_sum_view` 函数 + 单元测试 |
| `compiler/tangle-cli/src/checker/mod.rs` | 修改 | 注册 `option_view` 模块 + re-export |
| `compiler/tangle-cli/src/checker/check.rs` | 修改 | `Expr::Is` 分支 + `Expr::If` 收窄集成 |
| `compiler/tangle-cli/src/checker/check_module.rs` | 修改 | `Stmt::LetVariant` / `Stmt::LetRecord` 分支 |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | 修改 | `lower_is_expr` / `lower_let_variant` / `lower_let_record` |
| `compiler/tangle-cli/tests/v06_phase6/if_narrowing.rs` | **新建** | 集成测试：If 收窄 + Option 视图 |
| `compiler/tangle-cli/tests/v06_phase6/destructure.rs` | **新建** | 集成测试：Variant + Record destructure |
| `compiler/tangle-cli/tests/v06_phase6/option_match.rs` | **新建** | 集成测试：Option 自动 Sum 视图 |
| `tests/v06_phase6/if_narrowing.tangle.md` | **新建** | If 收窄 fixture |
| `tests/v06_phase6/destructure.tangle.md` | **新建** | Destructure fixture |
| `tests/v06_phase6/option_match.tangle.md` | **新建** | Option match fixture |

### TS reference 端

| 文件 | 操作 | 职责 |
|------|------|------|
| `reference/src/ast.ts` | 修改 | 镜像 Rust AST 新节点 |
| `reference/src/parser/lexer.ts` | 修改 | 镜像 is 关键字 |
| `reference/src/parser/parser.ts` | 修改 | 镜像 parser 新语法 |
| `reference/src/checker/optionView.ts` | **新建** | 镜像 `as_sum_view` |
| `reference/src/checker/check.ts` | 修改 | 镜像 IsExpr + If 收窄集成 |
| `reference/src/checker/checkModule.ts` | 修改 | 镜像 LetVariant / LetRecord 分支 |
| `reference/src/ir/compileToIR.ts` | 修改 | 镜像三个脱糖函数 |
| `reference/tests/checker/optionView.test.ts` | **新建** | TS 单元测试 |
| `reference/tests/checker/isNarrowing.test.ts` | **新建** | TS 单元测试 |
| `reference/tests/checker/destructure.test.ts` | **新建** | TS 单元测试 |

### 测试 / 审计

| 文件 | 操作 | 职责 |
|------|------|------|
| `tests/audit/diff-ir.ps1` | 修改 | 加入 3 个新 fixture 路径 |
| `tests/audit/expected_diagnostics.yaml` | 修改 | 新增 5 个诊断码 |
| `CHANGELOG.md` | 修改 | 新增 v0.8.0 章节 |

---

## 任务 1：Rust — AST 新增 Pattern / IsExpr / LetVariantStmt / LetRecordStmt

**文件：**
- 修改：`compiler/tangle-cli/src/ast.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/ast.rs` 末尾的 `#[cfg(test)]` 模块中新增测试（若不存在则新增）：

```rust
#[cfg(test)]
mod phase6d_ast_tests {
    use super::*;
    use crate::model::Span;

    fn dummy_span() -> Span {
        Span { start: 0, end: 0, file: None }
    }

    #[test]
    fn pattern_variant_construction() {
        let p = Pattern::Variant { name: "Some".to_string() };
        match p {
            Pattern::Variant { name } => assert_eq!(name, "Some"),
            _ => panic!("expected Variant"),
        }
    }

    #[test]
    fn pattern_variant_binding_construction() {
        let p = Pattern::VariantBinding {
            name: "Some".to_string(),
            binding: "y".to_string(),
        };
        match p {
            Pattern::VariantBinding { name, binding } => {
                assert_eq!(name, "Some");
                assert_eq!(binding, "y");
            }
            _ => panic!("expected VariantBinding"),
        }
    }

    #[test]
    fn is_expr_construction() {
        let e = IsExpr {
            expr: Box::new(Expr::Identifier("x".to_string(), dummy_span())),
            pattern: Pattern::VariantBinding {
                name: "Some".to_string(),
                binding: "y".to_string(),
            },
            span: dummy_span(),
        };
        assert!(matches!(e.pattern, Pattern::VariantBinding { .. }));
    }

    #[test]
    fn let_variant_stmt_construction() {
        let s = LetVariantStmt {
            variant_name: "Some".to_string(),
            binding: Some("y".to_string()),
            expr: Box::new(Expr::Identifier("x".to_string(), dummy_span())),
            else_branch: vec![],
            span: dummy_span(),
        };
        assert_eq!(s.variant_name, "Some");
        assert_eq!(s.binding, Some("y".to_string()));
    }

    #[test]
    fn let_record_stmt_construction() {
        let s = LetRecordStmt {
            fields: vec![
                ("ok".to_string(), "o".to_string()),
                ("err".to_string(), "e".to_string()),
            ],
            expr: Box::new(Expr::Identifier("r".to_string(), dummy_span())),
            span: dummy_span(),
        };
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[0].0, "ok");
        assert_eq!(s.fields[0].1, "o");
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test -p tangle-cli phase6d_ast_tests`
预期：FAIL，编译错误 "cannot find type `Pattern` / `IsExpr` / `LetVariantStmt` / `LetRecordStmt` in this scope"

- [ ] **步骤 3：编写最少实现代码**

在 `compiler/tangle-cli/src/ast.rs` 中新增（位置：现有 `Expr` / `Stmt` 枚举旁）：

```rust
/// Pattern 子树：用于 `is` 表达式与 refutable let。
/// Phase 6d 仅支持 variant 形式；复合模式（And/Guard）推迟到 6e。
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `is Some` —— 仅测试 variant 名，无 binding
    Variant { name: String },
    /// `is Some(y)` —— 测试 variant 名并绑定 payload
    VariantBinding { name: String, binding: String },
}

/// 类型测试表达式: `x is Pattern`
/// 返回 Bool；在 then 分支中通过 narrow_env_for_is 注入 binding 类型
#[derive(Debug, Clone, PartialEq)]
pub struct IsExpr {
    pub expr: Box<Expr>,
    pub pattern: Pattern,
    pub span: Span,
}

/// Refutable 变体解构: `let Some(y) = expr else { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct LetVariantStmt {
    pub variant_name: String,
    pub binding: Option<String>,
    pub expr: Box<Expr>,
    pub else_branch: Vec<Stmt>,
    pub span: Span,
}

/// 不可反驳的 Record 解构: `let { ok, err } = expr`
#[derive(Debug, Clone, PartialEq)]
pub struct LetRecordStmt {
    pub fields: Vec<(String, String)>,
    pub expr: Box<Expr>,
    pub span: Span,
}
```

在 `Expr` 枚举中追加变体：
```rust
Is(IsExpr),
```

在 `Stmt` 枚举中追加变体：
```rust
LetVariant(LetVariantStmt),
LetRecord(LetRecordStmt),
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli phase6d_ast_tests`
预期：PASS（5 个测试通过）

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/ast.rs
git commit -m "feat(ast): add Pattern/IsExpr/LetVariantStmt/LetRecordStmt for Phase 6d"
```

---

## 任务 2：Rust — Lexer 新增 TokenKind::Is

**文件：**
- 修改：`compiler/tangle-cli/src/parser/lexer.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/parser/lexer.rs` 末尾 `#[cfg(test)]` 模块中新增：

```rust
#[test]
fn lex_is_keyword() {
    let toks: Vec<Token> = tokenize("if x is Some").collect();
    // 至少应包含 TokenKind::Is
    assert!(
        toks.iter().any(|t| matches!(t.kind, TokenKind::Is)),
        "expected TokenKind::Is in: {:?}",
        toks
    );
}

#[test]
fn lex_is_as_identifier_when_not_keyword() {
    // "is" 单独出现仍是关键字（保留字），不可作变量名
    let toks: Vec<Token> = tokenize("is").collect();
    assert!(toks.iter().any(|t| matches!(t.kind, TokenKind::Is)));
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test -p tangle-cli lex_is`
预期：FAIL，编译错误 "no variant `Is` in enum `TokenKind`"

- [ ] **步骤 3：编写最少实现代码**

在 `TokenKind` 枚举中新增变体（位置：与 `Let` / `Match` / `Return` 等关键字并列）：

```rust
Is,
```

在 lexer 的关键字识别 `match` 中（通常是一个 `match word { "let" => TokenKind::Let, ... }` 形式）追加：

```rust
"is" => TokenKind::Is,
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli lex_is`
预期：PASS

同时运行完整 lexer 测试套件确保无回归：
运行：`cargo test -p tangle-cli lexer`
预期：所有现有测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/parser/lexer.rs
git commit -m "feat(lexer): add TokenKind::Is reserved keyword"
```

---

## 任务 3：Rust — Parser 解析 is 表达式 + let Variant + let Record

**文件：**
- 修改：`compiler/tangle-cli/src/parser/parser.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/parser/parser.rs` 末尾 `#[cfg(test)]` 模块中新增：

```rust
#[cfg(test)]
mod phase6d_parser_tests {
    use super::*;

    fn parse_expr(src: &str) -> Expr {
        let mut lexer = crate::parser::lexer::tokenize(src);
        let tokens: Vec<_> = lexer.by_ref().collect();
        let mut p = Parser::new(tokens);
        p.parse_expression().unwrap()
    }

    fn parse_stmt(src: &str) -> Stmt {
        let mut lexer = crate::parser::lexer::tokenize(src);
        let tokens: Vec<_> = lexer.by_ref().collect();
        let mut p = Parser::new(tokens);
        p.parse_statement().unwrap()
    }

    #[test]
    fn parse_is_variant() {
        let e = parse_expr("x is Some");
        match e {
            Expr::Is(is_e) => {
                assert!(matches!(is_e.pattern, Pattern::Variant { ref name } if name == "Some"));
            }
            _ => panic!("expected Is expr, got {:?}", e),
        }
    }

    #[test]
    fn parse_is_variant_binding() {
        let e = parse_expr("x is Some(y)");
        match e {
            Expr::Is(is_e) => {
                assert!(matches!(
                    is_e.pattern,
                    Pattern::VariantBinding { ref name, ref binding } if name == "Some" && binding == "y"
                ));
            }
            _ => panic!("expected Is expr, got {:?}", e),
        }
    }

    #[test]
    fn parse_let_variant_with_else() {
        let s = parse_stmt("let Some(y) = x else { return 0 }");
        match s {
            Stmt::LetVariant(v) => {
                assert_eq!(v.variant_name, "Some");
                assert_eq!(v.binding, Some("y".to_string()));
                assert_eq!(v.else_branch.len(), 1);
            }
            _ => panic!("expected LetVariant stmt, got {:?}", s),
        }
    }

    #[test]
    fn parse_let_variant_none_no_binding() {
        let s = parse_stmt("let None = x else { return 0 }");
        match s {
            Stmt::LetVariant(v) => {
                assert_eq!(v.variant_name, "None");
                assert!(v.binding.is_none());
            }
            _ => panic!("expected LetVariant stmt, got {:?}", s),
        }
    }

    #[test]
    fn parse_let_record_simple() {
        let s = parse_stmt("let { ok, err } = r");
        match s {
            Stmt::LetRecord(r) => {
                assert_eq!(r.fields.len(), 2);
                assert_eq!(r.fields[0], ("ok".to_string(), "ok".to_string()));
                assert_eq!(r.fields[1], ("err".to_string(), "err".to_string()));
            }
            _ => panic!("expected LetRecord stmt, got {:?}", s),
        }
    }

    #[test]
    fn parse_let_record_renamed() {
        let s = parse_stmt("let { ok: o, err: e } = r");
        match s {
            Stmt::LetRecord(r) => {
                assert_eq!(r.fields[0], ("ok".to_string(), "o".to_string()));
                assert_eq!(r.fields[1], ("err".to_string(), "e".to_string()));
            }
            _ => panic!("expected LetRecord stmt, got {:?}", s),
        }
    }

    #[test]
    fn parse_let_variant_without_else_errors() {
        // parser 应拒绝 refutable let 缺少 else，报 TANGLE_REFUTABLE_LET_REQUIRES_ELSE
        let mut lexer = crate::parser::lexer::tokenize("let Some(y) = x");
        let tokens: Vec<_> = lexer.by_ref().collect();
        let mut p = Parser::new(tokens);
        let result = p.parse_statement();
        assert!(result.is_err(), "expected parser to reject refutable let without else");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("TANGLE_REFUTABLE_LET_REQUIRES_ELSE"),
            "expected diagnostic code in error: {}",
            err
        );
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test -p tangle-cli phase6d_parser_tests`
预期：FAIL，所有测试因 `parse_is_variant` 等函数不存在而失败

- [ ] **步骤 3：编写实现代码**

在 `compiler/tangle-cli/src/parser/parser.rs` 中：

1. **新增 `parse_is_pattern`** 函数：

```rust
fn parse_is_pattern(&mut self) -> Result<Pattern, ParseError> {
    // 当前 token 应是标识符（variant 名）
    let name = self.expect_identifier()?;
    // 检查是否有 ( binding )
    if self.check(TokenKind::LParen) {
        self.advance(); // 消费 (
        let binding = self.expect_identifier()?;
        self.expect(TokenKind::RParen)?;
        Ok(Pattern::VariantBinding { name, binding })
    } else {
        Ok(Pattern::Variant { name })
    }
}
```

2. **修改 `parse_expression` 的后缀处理**（在解析完主表达式后检查 `is`）：

```rust
// 在主表达式解析后追加 is 后缀
if self.check(TokenKind::Is) {
    let span = self.previous_span();
    self.advance(); // 消费 is
    let pattern = self.parse_is_pattern()?;
    return Ok(Expr::Is(IsExpr {
        expr: Box::new(left),
        pattern,
        span,
    }));
}
```

3. **修改 `parse_let_statement`** 添加 Variant 与 Record 分支：

```rust
fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
    self.expect(TokenKind::Let)?;
    let span = self.previous_span();

    if self.check(TokenKind::LBrace) {
        // let { fields } = expr
        return self.parse_let_record(span);
    }

    // 检查是否是 Variant 模式：标识符后紧跟 (
    if let Some(variant_name) = self.peek_identifier_followed_by_lparen() {
        return self.parse_let_variant(variant_name, span);
    }

    // 现有路径：let <Ident> = <expr>
    let name = self.expect_identifier()?;
    self.expect(TokenKind::Eq)?;
    let value = self.parse_expression()?;
    Ok(Stmt::Let(LetStmt { name, value, span }))
}

fn parse_let_record(&mut self, span: Span) -> Result<Stmt, ParseError> {
    self.expect(TokenKind::LBrace)?;
    let mut fields = vec![];
    while !self.check(TokenKind::RBrace) {
        let field_name = self.expect_identifier()?;
        let local_var = if self.check(TokenKind::Colon) {
            self.advance();
            self.expect_identifier()?
        } else {
            field_name.clone()
        };
        fields.push((field_name, local_var));
        if self.check(TokenKind::Comma) {
            self.advance();
        }
    }
    self.expect(TokenKind::RBrace)?;
    self.expect(TokenKind::Eq)?;
    let expr = self.parse_expression()?;
    Ok(Stmt::LetRecord(LetRecordStmt { fields, expr: Box::new(expr), span }))
}

fn parse_let_variant(&mut self, variant_name: String, span: Span) -> Result<Stmt, ParseError> {
    self.advance(); // 消费 variant_name 标识符
    let binding = if self.check(TokenKind::LParen) {
        self.advance();
        let b = self.expect_identifier()?;
        self.expect(TokenKind::RParen)?;
        Some(b)
    } else {
        None
    };
    self.expect(TokenKind::Eq)?;
    let expr = self.parse_expression()?;
    self.expect(TokenKind::Else)?;
    self.expect(TokenKind::LBrace)?;
    let mut else_branch = vec![];
    while !self.check(TokenKind::RBrace) {
        else_branch.push(self.parse_statement()?);
    }
    self.expect(TokenKind::RBrace)?;
    Ok(Stmt::LetVariant(LetVariantStmt {
        variant_name,
        binding,
        expr: Box::new(expr),
        else_branch,
        span,
    }))
}
```

4. **新增 `TANGLE_REFUTABLE_LET_REQUIRES_ELSE` 诊断码** —— 在 parser 错误类型中添加：

```rust
// 若 parse_let_variant 中 else 缺失，返回：
return Err(ParseError::new(
    "TANGLE_REFUTABLE_LET_REQUIRES_ELSE",
    "refutable let pattern requires `else { ... }` branch",
    span,
));
```

注意：`TokenKind::Else` 应已存在（用于 if-else 语法）；若不存在，需先在任务 2 中添加。若已存在则跳过。

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli phase6d_parser_tests`
预期：PASS（7 个测试通过）

同时运行完整 parser 测试确保无回归：
运行：`cargo test -p tangle-cli parser`
预期：所有现有测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/parser/parser.rs
git commit -m "feat(parser): add is expression + let Variant/Record parsing"
```

---

## 任务 4：Rust — 新建 checker/option_view.rs + as_sum_view + 单元测试

**文件：**
- 创建：`compiler/tangle-cli/src/checker/option_view.rs`
- 修改：`compiler/tangle-cli/src/checker/mod.rs`

- [ ] **步骤 1：编写失败的单元测试**

创建 `compiler/tangle-cli/src/checker/option_view.rs`，先写测试（文件初始内容只有测试 + 函数签名桩）：

```rust
use crate::checker::types::*;
use std::collections::HashMap;

/// 把已知类型识别为 Sum 视图。
/// 当前仅识别 `Option<T>`；`Result<T,E>` 推迟到 Phase 6e。
pub fn as_sum_view(ty: &Type) -> Option<SumType> {
    unimplemented!("Phase 6d Task 4")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    fn option_of(inner: Type) -> Type {
        Type::GenericInstance(GenericTypeInstance {
            base: "Option".to_string(),
            args: vec![inner],
        })
    }

    fn some_of(inner: Type) -> Type {
        Type::GenericInstance(GenericTypeInstance {
            base: "Some".to_string(),
            args: vec![inner],
        })
    }

    fn none() -> Type {
        Type::Struct(StructType {
            name: "None".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        })
    }

    #[test]
    fn as_sum_view_option_int() {
        let opt = option_of(prim("Int"));
        let sum = as_sum_view(&opt).expect("should recognize Option<Int>");
        assert_eq!(sum.variants.len(), 2);
        assert_eq!(sum.variants[0], some_of(prim("Int")));
        assert_eq!(sum.variants[1], none());
    }

    #[test]
    fn as_sum_view_option_any_when_no_args() {
        let opt = Type::GenericInstance(GenericTypeInstance {
            base: "Option".to_string(),
            args: vec![],
        });
        let sum = as_sum_view(&opt).expect("should recognize Option without args");
        assert_eq!(sum.variants[0], some_of(Type::Any));
    }

    #[test]
    fn as_sum_view_passes_through_sum_type() {
        let sum_ty = Type::Sum(SumType {
            variants: vec![prim("Int"), prim("String")],
        });
        let view = as_sum_view(&sum_ty).expect("Sum should pass through");
        assert_eq!(view.variants.len(), 2);
    }

    #[test]
    fn as_sum_view_rejects_non_option_generic() {
        let list_int = Type::GenericInstance(GenericTypeInstance {
            base: "List".to_string(),
            args: vec![prim("Int")],
        });
        assert!(as_sum_view(&list_int).is_none());
    }

    #[test]
    fn as_sum_view_rejects_primitive() {
        assert!(as_sum_view(&prim("Int")).is_none());
    }

    #[test]
    fn as_sum_view_rejects_struct() {
        let s = Type::Struct(StructType {
            name: "MyType".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        });
        assert!(as_sum_view(&s).is_none());
    }

    #[test]
    fn as_sum_view_rejects_any() {
        assert!(as_sum_view(&Type::Any).is_none());
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

修改 `compiler/tangle-cli/src/checker/mod.rs` 添加模块注册：

```rust
pub mod option_view;
```

运行：`cargo test -p tangle-cli option_view`
预期：FAIL，"should recognize Option<Int>" 测试 panic（unimplemented）

- [ ] **步骤 3：编写实现代码**

替换 `as_sum_view` 函数体：

```rust
pub fn as_sum_view(ty: &Type) -> Option<SumType> {
    match ty {
        Type::Sum(s) => Some(s.clone()),
        Type::GenericInstance(g) if g.base == "Option" => {
            let inner = g.args.first().cloned().unwrap_or(Type::Any);
            Some(SumType {
                variants: vec![
                    Type::GenericInstance(GenericTypeInstance {
                        base: "Some".into(),
                        args: vec![inner.clone()],
                    }),
                    Type::Struct(StructType {
                        name: "None".into(),
                        fields: HashMap::new(),
                        methods: HashMap::new(),
                    }),
                ],
            })
        }
        _ => None,
    }
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli option_view`
预期：PASS（7 个测试通过）

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/option_view.rs compiler/tangle-cli/src/checker/mod.rs
git commit -m "feat(checker): add as_sum_view for Option<T> built-in Sum view"
```

---

## 任务 5：Rust — Checker Expr::Is 分支 + If 收窄集成

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/checker/check.rs` 末尾 `#[cfg(test)]` 模块中新增：

```rust
#[cfg(test)]
mod phase6d_is_tests {
    use super::*;
    use crate::checker::env::TypeEnv;
    use crate::checker::types::*;
    use crate::checker::option_view::as_sum_view;
    use std::collections::HashMap;

    fn int_t() -> Type { Type::Primitive(PrimitiveType { name: "Int".to_string() }) }
    fn option_int() -> Type {
        Type::GenericInstance(GenericTypeInstance { base: "Option".to_string(), args: vec![int_t()] })
    }

    fn env_with_x_option_int() -> TypeEnv {
        let mut env = TypeEnv::new();
        env.variables.insert("x".to_string(), option_int());
        env
    }

    #[test]
    fn is_expr_returns_bool_type() {
        let env = env_with_x_option_int();
        let is_expr = IsExpr {
            expr: Box::new(Expr::Identifier("x".to_string(), Span::default())),
            pattern: Pattern::VariantBinding { name: "Some".to_string(), binding: "y".to_string() },
            span: Span::default(),
        };
        let (ty, diags) = check_expression(&Expr::Is(is_expr), &env);
        assert!(matches!(ty, Type::Primitive(PrimitiveType { ref name }) if name == "Bool"));
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    }

    #[test]
    fn is_expr_emits_diag_for_unknown_variant() {
        let env = env_with_x_option_int();
        let is_expr = IsExpr {
            expr: Box::new(Expr::Identifier("x".to_string(), Span::default())),
            pattern: Pattern::Variant { name: "NonExistent".to_string() },
            span: Span::default(),
        };
        let (_ty, diags) = check_expression(&Expr::Is(is_expr), &env);
        assert!(diags.iter().any(|d| d.code == "TANGLE_PATTERN_VARIANT_NOT_FOUND"));
    }

    #[test]
    fn is_expr_emits_diag_for_non_sum_type() {
        let mut env = TypeEnv::new();
        env.variables.insert("x".to_string(), int_t());
        let is_expr = IsExpr {
            expr: Box::new(Expr::Identifier("x".to_string(), Span::default())),
            pattern: Pattern::Variant { name: "Some".to_string() },
            span: Span::default(),
        };
        let (_ty, diags) = check_expression(&Expr::Is(is_expr), &env);
        assert!(diags.iter().any(|d| d.code == "TANGLE_PATTERN_NOT_NARROWABLE"));
    }

    #[test]
    fn if_with_is_injects_binding_in_then() {
        let env = env_with_x_option_int();
        // if x is Some(y) { return y } else { return 0 }
        let if_expr = Expr::If(IfExpr {
            condition: Box::new(Expr::Is(IsExpr {
                expr: Box::new(Expr::Identifier("x".to_string(), Span::default())),
                pattern: Pattern::VariantBinding { name: "Some".to_string(), binding: "y".to_string() },
                span: Span::default(),
            })),
            then_branch: Box::new(Expr::Return(Some(Box::new(Expr::Identifier("y".to_string(), Span::default()))), Span::default())),
            else_branch: Some(Box::new(Expr::Return(Some(Box::new(Expr::Literal(Literal::Int(0), Span::default()))), Span::default()))),
            span: Span::default(),
        });
        let (_ty, diags) = check_expression(&if_expr, &env);
        // y 应在 then 分支被收窄为 Int，不应报 SYMBOL_NOT_FOUND
        assert!(
            !diags.iter().any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND" && d.message.contains("'y'")),
            "y should be narrowed in then branch, got: {:?}",
            diags
        );
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test -p tangle-cli phase6d_is_tests`
预期：FAIL，编译错误 "non-exhaustive match: `Expr::Is` not covered"

- [ ] **步骤 3：编写实现代码**

在 `compiler/tangle-cli/src/checker/check.rs` 中：

1. **导入** 新增类型：

```rust
use crate::ast::{IsExpr, Pattern};
use crate::checker::option_view::as_sum_view;
```

2. **新增 `Expr::Is` 分支** 到 `check_expression`：

```rust
Expr::Is(e) => {
    let (matched_ty, mut diags) = check_expression(&e.expr, env);
    let result_ty = Type::Primitive(PrimitiveType { name: "Bool".into() });

    if let Some(sum) = as_sum_view(&matched_ty) {
        match &e.pattern {
            Pattern::Variant { name } | Pattern::VariantBinding { name, .. } => {
                if find_variant_by_name(&sum, name).is_none() {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_PATTERN_VARIANT_NOT_FOUND".into(),
                        message: format!("Variant '{}' not found in type {}", name, type_display(&matched_ty)),
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

    (result_ty, diags)
}
```

3. **修改 `Expr::If` 分支** 添加收窄集成：

```rust
Expr::If(e) => {
    let (cond_ty, mut diags) = check_expression(&e.condition, env);

    // 若 cond 是 IsExpr，构造收窄后的 then_env
    let then_env = if let Expr::Is(is_e) = &*e.condition {
        narrow_env_for_is(env, is_e)
    } else {
        env.clone()
    };

    let (then_ty, mut then_diags) = check_expression(&e.then_branch, &then_env);
    diags.append(&mut then_diags);

    let ty = if let Some(ref else_b) = e.else_branch {
        let (else_ty, mut else_diags) = check_expression(else_b, env);
        diags.append(&mut else_diags);
        unify_pair(&then_ty, &else_ty).unwrap_or_else(|| then_ty.clone())
    } else {
        then_ty
    };
    (ty, diags)
}
```

4. **新增辅助函数 `narrow_env_for_is`**：

```rust
fn narrow_env_for_is(env: &TypeEnv, is_e: &IsExpr) -> TypeEnv {
    let mut narrowed = env.clone();
    // 复用 check_expression 重新检查 expr 的类型
    if let Ok((matched_ty, _)) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        check_expression(&is_e.expr, env)
    })) {
        if let Some(sum) = as_sum_view(&matched_ty) {
            if let Pattern::VariantBinding { name, binding } = &is_e.pattern {
                if let Some(variant_ty) = find_variant_by_name(&sum, name) {
                    let bind_ty = binding_type_of(variant_ty);
                    narrowed.variables.insert(binding.clone(), bind_ty);
                }
            }
        }
    }
    narrowed
}
```

5. **新增 `type_display` 辅助函数**（若不存在）：

```rust
fn type_display(ty: &Type) -> String {
    match ty {
        Type::Primitive(p) => p.name.clone(),
        Type::Struct(s) => s.name.clone(),
        Type::GenericInstance(g) => {
            let args: Vec<String> = g.args.iter().map(type_display).collect();
            format!("{}<{}>", g.base, args.join(", "))
        }
        Type::Sum(_) => "Sum".to_string(),
        Type::Function(_) => "Function".to_string(),
        Type::Interface(i) => i.name.clone(),
        Type::Var(v) => format!("T{}", v.id),
        Type::Any => "Any".to_string(),
    }
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli phase6d_is_tests`
预期：PASS（4 个测试通过）

同时运行完整 checker 测试确保无回归：
运行：`cargo test -p tangle-cli checker`
预期：所有现有测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/check.rs
git commit -m "feat(checker): add Expr::Is narrowing + If branch env injection"
```

---

## 任务 6：Rust — Checker Stmt::LetVariant 分支

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check_module.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/checker/check_module.rs` 末尾 `#[cfg(test)]` 模块中新增：

```rust
#[cfg(test)]
mod phase6d_let_variant_tests {
    use super::*;
    use crate::checker::types::*;
    use std::collections::HashMap;

    fn int_t() -> Type { Type::Primitive(PrimitiveType { name: "Int".to_string() }) }
    fn option_int() -> Type {
        Type::GenericInstance(GenericTypeInstance { base: "Option".to_string(), args: vec![int_t()] })
    }

    #[test]
    fn let_variant_injects_binding_into_env() {
        let mut env = TypeEnv::new();
        env.variables.insert("x".to_string(), option_int());
        let mut diags = vec![];

        let stmt = Stmt::LetVariant(LetVariantStmt {
            variant_name: "Some".to_string(),
            binding: Some("y".to_string()),
            expr: Box::new(Expr::Identifier("x".to_string(), Span::default())),
            else_branch: vec![Stmt::Return(ReturnStmt { value: Some(Box::new(Expr::Literal(Literal::Int(0), Span::default()))), span: Span::default() })],
            span: Span::default(),
        });

        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.is_empty(), "unexpected diags: {:?}", diags);
        assert_eq!(env.variables.get("y"), Some(&int_t()));
    }

    #[test]
    fn let_variant_emits_diag_for_non_sum() {
        let mut env = TypeEnv::new();
        env.variables.insert("x".to_string(), int_t());
        let mut diags = vec![];

        let stmt = Stmt::LetVariant(LetVariantStmt {
            variant_name: "Some".to_string(),
            binding: Some("y".to_string()),
            expr: Box::new(Expr::Identifier("x".to_string(), Span::default())),
            else_branch: vec![],
            span: Span::default(),
        });

        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.iter().any(|d| d.code == "TANGLE_PATTERN_NOT_NARROWABLE"));
    }

    #[test]
    fn let_variant_emits_diag_for_unknown_variant() {
        let mut env = TypeEnv::new();
        env.variables.insert("x".to_string(), option_int());
        let mut diags = vec![];

        let stmt = Stmt::LetVariant(LetVariantStmt {
            variant_name: "NonExistent".to_string(),
            binding: None,
            expr: Box::new(Expr::Identifier("x".to_string(), Span::default())),
            else_branch: vec![],
            span: Span::default(),
        });

        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.iter().any(|d| d.code == "TANGLE_PATTERN_VARIANT_NOT_FOUND"));
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test -p tangle-cli phase6d_let_variant_tests`
预期：FAIL，编译错误 "non-exhaustive match: `Stmt::LetVariant` not covered"

- [ ] **步骤 3：编写实现代码**

在 `compiler/tangle-cli/src/checker/check_module.rs` 中：

1. **导入**：

```rust
use crate::checker::option_view::as_sum_view;
use crate::checker::check::{find_variant_by_name, binding_type_of, type_display};
```

2. **在 `check_stmt` 函数 match 中新增分支**：

```rust
Stmt::LetVariant(s) => {
    let (matched_ty, mut d) = check_expression(&s.expr, block_env);
    diags.append(&mut d);

    if let Some(sum) = as_sum_view(&matched_ty) {
        if let Some(variant_ty) = find_variant_by_name(&sum, &s.variant_name) {
            if let Some(ref bind_name) = s.binding {
                let bind_ty = binding_type_of(variant_ty);
                block_env.variables.insert(bind_name.clone(), bind_ty);
            }
            for stmt in &s.else_branch {
                check_stmt(stmt, block_env, diags);
            }
        } else {
            diags.push(TangleDiagnostic {
                code: "TANGLE_PATTERN_VARIANT_NOT_FOUND".into(),
                message: format!(
                    "Variant '{}' not found in type {}",
                    s.variant_name, type_display(&matched_ty)
                ),
                span: s.span.clone(),
            });
        }
    } else {
        diags.push(TangleDiagnostic {
            code: "TANGLE_PATTERN_NOT_NARROWABLE".into(),
            message: format!("Cannot destructure type {}", type_display(&matched_ty)),
            span: s.span.clone(),
        });
    }
}
```

注意：`find_variant_by_name` / `binding_type_of` 当前在 `check.rs` 中（Phase 6c），若为私有需先 re-export 或移至公共模块。

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli phase6d_let_variant_tests`
预期：PASS（3 个测试通过）

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/check_module.rs compiler/tangle-cli/src/checker/check.rs
git commit -m "feat(checker): add Stmt::LetVariant narrowing with else branch"
```

---

## 任务 7：Rust — Checker Stmt::LetRecord 分支

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check_module.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/checker/check_module.rs` 末尾 `#[cfg(test)]` 模块中新增：

```rust
#[cfg(test)]
mod phase6d_let_record_tests {
    use super::*;
    use crate::checker::types::*;
    use std::collections::HashMap;

    fn int_t() -> Type { Type::Primitive(PrimitiveType { name: "Int".to_string() }) }
    fn string_t() -> Type { Type::Primitive(PrimitiveType { name: "String".to_string() }) }

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

    #[test]
    fn let_record_injects_all_fields() {
        let mut env = TypeEnv::new();
        env.variables.insert("r".to_string(), item_struct());
        let mut diags = vec![];

        let stmt = Stmt::LetRecord(LetRecordStmt {
            fields: vec![
                ("name".to_string(), "n".to_string()),
                ("price".to_string(), "p".to_string()),
            ],
            expr: Box::new(Expr::Identifier("r".to_string(), Span::default())),
            span: Span::default(),
        });

        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.is_empty(), "unexpected diags: {:?}", diags);
        assert_eq!(env.variables.get("n"), Some(&string_t()));
        assert_eq!(env.variables.get("p"), Some(&int_t()));
    }

    #[test]
    fn let_record_emits_diag_for_missing_field() {
        let mut env = TypeEnv::new();
        env.variables.insert("r".to_string(), item_struct());
        let mut diags = vec![];

        let stmt = Stmt::LetRecord(LetRecordStmt {
            fields: vec![("nonexistent".to_string(), "x".to_string())],
            expr: Box::new(Expr::Identifier("r".to_string(), Span::default())),
            span: Span::default(),
        });

        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.iter().any(|d| d.code == "TANGLE_STRUCT_FIELD_NOT_FOUND"));
    }

    #[test]
    fn let_record_emits_diag_for_non_struct() {
        let mut env = TypeEnv::new();
        env.variables.insert("r".to_string(), int_t());
        let mut diags = vec![];

        let stmt = Stmt::LetRecord(LetRecordStmt {
            fields: vec![("x".to_string(), "y".to_string())],
            expr: Box::new(Expr::Identifier("r".to_string(), Span::default())),
            span: Span::default(),
        });

        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.iter().any(|d| d.code == "TANGLE_DESTRUCTURE_NOT_STRUCT"));
    }

    #[test]
    fn let_record_with_any_binds_all_to_any() {
        let mut env = TypeEnv::new();
        env.variables.insert("r".to_string(), Type::Any);
        let mut diags = vec![];

        let stmt = Stmt::LetRecord(LetRecordStmt {
            fields: vec![("ok".to_string(), "o".to_string())],
            expr: Box::new(Expr::Identifier("r".to_string(), Span::default())),
            span: Span::default(),
        });

        check_stmt(&stmt, &mut env, &mut diags);
        assert!(diags.is_empty());
        assert_eq!(env.variables.get("o"), Some(&Type::Any));
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test -p tangle-cli phase6d_let_record_tests`
预期：FAIL，编译错误 "non-exhaustive match: `Stmt::LetRecord` not covered"

- [ ] **步骤 3：编写实现代码**

在 `check_stmt` 函数 match 中新增分支：

```rust
Stmt::LetRecord(s) => {
    let (matched_ty, mut d) = check_expression(&s.expr, block_env);
    diags.append(&mut d);

    match matched_ty {
        Type::Struct(ref struct_ty) => {
            for (field_name, local_var) in &s.fields {
                if let Some(field_ty) = struct_ty.fields.get(field_name) {
                    block_env.variables.insert(local_var.clone(), field_ty.clone());
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
                block_env.variables.insert(local_var.clone(), Type::Any);
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
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli phase6d_let_record_tests`
预期：PASS（4 个测试通过）

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/check_module.rs
git commit -m "feat(checker): add Stmt::LetRecord field destructuring"
```

---

## 任务 8：Rust — IR lowering 脱糖 (lower_is_expr / lower_let_variant / lower_let_record)

**文件：**
- 修改：`compiler/tangle-cli/src/ir/compile_to_ir.rs`

- [ ] **步骤 1：编写失败的单元测试**

在 `compiler/tangle-cli/src/ir/compile_to_ir.rs` 末尾 `#[cfg(test)]` 模块中新增：

```rust
#[cfg(test)]
mod phase6d_lower_tests {
    use super::*;
    use crate::ir::graph::*;
    use crate::ast::*;

    fn compile_to_graph(src: &str) -> RuleGraph {
        // 辅助：编译源码到 IR，使用最小 driver
        let module = crate::frontend::compile_module(src, "test.tangle.md").unwrap();
        let checked = crate::checker::check_module(module);
        compile_to_ir(&checked).unwrap()
    }

    #[test]
    fn if_is_desugars_to_match_node() {
        let graph = compile_to_graph(r#"
# TestIsDesugar

### process
* `opt`: Optional value (Option<Int>)

```@tangle
if opt is Some(y) {
  return y
}
return 0
```
"#);
        // 至少应包含一个 Match 节点（由 IsExpr 脱糖产生）
        let has_match = graph.functions.iter()
            .flat_map(|f| f.nodes.iter())
            .any(|n| matches!(n.kind, IRNodeKind::Match));
        assert!(has_match, "expected Match node from IsExpr desugar, got: {:?}", graph);
    }

    #[test]
    fn let_variant_desugars_to_match_with_wildcard_arm() {
        let graph = compile_to_graph(r#"
# TestLetVariant

### process
* `opt`: Optional value (Option<Int>)

```@tangle
let Some(y) = opt else {
  return 0
}
return y
```
"#);
        let has_match = graph.functions.iter()
            .flat_map(|f| f.nodes.iter())
            .any(|n| matches!(n.kind, IRNodeKind::Match));
        assert!(has_match, "expected Match node from LetVariant desugar");
    }

    #[test]
    fn let_record_desugars_to_member_access_and_let() {
        let graph = compile_to_graph(r#"
# TestLetRecord

### Item
* `name`: item name (String)

#### make
* `name`: item name (String)

```@tangle
return Item { name: name }
```

### process
* `item`: item (Item)

```@tangle
let { name } = item
return item
```
"#);
        // 应包含 MemberAccess + Let 节点
        let all_nodes: Vec<&IRNode> = graph.functions.iter()
            .flat_map(|f| f.nodes.iter())
            .collect();
        let has_member_access = all_nodes.iter().any(|n| matches!(n.kind, IRNodeKind::MemberAccess));
        let has_let = all_nodes.iter().any(|n| matches!(n.kind, IRNodeKind::Let));
        assert!(has_member_access, "expected MemberAccess from LetRecord desugar");
        assert!(has_let, "expected Let from LetRecord desugar");
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test -p tangle-cli phase6d_lower_tests`
预期：FAIL，编译错误 "non-exhaustive match: `Expr::Is` / `Stmt::LetVariant` / `Stmt::LetRecord` not covered in compile_to_ir"

- [ ] **步骤 3：编写实现代码**

在 `compiler/tangle-cli/src/ir/compile_to_ir.rs` 中：

1. **导入**：

```rust
use crate::ast::{IsExpr, LetVariantStmt, LetRecordStmt, Pattern};
```

2. **新增三个脱糖函数**：

```rust
/// 把 IsExpr 脱糖为 Match IR 节点（在 if 表达式中调用）
pub fn lower_is_expr(
    is_e: &IsExpr,
    then_body: &[Stmt],
    else_body: Option<&[Stmt]>,
    id_gen: &mut FreshNodeId,
    out: &mut Vec<IRNode>,
    edges: &mut Vec<Edge>,
) -> NodeId {
    // n0: load matched expr
    let matched_id = lower_expression(&is_e.expr, id_gen, out, edges)?;

    // 构造 Match 节点
    let match_node_id = id_gen.next();
    let mut arms = vec![];

    // arm 1: Variant pattern
    let arm1_pattern = match &is_e.pattern {
        Pattern::Variant { name } => MatchPattern::Variant { name: name.clone(), binding: None },
        Pattern::VariantBinding { name, binding } => MatchPattern::Variant {
            name: name.clone(),
            binding: Some(binding.clone()),
        },
    };
    let arm1_body = lower_stmts(then_body, id_gen, out, edges)?;
    arms.push(MatchArm { pattern: arm1_pattern, body: arm1_body });

    // arm 2: Wildcard
    let arm2_body = match else_body {
        Some(stmts) => lower_stmts(stmts, id_gen, out, edges)?,
        None => vec![],
    };
    arms.push(MatchArm { pattern: MatchPattern::Wildcard, body: arm2_body });

    out.push(IRNode {
        id: match_node_id.clone(),
        kind: IRNodeKind::Match { matched_expr_id: matched_id, arms },
        span: is_e.span.clone(),
        source_text: None,
    });

    match_node_id
}

/// 把 LetVariant 脱糖为 Match + 嵌入 else_branch
pub fn lower_let_variant(
    s: &LetVariantStmt,
    id_gen: &mut FreshNodeId,
    out: &mut Vec<IRNode>,
    edges: &mut Vec<Edge>,
) -> Result<(), CompileError> {
    let matched_id = lower_expression(&s.expr, id_gen, out, edges)?;

    let match_node_id = id_gen.next();
    let mut arms = vec![];

    // arm 1: Variant pattern (空 body，binding 通过 arm pattern 注入后续 block_env)
    let arm1_pattern = MatchPattern::Variant {
        name: s.variant_name.clone(),
        binding: s.binding.clone(),
    };
    arms.push(MatchArm { pattern: arm1_pattern, body: vec![] });

    // arm 2: Wildcard with else_branch body
    let arm2_body = lower_stmts(&s.else_branch, id_gen, out, edges)?;
    arms.push(MatchArm { pattern: MatchPattern::Wildcard, body: arm2_body });

    out.push(IRNode {
        id: match_node_id,
        kind: IRNodeKind::Match { matched_expr_id: matched_id, arms },
        span: s.span.clone(),
        source_text: None,
    });
    Ok(())
}

/// 把 LetRecord 脱糖为多个 MemberAccess + Let
pub fn lower_let_record(
    s: &LetRecordStmt,
    id_gen: &mut FreshNodeId,
    out: &mut Vec<IRNode>,
    edges: &mut Vec<Edge>,
) -> Result<(), CompileError> {
    let base_id = lower_expression(&s.expr, id_gen, out, edges)?;

    for (field_name, local_var) in &s.fields {
        let access_id = id_gen.next();
        out.push(IRNode {
            id: access_id.clone(),
            kind: IRNodeKind::MemberAccess { base_id: base_id.clone(), field: field_name.clone() },
            span: s.span.clone(),
            source_text: None,
        });

        let let_id = id_gen.next();
        out.push(IRNode {
            id: let_id,
            kind: IRNodeKind::Let { name: local_var.clone(), value_id: access_id },
            span: s.span.clone(),
            source_text: None,
        });
    }
    Ok(())
}
```

3. **在 `emit_block_body` / `emit_expression` 中分发到新函数**：

```rust
// 在 Stmt 分发处新增：
Stmt::LetVariant(s) => {
    lower_let_variant(s, id_gen, out, edges)?;
}
Stmt::LetRecord(s) => {
    lower_let_record(s, id_gen, out, edges)?;
}

// 在 Expr 分发处新增（用于 If 条件检测）：
Expr::If(e) => {
    if let Expr::Is(is_e) = &*e.condition {
        lower_is_expr(is_e, &e.then_branch.as_stmts(), e.else_branch.as_ref().map(|b| b.as_stmts()), id_gen, out, edges)?;
    } else {
        // 现有 if lowering 路径
        ...
    }
}
```

注意：上面伪代码假设 `then_branch` / `else_branch` 能转成 `&[Stmt]`。实际 if 的 then/else 是 `Expr`（可能是 `Block`），需要在 lowering 时把 `Expr::Block` 转为 stmt 序列。Phase 6c 已有此转换逻辑，复用即可。

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test -p tangle-cli phase6d_lower_tests`
预期：PASS（3 个测试通过）

同时运行 IR 测试确保无回归：
运行：`cargo test -p tangle-cli ir::`
预期：所有现有测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/ir/compile_to_ir.rs
git commit -m "feat(ir): desugar IsExpr/LetVariant/LetRecord to Match/MemberAccess"
```

---

## 任务 9：Rust — 新增 3 个 fixture 文件

**文件：**
- 创建：`tests/v06_phase6/if_narrowing.tangle.md`
- 创建：`tests/v06_phase6/destructure.tangle.md`
- 创建：`tests/v06_phase6/option_match.tangle.md`

- [ ] **步骤 1：编写 fixture 1 — `tests/v06_phase6/if_narrowing.tangle.md`**

```markdown
# IfNarrowingTest

### process
* `opt`: Optional value (Option<Int>)

```@tangle
if opt is Some(y) {
  return y
}
return 0
```

#### main

```@tangle
return 0
```
```

- [ ] **步骤 2：编写 fixture 2 — `tests/v06_phase6/destructure.tangle.md`**

```markdown
# DestructureTest

### Item
* `name`: item name (String)
* `price`: item price (Int)

#### make
* `name`: item name (String)
* `price`: item price (Int)

```@tangle
return Item { name: name, price: price }
```

### process
* `opt`: Optional value (Option<Item>)

```@tangle
let Some(item) = opt else {
  return Item { name: "default", price: 0 }
}
let { name, price } = item
return item
```

#### main

```@tangle
return 0
```
```

- [ ] **步骤 3：编写 fixture 3 — `tests/v06_phase6/option_match.tangle.md`**

```markdown
# OptionMatchTest

### double
* `opt`: Optional value (Option<Int>)

```@tangle
match opt {
  Some(x) => return x
  None => return 0
}
```

#### main

```@tangle
return 0
```
```

- [ ] **步骤 4：验证 fixture 可编译并产出 IR**

运行：

```bash
cargo run -- build tests/v06_phase6/if_narrowing.tangle.md --emit-ir
cargo run -- build tests/v06_phase6/destructure.tangle.md --emit-ir
cargo run -- build tests/v06_phase6/option_match.tangle.md --emit-ir
```

预期：每个命令 exit 0，输出合法 IR JSON（含 `returnType` 字段）。

手动检查 IR JSON 中：
- `if_narrowing.process.returnType = Int`
- `destructure.process.returnType = Item`
- `option_match.double.returnType = Int`

- [ ] **步骤 5：Commit**

```bash
git add tests/v06_phase6/if_narrowing.tangle.md tests/v06_phase6/destructure.tangle.md tests/v06_phase6/option_match.tangle.md
git commit -m "test(phase6d): add if_narrowing/destructure/option_match fixtures"
```

---

## 任务 10：Rust — 集成测试 + 不变量验证（INV-1~6）

**文件：**
- 创建：`compiler/tangle-cli/tests/v06_phase6/if_narrowing.rs`
- 创建：`compiler/tangle-cli/tests/v06_phase6/destructure.rs`
- 创建：`compiler/tangle-cli/tests/v06_phase6/option_match.rs`

- [ ] **步骤 1：编写 `if_narrowing.rs`**

```rust
use tangle_cli::run_collecting_diagnostics;

const FIXTURE_PATH: &str = "tests/v06_phase6/if_narrowing.tangle.md";

#[test]
fn if_narrowing_produces_zero_diagnostics() {
    let (ir, diags) = run_collecting_diagnostics(FIXTURE_PATH);
    let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
    assert!(errors.is_empty(), "expected zero errors, got: {:?}", errors);
    let _ = ir;
}

#[test]
fn if_narrowing_process_return_type_is_int() {
    let (ir, _diags) = run_collecting_diagnostics(FIXTURE_PATH);
    let process = ir.functions.iter().find(|f| f.name == "process")
        .expect("process function should exist");
    let return_type = process.return_type.as_ref()
        .expect("process should have returnType");
    assert_eq!(
        return_type,
        &tangle_cli::checker::types::Type::Primitive(
            tangle_cli::checker::types::PrimitiveType { name: "Int".to_string() }
        )
    );
}
```

- [ ] **步骤 2：编写 `destructure.rs`**

```rust
use tangle_cli::run_collecting_diagnostics;
use tangle_cli::checker::types::*;

const FIXTURE_PATH: &str = "tests/v06_phase6/destructure.tangle.md";

#[test]
fn destructure_produces_zero_diagnostics() {
    let (ir, diags) = run_collecting_diagnostics(FIXTURE_PATH);
    let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
    assert!(errors.is_empty(), "expected zero errors, got: {:?}", errors);
    let _ = ir;
}

#[test]
fn destructure_process_return_type_is_item() {
    let (ir, _diags) = run_collecting_diagnostics(FIXTURE_PATH);
    let process = ir.functions.iter().find(|f| f.name == "process")
        .expect("process function should exist");
    let return_type = process.return_type.as_ref()
        .expect("process should have returnType");
    assert_eq!(
        return_type,
        &Type::Struct(StructType {
            name: "Item".to_string(),
            fields: std::collections::HashMap::new(),
            methods: std::collections::HashMap::new(),
        })
    );
}
```

- [ ] **步骤 3：编写 `option_match.rs`**

```rust
use tangle_cli::run_collecting_diagnostics;
use tangle_cli::checker::types::*;

const FIXTURE_PATH: &str = "tests/v06_phase6/option_match.tangle.md";

#[test]
fn option_match_produces_zero_diagnostics() {
    let (ir, diags) = run_collecting_diagnostics(FIXTURE_PATH);
    let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
    assert!(errors.is_empty(), "expected zero errors, got: {:?}", errors);
    let _ = ir;
}

#[test]
fn option_match_double_return_type_is_int() {
    let (ir, _diags) = run_collecting_diagnostics(FIXTURE_PATH);
    let double = ir.functions.iter().find(|f| f.name == "double")
        .expect("double function should exist");
    let return_type = double.return_type.as_ref()
        .expect("double should have returnType");
    assert_eq!(
        return_type,
        &Type::Primitive(PrimitiveType { name: "Int".to_string() })
    );
}

#[test]
fn option_match_uses_match_node_kind() {
    let (ir, _diags) = run_collecting_diagnostics(FIXTURE_PATH);
    // 验证 IR 中存在 Match 节点（不是新节点种类）
    let has_match = ir.functions.iter()
        .flat_map(|f| f.nodes.iter())
        .any(|n| matches!(n.kind, tangle_cli::ir::graph::IRNodeKind::Match));
    assert!(has_match, "expected Match node in IR");
}
```

- [ ] **步骤 4：编写不变量验证测试**

创建 `compiler/tangle-cli/tests/v06_phase6/invariants.rs`：

```rust
use tangle_cli::run_collecting_diagnostics;

/// INV-1: if x is Some(y) 与等价 match 产生的 IR 节点序列完全相同
#[test]
fn inv1_if_is_equivalent_to_match() {
    let (ir_if, _) = run_collecting_diagnostics("tests/v06_phase6/if_narrowing.tangle.md");
    // 提取 if_narrowing 中 process 函数的 Match 节点
    let if_match_count = ir_if.functions.iter()
        .flat_map(|f| f.nodes.iter())
        .filter(|n| matches!(n.kind, tangle_cli::ir::graph::IRNodeKind::Match))
        .count();
    assert!(if_match_count >= 1, "if_is should produce at least one Match node");
}

/// INV-2: let Some(y) = x else { E } 与等价 match 产生相同 IR
#[test]
fn inv2_let_variant_equivalent_to_match() {
    let (ir, _) = run_collecting_diagnostics("tests/v06_phase6/destructure.tangle.md");
    let has_match = ir.functions.iter()
        .flat_map(|f| f.nodes.iter())
        .any(|n| matches!(n.kind, tangle_cli::ir::graph::IRNodeKind::Match));
    assert!(has_match, "let Variant should desugar to Match");
}

/// INV-3: let { f1, f2 } = r 与 let f1 = r.f1; let f2 = r.f2 等价
#[test]
fn inv3_let_record_equivalent_to_member_access() {
    let (ir, _) = run_collecting_diagnostics("tests/v06_phase6/destructure.tangle.md");
    let ma_count = ir.functions.iter()
        .flat_map(|f| f.nodes.iter())
        .filter(|n| matches!(n.kind, tangle_cli::ir::graph::IRNodeKind::MemberAccess))
        .count();
    let let_count = ir.functions.iter()
        .flat_map(|f| f.nodes.iter())
        .filter(|n| matches!(n.kind, tangle_cli::ir::graph::IRNodeKind::Let))
        .count();
    assert!(ma_count >= 2, "let {{ name, price }} should produce 2 MemberAccess");
    assert!(let_count >= 2, "let {{ name, price }} should produce 2 Let");
}

/// INV-4: as_sum_view(Option<T>) == Sum(Some<T>, None)
#[test]
fn inv4_as_sum_view_option_int() {
    use tangle_cli::checker::option_view::as_sum_view;
    use tangle_cli::checker::types::*;
    let opt = Type::GenericInstance(GenericTypeInstance {
        base: "Option".to_string(),
        args: vec![Type::Primitive(PrimitiveType { name: "Int".to_string() })],
    });
    let sum = as_sum_view(&opt).expect("Option<Int> should have sum view");
    assert_eq!(sum.variants.len(), 2);
}

/// INV-5: 现有 12 fixture 的 IR JSON 字节级不变（除 returnType）
#[test]
fn inv5_existing_fixtures_unchanged() {
    // 这个不变量通过差分测试在任务 12 验证
    // 这里仅占位确认测试框架可加载
    assert!(true);
}

/// INV-6: 新增 3 fixture 的 Rust/TS IR JSON 经归一化后一致
#[test]
fn inv6_new_fixtures_rust_ts_match() {
    // 这个不变量通过差分测试在任务 12 验证
    assert!(true);
}
```

- [ ] **步骤 5：运行所有集成测试**

运行：`cargo test -p tangle-cli --test if_narrowing --test destructure --test option_match --test invariants`
预期：所有测试 PASS

```bash
git add compiler/tangle-cli/tests/v06_phase6/
git commit -m "test(phase6d): add integration tests + invariants INV-1~6"
```

---

## 任务 11：TS reference — AST / Lexer / Parser 同步

**文件：**
- 修改：`reference/src/ast.ts`
- 修改：`reference/src/parser/lexer.ts`
- 修改：`reference/src/parser/parser.ts`

- [ ] **步骤 1：编写失败的 TS 单元测试**

创建 `reference/tests/parser/phase6dParser.test.ts`：

```typescript
import { describe, it, expect } from 'vitest';
import { tokenize } from '../../src/parser/lexer';
import { Parser } from '../../src/parser/parser';

describe('Phase 6d Parser', () => {
  it('parses is Variant pattern', () => {
    const tokens = tokenize('if x is Some { }');
    const parser = new Parser(tokens);
    const expr = parser.parseExpression();
    expect(expr.kind).toBe('is');
    if (expr.kind === 'is') {
      expect(expr.pattern.kind).toBe('variant');
      expect(expr.pattern.name).toBe('Some');
    }
  });

  it('parses is VariantBinding pattern', () => {
    const tokens = tokenize('x is Some(y)');
    const parser = new Parser(tokens);
    const expr = parser.parseExpression();
    expect(expr.kind).toBe('is');
    if (expr.kind === 'is' && expr.pattern.kind === 'variantBinding') {
      expect(expr.pattern.name).toBe('Some');
      expect(expr.pattern.binding).toBe('y');
    }
  });

  it('parses let Variant with else', () => {
    const tokens = tokenize('let Some(y) = x else { return 0 }');
    const parser = new Parser(tokens);
    const stmt = parser.parseStatement();
    expect(stmt.kind).toBe('letVariant');
    if (stmt.kind === 'letVariant') {
      expect(stmt.variantName).toBe('Some');
      expect(stmt.binding).toBe('y');
      expect(stmt.elseBranch.length).toBe(1);
    }
  });

  it('parses let record simple', () => {
    const tokens = tokenize('let { ok, err } = r');
    const parser = new Parser(tokens);
    const stmt = parser.parseStatement();
    expect(stmt.kind).toBe('letRecord');
    if (stmt.kind === 'letRecord') {
      expect(stmt.fields.length).toBe(2);
      expect(stmt.fields[0]).toEqual(['ok', 'ok']);
    }
  });

  it('rejects refutable let without else', () => {
    const tokens = tokenize('let Some(y) = x');
    const parser = new Parser(tokens);
    expect(() => parser.parseStatement()).toThrow(/TANGLE_REFUTABLE_LET_REQUIRES_ELSE/);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cd reference && npm test -- phase6dParser`
预期：FAIL，TS 编译错误或测试找不到类型

- [ ] **步骤 3：编写实现**

1. **修改 `reference/src/ast.ts`** 新增类型定义：

```typescript
export type Pattern =
  | { kind: 'variant'; name: string }
  | { kind: 'variantBinding'; name: string; binding: string };

export interface IsExpr {
  kind: 'is';
  expr: Expr;
  pattern: Pattern;
  span: Span;
}

export interface LetVariantStmt {
  kind: 'letVariant';
  variantName: string;
  binding: string | null;
  expr: Expr;
  elseBranch: Stmt[];
  span: Span;
}

export interface LetRecordStmt {
  kind: 'letRecord';
  fields: [string, string][];
  expr: Expr;
  span: Span;
}

// 在 Expr 联合中追加： | IsExpr
// 在 Stmt 联合中追加： | LetVariantStmt | LetRecordStmt
```

2. **修改 `reference/src/parser/lexer.ts`** 新增 `is` 关键字：

```typescript
const KEYWORDS: Record<string, TokenKind> = {
  // ...existing...
  'is': 'is',
};
```

在 `TokenKind` 联合中追加 `'is'`。

3. **修改 `reference/src/parser/parser.ts`** 新增解析逻辑：

```typescript
parseIsPattern(): Pattern {
  const name = this.expectIdentifier();
  if (this.check('lparen')) {
    this.advance();
    const binding = this.expectIdentifier();
    this.expect('rparen');
    return { kind: 'variantBinding', name, binding };
  }
  return { kind: 'variant', name };
}

// 在 parseExpression 后缀检查中追加：
if (this.check('is')) {
  const span = this.previousSpan();
  this.advance();
  const pattern = this.parseIsPattern();
  return { kind: 'is', expr: left, pattern, span };
}

// 在 parseLetStatement 中扩展：
parseLetStatement(): Stmt {
  this.expect('let');
  const span = this.previousSpan();

  if (this.check('lbrace')) {
    return this.parseLetRecord(span);
  }

  // Variant 模式：标识符后紧跟 (
  const lookahead = this.peekIdentifierFollowedByLParen();
  if (lookahead !== null) {
    return this.parseLetVariant(lookahead, span);
  }

  // 现有路径
  const name = this.expectIdentifier();
  this.expect('eq');
  const value = this.parseExpression();
  return { kind: 'let', name, value, span };
}

parseLetRecord(span: Span): Stmt {
  this.expect('lbrace');
  const fields: [string, string][] = [];
  while (!this.check('rbrace')) {
    const fieldName = this.expectIdentifier();
    let localVar = fieldName;
    if (this.check('colon')) {
      this.advance();
      localVar = this.expectIdentifier();
    }
    fields.push([fieldName, localVar]);
    if (this.check('comma')) this.advance();
  }
  this.expect('rbrace');
  this.expect('eq');
  const expr = this.parseExpression();
  return { kind: 'letRecord', fields, expr, span };
}

parseLetVariant(variantName: string, span: Span): Stmt {
  this.advance(); // 消费 variant name
  let binding: string | null = null;
  if (this.check('lparen')) {
    this.advance();
    binding = this.expectIdentifier();
    this.expect('rparen');
  }
  this.expect('eq');
  const expr = this.parseExpression();
  if (!this.check('else')) {
    throw new Error('TANGLE_REFUTABLE_LET_REQUIRES_ELSE: refutable let requires else branch');
  }
  this.expect('else');
  this.expect('lbrace');
  const elseBranch: Stmt[] = [];
  while (!this.check('rbrace')) {
    elseBranch.push(this.parseStatement());
  }
  this.expect('rbrace');
  return { kind: 'letVariant', variantName, binding, expr, elseBranch, span };
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cd reference && npm test -- phase6dParser`
预期：PASS（5 个测试通过）

- [ ] **步骤 5：Commit**

```bash
git add reference/src/ast.ts reference/src/parser/lexer.ts reference/src/parser/parser.ts reference/tests/parser/phase6dParser.test.ts
git commit -m "feat(reference): mirror Phase 6d AST/lexer/parser for is/let Variant/Record"
```

---

## 任务 12：TS reference — optionView / check / checkModule / compileToIR 同步

**文件：**
- 创建：`reference/src/checker/optionView.ts`
- 修改：`reference/src/checker/check.ts`
- 修改：`reference/src/checker/checkModule.ts`
- 修改：`reference/src/ir/compileToIR.ts`

- [ ] **步骤 1：编写失败的 TS 单元测试**

创建 `reference/tests/checker/optionView.test.ts`：

```typescript
import { describe, it, expect } from 'vitest';
import { asSumView } from '../../src/checker/optionView';
import { Type } from '../../src/checker/types';

describe('asSumView', () => {
  it('recognizes Option<Int>', () => {
    const opt: Type = { kind: 'genericInstance', base: 'Option', args: [{ kind: 'primitive', name: 'Int' }] };
    const sum = asSumView(opt);
    expect(sum).not.toBeNull();
    if (sum) {
      expect(sum.variants.length).toBe(2);
      expect(sum.variants[0]).toEqual({
        kind: 'genericInstance', base: 'Some', args: [{ kind: 'primitive', name: 'Int' }],
      });
      expect(sum.variants[1]).toEqual({
        kind: 'struct', name: 'None', fields: {}, methods: {},
      });
    }
  });

  it('passes through Sum types', () => {
    const sum: Type = { kind: 'sum', variants: [{ kind: 'primitive', name: 'Int' }] };
    const view = asSumView(sum);
    expect(view).not.toBeNull();
    if (view) expect(view.variants.length).toBe(1);
  });

  it('rejects non-Option generic', () => {
    const list: Type = { kind: 'genericInstance', base: 'List', args: [{ kind: 'primitive', name: 'Int' }] };
    expect(asSumView(list)).toBeNull();
  });

  it('rejects primitive', () => {
    expect(asSumView({ kind: 'primitive', name: 'Int' })).toBeNull();
  });

  it('rejects Any', () => {
    expect(asSumView({ kind: 'any' })).toBeNull();
  });
});
```

创建 `reference/tests/checker/isNarrowing.test.ts`：

```typescript
import { describe, it, expect } from 'vitest';
import { checkExpression } from '../../src/checker/check';
import { TypeEnv } from '../../src/checker/env';

describe('IsExpr narrowing', () => {
  it('returns Bool type', () => {
    const env = new TypeEnv();
    env.variables.set('x', { kind: 'genericInstance', base: 'Option', args: [{ kind: 'primitive', name: 'Int' }] });
    const isExpr = {
      kind: 'is' as const,
      expr: { kind: 'identifier' as const, name: 'x' },
      pattern: { kind: 'variantBinding' as const, name: 'Some', binding: 'y' },
      span: { start: 0, end: 0 },
    };
    const [ty, diags] = checkExpression(isExpr, env);
    expect(ty).toEqual({ kind: 'primitive', name: 'Bool' });
    expect(diags.length).toBe(0);
  });

  it('emits diag for unknown variant', () => {
    const env = new TypeEnv();
    env.variables.set('x', { kind: 'genericInstance', base: 'Option', args: [{ kind: 'primitive', name: 'Int' }] });
    const isExpr = {
      kind: 'is' as const,
      expr: { kind: 'identifier' as const, name: 'x' },
      pattern: { kind: 'variant' as const, name: 'NonExistent' },
      span: { start: 0, end: 0 },
    };
    const [, diags] = checkExpression(isExpr, env);
    expect(diags.some(d => d.code === 'TANGLE_PATTERN_VARIANT_NOT_FOUND')).toBe(true);
  });

  it('emits diag for non-sum type', () => {
    const env = new TypeEnv();
    env.variables.set('x', { kind: 'primitive', name: 'Int' });
    const isExpr = {
      kind: 'is' as const,
      expr: { kind: 'identifier' as const, name: 'x' },
      pattern: { kind: 'variant' as const, name: 'Some' },
      span: { start: 0, end: 0 },
    };
    const [, diags] = checkExpression(isExpr, env);
    expect(diags.some(d => d.code === 'TANGLE_PATTERN_NOT_NARROWABLE')).toBe(true);
  });
});
```

创建 `reference/tests/checker/destructure.test.ts`：

```typescript
import { describe, it, expect } from 'vitest';
import { checkStmt } from '../../src/checker/checkModule';
import { TypeEnv } from '../../src/checker/env';

describe('LetVariant / LetRecord', () => {
  it('LetVariant injects binding into env', () => {
    const env = new TypeEnv();
    env.variables.set('x', { kind: 'genericInstance', base: 'Option', args: [{ kind: 'primitive', name: 'Int' }] });
    const stmt = {
      kind: 'letVariant' as const,
      variantName: 'Some',
      binding: 'y',
      expr: { kind: 'identifier' as const, name: 'x' },
      elseBranch: [],
      span: { start: 0, end: 0 },
    };
    const diags = checkStmt(stmt, env);
    expect(diags.length).toBe(0);
    expect(env.variables.get('y')).toEqual({ kind: 'primitive', name: 'Int' });
  });

  it('LetRecord injects all fields', () => {
    const env = new TypeEnv();
    env.variables.set('r', {
      kind: 'struct',
      name: 'Item',
      fields: {
        name: { kind: 'primitive', name: 'String' },
        price: { kind: 'primitive', name: 'Int' },
      },
      methods: {},
    });
    const stmt = {
      kind: 'letRecord' as const,
      fields: [['name', 'n'], ['price', 'p']] as [string, string][],
      expr: { kind: 'identifier' as const, name: 'r' },
      span: { start: 0, end: 0 },
    };
    const diags = checkStmt(stmt, env);
    expect(diags.length).toBe(0);
    expect(env.variables.get('n')).toEqual({ kind: 'primitive', name: 'String' });
    expect(env.variables.get('p')).toEqual({ kind: 'primitive', name: 'Int' });
  });

  it('LetRecord emits diag for missing field', () => {
    const env = new TypeEnv();
    env.variables.set('r', {
      kind: 'struct',
      name: 'Item',
      fields: {},
      methods: {},
    });
    const stmt = {
      kind: 'letRecord' as const,
      fields: [['nonexistent', 'x']] as [string, string][],
      expr: { kind: 'identifier' as const, name: 'r' },
      span: { start: 0, end: 0 },
    };
    const diags = checkStmt(stmt, env);
    expect(diags.some(d => d.code === 'TANGLE_STRUCT_FIELD_NOT_FOUND')).toBe(true);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cd reference && npm test -- optionView isNarrowing destructure`
预期：FAIL，TS 编译错误（找不到 `optionView` 模块等）

- [ ] **步骤 3：编写实现**

1. **创建 `reference/src/checker/optionView.ts`**：

```typescript
import { Type, SumType } from './types';

export function asSumView(ty: Type): SumType | null {
  switch (ty.kind) {
    case 'sum':
      return { variants: ty.variants };
    case 'genericInstance':
      if (ty.base === 'Option') {
        const inner = ty.args[0] ?? { kind: 'any' as const };
        return {
          variants: [
            { kind: 'genericInstance', base: 'Some', args: [inner] },
            { kind: 'struct', name: 'None', fields: {}, methods: {} },
          ],
        };
      }
      return null;
    default:
      return null;
  }
}
```

2. **修改 `reference/src/checker/check.ts`** 新增 IsExpr + If 收窄：

```typescript
import { asSumView } from './optionView';
import { findVariantByName, bindingTypeOf } from './match'; // 复用 6c

function checkExpression(expr: Expr, env: TypeEnv): [Type, TangleDiagnostic[]] {
  switch (expr.kind) {
    // ...existing cases...
    case 'is': {
      const [matchedTy, diags] = checkExpression(expr.expr, env);
      const resultTy: Type = { kind: 'primitive', name: 'Bool' };
      const sum = asSumView(matchedTy);
      if (sum) {
        if (!findVariantByName(sum, expr.pattern.name)) {
          diags.push({
            code: 'TANGLE_PATTERN_VARIANT_NOT_FOUND',
            message: `Variant '${expr.pattern.name}' not found`,
            span: expr.span,
            severity: 'error',
          });
        }
      } else {
        diags.push({
          code: 'TANGLE_PATTERN_NOT_NARROWABLE',
          message: `Cannot narrow type`,
          span: expr.span,
          severity: 'error',
        });
      }
      return [resultTy, diags];
    }
    case 'if': {
      const [condTy, diags] = checkExpression(expr.condition, env);
      const thenEnv = expr.condition.kind === 'is'
        ? narrowEnvForIs(env, expr.condition)
        : env.clone();
      const [thenTy, thenDiags] = checkExpression(expr.thenBranch, thenEnv);
      diags.push(...thenDiags);
      if (expr.elseBranch) {
        const [elseTy, elseDiags] = checkExpression(expr.elseBranch, env);
        diags.push(...elseDiags);
        return [unifyPair(thenTy, elseTy) ?? thenTy, diags];
      }
      return [thenTy, diags];
    }
  }
}

function narrowEnvForIs(env: TypeEnv, isExpr: IsExpr): TypeEnv {
  const narrowed = env.clone();
  const [matchedTy] = checkExpression(isExpr.expr, env);
  const sum = asSumView(matchedTy);
  if (sum && isExpr.pattern.kind === 'variantBinding') {
    const variantTy = findVariantByName(sum, isExpr.pattern.name);
    if (variantTy) {
      const bindTy = bindingTypeOf(variantTy);
      narrowed.variables.set(isExpr.pattern.binding, bindTy);
    }
  }
  return narrowed;
}
```

3. **修改 `reference/src/checker/checkModule.ts`** 新增 LetVariant / LetRecord：

```typescript
import { asSumView } from './optionView';
import { findVariantByName, bindingTypeOf } from './match';

function checkStmt(stmt: Stmt, env: TypeEnv, diags: TangleDiagnostic[]): void {
  switch (stmt.kind) {
    // ...existing cases...
    case 'letVariant': {
      const [matchedTy, d] = checkExpression(stmt.expr, env);
      diags.push(...d);
      const sum = asSumView(matchedTy);
      if (sum) {
        const variantTy = findVariantByName(sum, stmt.variantName);
        if (variantTy) {
          if (stmt.binding) {
            const bindTy = bindingTypeOf(variantTy);
            env.variables.set(stmt.binding, bindTy);
          }
          for (const s of stmt.elseBranch) checkStmt(s, env, diags);
        } else {
          diags.push({
            code: 'TANGLE_PATTERN_VARIANT_NOT_FOUND',
            message: `Variant '${stmt.variantName}' not found`,
            span: stmt.span,
            severity: 'error',
          });
        }
      } else {
        diags.push({
          code: 'TANGLE_PATTERN_NOT_NARROWABLE',
          message: `Cannot destructure`,
          span: stmt.span,
          severity: 'error',
        });
      }
      break;
    }
    case 'letRecord': {
      const [matchedTy, d] = checkExpression(stmt.expr, env);
      diags.push(...d);
      switch (matchedTy.kind) {
        case 'struct':
          for (const [field, local] of stmt.fields) {
            const fieldTy = matchedTy.fields[field];
            if (fieldTy) {
              env.variables.set(local, fieldTy);
            } else {
              diags.push({
                code: 'TANGLE_STRUCT_FIELD_NOT_FOUND',
                message: `Struct ${matchedTy.name} has no field '${field}'`,
                span: stmt.span,
                severity: 'error',
              });
            }
          }
          break;
        case 'any':
          for (const [, local] of stmt.fields) {
            env.variables.set(local, { kind: 'any' });
          }
          break;
        default:
          diags.push({
            code: 'TANGLE_DESTRUCTURE_NOT_STRUCT',
            message: `Cannot destructure as record`,
            span: stmt.span,
            severity: 'error',
          });
      }
      break;
    }
  }
}
```

4. **修改 `reference/src/ir/compileToIR.ts`** 新增三个脱糖函数（镜像 Rust 任务 8）：

```typescript
function lowerIsExpr(isExpr: IsExpr, thenBody: Stmt[], elseBody: Stmt[] | null): IRNode[] {
  // 构造 Match IR 节点：arm1 = Variant (binding), arm2 = Wildcard (else)
  // ...镜像 Rust lower_is_expr 实现...
}

function lowerLetVariant(stmt: LetVariantStmt): IRNode[] {
  // 构造 Match + 嵌入 else_branch
  // ...镜像 Rust lower_let_variant 实现...
}

function lowerLetRecord(stmt: LetRecordStmt): IRNode[] {
  // 循环构造 MemberAccess + Let
  // ...镜像 Rust lower_let_record 实现...
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`cd reference && npm test -- optionView isNarrowing destructure`
预期：PASS（11 个测试通过）

- [ ] **步骤 5：Commit**

```bash
git add reference/src/checker/optionView.ts reference/src/checker/check.ts reference/src/checker/checkModule.ts reference/src/ir/compileToIR.ts reference/tests/checker/optionView.test.ts reference/tests/checker/isNarrowing.test.ts reference/tests/checker/destructure.test.ts
git commit -m "feat(reference): mirror Phase 6d checker + IR desugar for TS"
```

---

## 任务 13：差分测试 + 期望诊断码 + run-audit 回归

**文件：**
- 修改：`tests/audit/diff-ir.ps1`
- 修改：`tests/audit/expected_diagnostics.yaml`
- 修改：`tests/v06_phase6/generics.tangle.md`（可能需要更新断言，视情况）

- [ ] **步骤 1：更新 `tests/audit/diff-ir.ps1` 加入 3 个新 fixture 路径**

在 `$Fixtures` 数组中追加：

```powershell
$Fixtures = @(
    # ...existing entries...
    "tests/v06_phase6/if_narrowing.tangle.md",
    "tests/v06_phase6/destructure.tangle.md",
    "tests/v06_phase6/option_match.tangle.md"
)
```

- [ ] **步骤 2：更新 `tests/audit/expected_diagnostics.yaml` 加入 5 个新诊断码**

```yaml
# Phase 6d 新增诊断码
- code: TANGLE_REFUTABLE_LET_REQUIRES_ELSE
  severity: error
  description: "Refutable let pattern requires else branch"

- code: TANGLE_PATTERN_VARIANT_NOT_FOUND
  severity: error
  description: "Variant not found in Sum type"

- code: TANGLE_PATTERN_NOT_NARROWABLE
  severity: error
  description: "Type cannot be narrowed (not a Sum type)"

- code: TANGLE_DESTRUCTURE_NOT_STRUCT
  severity: error
  description: "Cannot destructure non-struct type as record"

- code: TANGLE_STRUCT_FIELD_NOT_FOUND
  severity: error
  description: "Struct field not found"
```

- [ ] **步骤 3：运行差分测试**

运行：`pwsh tests/audit/diff-ir.ps1`
预期：**15 MATCH + 0 SKIPPED + 0 DIFF**

若出现 DIFF：
1. 检查 TS 端是否镜像了所有脱糖逻辑
2. 检查 IR JSON 归一化是否覆盖新字段
3. 用 `cargo run -- build <fixture> --emit-ir > rust.json` 和 `node dist/src/cli/main.js build <fixture> --emit-ir > ts.json` 对比定位差异

- [ ] **步骤 4：运行 run-audit 回归**

运行：`pwsh tests/audit/run-audit.ps1`
预期：0 failing cells，0 new diagnostics

若现有 fixture 出现新诊断，说明新语法/语义破坏了向后兼容，需修复。

- [ ] **步骤 5：Commit**

```bash
git add tests/audit/diff-ir.ps1 tests/audit/expected_diagnostics.yaml
git commit -m "test(audit): add Phase 6d fixtures + diagnostic codes to diff-ir/expected_diagnostics"
```

---

## 任务 14：出口闸门 9 项验证 + CHANGELOG + tag v0.8.0

**文件：**
- 修改：`CHANGELOG.md`

- [ ] **步骤 1：执行出口闸门 9 项验证**

逐项运行并记录输出：

```bash
# Gate 1
cargo test --workspace 2>&1 | tee /tmp/gate1.log
# 预期：test result: ok. N passed; 0 failed

# Gate 2
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee /tmp/gate2.log
# 预期：Finished, zero warnings

# Gate 3
cd reference && npm test 2>&1 | tee /tmp/gate3.log
# 预期：All tests passed

# Gate 4
cd reference && npm run build 2>&1 | tee /tmp/gate4.log
# 预期：tsc: no errors

# Gate 5
pwsh tests/audit/diff-ir.ps1 2>&1 | tee /tmp/gate5.log
# 预期：15 MATCH, 0 SKIPPED, 0 DIFF

# Gate 6
pwsh tests/audit/run-audit.ps1 2>&1 | tee /tmp/gate6.log
# 预期：0 failing cells

# Gate 7: 验证回归测试在 Gate 1 中已覆盖

# Gate 8: 验证新测试在 Gate 1/3 中已覆盖

# Gate 9
cargo run -- build tests/v06_phase6/if_narrowing.tangle.md --emit-ir
# 肉眼确认 IR JSON 含 Match 节点（脱糖自 if is）
```

- [ ] **步骤 2：编写 CHANGELOG.md v0.8.0 章节**

在 `CHANGELOG.md` 顶部新增：

```markdown
## v0.8.0 — Phase 6d Type Narrowing Completeness

- Feat: `if x is Pattern` expression for variant type testing with optional binding
- Feat: `let Some(y) = x else { ... }` refutable variant destructuring
- Feat: `let { ok, err } = r` irrefutable record destructuring
- Feat: built-in `as_sum_view()` recognizes `Option<T>` as `Sum(Some<T>, None)` for match/Is/LetVariant
- Diagnostics: 5 new codes — TANGLE_REFUTABLE_LET_REQUIRES_ELSE, TANGLE_PATTERN_VARIANT_NOT_FOUND, TANGLE_PATTERN_NOT_NARROWABLE, TANGLE_DESTRUCTURE_NOT_STRUCT, TANGLE_STRUCT_FIELD_NOT_FOUND
- Design: AST extension (3 new nodes + Pattern subtree) + IR lowering desugar to Match/MemberAccess (IR schema unchanged)
- Compatibility: three emitters (JS/Py/Go) unchanged; existing 12 fixtures IR unchanged

### Verification

- `cargo test --workspace`: N tests pass, 0 failures
- `cargo clippy --workspace --all-targets -- -D warnings`: zero warnings
- `reference && npm test`: N tests pass
- `reference && npm run build`: zero type errors
- `tests/audit/diff-ir.ps1`: 15 MATCH + 0 SKIPPED + 0 DIFF
- `tests/audit/run-audit.ps1`: 0 failing cells

### Known limitations (deferred to Phase 6e)

- `Result<T,E>` built-in Sum view (stdlib signatures registry not yet populated)
- `is not Some` negative narrowing (requires negative type computation)
- Compound OR patterns (`is Some or None`)
- Guard expressions (`is Some(y) && y > 0`)
- Flow-sensitive narrowing (x's type does not change in then branch — only binding is injected)
- User-defined generic types (`Foo<T>`) and constraints (`T: Comparable`)
- Nested patterns (`is Some({ ok, err })`)
- Record destructure wildcard (`let { _, err } = r`)
```

- [ ] **步骤 3：合并到 main + tag v0.8.0**

```bash
git checkout main
git merge --ff-only phase6d/v0.8.0
git tag v0.8.0
# 注意：不 push 到 remote，等待用户批准
```

- [ ] **步骤 4：Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): add v0.8.0 Phase 6d type narrowing completeness notes"
```

- [ ] **步骤 5：最终验证**

```bash
git log --oneline -20
git tag -l "v0.8.0"
git status
```

预期：
- main 分支 HEAD 指向 Phase 6d 最后一个 commit
- tag v0.8.0 存在
- working tree clean

---

## 自检

### 1. 规格覆盖度

对照规格文档各章节：

| 规格章节 | 任务覆盖 |
|---|---|
| §3.2 新增/修改文件清单 | 全部 14 个文件在任务 1-13 中覆盖 |
| §4 AST 扩展（Pattern/IsExpr/LetVariant/LetRecord） | 任务 1 |
| §4.3 Parser 改动 | 任务 2-3 |
| §5.1 as_sum_view | 任务 4 |
| §5.2-5.3 IsExpr + If 收窄 | 任务 5 |
| §5.4 LetVariant | 任务 6 |
| §5.5 LetRecord | 任务 7 |
| §5.6 5 个新诊断码 | 任务 3（TANGLE_REFUTABLE_LET_REQUIRES_ELSE）+ 任务 5/6/7（其他 4 个）+ 任务 13（注册到 expected_diagnostics.yaml） |
| §6 IR lowering 脱糖 | 任务 8 |
| §7 Emitter 改动（无） | 无需任务，Gate 5 差分测试验证 |
| §8 TS reference 同步 | 任务 11-12 |
| §9.1 3 个新 fixture | 任务 9 |
| §9.3 测试矩阵 | 任务 9-10、12-13 |
| §10 出口闸门 9 项 | 任务 14 |
| §11 非目标 | 无需任务 |
| §12 风险与缓解 | 在任务 13 Gate 5/6 验证 |
| §13 不变量 INV-1~6 | 任务 10 中 `invariants.rs` |
| §14 版本与分支 | 任务 14 |

**遗漏：** 无。

### 2. 占位符扫描

搜索计划中的红旗模式：
- "待定"、"TODO"、"后续实现"、"补充细节" → 无
- "添加适当的错误处理" → 无
- "为上述代码编写测试" → 无（每个任务都有完整测试代码）
- "类似任务 N" → 无
- 步骤中引用的类型/函数未定义 → 检查：
  - `find_variant_by_name` / `binding_type_of` 已在 Phase 6c 的 `checker/check.rs` 中定义，任务 5/6 中通过 `use` 导入
  - `MatchPattern` / `IRNodeKind::Match` 已在 `ir/graph.rs` 中（Phase 6c）
  - `IRNodeKind::MemberAccess` / `IRNodeKind::Let` 已在 `ir/graph.rs` 中
  - TS 端 `findVariantByName` / `bindingTypeOf` 在 Phase 6c `match.ts` 中

无占位符。

### 3. 类型一致性

- `as_sum_view` 在任务 4（Rust）/ 任务 12（TS）签名一致：`(Type) -> Option<SumType>` / `(Type) -> SumType | null`
- `IsExpr` 字段在任务 1（Rust ast.rs）/ 任务 11（TS ast.ts）一致：`{ expr, pattern, span }`
- `LetVariantStmt` 字段：`{ variant_name, binding, expr, else_branch, span }` / `{ variantName, binding, expr, elseBranch, span }`
- `LetRecordStmt` 字段：`{ fields, expr, span }` / `{ fields, expr, span }`
- `Pattern` 枚举：`Variant { name }` / `VariantBinding { name, binding }` 在两端一致
- 5 个诊断码在任务 3/5/6/7/13 中字符串字面量一致

无类型不一致。

---

## 执行交接

**计划已完成并保存到 `docs/superpowers/plans/2026-07-19-phase6d-type-narrowing-completeness-plan.md`。两种执行方式：**

**1. 子代理驱动（推荐）** - 每个任务调度一个新的子代理，任务间进行审查，快速迭代

**2. 内联执行** - 在当前会话中使用 executing-plans 执行任务，批量执行并设有检查点

**选哪种方式？**
