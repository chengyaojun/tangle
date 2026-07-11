# Phase 1: Call 表达式完整类型检查 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让 checker 的 `Expr::Call` 基于真实 stdlib 签名执行 arity + 参数类型检查，解决 F-024 顶层 callable 解析，引入 `Type::Any` 避免误报。

**架构：** 在 `audit/v0.2.1` 基础上，新增 `stdlib/signatures.rs` 签名注册表（19 模块），扩展 `Type` 枚举（`Any` 变体）和 `FunctionType`/`CallableSignature`（`is_variadic` 字段），改造 `check.rs` 的 Call/MemberAccess/Identifier 分支和 `resolve.rs` 的顶层 Callable 收集。

**技术栈：** Rust 2021, LazyLock, HashMap, TangleDiagnostic

**前置条件：** `audit/v0.2.1` 分支已通过 5 项出口闸，包含 `audit_support.rs` 和 `tests/audit_regression/`。

**规格说明：** [docs/superpowers/specs/2026-07-12-phase1-call-type-checking-design.md](file:///e:/GitProjects/tangle/docs/superpowers/specs/2026-07-12-phase1-call-type-checking-design.md)

**模块数修正：** 规格中写"22 模块"，实际为 19 模块（18 个有 prelude 实现 + 1 个 String 在 stdlib_ops 中但无 prelude）。

---

## 文件结构

| 文件 | 职责 | 改动类型 |
|------|------|---------|
| `compiler/tangle-cli/src/checker/types.rs` | Type 枚举、FunctionType、CallableSignature、types_equal、is_subtype | 修改 |
| `compiler/tangle-cli/src/checker/env.rs` | TypeEnv 增加 functions 表 | 修改 |
| `compiler/tangle-cli/src/checker/check.rs` | Call/MemberAccess/Identifier 分支 | 修改 |
| `compiler/tangle-cli/src/checker/resolve.rs` | resolve_types 收集顶层 Callable；collect_method_sigs 返回 Any | 修改 |
| `compiler/tangle-cli/src/checker/check_module.rs` | resolve_stdlib_imports 用真实签名；移除 stdlib_ops | 修改 |
| `compiler/tangle-cli/src/stdlib/signatures.rs` | 19 模块签名注册表 | 新建 |
| `compiler/tangle-cli/src/stdlib/mod.rs` | pub mod signatures | 修改 |
| `compiler/tangle-cli/tests/v03_phase1/` | 回归测试 | 新建 |
| `compiler/tangle-cli/Cargo.toml` | 测试声明 | 修改 |

---

## 任务 0：创建 Phase 1 worktree

**文件：** 无代码文件

- [ ] **步骤 1：从 audit/v0.2.1 创建新 worktree**

运行：
```bash
git worktree add .worktrees/phase1-v0.3.0 -b phase1/v0.3.0 audit/v0.2.1
```

- [ ] **步骤 2：验证 worktree 可编译**

运行：
```bash
cd .worktrees/phase1-v0.3.0
cargo test --workspace
```
预期：全部测试通过（108+ 测试）

---

## 任务 1：类型系统基础（Type::Any + is_variadic）

**文件：**
- 修改：`compiler/tangle-cli/src/checker/types.rs`
- 修改：`compiler/tangle-cli/src/checker/resolve.rs:79`（CallableSignature 构造）
- 修改：`compiler/tangle-cli/src/checker/check.rs:160`（FunctionType 构造）
- 修改：`compiler/tangle-cli/src/checker/check_module.rs:82,91`（临时添加 is_variadic:false，任务 4 会重写）

- [ ] **步骤 1：编写失败测试 — Type::Any 与 types_equal**

在 `compiler/tangle-cli/src/checker/types.rs` 的 `#[cfg(test)] mod tests` 末尾添加：

```rust
    // --- 7. Type::Any matches everything ---
    #[test]
    fn types_equal_any_matches_all() {
        let any = Type::Any;
        let str_t = prim("String");
        let int_t = prim("Int");
        let struct_t = struct_type("Foo");

        assert!(types_equal(&any, &str_t));
        assert!(types_equal(&str_t, &any));
        assert!(types_equal(&any, &int_t));
        assert!(types_equal(&any, &struct_t));
        assert!(types_equal(&any, &any));
    }

    // --- 8. is_subtype with Any as top type ---
    #[test]
    fn is_subtype_any_is_top() {
        let any = Type::Any;
        let str_t = prim("String");
        let struct_t = struct_type("Foo");

        assert!(is_subtype(&str_t, &any));
        assert!(is_subtype(&struct_t, &any));
        assert!(is_subtype(&any, &any));
    }

    // --- 9. FunctionType has is_variadic field ---
    #[test]
    fn function_type_variadic() {
        let fixed = FunctionType {
            params: vec![prim("Int")],
            returns: Box::new(prim("Bool")),
            is_variadic: false,
        };
        let variadic = FunctionType {
            params: vec![prim("String")],
            returns: Box::new(prim("Void")),
            is_variadic: true,
        };
        assert!(!fixed.is_variadic);
        assert!(variadic.is_variadic);
    }
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --lib checker::types::tests`
预期：编译失败 — `Type::Any` 不存在，`is_variadic` 字段不存在

- [ ] **步骤 3：实现 Type::Any 和 is_variadic**

在 `compiler/tangle-cli/src/checker/types.rs` 中：

3a. Type 枚举添加 Any 变体（第 4-12 行替换）：

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Primitive(PrimitiveType),
    Struct(StructType),
    Sum(SumType),
    GenericInstance(GenericTypeInstance),
    Function(FunctionType),
    Interface(InterfaceType),
    Var(TypeVariable),
    Any,
}
```

3b. FunctionType 添加 is_variadic（第 37-41 行替换）：

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub returns: Box<Type>,
    pub is_variadic: bool,
}
```

