# Phase 1: Call 表达式完整类型检查

**日期：** 2026-07-12
**版本目标：** v0.3.0 Phase 1
**前置条件：** v0.2.1 质量审计已完成，5 项出口闸全通过

## 背景与动机

v0.2.1 审计（F-001~F-023 已修复，F-024 推迟）暴露 checker 存在系统性类型检查缺口：

- `Expr::Call` 分支返回硬编码 `Type::Bool`，完全不检查 arity 和参数类型
- `Expr::MemberAccess` 方法分支返回 `Bool`，丢失返回类型信息
- stdlib 函数导入使用 dummy 签名（`params: vec![], returns: String`），无法支撑检查
- `TypeEnv` 没有 `functions` 表，顶层 callable（非 struct 方法）无法解析（F-024）

这些缺口导致 checker 无法捕获真实的 Call 类型错误，也无法为 Phase 3（Typed Codegen）提供类型指导。

## 目标

1. 让 `Expr::Call` 基于真实签名执行 arity + 参数类型检查
2. 建立 22 模块 stdlib 函数签名注册表，替代 dummy 签名
3. 解决 F-024：顶层 callable 符号可被 checker 解析
4. 引入 `Type::Any` 通配符，避免在类型信息不全时产生误报
5. 零回归：v0.2.1 基线的 210 audit cell 不产生新诊断

## 非目标（推迟到 Phase 2+）

- 泛型类型系统（`List<T>`、`Map<K,V>`）
- 返回类型推导（用户函数返回类型仍用 `Any`）
- 跨模块用户函数调用解析
- 签名自动从 prelude 生成
- 用户方法返回类型标注解析
- 类型收窄（type narrowing）

## 设计方案

采用**独立静态签名注册表**（方案 A）：签名数据与宿主 prelude 代码分离，职责清晰。

### 1. 类型系统扩展

#### Type 枚举新增 `Any` 变体

```rust
pub enum Type {
    Primitive(PrimitiveType),
    Struct(StructType),
    Sum(SumType),
    GenericInstance(GenericTypeInstance),
    Function(FunctionType),
    Interface(InterfaceType),
    Var(TypeVariable),
    Any,  // 新增：通配符，匹配任何类型
}
```

`Any` 语义：作为类型的顶层，匹配一切。用于：
- stdlib 泛型函数（如 `List.map`）的参数和返回类型
- heading 不携带返回类型时的默认返回
- 非函数类型调用的返回值（避免级联误报）

#### FunctionType 新增 `is_variadic`

```rust
pub struct FunctionType {
    pub params: Vec<Type>,
    pub returns: Box<Type>,
    pub is_variadic: bool,  // 新增：true 时最后一个 param 类型可重复 0..N 次
}
```

#### types_equal / is_subtype 更新

```rust
pub fn types_equal(a: &Type, b: &Type) -> bool {
    matches!(a, Type::Any) || matches!(b, Type::Any)  // Any 匹配一切
        || match (a, b) { /* 现有 Primitive/Struct/Interface 逻辑 */ }
}

pub fn is_subtype(sub: &Type, sup: &Type) -> bool {
    matches!(sup, Type::Any)  // 任何类型都是 Any 的子类型
        || match (sub, sup) { /* 现有 struct/interface 逻辑 */ }
}
```

#### Void 类型

不新增 `Void` 枚举变体。无返回值的函数（如 `fmt.println`）用 `Type::Primitive("Void")` 表示，与现有 `String`/`Int`/`Bool` 同级。

### 2. stdlib 签名注册表

新建 `compiler/tangle-cli/src/stdlib/signatures.rs`，用 `LazyLock<HashMap>` 存储全部 22 模块的函数签名。

#### 结构

```rust
use std::collections::HashMap;
use std::sync::LazyLock;
use crate::checker::types::{CallableSignature, Type, PrimitiveType};

/// 全模块签名表：module -> (function -> signature)
static STDLIB_SIGNATURES: LazyLock<HashMap<&'static str, HashMap<&'static str, CallableSignature>>> = ...;

/// 查询入口：返回某模块某函数的签名
pub fn stdlib_signature(module: &str, function: &str) -> Option<&'static CallableSignature> { ... }

/// 返回某模块全部函数签名（供 resolve_stdlib_imports 批量注入）
pub fn stdlib_module_signatures(module: &str) -> Option<&'static HashMap<&'static str, CallableSignature>> { ... }
```

#### 签名模式

三种典型签名模式：

```rust
// 变参函数：fmt.println(...args) -> Void
("println", CallableSignature {
    params: vec![("args".into(), Type::Any)],
    returns: Type::Primitive(PrimitiveType { name: "Void".into() }),
    is_variadic: true,
})

// 精确类型：IO.readFile(path: String) -> String
("readFile", CallableSignature {
    params: vec![("path".into(), Type::Primitive(PrimitiveType { name: "String".into() }))],
    returns: Type::Primitive(PrimitiveType { name: "String".into() }),
    is_variadic: false,
})

// 泛型函数用 Any：List.map(list: Any, fn: Any) -> Any
("map", CallableSignature {
    params: vec![("list".into(), Type::Any), ("fn".into(), Type::Any)],
    returns: Type::Any,
    is_variadic: false,
})
```

