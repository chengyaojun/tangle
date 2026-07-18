# Phase 6d: 类型收窄完整性 设计规格

> **本阶段属于 B5 类型系统扩展的第四阶段。** 完整 B5 通过分阶段实现：
> - **Phase 6a（已完成）：** TS 端对齐 Rust（闭合 order-service）+ Rust/TS 双端局部泛型推导
> - **Phase 6b（已完成）：** 泛型类型信息从 checker 贯通到 IR 与 codegen，Py/Go emitter 生成类型标注
> - **Phase 6c（已完成）：** 返回类型推断 + Match arm 类型收窄
> - **Phase 6d（本规格）：** If 条件收窄 + Variant/Record destructure + Option<T> 内置 Sum 视图

**目标：** 在 Phase 6c 已建立的 Match arm 收窄基础上，新增三类源代码层收窄机制 —— `if x is Pattern`、`let Pattern = expr else { ... }`、`let { fields } = expr`，并通过内置 `as_sum_view()` 让 `Option<T>` 自动暴露为 `Sum(Some<T>, None)`，让用户在不显式声明 Sum 类型的前提下使用 `match`/`is`/refutable let。

**成功指标：** `tests/audit/diff-ir.ps1` 达成 **15 MATCH + 0 SKIPPED + 0 DIFF**（12 现有 + 3 新增 fixture，Rust/TS 双端 IR 一致）；IR schema 零变更；三个 emitter 零改动；新增 5 个诊断码在 Rust/TS 双端对齐。

---

## 1. 现状分析

### 1.1 Phase 6c 已具备的基础设施

- `Type::Sum` 变体已支持 serde 序列化
- `find_variant_by_name` / `variant_name` / `binding_type_of` 三个辅助函数（[check.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs)）已实现，作用于任意 `SumType`
- `unify_all` / `unify_pair` 共享辅助函数（[unify.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/unify.rs)）
- `infer_return_types` pass（[infer_return_types.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/infer_return_types.rs)）通过 `Stmt` 分发覆盖所有语句类型
- `Expr::Match` 分支已支持 arm binding 类型注入 + arm body 类型统一
- `IRFunction.return_type: Option<Type>` 字段已填充

### 1.2 stdlib `Option<T>` 当前作为 Struct 建模

[stdlib/signatures.rs:157](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L157) 中 `Option` 模块以 `Some`/`None` 作为其方法/构造器，类型 `Option<T>` 是 `GenericInstance { base: "Option", args: [T] }`，**不是** `Type::Sum`。用户不能直接 `match opt { Some(y) => ...; None => ... }`，因为 `Match` 收窄分支在 6c 仅识别 `Type::Sum`。

### 1.3 `Result<T,E>` 未在 Rust 编译器建模

stdlib 注册表无 `Result` 模块（仅 JS runtime 有 `TangleResult<T>`）。Phase 6d 不引入 `Result`。

### 1.4 现有 `let` 语句仅支持单标识符

[parser.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/parser/parser.rs) 的 `let` 解析路径只接受 `let <Ident> = <expr>`，不支持 `let Pattern = expr` 或 `let { fields } = expr`。

### 1.5 `if` 表达式条件不支持类型测试

[check.rs:199-209](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L199-L209) 的 `Expr::If` 分支当前只检查 `condition` 类型是否为 `Bool`，不做收窄。

### 1.6 IR 不识别"类型测试"节点

[compile_to_ir.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/compile_to_ir.rs) 现有节点种类（`Match` / `MemberAccess` / `Let` / `Load` / `Return` 等）已能表达脱糖后的 IsExpr / LetVariant / LetRecord，无需新增节点种类。

---

## 2. 方案选择

### 2.1 采用方案：AST 扩展 + IR 脱糖（方案 C）

**核心思路：**
- AST 引入 3 个新节点（`Expr::Is` / `Stmt::LetVariant` / `Stmt::LetRecord`）+ `Pattern` 子树
- Checker 直接处理新节点，复用 6c 的 `arm_env` / `unify_all` / `binding_type_of` 基础设施
- IR lowering 阶段在 `compile_to_ir.rs` 中将新节点脱糖为现有 `Match` / `MemberAccess` / `Let` 组合
- IR schema 零变更，三个 emitter 零改动，差分测试维持现有 MATCH 数量

### 2.2 不采用的方案

**方案 A：Parser 层完全脱糖**
- 劣势：无法给出"refutable let 缺少 else"、"is pattern 类型不匹配"等针对性诊断
- 劣势：错误信息指向脱糖后的 match 而非源代码位置

**方案 B：AST 一等公民 + 全栈贯通**
- 劣势：IR schema 变更影响差分测试（KNOWN_DIFF 可能新增）
- 劣势：三套 emitter 都需改，任务量翻倍（~18-22 任务）