3c. CallableSignature 添加 is_variadic（第 54-58 行替换）：

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CallableSignature {
    pub params: Vec<(String, Type)>,
    pub returns: Type,
    pub is_variadic: bool,
}
```

3d. types_equal 添加 Any 处理（第 60-67 行替换）：

```rust
pub fn types_equal(a: &Type, b: &Type) -> bool {
    if matches!(a, Type::Any) || matches!(b, Type::Any) {
        return true;
    }
    match (a, b) {
        (Type::Primitive(a), Type::Primitive(b)) => a.name == b.name,
        (Type::Struct(a), Type::Struct(b)) => a.name == b.name,
        (Type::Interface(a), Type::Interface(b)) => a.name == b.name,
        _ => false,
    }
}
```

3e. is_subtype 添加 Any 处理（第 69-77 行替换）：

```rust
pub fn is_subtype(sub: &Type, sup: &Type) -> bool {
    if matches!(sup, Type::Any) {
        return true;
    }
    match (sub, sup) {
        (Type::Struct(s), Type::Interface(i)) => i
            .methods
            .iter()
            .all(|(name, sig)| s.methods.get(name).map_or(false, |ms| callable_sigs_match(ms, sig))),
        _ => types_equal(sub, sup),
    }
}
```

- [ ] **步骤 4：修复所有 CallableSignature 构造点添加 is_variadic: false**

在 `compiler/tangle-cli/src/checker/resolve.rs` 第 79-84 行，`collect_method_sigs` 内：

```rust
                methods.insert(
                    name.clone(),
                    CallableSignature {
                        params,
                        returns: Type::Primitive(PrimitiveType {
                            name: "Bool".into(),
                        }),
                        is_variadic: false,
                    },
                );
```

在 `compiler/tangle-cli/src/checker/check_module.rs` 第 82-85 行（FunctionType）：

```rust
                        env.variables.insert(alias.to_string(), Type::Function(FunctionType {
                            params: vec![],
                            returns: Box::new(Type::Primitive(PrimitiveType { name: "String".into() })),
                            is_variadic: false,
                        }));
```

在 `compiler/tangle-cli/src/checker/check_module.rs` 第 91-94 行（CallableSignature）：

```rust
                    (op.to_string(), CallableSignature {
                        params: vec![],
                        returns: Type::Primitive(PrimitiveType { name: "String".into() }),
                        is_variadic: false,
                    })
```

在 `compiler/tangle-cli/src/checker/check.rs` 第 160-163 行（Expr::Arrow）：

```rust
        Expr::Arrow(_e) => {
            Type::Function(FunctionType {
                params: vec![],
                returns: Box::new(Type::Primitive(PrimitiveType { name: "Bool".into() })),
                is_variadic: false,
            })
        }
```

在 `compiler/tangle-cli/src/checker/types.rs` 测试 helper `sig` 函数（第 116-124 行）：

```rust
    fn sig(params: Vec<(&str, Type)>, returns: Type) -> CallableSignature {
        CallableSignature {
            params: params
                .into_iter()
                .map(|(n, t)| (n.to_string(), t))
                .collect(),
            returns,
            is_variadic: false,
        }
    }
```

- [ ] **步骤 5：运行测试验证通过**

运行：`cargo test --lib checker::types::tests`
预期：全部通过（原有 6 个 + 新增 3 个 = 9 个）

- [ ] **步骤 6：运行 workspace 全量测试确认无回归**

运行：`cargo test --workspace`
预期：全部通过

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/checker/types.rs compiler/tangle-cli/src/checker/resolve.rs compiler/tangle-cli/src/checker/check.rs compiler/tangle-cli/src/checker/check_module.rs
git commit -m "feat(checker): add Type::Any and is_variadic to FunctionType/CallableSignature"
```

---

## 任务 2：F-024 — 顶层 callable 符号解析

**文件：**
- 修改：`compiler/tangle-cli/src/checker/env.rs`
- 修改：`compiler/tangle-cli/src/checker/resolve.rs`
- 创建：`compiler/tangle-cli/tests/v03_phase1/toplevel_callable.rs`
- 创建：`compiler/tangle-cli/tests/v03_phase1/fixtures/toplevel_func_call.tangle.md`
- 修改：`compiler/tangle-cli/Cargo.toml`

- [ ] **步骤 1：创建测试 fixture**

创建 `compiler/tangle-cli/tests/v03_phase1/fixtures/toplevel_func_call.tangle.md`：

```markdown
# TopLevelFuncTest

[fmt](fmt)

## 函数

#### process

@tangle
let msg = fmt.println("hello")
return msg
@end
```

- [ ] **步骤 2：编写失败测试**

创建 `compiler/tangle-cli/tests/v03_phase1/toplevel_callable.rs`：

```rust
//! F-024: Top-level callable symbol resolution regression test.
//!
//! Root cause: TypeEnv had no `functions` table; resolve_types only
//! collected Callables that were children of Type headings (struct methods).
//! Top-level Callables (not under any struct) were invisible to the checker.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn f024_toplevel_callable_no_symbol_not_found() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/toplevel_func_call.tangle.md");
    let has_symbol_not_found = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND");
    assert!(
        !has_symbol_not_found,
        "Top-level callable should be resolvable, got diagnostics:\n{}",
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}
```

- [ ] **步骤 3：在 Cargo.toml 声明测试**

在 `compiler/tangle-cli/Cargo.toml` 的最后一个 `[[test]]` 之后添加：

```toml
[[test]]
name = "toplevel_callable"
path = "tests/v03_phase1/toplevel_callable.rs"
```

- [ ] **步骤 4：运行测试验证失败**

运行：`cargo test --test toplevel_callable`
预期：FAIL — `process` 未被 resolve_types 收集，可能产生 `TANGLE_SYMBOL_NOT_FOUND` 或被忽略

注意：如果当前测试恰好通过（因为 `process` 被当作表达式而非函数调用），先确认测试逻辑正确——fixture 中 `process` 必须被作为函数调用才能触发 F-024。

- [ ] **步骤 5：TypeEnv 增加 functions 表**

在 `compiler/tangle-cli/src/checker/env.rs` 中：

5a. 在 `use` 语句后添加 FunctionType 导入：

```rust
use crate::checker::errors::ErrorRegistry;
use crate::checker::types::{FunctionType, Type};
use std::collections::HashMap;
```

5b. TypeEnv 结构体添加 functions 字段（第 11-18 行替换）：

```rust
#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub variables: HashMap<String, Type>,
    pub structs: HashMap<String, Type>,
    pub interfaces: HashMap<String, Type>,
    pub functions: HashMap<String, FunctionType>,
    pub receiver: Option<ReceiverContext>,
    pub error_registry: Option<ErrorRegistry>,
}
```