#### 与 stdlib_ops() 的关系

`stdlib_ops()`（`check_module.rs:53-67`）将被移除，其模块名列表合并进 `signatures.rs` 的注册表 keys。`is_stdlib_import()` 保留不变。

#### 数据来源

签名依据 `bindings.rs` 中各宿主 prelude 的实际行为编写，逐模块对照 JS/Python/Go 三端实现确保一致。

### 3. checker 改动（Call 检查 + MemberAccess）

#### resolve_stdlib_imports 改用真实签名

- 函数导入 `[println](fmt)`：查 `stdlib_signature("fmt", "println")`，注入变量类型 `Type::Function`（含真实 params/returns/is_variadic）
- 模块导入 `[fmt](fmt)`：查 `stdlib_module_signatures("fmt")`，注入 struct 的 methods 用真实 `CallableSignature`
- 移除 `stdlib_ops()` 函数，模块名列表改从注册表 keys 获取

#### Expr::MemberAccess 方法分支返回 Type::Function

```rust
// 现在：返回 Bool
} else if let Some(_method) = s.methods.get(&e.member) {
    Type::Primitive(PrimitiveType { name: "Bool".into() })
}
// 改为：返回方法的函数类型
} else if let Some(sig) = s.methods.get(&e.member) {
    Type::Function(FunctionType {
        params: sig.params.iter().map(|(_, t)| t.clone()).collect(),
        returns: Box::new(sig.returns.clone()),
        is_variadic: sig.is_variadic,
    })
}
```

#### Expr::Call 增加完整类型检查

```rust
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
            // arity 检查
            let expected = sig.params.len();
            let actual = arg_types.len();
            if sig.is_variadic {
                if actual < expected.saturating_sub(1) {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_ARITY_MISMATCH".into(),
                        message: format!("Expected at least {} args, got {}", expected - 1, actual),
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
            // 参数类型检查（Any 匹配一切）
            for (i, (arg_ty, param_ty)) in arg_types.iter().zip(&sig.params).enumerate() {
                if !types_equal(arg_ty, param_ty) {
                    diags.push(TangleDiagnostic {
                        code: "TANGLE_TYPE_ERROR".into(),
                        message: format!("Arg {} type mismatch", i + 1),
                        span: e.span.clone(),
                    });
                }
            }
            (*sig.returns).clone()
        }
        _ => Type::Any  // 非函数类型调用：不产生误报，返回 Any
    }
}
```

**设计原则：** 非函数类型的调用返回 `Any` 而非 `Bool`，避免级联误报。只有能查到签名时才做检查——查不到不报错。

#### 新增诊断码

- `TANGLE_ARITY_MISMATCH`：参数个数不匹配
- `TANGLE_TYPE_ERROR`（复用现有）：参数类型不匹配

### 4. F-024 — 顶层 callable 符号解析

#### 问题

- `TypeEnv` 只有 `variables`/`structs`/`interfaces`/`receiver`，没有 `functions` 表
- `resolve_types` 只收集 `HeadingRole::Type` 标题（depth-3），其 `HeadingRole::Callable` 子标题成为 struct 方法
- 不属于任何 Type 的顶层 Callable 标题被完全忽略——这是 F-024 的根因
- `check_expression` 的 `Identifier` 分支只查 `variables` → `receiver.fields` → `structs`，不查顶层函数

#### 改动

**1. TypeEnv 增加 `functions` 表：**

```rust
pub struct TypeEnv {
    pub variables: HashMap<String, Type>,
    pub structs: HashMap<String, Type>,
    pub interfaces: HashMap<String, Type>,
    pub functions: HashMap<String, FunctionType>,  // 新增
    pub receiver: Option<ReceiverContext>,
    pub error_registry: Option<ErrorRegistry>,
}
```

**2. `resolve_types` 增加顶层 Callable 收集：**

在现有 Type 收集 pass 之后，新增一个 pass 遍历所有标题，收集**父标题不是 Type** 的 Callable heading：

```rust
// 新增：收集顶层 callable（不属于任何 struct 的函数）
for heading in flatten_headings(&module.headings) {
    if heading.role == HeadingRole::Callable {
        if !is_child_of_type_heading(&heading.id, &module.headings) {
            if let Some(ref name) = heading.symbol_name {
                let params: Vec<Type> = heading.params.iter()
                    .map(|p| type_name_to_type(&p.type_name.clone().unwrap_or_default())
                        .unwrap_or(Type::Any))
                    .collect();
                env.functions.insert(name.clone(), FunctionType {
                    params,
                    returns: Box::new(Type::Any),  // heading 不携带返回类型，用 Any
                    is_variadic: false,
                });
            }
        }
    }
}
```

**3. `check_expression` Identifier 分支增加 `functions` 查找：**

现有查找顺序：`variables` → `receiver.fields` → `structs`。新增：→ `functions`（找到则返回 `Type::Function`）。