### 2.3 关键设计决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| 范围 | If 收窄 + Variant destructure + Record destructure + Option 视图 | 闭合 6c 推迟的"收窄完整性"主题 |
| Result 类型 | 推迟到 Phase 6e | stdlib 未建模 Result，避免范围扩张 |
| Option 视图机制 | 内置 `as_sum_view` | 不改 stdlib 结构，不动类型等价语义 |
| If 语法范围 | 仅 `is Variant` + `is Variant(binding)` | 覆盖 80% 场景；负向收窄与复合 OR 推迟 |
| Destructure 范围 | Variant（refutable）+ Record（irrefutable） | 一次性闭合解构能力 |
| refutable let 缺 else | parser 阶段拒绝 | 避免不完整 AST 流入 checker |
| IsExpr 行为 | 只注入 binding，不改变 `x` 类型 | 与 6c Match arm 行为一致；避免 flow-sensitive |
| TS reference | 同步实现 parser/checker/IR 脱糖 | 差分测试要求双端 IR JSON 一致 |

---

## 3. 架构概览

### 3.1 数据流

```
TangleModule
    ↓
frontend::compile_module          ← Parser 新增 is/let-pattern/let-record 语法
    ↓
TangleModule (含 Expr::Is / Stmt::LetVariant / Stmt::LetRecord 节点)
    ↓
check_module                      ← Checker 新增 Pattern 收窄逻辑 + as_sum_view()
    ↓
CheckedModule { type_env, return_types, ... }
    ↓
compile_to_ir                     ← IR lowering 脱糖为 Match / MemberAccess / Let
    ↓
RuleGraph (无新节点种类)          ← IR schema 零变更
    ↓
emit_js / emit_py / emit_go       ← 完全不变
```

### 3.2 核心设计原则

1. **AST 保留新节点** —— 用于精确诊断与 source span 跟踪
2. **Checker 直接处理新节点** —— 复用 6c 的 `arm_env`、`unify_all`、`binding_type_of` 基础设施
3. **IR 脱糖** —— `compile_to_ir` 将新节点翻译为现有 `Match` / `MemberAccess` / `Let` 组合
4. **IR schema 零变更** —— 差分测试维持 12 MATCH，TS 端只需镜像 checker + parser
5. **Option<T> Sum 视图** —— checker 内置 `as_sum_view(&Type) -> Option<SumType>`，识别 `Option<T>` 名称并合成 `Sum(Some<T>, None)`

### 3.3 新增/修改文件清单

| 文件 | 操作 | 职责 |
|------|------|------|
| `compiler/tangle-cli/src/ast.rs` | 修改 | 新增 `IsExpr` / `LetVariantStmt` / `LetRecordStmt` / `Pattern` |
| `compiler/tangle-cli/src/parser/lexer.rs` | 修改 | 新增 `TokenKind::Is` 保留字 |
| `compiler/tangle-cli/src/parser/parser.rs` | 修改 | 解析 `is` 表达式与扩展 `let` 语法 |
| `compiler/tangle-cli/src/checker/option_view.rs` | **新建** | `as_sum_view` 函数 + 单元测试 |
| `compiler/tangle-cli/src/checker/mod.rs` | 修改 | 注册新模块 + re-export |
| `compiler/tangle-cli/src/checker/check.rs` | 修改 | `Expr::Is` 分支 + `Expr::If` 收窄集成 |
| `compiler/tangle-cli/src/checker/check_module.rs` | 修改 | `Stmt::LetVariant` / `Stmt::LetRecord` 分支 |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | 修改 | `lower_is_expr` / `lower_let_variant` / `lower_let_record` 三个新函数 |
| `reference/src/ast.ts` | 修改 | 镜像 Rust AST 新节点 |
| `reference/src/parser/lexer.ts` | 修改 | 镜像 is 关键字 |
| `reference/src/parser/parser.ts` | 修改 | 镜像 parser 新语法 |
| `reference/src/checker/optionView.ts` | **新建** | 镜像 `as_sum_view` |
| `reference/src/checker/check.ts` | 修改 | 镜像 IsExpr + If 收窄集成 |
| `reference/src/checker/checkModule.ts` | 修改 | 镜像 LetVariant / LetRecord 分支 |
| `reference/src/ir/compileToIR.ts` | 修改 | 镜像三个脱糖函数 |

---

## 4. AST 扩展

### 4.1 新增节点定义

在 [ast.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ast.rs) 中扩展：

```rust
/// 类型测试表达式: `x is Pattern`
/// 返回 Bool，副作用是当匹配成功时在 then 分支中收窄 binding 类型
pub struct IsExpr {
    pub expr: Box<Expr>,
    pub pattern: Pattern,
    pub span: Span,
}

/// Refutable 变体解构: `let Some(y) = expr else { ... }`
pub struct LetVariantStmt {
    pub variant_name: String,        // "Some"
    pub binding: Option<String>,    // Some(y) → Some, None → None
    pub expr: Box<Expr>,
    pub else_branch: Vec<Stmt>,     // 必须存在 else 分支
    pub span: Span,
}

/// 不可反驳的 Record 解构: `let { ok, err } = expr`
pub struct LetRecordStmt {
    pub fields: Vec<(String, String)>,  // (field_name, local_var)
    pub expr: Box<Expr>,
    pub span: Span,
}
```

`Expr` 枚举新增 `Is(IsExpr)`；`Stmt` 枚举新增 `LetVariant(LetVariantStmt)` 与 `LetRecord(LetRecordStmt)`。

### 4.2 Pattern 子树

`IsExpr.pattern` 复用一个统一的 `Pattern` 枚举：

```rust
pub enum Pattern {
    /// `is Some` —— 仅测试 variant 名，无 binding
    Variant { name: String },
    /// `is Some(y)` —— 测试 variant 名并绑定 payload
    VariantBinding { name: String, binding: String },
}
```