5c. TypeEnv::new() 初始化 functions（第 21-29 行替换）：

```rust
impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            variables: HashMap::new(),
            structs: HashMap::new(),
            interfaces: HashMap::new(),
            functions: HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }
}
```

- [ ] **步骤 6：resolve_types 收集顶层 Callable**

在 `compiler/tangle-cli/src/checker/resolve.rs` 中：

6a. 在 `resolve_types` 函数中，在 `(env, diagnostics)` 返回之前（第 53 行之后）添加顶层 Callable 收集 pass：

```rust
    // Pass 2: Collect top-level Callables (not children of any Type heading)
    for heading in flatten_headings(&module.headings) {
        if heading.role == HeadingRole::Callable
            && !is_child_of_type_heading(&heading.id, &module.headings)
        {
            if let Some(ref name) = heading.symbol_name {
                let params: Vec<Type> = heading
                    .params
                    .iter()
                    .map(|p| {
                        p.type_name
                            .as_ref()
                            .and_then(|tn| type_name_to_type(tn))
                            .unwrap_or(Type::Any)
                    })
                    .collect();
                env.functions.insert(
                    name.clone(),
                    FunctionType {
                        params,
                        returns: Box::new(Type::Any),
                        is_variadic: false,
                    },
                );
            }
        }
    }
```

6b. 在文件末尾（`type_name_to_type` 之后）添加 helper 函数：

```rust
/// Flatten the heading tree into a flat list (depth-first).
fn flatten_headings(headings: &[TangleHeading]) -> Vec<&TangleHeading> {
    let mut result = vec![];
    for h in headings {
        result.push(h);
        result.extend(flatten_headings(&h.children));
    }
    result
}

/// Check if a heading (by id) is a direct child of a Type heading.
fn is_child_of_type_heading(heading_id: &str, headings: &[TangleHeading]) -> bool {
    for h in headings {
        if h.role == HeadingRole::Type {
            if h.children.iter().any(|c| c.id == heading_id) {
                return true;
            }
        }
        if is_child_of_type_heading(heading_id, &h.children) {
            return true;
        }
    }
    false
}
```

6c. `collect_method_sigs` 返回类型从 `Bool` 改为 `Any`（第 81-83 行替换）：

```rust
                methods.insert(
                    name.clone(),
                    CallableSignature {
                        params,
                        returns: Type::Any,
                        is_variadic: false,
                    },
                );
```

- [ ] **步骤 7：check_expression Identifier 分支增加 functions 查找**

在 `compiler/tangle-cli/src/checker/check.rs` 的 `Expr::Identifier` 分支中（第 14-39 行），在 `structs` 查找之后、`TANGLE_SYMBOL_NOT_FOUND` 之前添加 `functions` 查找。

将第 14-39 行替换为：

```rust
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
```

- [ ] **步骤 8：运行测试验证通过**

运行：`cargo test --test toplevel_callable`
预期：PASS

- [ ] **步骤 9：运行 workspace 全量测试确认无回归**

运行：`cargo test --workspace`
预期：全部通过

- [ ] **步骤 10：Commit**

```bash
git add compiler/tangle-cli/src/checker/env.rs compiler/tangle-cli/src/checker/resolve.rs compiler/tangle-cli/src/checker/check.rs compiler/tangle-cli/tests/v03_phase1/ compiler/tangle-cli/Cargo.toml
git commit -m "feat(checker): resolve top-level callable symbols (F-024)"
```

---

## 任务 3：stdlib 签名注册表

**文件：**
- 创建：`compiler/tangle-cli/src/stdlib/signatures.rs`
- 修改：`compiler/tangle-cli/src/stdlib/mod.rs`
- 创建：`compiler/tangle-cli/tests/v03_phase1/stdlib_signatures.rs`
- 修改：`compiler/tangle-cli/Cargo.toml`

- [ ] **步骤 1：编写失败测试 — 注册表完整性**

创建 `compiler/tangle-cli/tests/v03_phase1/stdlib_signatures.rs`：

```rust
//! stdlib signature registry completeness tests.
//! Verifies all 19 modules have signatures and key functions are present.

use tangle_cli::stdlib::signatures::{stdlib_signature, stdlib_module_signatures};

const EXPECTED_MODULES: &[&str] = &[
    "fmt", "IO", "List", "Map", "Set", "Option", "Math", "String",
    "Env", "Path", "JSON", "DateTime", "Random", "Encoding", "Sort",
    "Process", "Task", "Channel", "Sync",
];

#[test]
fn all_19_modules_have_signatures() {
    for module in EXPECTED_MODULES {
        assert!(
            stdlib_module_signatures(module).is_some(),
            "Module '{}' missing from signature registry",
            module
        );
    }
}

#[test]
fn fmt_module_has_expected_functions() {
    let sigs = stdlib_module_signatures("fmt").expect("fmt module must exist");
    for fn_name in &["print", "println", "input", "debug", "error", "format"] {
        assert!(sigs.contains_key(*fn_name), "fmt.{} missing", fn_name);
    }
}

#[test]
fn println_is_variadic() {
    let sig = stdlib_signature("fmt", "println").expect("fmt.println must exist");
    assert!(sig.is_variadic, "fmt.println should be variadic");
}

#[test]
fn readfile_has_string_param_and_return() {
    let sig = stdlib_signature("IO", "readFile").expect("IO.readFile must exist");
    assert_eq!(sig.params.len(), 1);
    assert!(!sig.is_variadic);
}

#[test]
fn json_parse_and_stringify_exist() {
    assert!(stdlib_signature("JSON", "parse").is_some());
    assert!(stdlib_signature("JSON", "stringify").is_some());
}

#[test]
fn sync_has_all_wait_group_ops() {
    let sigs = stdlib_module_signatures("Sync").expect("Sync module must exist");
    for op in &["mutex_new", "mutex_lock", "mutex_unlock", "once_do",
                "wait_group_new", "wait_group_add", "wait_group_done", "wait_group_wait"] {
        assert!(sigs.contains_key(*op), "Sync.{} missing", op);
    }
}
```

- [ ] **步骤 2：在 Cargo.toml 声明测试**

在 `compiler/tangle-cli/Cargo.toml` 的 `[[test]]` 列表末尾添加：