**4. `collect_method_sigs` 返回类型从 `Bool` 改为 `Any`：**

当前所有用户定义方法返回类型硬编码为 `Bool`，改为 `Any` 避免返回值误报。heading 不携带返回类型信息，`Any` 是唯一不产生误报的选择。

## 测试与兼容性策略

### 测试组织

新建 `compiler/tangle-cli/tests/v03_phase1/` 目录：

```
tests/v03_phase1/
├── mod.rs                          # 测试模块入口
├── stdlib_signatures.rs            # 签名注册表完整性测试
├── call_type_checking.rs           # Call 检查逻辑测试
├── toplevel_callable.rs            # F-024 回归测试
└── fixtures/                       # 测试用 .tangle.md 文件
    ├── call_arity_ok.tangle.md
    ├── call_arity_wrong.tangle.md
    ├── call_arg_type_ok.tangle.md
    ├── call_arg_type_wrong.tangle.md
    ├── toplevel_func_call.tangle.md
    └── stdlib_module_call.tangle.md
```

### 测试分类

| 类别 | 测试内容 | 断言 |
|------|---------|------|
| 签名注册表 | 22 模块全部有签名；每个函数的 params/returns/is_variadic 字段非空 | `stdlib_signature("fmt", "println").is_some()` 等 |
| Call 正例 | 正确 arity + 正确类型 → 零诊断 | `diagnostics.is_empty()` |
| Call 反例 | 错误 arity → `TANGLE_ARITY_MISMATCH`；错误类型 → `TANGLE_TYPE_ERROR` | `diagnostics.len() == 1` + code 匹配 |
| 变参 | `fmt.println("a", "b", 123)` → 零诊断（Any + variadic） | `diagnostics.is_empty()` |
| F-024 | 顶层函数 `process()` 被正确解析 → 零诊断 | `diagnostics.is_empty()` |
| 兼容性 | 全部 6 examples + 9 tests/ fixture 重跑 → 零新诊断 | 对照 v0.2.1 基线 |

### 兼容性验证策略

1. 实现完成后，先跑 `cargo test --workspace`（现有 108+ 测试不能变红）
2. 再跑 `tests/audit/run-audit.ps1` 重跑全部 210 cell——对照 v0.2.1 基线，零新增诊断
3. 如果某 fixture 产生新诊断，说明该 fixture 有真实类型错误——修 fixture 而非放宽检查
4. 如果新诊断是误报（签名写错），修签名

### helper 复用

使用 v0.2.1 已暴露的 `run_collecting_diagnostics()`（`lib.rs`）做断言。

## 出口闸

5 个条件全满足才可提交：

1. `cargo test --workspace` — 108+ 现有测试全绿，新增 Phase 1 回归测试全绿
2. `cargo clippy --workspace --all-targets -- -D warnings` — 零警告
3. `tests/audit/run-audit.ps1` — 210 cell 重跑，对照 v0.2.1 基线零新增诊断
4. 6 examples + 9 tests/ fixture 通过 `tangle check` 无误报
5. 签名注册表覆盖全部 22 模块（`stdlib_signature` 对每个模块至少返回 `Some`）

## 范围边界

| 在范围内 | 不在范围内（推迟） |
|---------|----------------|
| Type::Any 变体 | 泛型类型系统（List\<T\>、Map\<K,V\>） |
| Call 的 arity + 参数类型检查 | 返回类型推导（仍用 Any） |
| 顶层 callable 解析（F-024） | 跨模块用户函数调用解析 |
| 22 模块 stdlib 签名注册表 | 签名自动从 prelude 生成 |
| collect_method_sigs 返回 Any | 用户方法返回类型标注解析 |
| TANGLE_ARITY_MISMATCH 诊断码 | 类型收窄、type narrowing |

## 依赖链

```
Type::Any + FunctionType.is_variadic  (类型地基)
        ↓
signatures.rs 注册表                   (签名数据)
        ↓
resolve_stdlib_imports 用真实签名      (注入)
        ↓
Expr::MemberAccess 返回 Type::Function (传播)
        ↓
Expr::Call arity + 类型检查            (核心)
        ↓
TypeEnv.functions + resolve_types      (F-024)
        ↓
回归测试 + 兼容性验证                   (出口)
```

## 受影响文件清单

| 文件 | 改动类型 |
|------|---------|
| `checker/types.rs` | 新增 `Any` 变体；`FunctionType` 加 `is_variadic` |
| `checker/env.rs` | `TypeEnv` 加 `functions` 表 |
| `checker/check.rs` | `Call` + `MemberAccess` + `Identifier` 分支 |
| `checker/resolve.rs` | `resolve_types` 收集顶层 Callable；`collect_method_sigs` 返回 Any |
| `checker/check_module.rs` | `resolve_stdlib_imports` 用真实签名；移除 `stdlib_ops` |
| `stdlib/signatures.rs` | 新建 — 22 模块签名注册表 |
| `checker/mod.rs` | `pub mod signatures;` 或归入 stdlib mod |
| `tests/v03_phase1/` | 新建 — 回归测试 |