**设计理由：**
- Phase 6d 范围内 Pattern 只需覆盖 variant 形式（与 `is` 语法匹配）
- Record destructure 走独立的 `LetRecordStmt`，不复用 `Pattern`（其语义是字段提取，不是收窄）
- 未来若需 `is Some(y) and y > 0` 复合模式，可扩展 `Pattern::And`/`Pattern::Guard`

### 4.3 Parser 改动

[parser.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/parser/parser.rs) 新增：

1. **`is` 关键字**：新增 `TokenKind::Is`（保留字），lexer 识别
2. **`if` 表达式解析扩展**：解析 `if <expr> is <Pattern> { ... }` 时构造 `IsExpr`
3. **`let` 语句解析扩展**：
   - `let <Ident> = <expr>` —— 现有路径（无变化）
   - `let <Variant>(<Ident>) = <expr> else { ... }` —— 构造 `LetVariantStmt`
   - `let { <field>, <field>: <local>, ... } = <expr>` —— 构造 `LetRecordStmt`

### 4.4 语法形式表

| 语法形式 | AST 节点 | 备注 |
|---|---|---|
| `if x is Some { ... }` | `If { cond: Is(x, Variant("Some")), then, else }` | 无 binding，then 中 x 仍为原类型 |
| `if x is Some(y) { ... }` | `If { cond: Is(x, VariantBinding("Some","y")), then, else }` | then 中 y 收窄为 payload 类型 |
| `let Some(y) = x else { ... }` | `LetVariant { variant_name: "Some", binding: Some("y"), expr, else_branch }` | else 分支必需 |
| `let None = x else { ... }` | `LetVariant { variant_name: "None", binding: None, expr, else_branch }` | 用于"断言为 None" |
| `let { ok, err } = r` | `LetRecord { fields: [("ok","ok"),("err","err")], expr }` | 字段名 = 局部变量名 |
| `let { ok: o, err: e } = r` | `LetRecord { fields: [("ok","o"),("err","e")], expr }` | 重命名语法 |

### 4.5 拒绝的语法

- `if x is not Some` —— 负向收窄推迟
- `let Some(y) = x`（无 else） —— parser 阶段拒绝，报 `TANGLE_REFUTABLE_LET_REQUIRES_ELSE`
- `if x is Some(y) or None` —— 复合 OR 模式推迟
- `if x is Some(y) && y > 0` —— guard 表达式推迟
- `let { _, err } = r` —— 通配符字段推迟到 Phase 6e
- `is Some({ ok, err })` —— 嵌套 pattern 推迟到 Phase 6e

---

## 5. Checker 收窄逻辑

### 5.1 内置 Sum 视图 `as_sum_view`

新建 [checker/option_view.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/option_view.rs)：