```toml
[[test]]
name = "stdlib_signatures"
path = "tests/v03_phase1/stdlib_signatures.rs"
```

- [ ] **步骤 3：运行测试验证失败**

运行：`cargo test --test stdlib_signatures`
预期：编译失败 — `tangle_cli::stdlib::signatures` 模块不存在

- [ ] **步骤 4：创建 signatures.rs**

创建 `compiler/tangle-cli/src/stdlib/signatures.rs`：

```rust
//! Static signature registry for all 19 stdlib modules.
//! Replaces the dummy signatures in stdlib_ops() with real type information.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::checker::types::{CallableSignature, PrimitiveType, Type};

// --- type helpers ---

fn prim(name: &str) -> Type {
    Type::Primitive(PrimitiveType { name: name.into() })
}

fn str_t() -> Type { prim("String") }
fn int_t() -> Type { prim("Int") }
fn bool_t() -> Type { prim("Bool") }
fn void_t() -> Type { prim("Void") }
fn any_t() -> Type { Type::Any }

// --- signature helpers ---

fn sig_fixed(params: &[(&str, Type)], returns: Type) -> CallableSignature {
    CallableSignature {
        params: params.iter().map(|(n, t)| (n.to_string(), t.clone())).collect(),
        returns,
        is_variadic: false,
    }
}

fn sig_variadic(params: &[(&str, Type)], returns: Type) -> CallableSignature {
    CallableSignature {
        params: params.iter().map(|(n, t)| (n.to_string(), t.clone())).collect(),
        returns,
        is_variadic: true,
    }
}

// --- registry ---

static STDLIB_SIGNATURES: LazyLock<HashMap<&'static str, HashMap<&'static str, CallableSignature>>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();

        m.insert("fmt", module(&[
            ("print", sig_variadic(&[("args", any_t())], void_t())),
            ("println", sig_variadic(&[("args", any_t())], void_t())),
            ("input", sig_fixed(&[("prompt", str_t())], str_t())),
            ("debug", sig_variadic(&[("args", any_t())], void_t())),
            ("error", sig_variadic(&[("args", any_t())], void_t())),
            ("format", sig_variadic(&[("s", str_t()), ("args", any_t())], str_t())),
        ]));

        m.insert("IO", module(&[
            ("readFile", sig_fixed(&[("path", str_t())], str_t())),
            ("writeFile", sig_fixed(&[("path", str_t()), ("data", str_t())], void_t())),
            ("exists", sig_fixed(&[("path", str_t())], bool_t())),
            ("stat", sig_fixed(&[("path", str_t())], any_t())),
            ("mkdir", sig_fixed(&[("path", str_t())], void_t())),
            ("read_dir", sig_fixed(&[("path", str_t())], any_t())),
            ("remove", sig_fixed(&[("path", str_t())], void_t())),
            ("rename", sig_fixed(&[("from", str_t()), ("to", str_t())], void_t())),
            ("copy", sig_fixed(&[("from", str_t()), ("to", str_t())], void_t())),
            ("chmod", sig_fixed(&[("path", str_t()), ("mode", int_t())], void_t())),
            ("size", sig_fixed(&[("path", str_t())], int_t())),
            ("is_dir", sig_fixed(&[("path", str_t())], bool_t())),
            ("is_file", sig_fixed(&[("path", str_t())], bool_t())),
        ]));

        m.insert("List", module(&[
            ("length", sig_fixed(&[("list", any_t())], int_t())),
            ("map", sig_fixed(&[("list", any_t()), ("fn", any_t())], any_t())),
            ("filter", sig_fixed(&[("list", any_t()), ("fn", any_t())], any_t())),
            ("push", sig_fixed(&[("list", any_t()), ("item", any_t())], any_t())),
            ("get", sig_fixed(&[("list", any_t()), ("index", int_t())], any_t())),
        ]));

        m.insert("Map", module(&[
            ("get", sig_fixed(&[("map", any_t()), ("key", any_t())], any_t())),
            ("set", sig_fixed(&[("map", any_t()), ("key", any_t()), ("value", any_t())], any_t())),
            ("has", sig_fixed(&[("map", any_t()), ("key", any_t())], bool_t())),
            ("keys", sig_fixed(&[("map", any_t())], any_t())),
            ("values", sig_fixed(&[("map", any_t())], any_t())),
            ("delete", sig_fixed(&[("map", any_t()), ("key", any_t())], any_t())),
        ]));

        m.insert("Set", module(&[
            ("add", sig_fixed(&[("set", any_t()), ("value", any_t())], any_t())),
            ("remove", sig_fixed(&[("set", any_t()), ("value", any_t())], any_t())),
            ("contains", sig_fixed(&[("set", any_t()), ("value", any_t())], bool_t())),
            ("size", sig_fixed(&[("set", any_t())], int_t())),
            ("union", sig_fixed(&[("set", any_t()), ("other", any_t())], any_t())),
            ("intersection", sig_fixed(&[("set", any_t()), ("other", any_t())], any_t())),
            ("difference", sig_fixed(&[("set", any_t()), ("other", any_t())], any_t())),
            ("to_list", sig_fixed(&[("set", any_t())], any_t())),
        ]));

        m.insert("Option", module(&[
            ("Some", sig_fixed(&[("value", any_t())], any_t())),
            ("None", sig_fixed(&[], any_t())),
            ("unwrap", sig_fixed(&[("opt", any_t())], any_t())),
            ("is_some", sig_fixed(&[("opt", any_t())], bool_t())),
            ("is_none", sig_fixed(&[("opt", any_t())], bool_t())),
            ("map", sig_fixed(&[("opt", any_t()), ("fn", any_t())], any_t())),
            ("or_else", sig_fixed(&[("opt", any_t()), ("fn", any_t())], any_t())),
        ]));

        m.insert("Math", module(&[
            ("abs", sig_fixed(&[("n", int_t())], int_t())),
            ("min", sig_fixed(&[("a", int_t()), ("b", int_t())], int_t())),
            ("max", sig_fixed(&[("a", int_t()), ("b", int_t())], int_t())),
            ("floor", sig_fixed(&[("n", any_t())], any_t())),
            ("ceil", sig_fixed(&[("n", any_t())], any_t())),
            ("round", sig_fixed(&[("n", any_t())], any_t())),
            ("sqrt", sig_fixed(&[("n", any_t())], any_t())),
            ("pow", sig_fixed(&[("base", any_t()), ("exp", any_t())], any_t())),
        ]));

        m.insert("String", module(&[
            ("length", sig_fixed(&[("s", str_t())], int_t())),
            ("concat", sig_fixed(&[("a", str_t()), ("b", str_t())], str_t())),
            ("split", sig_fixed(&[("s", str_t()), ("sep", str_t())], any_t())),
            ("replace", sig_fixed(&[("s", str_t()), ("from", str_t()), ("to", str_t())], str_t())),
            ("to_upper", sig_fixed(&[("s", str_t())], str_t())),
            ("to_lower", sig_fixed(&[("s", str_t())], str_t())),
            ("trim", sig_fixed(&[("s", str_t())], str_t())),
            ("contains", sig_fixed(&[("s", str_t()), ("sub", str_t())], bool_t())),
        ]));

        m.insert("Env", module(&[
            ("get", sig_fixed(&[("key", str_t())], str_t())),
            ("set", sig_fixed(&[("key", str_t()), ("value", str_t())], void_t())),
            ("remove", sig_fixed(&[("key", str_t())], void_t())),
            ("args", sig_fixed(&[], any_t())),
            ("current_dir", sig_fixed(&[], str_t())),
            ("exit", sig_fixed(&[("code", int_t())], void_t())),
        ]));

        m.insert("Path", module(&[
            ("join", sig_variadic(&[("parts", str_t())], str_t())),
            ("basename", sig_fixed(&[("path", str_t())], str_t())),
            ("dirname", sig_fixed(&[("path", str_t())], str_t())),
            ("extension", sig_fixed(&[("path", str_t())], str_t())),
            ("is_absolute", sig_fixed(&[("path", str_t())], bool_t())),
            ("normalize", sig_fixed(&[("path", str_t())], str_t())),
            ("relative", sig_fixed(&[("from", str_t()), ("to", str_t())], str_t())),
            ("split", sig_fixed(&[("path", str_t())], any_t())),
        ]));

        m.insert("JSON", module(&[
            ("parse", sig_fixed(&[("s", str_t())], any_t())),
            ("stringify", sig_fixed(&[("value", any_t())], str_t())),
        ]));

        m.insert("DateTime", module(&[
            ("now", sig_fixed(&[], any_t())),
            ("format", sig_fixed(&[("date", any_t()), ("format", str_t())], str_t())),
            ("timestamp", sig_fixed(&[("date", any_t())], int_t())),
        ]));

        m.insert("Random", module(&[
            ("int", sig_fixed(&[], int_t())),
            ("int_range", sig_fixed(&[("lo", int_t()), ("hi", int_t())], int_t())),
            ("float", sig_fixed(&[], any_t())),
            ("bool", sig_fixed(&[], bool_t())),
            ("bytes", sig_fixed(&[("n", int_t())], any_t())),
            ("shuffle", sig_fixed(&[("arr", any_t())], any_t())),
            ("choice", sig_fixed(&[("arr", any_t())], any_t())),
        ]));

        m.insert("Encoding", module(&[
            ("hex_encode", sig_fixed(&[("data", any_t())], str_t())),
            ("hex_decode", sig_fixed(&[("s", str_t())], any_t())),
            ("base64_encode", sig_fixed(&[("data", any_t())], str_t())),
            ("base64_decode", sig_fixed(&[("s", str_t())], any_t())),
            ("url_encode", sig_fixed(&[("s", str_t())], str_t())),
            ("url_decode", sig_fixed(&[("s", str_t())], str_t())),
        ]));

        m.insert("Sort", module(&[
            ("asc", sig_fixed(&[("arr", any_t())], any_t())),
            ("desc", sig_fixed(&[("arr", any_t())], any_t())),
            ("by_key_asc", sig_fixed(&[("arr", any_t()), ("fn", any_t())], any_t())),
            ("by_key_desc", sig_fixed(&[("arr", any_t()), ("fn", any_t())], any_t())),
            ("is_sorted", sig_fixed(&[("arr", any_t())], bool_t())),
            ("min", sig_fixed(&[("arr", any_t())], any_t())),
            ("max", sig_fixed(&[("arr", any_t())], any_t())),
        ]));

        m.insert("Process", module(&[
            ("run", sig_fixed(&[("cmd", str_t()), ("args", any_t())], any_t())),
            ("exec", sig_fixed(&[("cmd", str_t())], str_t())),
            ("spawn", sig_fixed(&[("cmd", str_t()), ("args", any_t())], any_t())),
            ("exit", sig_fixed(&[("code", int_t())], void_t())),
            ("pid", sig_fixed(&[], int_t())),
            ("args", sig_fixed(&[], any_t())),
            ("stdout", sig_fixed(&[], any_t())),
            ("stderr", sig_fixed(&[], any_t())),
            ("status", sig_fixed(&[], int_t())),
        ]));

        m.insert("Task", module(&[
            ("spawn", sig_fixed(&[("fn", any_t())], any_t())),
            ("await", sig_fixed(&[("task", any_t())], any_t())),
            ("sleep", sig_fixed(&[("ms", int_t())], void_t())),
            ("join", sig_variadic(&[("tasks", any_t())], any_t())),
            ("parallel", sig_fixed(&[("fns", any_t())], any_t())),
            ("race", sig_fixed(&[("fns", any_t())], any_t())),
            ("all", sig_fixed(&[("fns", any_t())], any_t())),
            ("timeout", sig_fixed(&[("task", any_t()), ("ms", int_t())], any_t())),
        ]));

        m.insert("Channel", module(&[
            ("new", sig_fixed(&[("cap", int_t())], any_t())),
            ("send", sig_fixed(&[("ch", any_t()), ("value", any_t())], void_t())),
            ("recv", sig_fixed(&[("ch", any_t())], any_t())),
            ("close", sig_fixed(&[("ch", any_t())], void_t())),
            ("len", sig_fixed(&[("ch", any_t())], int_t())),
            ("cap", sig_fixed(&[("ch", any_t())], int_t())),
            ("select", sig_fixed(&[("chs", any_t())], any_t())),
            ("try_send", sig_fixed(&[("ch", any_t()), ("value", any_t())], bool_t())),
            ("try_recv", sig_fixed(&[("ch", any_t())], any_t())),
        ]));

        m.insert("Sync", module(&[
            ("mutex_new", sig_fixed(&[], any_t())),
            ("mutex_lock", sig_fixed(&[("m", any_t())], void_t())),
            ("mutex_unlock", sig_fixed(&[("m", any_t())], void_t())),
            ("once_do", sig_fixed(&[("fn", any_t())], any_t())),
            ("wait_group_new", sig_fixed(&[], any_t())),
            ("wait_group_add", sig_fixed(&[("wg", any_t()), ("n", int_t())], void_t())),
            ("wait_group_done", sig_fixed(&[("wg", any_t())], void_t())),
            ("wait_group_wait", sig_fixed(&[("wg", any_t())], void_t())),
        ]));

        m
    });

fn module(fns: &[(&str, CallableSignature)]) -> HashMap<&'static str, CallableSignature> {
    fns.iter().map(|(name, sig)| (*name, sig.clone())).collect()
}

/// Look up a single function signature by module and function name.
pub fn stdlib_signature(module: &str, function: &str) -> Option<&'static CallableSignature> {
    STDLIB_SIGNATURES.get(module).and_then(|m| m.get(function))
}

/// Get all function signatures for a module.
pub fn stdlib_module_signatures(module: &str) -> Option<&'static HashMap<&'static str, CallableSignature>> {
    STDLIB_SIGNATURES.get(module)
}

/// List all module names in the registry.
pub fn stdlib_modules() -> Vec<&'static str> {
    STDLIB_SIGNATURES.keys().copied().collect()
}
```

- [ ] **步骤 5：在 stdlib/mod.rs 添加模块声明**

将 `compiler/tangle-cli/src/stdlib/mod.rs` 替换为：

```rust
pub mod bindings;
pub mod signatures;
pub use bindings::*;
```

- [ ] **步骤 6：运行测试验证通过**

运行：`cargo test --test stdlib_signatures`
预期：全部 6 个测试通过

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/stdlib/signatures.rs compiler/tangle-cli/src/stdlib/mod.rs compiler/tangle-cli/tests/v03_phase1/stdlib_signatures.rs compiler/tangle-cli/Cargo.toml
git commit -m "feat(stdlib): add 19-module signature registry"
```

---

## 任务 4：resolve_stdlib_imports 使用真实签名

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check_module.rs`

- [ ] **步骤 1：编写失败测试 — 真实签名注入**

创建 `compiler/tangle-cli/tests/v03_phase1/stdlib_module_call.rs`：

```rust
//! Verify resolve_stdlib_imports injects real signatures from the registry.
//! After this change, fmt.println should have is_variadic=true and IO.readFile
//! should have a String param — not the old dummy (params=[], returns=String).

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn stdlib_module_import_no_false_diagnostics() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/stdlib_module_call.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "Expected zero diagnostics, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn stdlib_function_import_no_false_diagnostics() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/stdlib_fn_call.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "Expected zero diagnostics, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}
```