```rust
use crate::checker::types::*;

/// 把已知类型识别为 Sum 视图。
/// 当前仅识别 `Option<T>`；`Result<T,E>` 推迟到 Phase 6e。
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

**复用点：** Phase 6c 的 `find_variant_by_name`、`variant_name`、`binding_type_of` 三个辅助函数仍直接作用于 `SumType`，无需修改 —— 它们只看 `SumType.variants`，不关心 Sum 是用户声明还是 Option 视图。

### 5.2 IsExpr 收窄

修改 [check.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs) 新增 `Expr::Is` 分支：

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

### 5.3 If 表达式收窄集成

修改 `Expr::If` 分支，新增对 `condition = IsExpr` 的特殊处理：

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

    // else 分支不收窄（已确认 Phase 6d 不做负向收窄）
    let ty = if let Some(ref else_b) = e.else_branch {
        let (else_ty, mut else_diags) = check_expression(else_b, env);
        diags.append(&mut else_diags);
        unify_pair(&then_ty, &else_ty).unwrap_or_else(|| then_ty.clone())
    } else {
        then_ty
    };
    (ty, diags)
}

/// 根据 IsExpr 构造收窄后的 env：注入 binding 类型到 then 分支可见
fn narrow_env_for_is(env: &TypeEnv, is_e: &IsExpr) -> TypeEnv {
    let mut narrowed = env.clone();
    if let Ok((matched_ty, _)) = try_check_expression(&is_e.expr, env) {
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

**关键点：** `IsExpr` 在 then 分支内**只注入 binding**（如 `y`），**不改变 `x` 本身的类型**。即 `if x is Some(y) { ... }` 中，then 分支 `x` 仍是 `Option<Int>`，但新增了 `y: Int`。这避免了对"then 分支中 x 已收窄"的复杂 flow-sensitive 分析，与 Phase 6c Match arm 行为一致。

### 5.4 LetVariant 处理

修改 [check_module.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check_module.rs) 中 `Stmt` 分发，新增 `Stmt::LetVariant`：

```rust
Stmt::LetVariant(s) => {
    let (matched_ty, mut diags) = check_expression(&s.expr, block_env);

    if let Some(sum) = as_sum_view(&matched_ty) {
        if let Some(variant_ty) = find_variant_by_name(&sum, &s.variant_name) {
            if let Some(ref bind_name) = s.binding {
                let bind_ty = binding_type_of(variant_ty);
                block_env.variables.insert(bind_name.clone(), bind_ty);
            }
            for stmt in &s.else_branch {
                check_stmt(stmt, block_env, &mut diags);
            }
        } else {
            diags.push(TangleDiagnostic {
                code: "TANGLE_PATTERN_VARIANT_NOT_FOUND".into(),
                message: format!("Variant '{}' not found in type {}", s.variant_name, type_display(&matched_ty)),
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
    diags
}
```

**else 缺失诊断：** 在 parser 阶段就拒绝 `let Some(y) = x`（无 else）—— 报 `TANGLE_REFUTABLE_LET_REQUIRES_ELSE`，避免 checker 收到不完整 AST。

### 5.5 LetRecord 处理

新增 `Stmt::LetRecord` 分支：

```rust
Stmt::LetRecord(s) => {
    let (matched_ty, mut diags) = check_expression(&s.expr, block_env);

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
            // Any 兼容：所有 binding 推断为 Any
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
    diags
}
```

### 5.6 新增诊断码汇总

| 诊断码 | 触发场景 | 严重性 |
|---|---|---|
| `TANGLE_REFUTABLE_LET_REQUIRES_ELSE` | parser 阶段：`let Variant(...) = x` 无 else | error（parse 阶段拒绝） |
| `TANGLE_PATTERN_VARIANT_NOT_FOUND` | `is Some` / `let Some = x` 但 Sum 无 Some variant | error |
| `TANGLE_PATTERN_NOT_NARROWABLE` | `is` / `let Variant` 作用于非 Sum 类型（如 Int、Struct） | error |
| `TANGLE_DESTRUCTURE_NOT_STRUCT` | `let { ... } = x` 但 x 非 Struct | error |
| `TANGLE_STRUCT_FIELD_NOT_FOUND` | `let { foo } = r` 但 Struct 无 foo 字段 | error |

**TS fatal 兼容性：** 所有新诊断码以 `TANGLE_PATTERN_*` / `TANGLE_DESTRUCTURE_*` / `TANGLE_REFUTABLE_*` / `TANGLE_STRUCT_FIELD_*` 为前缀，**不属于** TS 现有 fatal 集合 `TANGLE_TYPE_*` / `TANGLE_PARSE_*`。Fixture 设计需避免触发，但即便误触发也不会让 TS fatal exit。

### 5.7 复用 6c 基础设施清单

| 6c 已有 | Phase 6d 直接复用 |
|---|---|
| `unify_all` / `unify_pair`（[unify.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/unify.rs)） | If 表达式 then/else 统一（无变化） |
| `find_variant_by_name`（[check.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs)） | IsExpr / LetVariant 查找 variant |
| `variant_name`（同上） | `as_sum_view` 合成的 variants 名识别 |
| `binding_type_of`（同上） | IsExpr / LetVariant 绑定 payload 类型 |
| `infer_return_types` pass | 通过新 Stmt 分发自动覆盖（IsExpr/LetVariant/LetRecord 内的 return 也被收集） |

---

## 6. IR lowering 脱糖

### 6.1 设计原则

IR 节点不引入新种类。`compile_to_ir.rs` 在遍历 AST 时遇到 `Expr::Is` / `Stmt::LetVariant` / `Stmt::LetRecord` 即就地翻译为现有 IR 节点组合，让下游 codegen 完全无感知。

### 6.2 Expr::Is 脱糖 → Match 表达式

`if x is Some(y) { then_body }` 的 IR 结构：

```
Before lowering (AST):
  If {
    cond: Is(x, VariantBinding("Some", "y")),
    then: [Stmt...],
    else: None
  }

After lowering (IR, 节点级):
  Match {
    matched_expr: x,                 ← load x to n0
    arms: [
      Arm {
        pattern: Variant("Some", "y"),
        body: [then_body stmts]        ← 直接嵌入 then 分支节点
      },
      Arm {
        pattern: Wildcard,
        body: [exit / continue]        ← 隐式 fallthrough
      }
    ]
  }
```

**实现位置：** [compile_to_ir.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/compile_to_ir.rs) 的 `emit_if_branch_body` 函数中，检测 `condition = Expr::Is` 时走专用 lowering 路径，构造等价 Match IR。

**Match arm 收窄复用：** Phase 6c 已有的 `lower_match_arms` 会自动处理 `Variant("Some", "y")` 模式 —— 直接复用，无需新代码。

### 6.3 Stmt::LetVariant 脱糖 → Match + else 分支

```
AST:
  LetVariant {
    variant_name: "Some",
    binding: Some("y"),
    expr: x,
    else_branch: [stmt1, stmt2, ...]
  }

IR (节点序列):
  n0: load x
  n1: Match {
    matched: n0,
    arms: [
      Arm {
        pattern: Variant("Some", "y"),
        body: []                       ← 空 body，binding y 通过 arm pattern 注入后续 block_env
      },
      Arm {
        pattern: Wildcard,
        body: [stmt1, stmt2, ...]      ← else_branch 节点直接嵌入
      }
    ]
  }
  n2..: 后续语句（在 then 分支的"成功"路径上）
```

**关键点：** Match IR 的执行模型已支持 arm body 为节点序列；wildcard arm 嵌入 else_branch 节点即可。codegen 端 `emit_match` 已处理 wildcard arm 与 variant arm，无需修改。

### 6.4 Stmt::LetRecord 脱糖 → 多个 MemberAccess + Let

```
AST:
  LetRecord {
    fields: [("ok", "o"), ("err", "e")],
    expr: r
  }

IR (节点序列):
  n0: load r                           ← temp expr
  n1: Let { name: "o", value: MemberAccess(n0, "ok") }
  n2: Let { name: "e", value: MemberAccess(n0, "err") }
```

**实现：** `compile_to_ir` 在 `Stmt::LetRecord` 分支中循环生成 `MemberAccess` + `Let` 节点。复用 Phase 6a 已有的 `lower_member_access` 与 `lower_let` 函数。

### 6.5 Option<T> 视图的 IR 处理

**重要：** `as_sum_view` 仅在 **checker** 中使用。IR 不需要识别 Option —— 因为：
- 用户写 `match option { Some(y) => ...; None => ... }` 时，AST 已是 `Match` 表达式
- checker 阶段 `Expr::Match` 分支用 `as_sum_view(&matched_ty)` 识别 Option 为 Sum，做 arm 收窄
- IR lowering 阶段看到的是 `Match` AST 节点（已是标准形式），按现有 Match lowering 处理
- Codegen 端 JS/Py/Go emitter 已支持 Match IR 节点的输出，无需变更

**唯一例外：** IsExpr / LetVariant 中若 `matched_ty` 是 `Option<T>`，IR lowering 仍按 Match 脱糖（生成 variant pattern `Some`/`None`），与用户显式 Match 行为一致。

### 6.6 节点 ID 生成与 source span

- IsExpr 脱糖生成的 Match 节点继承 `IsExpr.span`
- LetVariant 脱糖生成的 Match 节点继承 `LetVariantStmt.span`
- LetRecord 脱糖生成的每个 MemberAccess/Let 节点继承 `LetRecordStmt.span`（同一行）
- 节点 ID 由 `FreshNodeId::next()` 分配，遵循现有 `n0, n1, ...` 命名约定

### 6.7 compile_to_ir 改动清单

| 函数 | 改动 |
|---|---|
| `collect_functions` | 无变化（仍遍历 Callable heading 的 parsed_blocks） |
| `emit_block_body` | 新增 `Stmt::LetVariant` / `Stmt::LetRecord` 分支，调用新增 `lower_let_variant` / `lower_let_record` |
| `emit_expression` | 新增 `Expr::Is` 分支，调用新增 `lower_is_expr` |
| `lower_let_variant`（新增） | 构造 Match IR + 嵌入 else_branch 节点 |
| `lower_let_record`（新增） | 循环构造 MemberAccess + Let 节点 |
| `lower_is_expr`（新增） | 构造 Match IR + 嵌入 then 分支节点（供 if 表达式使用） |

### 6.8 IR JSON 输出（脱糖后等价于显式 Match）

`if x is Some(y) { return y }` 脱糖后 IR JSON 与 `match x { Some(y) => return y; _ => {} }` 完全一致：

```json
{
  "kind": "match",
  "matchedExpr": { "kind": "load", "name": "x" },
  "arms": [
    { "pattern": { "kind": "variant", "name": "Some", "binding": "y" }, "body": [...] },
    { "pattern": { "kind": "wildcard" }, "body": [] }
  ]
}
```

差分测试维持现有 12 MATCH，因 IR schema 未变。新增 3 个 fixture 后目标 15 MATCH。

---

## 7. Emitter 改动（无）

### 7.1 三个 emitter 均无改动

- [js_emitter.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/js_emitter.rs)：JS 看到的 IR 节点全是 `Match` / `MemberAccess` / `Let`，与现有处理路径完全一致
- [py_emitter.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/py_emitter.rs)：同上
- [go_emitter.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/go_emitter.rs)：同上
- [type_map.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/type_map.rs)：无需扩展，Option Sum 视图的 variant 类型（`Some<T>`、`None`）已在 6b 覆盖

### 7.2 IR JSON 输出

[ir_json.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/ir_json.rs) 无需修改 —— 脱糖后的 IR 节点种类未变，序列化字段不变。

### 7.3 差分测试预期

- `tests/audit/diff-ir.ps1` 维持 **12 MATCH + 0 SKIPPED + 0 DIFF**（Phase 6c 状态）
- Phase 6d 新增 3 个 fixture 后，目标变为 **15 MATCH + 0 SKIPPED + 0 DIFF**

---

## 8. TS reference 同步

### 8.1 文件对应清单

| Rust 文件 | TS 对应 | 改动类型 |
|---|---|---|
| `checker/option_view.rs`（新建） | `reference/src/checker/optionView.ts`（新建） | 镜像 `as_sum_view` |
| `checker/check.rs` Expr::Is 分支 | `reference/src/checker/check.ts` | 新增 IsExpr 处理 + If 收窄集成 |
| `checker/check_module.rs` LetVariant/LetRecord | `reference/src/checker/checkModule.ts` | 新增两条 stmt 分发分支 |
| `parser/parser.rs` is 关键字 + let 扩展 | `reference/src/parser/parser.ts` | 镜像新语法 |
| `parser/lexer.rs` TokenKind::Is | `reference/src/parser/lexer.ts` | 新增 is 关键字 |
| `ast.rs` IsExpr/LetVariant/LetRecord/Pattern | `reference/src/ast.ts` | 新增类型定义 |
| `ir/compile_to_ir.rs` lower_is/let_variant/let_record | `reference/src/ir/compileToIR.ts` | 镜像脱糖逻辑 |

### 8.2 TS 端注意事项

- **fatal 风险：** 新诊断码 `TANGLE_PATTERN_*` / `TANGLE_DESTRUCTURE_*` / `TANGLE_REFUTABLE_*` / `TANGLE_STRUCT_FIELD_*` 不在 TS fatal 集合内（`TANGLE_TYPE_*` / `TANGLE_PARSE_*`）。Fixture 设计时仍需避免触发，但即便误触发也不会让 TS fatal exit
- **`as_sum_view` TS 实现：** `Type::GenericInstance` 在 TS 是 `{ kind: "genericInstance", base, args }`，`base === "Option"` 即可识别
- **`is` 关键字：** TS lexer 需在标识符扫描时识别 `is` 为保留字（与 `match`/`return`/`let` 等并列）
- **unify 已有：** Phase 6a 已在 TS 端实现 `unify/substitute`，Phase 6c 已实现 `unify_all/unify_pair`，可直接复用
- **noUncheckedIndexedAccess：** TS `args[N]` 需 `!` 断言，与 6c 一致

### 8.3 TS 端单元测试

新增三个测试文件，镜像 Rust 单元测试：

| 测试文件 | 覆盖内容 |
|---|---|
| `reference/tests/checker/optionView.test.ts` | `as_sum_view` 识别 Option<T>、不识别 Result、不识别其他类型 |
| `reference/tests/checker/isNarrowing.test.ts` | IsExpr 类型推导 + binding 注入 + 诊断码 |
| `reference/tests/checker/destructure.test.ts` | LetVariant / LetRecord binding 注入 + 诊断码 |

### 8.4 IR JSON 一致性

TS 端 `compileToIR.ts` 的脱糖产物必须与 Rust 端逐字节一致（经 ir-diff 归一化后）。归一化规则复用 Phase 6b/6c 已有的：
- null strip（`returnType`/`guard`/`type`/`group`/`style`/`priority` 等）
- node ID 重映射（`n0, n1, ...`）
- `methods: {}` strip（TS 总是序列化，Rust skip_serializing_if）

### 8.5 tsconfig.json

无需修改 —— 新文件位于 `reference/src/` 与 `reference/tests/` 下，沿用现有严格模式。

---

## 9. Fixture 与测试策略

### 9.1 新增 Fixture（3 个，位于 `tests/v06_phase6/`）

**Fixture 1: `if_narrowing.tangle.md`** — If 收窄 + Option 视图

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

**预期：**
- `process.returnType = Int`（then 分支 y:Int，else 隐式 Int）
- IR 中 `if opt is Some(y)` 脱糖为 Match 节点

**Fixture 2: `destructure.tangle.md`** — Variant + Record destructure

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

**预期：**
- `process.returnType = Item`（Some 分支 binding: Item，else 分支构造 Item）
- IR 中 `let Some(item) = opt else { ... }` 脱糖为 Match + wildcard arm
- IR 中 `let { name, price } = item` 脱糖为两个 MemberAccess + Let

**Fixture 3: `option_match.tangle.md`** — Option 自动 Sum 视图与现有 match 一致

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

**预期：**
- `double.returnType = Int`（Some arm 收窄 x:Int，None arm 字面量 Int，统一成功）
- 验证 `Option<T>` 经 `as_sum_view` 与用户显式 Sum 类型走同一收窄路径

### 9.2 Fixture 设计约束

为保证 TS reference 不 fatal exit：
1. 函数体保持简单（`return items` / `return 0` / 简单 match），避免 `List.map` / lambda / `[1,2,3]` 等触发 TS 类型错误的特性（与 Phase 6b/6c 一致）
2. 不使用 `Result<T,E>`（推迟到 6e）
3. 不使用复合模式（`is Some or None`、`is Some(y) && y > 0`）
4. 不使用 refutable let 缺少 else 的形式（parser 阶段就拒绝）
5. Type heading 使用 PascalCase（depth 1-3），callable 使用 camelCase（depth 4+）

### 9.3 测试矩阵

| 测试类型 | 位置 | 内容 |
|---|---|---|
| Rust 单元测试 | `checker/option_view.rs` `#[cfg(test)]` | `as_sum_view` 识别 Option/Sum/不识别其他 |
| Rust 单元测试 | `checker/check.rs` `#[cfg(test)]` | IsExpr 类型 + If 收窄 binding 注入 |
| Rust 单元测试 | `checker/check_module.rs` `#[cfg(test)]` | LetVariant / LetRecord binding 注入 + 诊断码 |
| Rust 单元测试 | `parser/parser.rs` `#[cfg(test)]` | is 关键字、let Variant、let Record 语法解析 |
| Rust 集成测试 | `compiler/tangle-cli/tests/v06_phase6/if_narrowing.rs`（新建） | 加载 3 个新 fixture，断言 IR 结构与 `returnType` |
| Rust 集成测试 | `compiler/tangle-cli/tests/v06_phase6/destructure.rs`（新建） | 同上 |
| Rust 集成测试 | `compiler/tangle-cli/tests/v06_phase6/option_match.rs`（新建） | 同上 |
| Rust 回归测试 | 现有 `tests/v03_phase1/`、`v03_phase2/`、`v06_phase6/` | 不回归；若有 IR 断言需更新 |
| TS 单元测试 | `reference/tests/checker/optionView.test.ts`（新建） | 镜像 Rust 单元测试 |
| TS 单元测试 | `reference/tests/checker/isNarrowing.test.ts`（新建） | 镜像 IsExpr 测试 |
| TS 单元测试 | `reference/tests/checker/destructure.test.ts`（新建） | 镜像 LetVariant / LetRecord 测试 |
| 差分测试 | `tests/audit/diff-ir.ps1` | **15 MATCH + 0 SKIPPED + 0 DIFF**（12 现有 + 3 新增） |

### 9.4 回归扫描

实现前扫描以下位置，查找需更新的断言：

```text
compiler/tangle-cli/tests/             # 所有 .rs 文件
reference/tests/                       # 所有 .test.ts 文件
tests/audit/diff-ir.ps1               # KnownDiffs / Skipped 列表
tests/audit/expected_diagnostics.yaml # 期望诊断码集合（需添加新诊断码）
```

特别检查：
- `return_type: None` 断言（Phase 6c 已基本清理，需确认无残留）
- IR JSON 快照中无 `kind: "is"` / `kind: "letVariant"` / `kind: "letRecord"` 的断言（不应出现 —— 这些节点脱糖了）
- 现有 fixture 若新增 `returnType` 字段，差分测试应保持 MATCH（已验证 TS 端会同步）

---

## 10. 出口闸门（9 项）

| # | 标准 | 验证方式 |
|---|------|---------|
| 1 | `cargo test --workspace` 全绿 | 含新 option_view / check / check_module / parser 测试 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` 零警告 | |
| 3 | `cd reference && npm test` 全绿 | 含新 optionView / isNarrowing / destructure 测试 |
| 4 | `cd reference && npm run build` 零类型错误 | TS 编译通过 |
| 5 | `pwsh tests/audit/diff-ir.ps1` **15 MATCH + 0 SKIPPED + 0 DIFF** | 12 现有 + 3 新增 fixture |
| 6 | `pwsh tests/audit/run-audit.ps1` 0 failing | |
| 7 | Phase 4/5/6a/6b/6c 回归测试通过 | |
| 8 | Phase 6d 新测试通过（IsExpr / LetVariant / LetRecord / Option 视图） | |
| 9 | 手动验证 IR JSON：`cargo run -- build tests/v06_phase6/if_narrowing.tangle.md --emit-ir` 输出含正确脱糖后的 Match 节点 | 肉眼确认 |

---

## 11. 非目标（推迟到 Phase 6e 或更晚）

- **`Result<T,E>` 内置 Sum 视图：** 需新增 stdlib 签名表项；推迟到 Phase 6e
- **`if x is not Some` 负向收窄：** 需负向类型计算；推迟
- **else 分支中 `x` 的负向收窄：** 同上
- **复合 OR 模式（`is Some or None`）：** 需复合 pattern 语法；推迟
- **guard 表达式（`is Some(y) && y > 0`）：** 需 guard 子语法；推迟
- **If 条件收窄后 `x` 本身类型变化（flow-sensitive）：** 设计明确只注入 binding，不改 `x` 类型；推迟
- **用户自定义泛型类型 `Foo<T>`：** 需四层改动，独立 phase
- **泛型约束 `T: Comparable`：** 同上
- **类型别名 `type Foo<T> = Bar<T>`：** 独立语言特性
- **局部变量类型注解（Py `x: int = ...` / Go `var x int`）：** Phase 6c 已推迟
- **IRNode 携带类型字段：** 同上
- **JS JSDoc 生成：** 同上
- **返回类型冲突诊断（不报错仅回退 Any）：** 保持 TS 兼容
- **`Function<T,U>` 精确映射到 Py `Callable` / Go `func()`：** 类型映射完备性，独立改进
- **元组 / 解构中的 `_` 通配符（如 `let { _, err } = r`）：** Phase 6e 考虑
- **嵌套 pattern（如 `is Some({ ok, err })`）：** 需复合 Pattern 子树，Phase 6e 考虑

---

## 12. 风险与缓解

| 风险 | 缓解 |
|---|---|
| `is` 关键字破坏现有 fixture（如变量名 `is`） | 实现前先 `grep -r '\bis\b' tests/ examples/` 检查冲突；若存在，添加保留字时给出迁移警告 |
| `as_sum_view` 识别 `Option<T>` 仅按名称匹配，用户自定义 `Option<T>` 类型会被误识别 | Phase 6d 接受此行为（stdlib Option 是公认内置）；Phase 6e 引入类型别名后可显式声明 |
| TS fatal 风险：fixture 误触发 `TANGLE_PATTERN_*` | 新诊断码不在 TS fatal 集合，但 fixture 设计时仍规避；TS 单元测试覆盖诊断码触发场景 |
| IR 脱糖翻译代码量增加 ~150 行 | 集中在 `compile_to_ir.rs` 三个新函数（`lower_is_expr` / `lower_let_variant` / `lower_let_record`），独立可测 |
| `let Some(y) = x else { ... }` 的 else 分支可能含 return，导致控制流分析复杂 | else 分支当作普通 stmt 序列处理；若含 return，IR 中自然产生终结节点，与现有语义一致 |
| IsExpr 在 then 分支只注入 binding、不改 `x` 类型，用户可能期望 `x` 被收窄为 `Some<T>` | 文档明确说明：then 分支中 `x` 仍为 `Option<T>`，新增 `y: T`；与 Rust `if let` 行为不同；后续 Phase 6e 可考虑 flow-sensitive 收窄 |
| TS reference 同步成本：~7 个文件需改 | 文件对应清单已明确；TS 端复用 6c 已有 unify / arm_env 设施；新增代码 ~200 行 |
| 现有 fixture 的 IR JSON 可能因 `IsExpr`/`LetVariant` 未使用而无变化 | 现有 12 fixture 不使用新语法 → IR 完全不变 → 差分测试稳定维持 12 MATCH |
| Phase 6c `match_narrowing.tangle` 与新 `option_match.tangle` 行为可能重复 | 设计差异化：6c fixture 用用户显式 Sum，6d fixture 用 `Option<T>` 视图；差分测试覆盖两条路径 |

---

## 13. 不变量

| 编号 | 不变量 | 验证方式 |
|---|---|---|
| INV-1 | `if x is Some(y) { ... }` 与等价 `match x { Some(y) => ..., _ => {} }` 产生的 IR 节点序列完全相同 | Rust 单元测试：构造两份 AST，对比 IR JSON |
| INV-2 | `let Some(y) = x else { E }` 与等价 `match x { Some(y) => let y = y; _ => E }` 产生的 IR 节点序列完全相同 | 同上 |
| INV-3 | `let { f1, f2 } = r` 与 `let f1 = r.f1; let f2 = r.f2` 产生的 IR 节点序列完全相同 | 同上 |
| INV-4 | `as_sum_view(Option<T>) == Some(Sum(Some<T>, None))` | Rust 单元测试 + TS 镜像测试 |
| INV-5 | 现有 12 fixture 的 IR JSON 字节级不变（除 `returnType` 字段可能因 6c 推断已存在） | 差分测试维持 12 MATCH |
| INV-6 | 新增 3 fixture 的 Rust/TS IR JSON 经 ir-diff 归一化后字节级一致 | 差分测试 3 新增 MATCH |

---

## 14. 版本与分支

### 14.1 分支策略

- **新分支：** `phase6d/v0.8.0`，基于 `main`（当前 HEAD `69b0e8e`）
- **worktree 路径：** `.worktrees/phase6d-v0.8.0`
- **基线：** Phase 6c 完成后的 main 分支（v0.7.0 tag）

### 14.2 版本号

- **新 tag：** `v0.8.0`
- **理由：** minor 版本升级
  - 新增 `is` 关键字与 `let Variant` / `let { ... }` 语法（用户可见新特性）
  - 新增 `Option<T>` 自动 Sum 视图（语义改进）
  - 5 个新诊断码
  - 无破坏性变更（现有 fixture IR 不变）
- **CHANGELOG.md 增量：** `## v0.8.0 — Phase 6d Type Narrowing Completeness` 章节

### 14.3 推送策略

- 完成后合并到 `main`（fast-forward）
- 打 tag `v0.8.0`
- **tag v0.8.0 不 push remote，直到用户批准**（与 6b/6c 一致）

### 14.4 实现任务预估（13 个任务）

| # | 任务 | 模块 |
|---|---|---|
| 1 | 新增 `TokenKind::Is` + lexer 识别 + AST Pattern/IsExpr/LetVariant/LetRecord 类型定义 | parser/lexer + ast |
| 2 | Parser 解析 `if expr is Pattern` + `let Variant(...) = expr else { ... }` + `let { fields } = expr` | parser |
| 3 | 新建 `checker/option_view.rs` + `as_sum_view` + 单元测试 | checker |
| 4 | Checker Expr::Is 分支 + If 收窄集成 + 单元测试 | checker |
| 5 | Checker Stmt::LetVariant 分支 + 诊断 + 单元测试 | checker |
| 6 | Checker Stmt::LetRecord 分支 + 诊断 + 单元测试 | checker |
| 7 | IR lowering：`lower_is_expr` + `lower_let_variant` + `lower_let_record` | ir/compile_to_ir |
| 8 | 新增 3 个 fixture（if_narrowing / destructure / option_match） | tests/v06_phase6 |
| 9 | Rust 集成测试 + 不变量验证（INV-1~6） | tests/v06_phase6 |
| 10 | TS reference：ast.ts / lexer.ts / parser.ts / optionView.ts / check.ts / checkModule.ts / compileToIR.ts 同步 | reference/src |
| 11 | TS reference：3 个新单元测试 + 镜像 Rust 用例 | reference/tests |
| 12 | 差分测试 + 期望诊断码集合更新 + run-audit 回归 | tests/audit |
| 13 | 出口闸门 9 项验证 + CHANGELOG + tag v0.8.0 | 全局 |

### 14.5 实施风格

- **TDD：** 每个任务先写测试，后写实现（遵循 test-driven-development 技能）
- **隔离：** 在 worktree `.worktrees/phase6d-v0.8.0` 上工作，不污染 main
- **提交粒度：** 每个任务一个 commit，遵循 Conventional Commits 风格（与 6b/6c 一致）