创建 `compiler/tangle-cli/tests/v03_phase1/fixtures/stdlib_module_call.tangle.md`：

```markdown
# StdlibModuleCallTest

[fmt](fmt)
[IO](IO)

## 测试

@tangle
let line = fmt.println("hello", "world")
let content = IO.readFile("/tmp/test.txt")
return line
@end
```

创建 `compiler/tangle-cli/tests/v03_phase1/fixtures/stdlib_fn_call.tangle.md`：

```markdown
# StdlibFnCallTest

[println, readFile](fmt)

## 测试

@tangle
let line = println("hello")
return line
@end
```

注意：`readFile` 在 `fmt` 模块中不存在，此 fixture 测试 `println` 单函数导入。如果 `readFile` 不在 fmt 中，只导入 `println`。

将 fixture 修正为：

```markdown
# StdlibFnCallTest

[println](fmt)

## 测试

@tangle
let line = println("hello")
return line
@end
```

- [ ] **步骤 2：在 Cargo.toml 声明测试**

在 `compiler/tangle-cli/Cargo.toml` 的 `[[test]]` 列表末尾添加：

```toml
[[test]]
name = "stdlib_module_call"
path = "tests/v03_phase1/stdlib_module_call.rs"
```

- [ ] **步骤 3：运行测试验证当前状态**

运行：`cargo test --test stdlib_module_call`
预期：可能通过或失败——取决于 dummy 签名是否产生误报。记录当前结果作为基线。

- [ ] **步骤 4：重写 resolve_stdlib_imports**

在 `compiler/tangle-cli/src/checker/check_module.rs` 中：

4a. 在文件顶部添加 signatures 导入（第 10 行 `use std::collections::HashMap;` 之后添加）：

```rust
use crate::stdlib::signatures::{stdlib_signature, stdlib_module_signatures};
```

4b. 删除 `stdlib_ops` 函数（第 53-67 行整段删除）

4c. 替换 `resolve_stdlib_imports` 函数（第 69-104 行替换为）：

```rust
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
```

- [ ] **步骤 5：运行测试验证通过**

运行：`cargo test --test stdlib_module_call`
预期：全部通过

- [ ] **步骤 6：运行 workspace 全量测试确认无回归**

运行：`cargo test --workspace`
预期：全部通过

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/checker/check_module.rs compiler/tangle-cli/tests/v03_phase1/stdlib_module_call.rs compiler/tangle-cli/tests/v03_phase1/fixtures/stdlib_module_call.tangle.md compiler/tangle-cli/tests/v03_phase1/fixtures/stdlib_fn_call.tangle.md compiler/tangle-cli/Cargo.toml
git commit -m "feat(checker): inject real stdlib signatures from registry"
```

---

## 任务 5：MemberAccess 返回 Type::Function

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check.rs`

- [ ] **步骤 1：编写失败测试 — 方法访问返回函数类型**

创建 `compiler/tangle-cli/tests/v03_phase1/member_access_type.rs`：

```rust
//! Verify MemberAccess on struct methods returns Type::Function (not Bool).
//! This enables Call checking to see the real signature.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn method_access_does_not_cascade_bool() {
    // In account.tangle.md, Account.open() is a method call.
    // Before: MemberAccess returned Bool, then Call returned Bool — no type errors.
    // After: MemberAccess returns Function, Call checks arity/types — should still pass.
    let run = run_collecting_diagnostics("../../examples/account.tangle.md");
    let has_false_type_error = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_TYPE_ERROR" && d.message.contains("Member access"));
    assert!(
        !has_false_type_error,
        "Member access on struct method should not produce type errors"
    );
}
```

- [ ] **步骤 2：在 Cargo.toml 声明测试**

在 `compiler/tangle-cli/Cargo.toml` 的 `[[test]]` 列表末尾添加：

```toml
[[test]]
name = "member_access_type"
path = "tests/v03_phase1/member_access_type.rs"
```

- [ ] **步骤 3：运行测试验证当前状态**

运行：`cargo test --test member_access_type`
预期：通过（当前 MemberAccess 返回 Bool，不会产生 type error）。记录为基线。

- [ ] **步骤 4：修改 MemberAccess 方法分支**

在 `compiler/tangle-cli/src/checker/check.rs` 的 `Expr::MemberAccess` 分支中（第 48-49 行），替换方法访问返回值：

```rust
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
```

- [ ] **步骤 5：运行测试验证通过**

运行：`cargo test --test member_access_type`
预期：通过

- [ ] **步骤 6：运行 workspace 全量测试**

运行：`cargo test --workspace`
预期：全部通过

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/checker/check.rs compiler/tangle-cli/tests/v03_phase1/member_access_type.rs compiler/tangle-cli/Cargo.toml
git commit -m "feat(checker): MemberAccess returns Type::Function for methods"
```

---

## 任务 6：Call 表达式 arity + 参数类型检查

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check.rs`
- 创建：`compiler/tangle-cli/tests/v03_phase1/call_type_checking.rs`
- 创建：4 个 fixture 文件
- 修改：`compiler/tangle-cli/Cargo.toml`

- [ ] **步骤 1：创建正例 fixture — arity 正确**

创建 `compiler/tangle-cli/tests/v03_phase1/fixtures/call_arity_ok.tangle.md`：

```markdown
# CallArityOk

[println](fmt)
[readFile](IO)

## 测试

@tangle
let line = println("hello", "world")
let content = readFile("/tmp/test.txt")
return line
@end
```

- [ ] **步骤 2：创建反例 fixture — arity 错误**

创建 `compiler/tangle-cli/tests/v03_phase1/fixtures/call_arity_wrong.tangle.md`：

```markdown
# CallArityWrong

[readFile](IO)

## 测试

@tangle
let content = readFile()
return content
@end
```

- [ ] **步骤 3：创建反例 fixture — 参数类型错误**

创建 `compiler/tangle-cli/tests/v03_phase1/fixtures/call_arg_type_wrong.tangle.md`：

```markdown
# CallArgTypeWrong

[readFile](IO)

## 测试

@tangle
let content = readFile(123)
return content
@end
```

- [ ] **步骤 4：创建正例 fixture — 变参 + Any 类型**

创建 `compiler/tangle-cli/tests/v03_phase1/fixtures/call_variadic_ok.tangle.md`：

```markdown
# CallVariadicOk

[println](fmt)

## 测试

@tangle
let a = println("hello")
let b = println("hello", "world")
let c = println("hello", 123, true)
return a
@end
```

- [ ] **步骤 5：编写失败测试**

创建 `compiler/tangle-cli/tests/v03_phase1/call_type_checking.rs`：

```rust
//! Call expression type checking tests.
//! Verifies arity and parameter type checking with real signatures.

use tangle_cli::audit_support::run_collecting_diagnostics;

fn diags_for(fixture: &str) -> Vec<String> {
    let run = run_collecting_diagnostics(fixture);
    run.diagnostics.iter().map(|d| d.code.clone()).collect()
}

#[test]
fn call_arity_ok_no_diagnostics() {
    let codes = diags_for("tests/v03_phase1/fixtures/call_arity_ok.tangle.md");
    assert!(
        !codes.contains(&"TANGLE_ARITY_MISMATCH".to_string()),
        "Expected no arity mismatch, got diagnostics: {:?}", codes
    );
}

#[test]
fn call_arity_wrong_produces_diagnostic() {
    let codes = diags_for("tests/v03_phase1/fixtures/call_arity_wrong.tangle.md");
    assert!(
        codes.contains(&"TANGLE_ARITY_MISMATCH".to_string()),
        "Expected TANGLE_ARITY_MISMATCH for readFile() with 0 args, got: {:?}", codes
    );
}

#[test]
fn call_arg_type_wrong_produces_diagnostic() {
    let codes = diags_for("tests/v03_phase1/fixtures/call_arg_type_wrong.tangle.md");
    assert!(
        codes.contains(&"TANGLE_TYPE_ERROR".to_string()),
        "Expected TANGLE_TYPE_ERROR for readFile(123) where String expected, got: {:?}", codes
    );
}

#[test]
fn call_variadic_ok_no_diagnostics() {
    let codes = diags_for("tests/v03_phase1/fixtures/call_variadic_ok.tangle.md");
    assert!(
        !codes.contains(&"TANGLE_ARITY_MISMATCH".to_string()),
        "Variadic println should accept any number of args, got: {:?}", codes
    );
}
```

- [ ] **步骤 6：在 Cargo.toml 声明测试**

在 `compiler/tangle-cli/Cargo.toml` 的 `[[test]]` 列表末尾添加：

```toml
[[test]]
name = "call_type_checking"
path = "tests/v03_phase1/call_type_checking.rs"
```

- [ ] **步骤 7：运行测试验证失败**

运行：`cargo test --test call_type_checking`
预期：
- `call_arity_ok_no_diagnostics` — 通过（当前 Call 返回 Bool，不检查 arity）
- `call_arity_wrong_produces_diagnostic` — FAIL（当前不产生 TANGLE_ARITY_MISMATCH）
- `call_arg_type_wrong_produces_diagnostic` — FAIL（当前不产生 TANGLE_TYPE_ERROR）
- `call_variadic_ok_no_diagnostics` — 通过

- [ ] **步骤 8：实现 Call 表达式类型检查**

在 `compiler/tangle-cli/src/checker/check.rs` 中，替换 `Expr::Call` 分支（第 69-77 行）：

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
                _ => Type::Any,
            }
        }
```

- [ ] **步骤 9：运行测试验证通过**

运行：`cargo test --test call_type_checking`
预期：全部 4 个测试通过

- [ ] **步骤 10：运行 workspace 全量测试**

运行：`cargo test --workspace`
预期：全部通过。如果有现有测试失败，检查是否是 fixture 中的真实类型错误——修 fixture 而非放宽检查。

- [ ] **步骤 11：Commit**

```bash
git add compiler/tangle-cli/src/checker/check.rs compiler/tangle-cli/tests/v03_phase1/call_type_checking.rs compiler/tangle-cli/tests/v03_phase1/fixtures/call_arity_ok.tangle.md compiler/tangle-cli/tests/v03_phase1/fixtures/call_arity_wrong.tangle.md compiler/tangle-cli/tests/v03_phase1/fixtures/call_arg_type_wrong.tangle.md compiler/tangle-cli/tests/v03_phase1/fixtures/call_variadic_ok.tangle.md compiler/tangle-cli/Cargo.toml
git commit -m "feat(checker): Call expression arity + parameter type checking"
```

---

## 任务 7：兼容性验证 + Clippy

**文件：** 无新代码文件，仅验证和可能的 fixture 修复

- [ ] **步骤 1：运行 workspace 全量测试**

运行：
```bash
cd .worktrees/phase1-v0.3.0
cargo test --workspace
```
预期：全部通过。记录测试数量。

- [ ] **步骤 2：运行 Clippy**

运行：
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
预期：零警告。如果有警告，修复后重新运行。

- [ ] **步骤 3：运行 audit 矩阵（如果 worktree 中有 run-audit.ps1）**

运行：
```powershell
.\tests\audit\run-audit.ps1
```
预期：对照 v0.2.1 基线，零新增诊断。如果有新诊断：
- 如果是误报（签名写错）→ 修 `signatures.rs`
- 如果是 fixture 真实类型错误 → 修 fixture

- [ ] **步骤 4：验证 6 examples 无误报**

对每个 example 运行：
```bash
cargo run --bin tangle -- check examples/account.tangle.md
cargo run --bin tangle -- check examples/math-data.tangle.md
cargo run --bin tangle -- check examples/io-system.tangle.md
cargo run --bin tangle -- check examples/crypto.tangle.md
cargo run --bin tangle -- check examples/concurrency.tangle.md
cargo run --bin tangle -- check examples/collections.tangle.md
```
预期：零诊断或仅有 v0.2.1 已知的诊断。

- [ ] **步骤 5：验证 9 tests/ fixture 无误报**

对每个 test fixture 运行：
```bash
cargo run --bin tangle -- check tests/basic/hello.tangle.md
cargo run --bin tangle -- check tests/basic/expression.tangle.md
cargo run --bin tangle -- check tests/structs/user.tangle.md
cargo run --bin tangle -- check tests/rules/feature-toggles.tangle.md
cargo run --bin tangle -- check tests/rules/decision-tree.tangle.md
cargo run --bin tangle -- check tests/rules/decision-table.tangle.md
cargo run --bin tangle -- check tests/rules/approval-flow.tangle.md
cargo run --bin tangle -- check tests/mvp/order-service.tangle.md
cargo run --bin tangle -- check tests/errors/payment.tangle.md
```
预期：零诊断或仅有 v0.2.1 已知的诊断。

- [ ] **步骤 6：如果有 fixture 修复，commit**

```bash
git add -A
git commit -m "fix: update fixtures for Phase 1 type checking compatibility"
```

- [ ] **步骤 7：最终全量验证**

运行：
```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```
预期：全绿

- [ ] **步骤 8：Commit 版本号更新**

在 `compiler/tangle-cli/Cargo.toml` 中将 version 从 `0.2.1` 改为 `0.3.0`：

```toml
version = "0.3.0"
```

```bash
git add compiler/tangle-cli/Cargo.toml
git commit -m "release: v0.3.0 Phase 1 Call type checking"
```

- [ ] **步骤 9：打 tag（不推送）**

```bash
git tag v0.3.0
```

---

## 自检

### 规格覆盖度

| 规格章节 | 覆盖任务 |
|---------|---------|
| 1. 类型系统扩展（Any, is_variadic, types_equal, is_subtype） | 任务 1 |
| 2. stdlib 签名注册表 | 任务 3 |
| 3. checker 改动 — resolve_stdlib_imports | 任务 4 |
| 3. checker 改动 — MemberAccess | 任务 5 |
| 3. checker 改动 — Call 检查 | 任务 6 |
| 4. F-024 — TypeEnv.functions | 任务 2 |
| 4. F-024 — resolve_types 收集 | 任务 2 |
| 4. F-024 — Identifier 分支 | 任务 2 |
| 4. F-024 — collect_method_sigs 返回 Any | 任务 2 |
| 5. 测试与兼容性策略 | 任务 1-6 + 任务 7 |
| 出口闸 1: cargo test --workspace | 任务 7 步骤 1 |
| 出口闸 2: cargo clippy | 任务 7 步骤 2 |
| 出口闸 3: audit 矩阵 | 任务 7 步骤 3 |
| 出口闸 4: 6 examples + 9 tests | 任务 7 步骤 4-5 |
| 出口闸 5: 签名注册表覆盖 19 模块 | 任务 3 |

遗漏：无。

### 占位符扫描

无 TODO、待定、后续实现。所有代码步骤都包含完整代码。✅

### 类型一致性

- `FunctionType` 在任务 1 定义 `is_variadic: bool`，在任务 2/4/5/6 中使用 `is_variadic` 字段 — 一致。✅
- `CallableSignature` 在任务 1 定义 `is_variadic: bool`，在任务 3 的 `sig_fixed`/`sig_variadic` 中使用 — 一致。✅
- `Type::Any` 在任务 1 定义，在任务 2/3/6 中使用 — 一致。✅
- `TypeEnv.functions` 在任务 2 定义为 `HashMap<String, FunctionType>`，在任务 2 的 Identifier 分支中用 `env.functions.get(&e.name)` 返回 `Type::Function(ft.clone())` — 一致。✅
- `stdlib_signature` / `stdlib_module_signatures` 在任务 3 定义，在任务 4 中导入使用 — 一致。✅
- `flatten_headings` / `is_child_of_type_heading` 在任务 2 步骤 6b 定义，在步骤 6a 使用 — 一致。✅
