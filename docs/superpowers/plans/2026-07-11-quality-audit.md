# Tangle 质量审计阶段实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 通过穷举式审计 + 按根因批量修复，交付零虚假诊断的稳定基线 v0.2.1。

**架构：** 在 `compiler/tangle-cli/src/lib.rs` 暴露测试入口 `run_collecting_diagnostics`，在 `tests/audit/` 目录构建审计矩阵驱动脚本（PowerShell）+ ir-diff 工具（Rust 独立 crate）+ LSP 探测器（Rust 独立 crate），在 `compiler/tangle-cli/tests/audit_regression/` 下按根因组组织回归测试。修复走严格 TDD：写失败测试 → 验证红 → 最小修复 → 验证绿 → commit。

**技术栈：** Rust 2021 / cargo / PowerShell / 既有依赖（clap、serde、codespan-reporting、pulldown-cmark、tangle-std）

**规格来源：** [docs/superpowers/specs/2026-07-11-quality-audit-design.md](../specs/2026-07-11-quality-audit-design.md)

**预审计根因已知信息（来自头脑风暴期间代码审查）：**
- **G1 根因已确认：** [compiler/tangle-cli/src/checker/resolve.rs:11](../../compiler/tangle-cli/src/checker/resolve.rs#L11) 的 `resolve_types` 只迭代 `module.headings` 顶层，不递归 `heading.children`。所以 `### Account` 嵌套在 `# AccountDemo` 下时，未被注册进 `env.structs`，导致 `Account` 标识符在 `#### main` 中查找失败。
- **G2 真实根因（修正 spec 预测）：** 不是 `let` 绑定问题——`let` 已正确插入 `block_env.variables`（[check_module.rs:155](../../compiler/tangle-cli/src/checker/check_module.rs#L155)）。真实问题是方法参数未注入：[check_module.rs:147-148](../../compiler/tangle-cli/src/checker/check_module.rs#L147) 构造 `block_env` 时未把方法的 `params` 加入 `variables`，所以 `#### deposit` 体内 `account`/`amount` 标识符查找失败。
- **G3 是 G1 的级联：** `Account.open(100)` 中 `Account` 查不到 → 默认 `Type::Primitive(Bool)` → `.open(100)` 触发 "Member access on non-struct type"。修了 G1，G3 自动消失。
- **G4 已知：** [compiler/tangle-cli/src/cli/run.rs:17](../../compiler/tangle-cli/src/cli/run.rs#L17) `let (graph, source)` 的 `source` 未使用。

---

## 文件结构

**创建：**
- `compiler/tangle-cli/src/audit_support.rs` — 测试用入口模块（`run_collecting_diagnostics` + `TestRun` 结构）
- `compiler/tangle-cli/tests/audit_regression/mod.rs` — 共享测试 helper
- `compiler/tangle-cli/tests/audit_regression/G1_struct_symbol_resolution.rs`
- `compiler/tangle-cli/tests/audit_regression/G2_method_param_scope.rs`（修正命名：实际是方法参数而非 let 作用域）
- `compiler/tangle-cli/tests/audit_regression/G3_member_access_cascade.rs`
- `compiler/tangle-cli/tests/audit_regression/G5_doc_drift.rs`
- `compiler/tangle-cli/tests/audit_regression/G6_platform_diff.rs`
- `tests/audit/run-audit.ps1` — 审计矩阵驱动脚本
- `tests/audit/expected_diagnostics.yaml` — 预期诊断白名单
- `tests/audit/verify-exit-gate.ps1` — 出口闸验收脚本
- `tests/audit/ir-diff/Cargo.toml` — ir-diff 工具独立 crate
- `tests/audit/ir-diff/src/main.rs` — IR 语义比较工具
- `tests/audit/diff-ir.ps1` — 差分测试驱动脚本
- `tests/audit/lsp-probe/Cargo.toml` — LSP 探测器独立 crate
- `tests/audit/lsp-probe/src/main.rs` — LSP JSON-RPC 探测器
- `docs/audit/findings.md` — 审计 findings 报告
- `CHANGELOG.md` — 版本变更日志

**修改：**
- `compiler/tangle-cli/src/lib.rs` — 注册 `audit_support` 模块
- `compiler/tangle-cli/src/cli/run.rs` — G4 修复：删除未使用的 `source` 绑定
- `compiler/tangle-cli/src/checker/resolve.rs` — G1 修复：`resolve_types` 递归遍历
- `compiler/tangle-cli/src/checker/check_module.rs` — G2 修复：方法 `params` 注入 `block_env.variables`
- `compiler/tangle-cli/Cargo.toml` — 版本号 0.2.0 → 0.2.1
- `Cargo.lock` — 自动随 cargo build 更新
- `README.md` / `README.zh.md` — Roadmap 表新增 v0.2.1 行

---

## 任务 1：G4 修复 — 清理 unused variable 告警

**文件：**
- 修改：`compiler/tangle-cli/src/cli/run.rs:17`

**背景：** `cargo build` 输出 `warning: unused variable: source` 在 [run.rs:17](../../compiler/tangle-cli/src/cli/run.rs#L17)。`compile_file` 返回 `(graph, source)` 元组，但 `run` 函数只用 `graph`。

- [ ] **步骤 1：确认当前告警存在**

运行：
```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")
cargo build 2>&1 | Select-String "unused variable"
```
预期输出：包含 `warning: unused variable: 'source'` 一行。

- [ ] **步骤 2：修复 `run` 函数**

把 [run.rs:17](../../compiler/tangle-cli/src/cli/run.rs#L17) 的 `let (graph, source) = compile_file(&opts);` 改为 `let (graph, _source) = compile_file(&opts);`，明确标记为故意未使用。

```rust
pub fn run(opts: BuildOptions) {
    let (graph, _source) = compile_file(&opts);
    let module_name = module_name_from_file(&opts.file);
    // ... 其余不变
}
```

- [ ] **步骤 3：验证告警消失**

运行：
```powershell
cargo build 2>&1 | Select-String "unused variable"
```
预期：无输出（告警消失）。

- [ ] **步骤 4：验证既有测试不退化**

运行：`cargo test --workspace`
预期：99 个测试全过，0 失败。

- [ ] **步骤 5：验证 clippy 通过**

运行：`cargo clippy --workspace -- -D warnings`
预期：退出码 0，无告警。

- [ ] **步骤 6：Commit**

```powershell
git add compiler/tangle-cli/src/cli/run.rs
git commit -m "fix(audit): G4 clear unused variable warning in run.rs"
```

---

## 任务 2：添加测试入口 `run_collecting_diagnostics`

**文件：**
- 创建：`compiler/tangle-cli/src/audit_support.rs`
- 修改：`compiler/tangle-cli/src/lib.rs`（注册模块）

**背景：** 测试需要精确断言诊断列表，而不是 grep stderr 字符串。`run_collecting_diagnostics` 复用 `run_pipeline` 的逻辑但不执行生成代码，返回结构化诊断列表。

- [ ] **步骤 1：创建 `audit_support.rs`**

```rust
//! Test-only entry point that runs the compile pipeline and returns
//! structured diagnostics. Not part of the public API.

use crate::frontend::compile_module::{compile_module, CompileModuleInput};
use crate::checker::check_module::check_module;
use crate::ir::compile_to_ir::compile_to_ir;
use crate::model::TangleDiagnostic;

#[derive(Debug, Clone)]
pub struct TestRun {
    pub exit_code: i32,
    pub diagnostics: Vec<TangleDiagnostic>,
    pub stdout: String,
    pub stderr: String,
}

/// Run frontend → checker → IR pipeline on a file, collecting all diagnostics.
/// Does NOT execute the generated code. Suitable for audit regression tests.
pub fn run_collecting_diagnostics(file: &str) -> TestRun {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            return TestRun {
                exit_code: 1,
                diagnostics: vec![],
                stdout: String::new(),
                stderr: format!("Error reading file: {}", e),
            };
        }
    };

    let mut all_diags: Vec<TangleDiagnostic> = Vec::new();

    let module = compile_module(CompileModuleInput {
        file: file.to_string(),
        source: source.clone(),
    });
    all_diags.extend(module.diagnostics.clone());

    let checked = check_module(module);
    all_diags.extend(checked.diagnostics.clone());

    let (_graph, ir_diags) = compile_to_ir(&checked);
    all_diags.extend(ir_diags);

    let exit_code = if all_diags.is_empty() { 0 } else { 1 };
    TestRun {
        exit_code,
        diagnostics: all_diags,
        stdout: String::new(),
        stderr: String::new(),
    }
}
```

- [ ] **步骤 2：在 `lib.rs` 注册模块**

在 [lib.rs](../../compiler/tangle-cli/src/lib.rs) 末尾追加：

```rust
pub mod audit_support;
```

完整 `lib.rs`：

```rust
pub mod model;
pub mod ast;
pub mod diagnostic;
pub mod frontend;
pub mod markdown;
pub mod parser;
pub mod checker;
pub mod ir;
pub mod codegen;
pub mod cli;
pub mod stdlib;
pub mod incremental;
pub mod lsp;
pub mod docgen;
pub mod audit_support;
```

- [ ] **步骤 3：验证编译通过**

运行：`cargo build -p tangle-cli`
预期：退出码 0，无错误。

- [ ] **步骤 4：写一个冒烟测试验证 helper 工作**

创建临时测试文件 `compiler/tangle-cli/tests/audit_smoke.rs`：

```rust
use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn smoke_test_helper_returns_diagnostics_for_account_example() {
    let run = run_collecting_diagnostics("examples/account.tangle.md");
    // 当前已知 account.tangle.md 有虚假诊断，所以 diagnostics 非空
    assert!(!run.diagnostics.is_empty(), "expected false-positive diagnostics before G1 fix");
    println!("Captured {} diagnostics", run.diagnostics.len());
    for d in &run.diagnostics {
        println!("  [{}] {}", d.code, d.message);
    }
}
```

- [ ] **步骤 5：运行冒烟测试**

运行：`cargo test -p tangle-cli --test audit_smoke`
预期：通过，输出捕获的诊断数量（应为 4 条）。

- [ ] **步骤 6：Commit**

```powershell
git add compiler/tangle-cli/src/audit_support.rs compiler/tangle-cli/src/lib.rs compiler/tangle-cli/tests/audit_smoke.rs
git commit -m "feat(audit): add run_collecting_diagnostics test entry point"
```

---

## 任务 3：创建 audit_regression 目录与 G1 失败测试

**文件：**
- 创建：`compiler/tangle-cli/tests/audit_regression/G1_struct_symbol_resolution.rs`

**背景：** 这是 TDD 的红阶段。G1 测试断言 `examples/account.tangle.md` 经 `run_collecting_diagnostics` 后**零诊断**——当前会失败（有 4 条虚假诊断）。

- [ ] **步骤 1：创建测试文件**

```rust
//! G1: Struct/type symbol resolution regression tests.
//!
//! Root cause: resolve_types in compiler/tangle-cli/src/checker/resolve.rs
//! only iterated top-level headings, missing types nested under # Program.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn g1_account_example_has_no_diagnostics() {
    let run = run_collecting_diagnostics("examples/account.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "expected zero diagnostics, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn g1_struct_symbol_visible_in_main_block() {
    // account.tangle.md: ### Account nested under # AccountDemo
    // #### main references Account.open(100)? — Account must be resolvable.
    let run = run_collecting_diagnostics("examples/account.tangle.md");
    let has_symbol_not_found = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND" && d.message.contains("Account"));
    assert!(
        !has_symbol_not_found,
        "Account symbol should be resolvable in main block"
    );
}
```

- [ ] **步骤 2：运行测试验证失败（红）**

运行：`cargo test -p tangle-cli --test G1_struct_symbol_resolution`
预期：FAIL，断言失败信息包含 "expected zero diagnostics, got 4"。

- [ ] **步骤 3：Commit（红阶段）**

```powershell
git add compiler/tangle-cli/tests/audit_regression/G1_struct_symbol_resolution.rs
git commit -m "test(audit): G1 add failing regression test for struct symbol resolution"
```

---

## 任务 4：G1 修复 — resolve_types 递归遍历

**文件：**
- 修改：`compiler/tangle-cli/src/checker/resolve.rs:6-56`

**根因：** [resolve.rs:11](../../compiler/tangle-cli/src/checker/resolve.rs#L11) `for heading in &module.headings` 只看顶层。`### Account` 嵌套在 `# AccountDemo` 的 `children` 里，从未被访问。

- [ ] **步骤 1：将 `resolve_types` 改为递归遍历**

替换 [resolve.rs](../../compiler/tangle-cli/src/checker/resolve.rs) 的 `resolve_types` 函数：

```rust
pub fn resolve_types(module: &TangleModule) -> (TypeEnv, Vec<TangleDiagnostic>) {
    let diagnostics = vec![];
    let mut env = TypeEnv::new();
    collect_types_recursive(&module.headings, &mut env);
    (env, diagnostics)
}

fn collect_types_recursive(headings: &[TangleHeading], env: &mut TypeEnv) {
    for heading in headings {
        if heading.role == HeadingRole::Type {
            let name = heading
                .symbol_name
                .clone()
                .unwrap_or_else(|| heading.title.clone());
            let is_interface = heading.title.contains("接口") || heading.title.contains("interface");

            if is_interface {
                let methods = collect_method_sigs(&heading.children);
                env.interfaces.insert(
                    name.clone(),
                    Type::Interface(InterfaceType { name, methods }),
                );
            } else {
                let fields: HashMap<String, Type> = heading
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .type_name
                            .as_ref()
                            .and_then(|tn| type_name_to_type(tn))
                            .unwrap_or(Type::Primitive(PrimitiveType {
                                name: "String".into(),
                            }));
                        (p.name.clone(), ty)
                    })
                    .collect();

                let methods = collect_method_sigs(&heading.children);

                env.structs.insert(
                    name.clone(),
                    Type::Struct(StructType {
                        name,
                        fields,
                        methods,
                    }),
                );
            }
        }
        // Recurse into children regardless of this heading's role —
        // a Type heading may be nested under a Program or Section heading.
        collect_types_recursive(&heading.children, env);
    }
}
```

注意：保留原 `collect_method_sigs`、`type_name_to_type`、`find_receiver_heading`、`find_receiver_recursive` 不变。

- [ ] **步骤 2：运行 G1 测试验证通过（绿）**

运行：`cargo test -p tangle-cli --test G1_struct_symbol_resolution`
预期：两个测试都 PASS。

- [ ] **步骤 3：验证 account.tangle.md 实际运行干净**

运行：
```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")
cargo run --quiet -- run examples/account.tangle.md 2>&1
```
预期：stderr 不含 `TANGLE_SYMBOL_NOT_FOUND` 或 `TANGLE_TYPE_ERROR`；stdout 输出 `{ balance: 150 }`。

- [ ] **步骤 4：回归既有测试**

运行：`cargo test --workspace`
预期：99 + 新增测试全过。

- [ ] **步骤 5：Commit**

```powershell
git add compiler/tangle-cli/src/checker/resolve.rs
git commit -m "fix(audit): G1 resolve_types recurses into heading children

Root cause: resolve_types only iterated top-level module.headings,
missing types nested under # Program. Now uses collect_types_recursive
to walk the full heading tree.

Refs F-001 (account.tangle.md false positives)."
```

---

## 任务 5：G2 修复 — 方法参数注入 block_env

**文件：**
- 创建：`compiler/tangle-cli/tests/audit_regression/G2_method_param_scope.rs`
- 修改：`compiler/tangle-cli/src/checker/check_module.rs:122-150`

**根因：** [check_module.rs:147](../../compiler/tangle-cli/src/checker/check_module.rs#L147) 构造 `block_env` 时只设置了 `receiver`，没把当前方法的 `params` 加入 `variables`。所以 `#### deposit` 的 `account`/`amount` 参数在体内查不到。

- [ ] **步骤 1：写 G2 失败测试**

```rust
//! G2: Method parameter scope regression tests.
//!
//! Root cause: check_module built block_env with receiver but did not
//! inject the method's params into variables, so method body references
//! to declared parameters were reported as SYMBOL_NOT_FOUND.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn g2_method_params_resolvable_in_body() {
    let run = run_collecting_diagnostics("examples/account.tangle.md");
    let param_not_found = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND"
            && (d.message.contains("account") || d.message.contains("amount")));
    assert!(
        !param_not_found,
        "method params 'account' / 'amount' should be resolvable in deposit body, got: {}",
        run.diagnostics.iter().map(|d| format!("[{}] {}", d.code, d.message)).collect::<Vec<_>>().join("; ")
    );
}
```

- [ ] **步骤 2：运行验证失败（红）**

运行：`cargo test -p tangle-cli --test G2_method_param_scope`
预期：FAIL（如果 G1 已修，account 参数仍查不到）。

**注：** 如果 G1 修复后该测试意外通过（说明 G1 cascade 已覆盖），跳到步骤 4。否则继续。

- [ ] **步骤 3：在 `check_module.rs` 注入方法参数**

定位 [check_module.rs](../../compiler/tangle-cli/src/checker/check_module.rs) 中 `let mut block_env = type_env.clone();` 这一行（约 147 行）。在其后、`for stmt in &block.body.statements` 之前，注入当前 heading 的 params：

```rust
        let mut block_env = type_env.clone();
        block_env.receiver = receiver;

        // Inject current heading's params into local scope so method bodies
        // can reference their declared parameters.
        // Find the heading that owns this code block by heading_id.
        if let Some(owner) = find_heading_by_id(&block.heading_id, &module.headings) {
            for p in &owner.params {
                let ty = p.type_name.as_ref()
                    .and_then(|tn| match tn.as_str() {
                        "String" => Some(Type::Primitive(PrimitiveType { name: "String".into() })),
                        "Int" => Some(Type::Primitive(PrimitiveType { name: "Int".into() })),
                        "Bool" => Some(Type::Primitive(PrimitiveType { name: "Bool".into() })),
                        _ => Some(Type::Primitive(PrimitiveType { name: tn.clone() })),
                    })
                    .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                block_env.variables.insert(p.name.clone(), ty);
            }
        }

        for stmt in &block.body.statements {
            // ... 既有代码不变
```

并在 `check_module.rs` 末尾添加 helper：

```rust
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
```

- [ ] **步骤 4：运行 G2 测试验证通过（绿）**

运行：`cargo test -p tangle-cli --test G2_method_param_scope`
预期：PASS。

- [ ] **步骤 5：回归既有测试**

运行：`cargo test --workspace`
预期：全过。

- [ ] **步骤 6：Commit**

```powershell
git add compiler/tangle-cli/src/checker/check_module.rs compiler/tangle-cli/tests/audit_regression/G2_method_param_scope.rs
git commit -m "fix(audit): G2 inject method params into block_env variables

Root cause: check_module built block_env with receiver but did not
inject the method's declared params into variables. Now finds the
heading owning the code block by heading_id and seeds variables
with its params before type-checking the body."
```

---

## 任务 6：G3 验证 — member access 级联修复

**文件：**
- 创建：`compiler/tangle-cli/tests/audit_regression/G3_member_access_cascade.rs`

**背景：** G3 是 G1 的级联——`Account` 查不到导致默认 `Bool` 类型，`.open(...)` 触发 "Member access on non-struct type"。G1 修了 G3 应自动消失。本任务只加显式回归测试守护这个不变量。

- [ ] **步骤 1：写 G3 测试**

```rust
//! G3: Member access on non-struct type — was a cascade of G1.
//!
//! After G1 fix, Account resolves to Type::Struct, so Account.open(...)
//! no longer triggers TANGLE_TYPE_ERROR "Member access on non-struct type".

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn g3_no_member_access_on_non_struct_after_g1() {
    let run = run_collecting_diagnostics("examples/account.tangle.md");
    let has_member_access_error = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_TYPE_ERROR"
            && d.message.contains("Member access on non-struct type"));
    assert!(
        !has_member_access_error,
        "Member access on non-struct should not occur after G1 fix, got: {}",
        run.diagnostics.iter().map(|d| format!("[{}] {}", d.code, d.message)).collect::<Vec<_>>().join("; ")
    );
}
```

- [ ] **步骤 2：运行测试验证通过**

运行：`cargo test -p tangle-cli --test G3_member_access_cascade`
预期：PASS（G1 已修，无 member access 错误）。

**若失败：** 说明 G3 有独立根因，需单独修——检查 `check_expression` 中 `Expr::MemberAccess` 的对象类型推断路径，可能需要识别 `Account.open` 的 `Account` 解析为 `Type::Struct` 后访问 methods 而非 fields。

- [ ] **步骤 3：Commit**

```powershell
git add compiler/tangle-cli/tests/audit_regression/G3_member_access_cascade.rs
git commit -m "test(audit): G3 add regression for member access cascade from G1"
```

---

## 任务 7：删除冒烟测试，补全 G5/G6 占位测试结构

**文件：**
- 删除：`compiler/tangle-cli/tests/audit_smoke.rs`
- 创建：`compiler/tangle-cli/tests/audit_regression/G5_doc_drift.rs`
- 创建：`compiler/tangle-cli/tests/audit_regression/G6_platform_diff.rs`

**背景：** `audit_smoke.rs` 只是任务 2 验证 helper 的临时文件，正式回归测试已覆盖其功能。G5/G6 是 discovery-driven，先建空文件占位，审计后填具体测试。

- [ ] **步骤 1：删除冒烟测试**

```powershell
Remove-Item compiler/tangle-cli/tests/audit_smoke.rs
```

- [ ] **步骤 2：创建 G5 占位测试**

```rust
//! G5: Documentation/example drift regression tests.
//!
//! Populated after audit run (Task 11) surfaces specific drift findings.
//! Each finding becomes a test asserting that the example output matches
//! the documented behavior.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn g5_placeholder_all_examples_run_without_diagnostics() {
    // Audit baseline: every example should run with zero diagnostics after
    // G1-G3 fixes. This test will fail if any example regresses.
    let examples = [
        "examples/account.tangle.md",
        "examples/collections.tangle.md",
        "examples/concurrency.tangle.md",
        "examples/crypto.tangle.md",
        "examples/io-system.tangle.md",
        "examples/math-data.tangle.md",
    ];
    let mut failures = Vec::new();
    for ex in &examples {
        let run = run_collecting_diagnostics(ex);
        if !run.diagnostics.is_empty() {
            failures.push(format!(
                "{}: {} diagnostics",
                ex,
                run.diagnostics.len()
            ));
        }
    }
    assert!(failures.is_empty(), "examples with diagnostics:\n{}", failures.join("\n"));
}
```

- [ ] **步骤 3：创建 G6 占位测试**

```rust
//! G6: Platform/target differential regression tests.
//!
//! Populated after audit run (Task 11) if any diagnostics appear in
//! py/go targets but not js. Currently a baseline guard: all three
//! targets must produce equivalent diagnostic profiles.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn g6_placeholder_all_targets_equivalent_for_account() {
    // Baseline: account.tangle.md diagnostics should not depend on target.
    // Since run_collecting_diagnostics runs the frontend→checker→IR pipeline
    // (target-agnostic), this is automatically true today; the test exists
    // to fail loudly if codegen ever starts feeding back into the checker.
    let run = run_collecting_diagnostics("examples/account.tangle.md");
    assert!(run.diagnostics.is_empty(), "account.tangle.md should be clean across all targets");
}
```

- [ ] **步骤 4：运行所有 audit_regression 测试**

运行：`cargo test -p tangle-cli --test audit_regression -- --test-threads=1`（如果用 mod 组织）或 `cargo test -p tangle-cli --test G1_struct_symbol_resolution --test G2_method_param_scope --test G3_member_access_cascade --test G5_doc_drift --test G6_platform_diff`

预期：全部 PASS。

- [ ] **步骤 5：Commit**

```powershell
git add -A compiler/tangle-cli/tests/
git commit -m "test(audit): remove smoke test, add G5/G6 placeholder regression tests"
```

---

## 任务 8：创建审计矩阵驱动脚本 `run-audit.ps1`

**文件：**
- 创建：`tests/audit/run-audit.ps1`

**背景：** 遍历 §2 矩阵的所有 cell，每个 cell 跑一次 `tangle` 子命令，捕获退出码/stdout/stderr/诊断条目，输出 CSV + summary。

- [ ] **步骤 1：创建 `tests/audit/` 目录与脚本**

```powershell
New-Item -ItemType Directory -Force -Path tests/audit
New-Item -ItemType Directory -Force -Path tests/audit/output
```

写入 `tests/audit/run-audit.ps1`：

```powershell
#requires -Version 5.1
<#
.SYNOPSIS
    Audit matrix driver: runs every (CLI surface × target × mode × fixture) cell.
.DESCRIPTION
    Outputs:
      tests/audit/output/<timestamp>/matrix.csv   - one row per cell
      tests/audit/output/<timestamp>/cells/<id>.out - per-cell stdout+stderr
      tests/audit/output/<timestamp>/summary.md    - failing cells grouped
#>
[CmdletBinding()]
param(
    [string]$OutputDir = (Join-Path $PSScriptRoot "output")
)

$ErrorActionPreference = "Continue"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $OutputDir $timestamp
$cellsDir = Join-Path $runDir "cells"
New-Item -ItemType Directory -Force -Path $cellsDir | Out-Null

$csvPath = Join-Path $runDir "matrix.csv"
$summaryPath = Join-Path $runDir "summary.md"

# Fixtures
$examples = Get-ChildItem "examples\*.tangle.md" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
$tests = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
$fixtures = @($examples) + @($tests) | Where-Object { $_ -ne $null }

# Surfaces, targets, modes
$surfaces = @("run","build","doc")
$targets = @("js","py","go")
$modes = @("normal","incremental","interp")

"surface,target,mode,fixture,exit_code,diag_count,diag_codes" | Out-File -FilePath $csvPath -Encoding UTF8

$failingCells = [System.Collections.ArrayList]::new()
$cellCount = 0

foreach ($fixture in $fixtures) {
    foreach ($surface in $surfaces) {
        foreach ($target in $targets) {
            foreach ($mode in $modes) {
                # Skip invalid combinations
                if ($surface -eq "doc") {
                    if ($target -ne "js" -or $mode -ne "normal") { continue }
                }
                if ($surface -eq "build" -and $mode -eq "interp") { continue }
                if ($target -eq "py" -and $mode -eq "interp") { continue }
                if ($target -eq "go" -and $mode -eq "interp") { continue }

                $cellId = "${surface}_${target}_${mode}_$(Split-Path $fixture -Leaf)"
                $args = @($surface, $fixture, "--target", $target)
                if ($mode -eq "incremental") { $args += "--incremental" }
                if ($surface -eq "run" -and $mode -eq "interp") { $args += "--interp" }

                $cellOutFile = Join-Path $cellsDir "$cellId.out"
                $stderrFile = Join-Path $cellsDir "$cellId.err"

                $process = Start-Process -FilePath "cargo" -ArgumentList (@("run","--quiet","--") + $args) -NoNewWindow -PassThru -Wait -RedirectStandardOutput $cellOutFile -RedirectStandardError $stderrFile
                $exitCode = $process.ExitCode

                $stderrContent = Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue
                $diagMatches = [regex]::Matches($stderrContent, 'error\[(TANGLE_[A-Z_]+)\]')
                $diagCount = $diagMatches.Count
                $diagCodes = ($diagMatches | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique) -join '|'

                $csvRow = "$surface,$target,$mode,$fixture,$exitCode,$diagCount,$diagCodes"
                Add-Content -Path $csvPath -Value $csvRow -Encoding UTF8

                if ($diagCount -gt 0) {
                    $failingCells.Add([PSCustomObject]@{
                        Cell = $cellId; Surface = $surface; Target = $target; Mode = $mode; Fixture = $fixture; DiagCount = $diagCount; Codes = $diagCodes
                    }) | Out-Null
                }
                $cellCount++
            }
        }
    }
}

# Add emit-ir cells
foreach ($fixture in $fixtures) {
    $cellId = "build_emit-ir_normal_$(Split-Path $fixture -Leaf)"
    $cellOutFile = Join-Path $cellsDir "$cellId.out"
    $stderrFile = Join-Path $cellsDir "$cellId.err"
    $process = Start-Process -FilePath "cargo" -ArgumentList @("run","--quiet","--","build",$fixture,"--emit-ir") -NoNewWindow -PassThru -Wait -RedirectStandardOutput $cellOutFile -RedirectStandardError $stderrFile
    $stderrContent = Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue
    $diagMatches = [regex]::Matches($stderrContent, 'error\[(TANGLE_[A-Z_]+)\]')
    $diagCount = $diagMatches.Count
    $diagCodes = ($diagMatches | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique) -join '|'
    Add-Content -Path $csvPath -Value "build,emit-ir,normal,$fixture,$($process.ExitCode),$diagCount,$diagCodes" -Encoding UTF8
    if ($diagCount -gt 0) {
        $failingCells.Add([PSCustomObject]@{ Cell = $cellId; Surface = "build"; Target = "emit-ir"; Mode = "normal"; Fixture = $fixture; DiagCount = $diagCount; Codes = $diagCodes }) | Out-Null
    }
    $cellCount++
}

# Summary
$summary = @"
# Audit Run Summary

- Timestamp: $timestamp
- Total cells: $cellCount
- Failing cells (with diagnostics): $($failingCells.Count)

## Failing cells

| Cell | Surface | Target | Mode | Fixture | DiagCount | Codes |
|------|---------|--------|------|---------|-----------|-------|
"@
foreach ($f in $failingCells) {
    $summary += "`n| $($f.Cell) | $($f.Surface) | $($f.Target) | $($f.Mode) | $(Split-Path $f.Fixture -Leaf) | $($f.DiagCount) | $($f.Codes) |"
}
$summary | Out-File -FilePath $summaryPath -Encoding UTF8

Write-Host "Audit complete: $cellCount cells, $($failingCells.Count) failing"
Write-Host "Run dir: $runDir"
Write-Host "Summary: $summaryPath"
```

- [ ] **步骤 2：跑一遍验证脚本能工作**

运行：
```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")
.\tests\audit\run-audit.ps1
```
预期：脚本完成，输出 "Audit complete: N cells, M failing"；生成 `tests/audit/output/<timestamp>/` 目录含 `matrix.csv`、`summary.md`、`cells/`。

- [ ] **步骤 3：人工检查 summary.md 确认 G1/G2 修复后 account.tangle.md 不再失败**

读 `summary.md`，预期：account.tangle.md 相关 cell 的 DiagCount=0。

- [ ] **步骤 4：Commit**

```powershell
git add tests/audit/run-audit.ps1
git commit -m "feat(audit): add run-audit.ps1 matrix driver script"
```

---

## 任务 9：创建预期诊断白名单与出口闸骨架

**文件：**
- 创建：`tests/audit/expected_diagnostics.yaml`
- 创建：`tests/audit/verify-exit-gate.ps1`

- [ ] **步骤 1：创建空的白名单**

`tests/audit/expected_diagnostics.yaml`：

```yaml
# Expected diagnostics whitelist.
# A fixture not listed here is expected to produce zero diagnostics.
# A fixture listed here declares which TANGLE_* codes it intentionally triggers.
#
# Format:
#   <fixture-filename>:
#     <surface>/<target>/<mode>:
#       - TANGLE_CODE_1
#       - TANGLE_CODE_2
#
# Empty initially. Populate as audit reveals intentional error fixtures.

# Example (not yet populated — to be filled after first audit run):
# payment.tangle.md:
#   run/js/normal:
#     - TANGLE_PANIC_REACHED
```

- [ ] **步骤 2：创建出口闸脚本**

`tests/audit/verify-exit-gate.ps1`：

```powershell
#requires -Version 5.1
<#
.SYNOPSIS
    Exit gate verifier: runs all 5 exit gates and reports PASS/FAIL.
#>
$ErrorActionPreference = "Stop"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

$root = (Get-Item $PSScriptRoot).Parent.Parent.FullName
Set-Location $root

Write-Host "=== Exit Gate 1/5: cargo test --workspace ===" -ForegroundColor Cyan
cargo test --workspace 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "EXIT GATE: FAIL — cargo test" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 2/5: cargo clippy --workspace -- -D warnings ===" -ForegroundColor Cyan
cargo clippy --workspace -- -D warnings 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "EXIT GATE: FAIL — cargo clippy" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 3/5: run-audit.ps1 ===" -ForegroundColor Cyan
& $PSScriptRoot\run-audit.ps1 | Tee-Object -Variable auditOut
# Parse last line: "Audit complete: N cells, M failing"
$lastLine = ($auditOut -split "`n") | Select-Object -Last 1
if ($lastLine -match "(\d+) failing") {
    $failingCount = [int]$Matches[1]
    if ($failingCount -gt 0) {
        Write-Host "EXIT GATE: FAIL — audit has $failingCount failing cells" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "EXIT GATE: FAIL — could not parse audit output" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 4/5: diff-ir.ps1 ===" -ForegroundColor Cyan
& $PSScriptRoot\diff-ir.ps1 | Tee-Variable -Variable diffOut
$diffLast = ($diffOut -split "`n") | Select-Object -Last 1
if ($diffLast -match "(\d+) DIFF") {
    $diffCount = [int]$Matches[1]
    if ($diffCount -gt 0) {
        Write-Host "EXIT GATE: FAIL — diff-ir has $diffCount DIFFs" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "EXIT GATE: FAIL — could not parse diff-ir output" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 5/5: audit_regression tests ===" -ForegroundColor Cyan
cargo test -p tangle-cli --test G1_struct_symbol_resolution --test G2_method_param_scope --test G3_member_access_cascade --test G5_doc_drift --test G6_platform_diff 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "EXIT GATE: FAIL — audit_regression tests" -ForegroundColor Red
    exit 1
}

Write-Host "EXIT GATE: PASS" -ForegroundColor Green
exit 0
```

- [ ] **步骤 3：Commit**

```powershell
git add tests/audit/expected_diagnostics.yaml tests/audit/verify-exit-gate.ps1
git commit -m "feat(audit): add expected_diagnostics whitelist + exit gate verifier"
```

---

## 任务 10：创建 ir-diff 工具与差分测试脚本

**文件：**
- 创建：`tests/audit/ir-diff/Cargo.toml`
- 创建：`tests/audit/ir-diff/src/main.rs`
- 创建：`tests/audit/diff-ir.ps1`

**背景：** 比对 TS reference 的 `--emit-ir` 输出与 Rust 编译器的 `--emit-ir` 输出。`ir-diff` 做**语义比较**：忽略 JSON 键序、忽略 source span（file/line/column），只比 Rule Graph 结构。

- [ ] **步骤 1：创建 ir-diff crate**

`tests/audit/ir-diff/Cargo.toml`：

```toml
[package]
name = "ir-diff"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "ir-diff"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **步骤 2：写 ir-diff 主体**

`tests/audit/ir-diff/src/main.rs`：

```rust
//! ir-diff: semantic comparison of two Tangle IR JSON files.
//!
//! Strips source spans (file/line/column) and compares Rule Graph structure.
//! Exit code 0 = MATCH, 1 = DIFF (prints first difference to stderr).

use serde_json::{Value, JsonMap};
use std::env;
use std::fs;
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: ir-diff <ts-ir.json> <rs-ir.json>");
        exit(2);
    }
    let ts_json = fs::read_to_string(&args[1]).expect("read ts-ir");
    let rs_json = fs::read_to_string(&args[2]).expect("read rs-ir");

    let ts: Value = serde_json::from_str(&ts_json).expect("parse ts-ir JSON");
    let rs: Value = serde_json::from_str(&rs_json).expect("parse rs-ir JSON");

    let ts_normalized = normalize(ts);
    let rs_normalized = normalize(rs);

    if ts_normalized == rs_normalized {
        println!("MATCH");
        exit(0);
    } else {
        eprintln!("DIFF");
        let ts_str = serde_json::to_string_pretty(&ts_normalized).unwrap();
        let rs_str = serde_json::to_string_pretty(&rs_normalized).unwrap();
        eprintln!("--- ts-ir normalized ---\n{}", ts_str);
        eprintln!("--- rs-ir normalized ---\n{}", rs_str);
        exit(1);
    }
}

/// Recursively strip source span fields and sort object keys for stable comparison.
fn normalize(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut filtered = JsonMap::new();
            for (k, v) in map {
                // Skip span fields entirely
                if k == "span" || k == "file" || k == "start_line" || k == "start_column"
                    || k == "end_line" || k == "end_column" || k == "source" {
                    continue;
                }
                filtered.insert(k, normalize(v));
            }
            // Sort keys for stable comparison
            let mut sorted: Vec<(String, Value)> = filtered.into_iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize).collect()),
        other => other,
    }
}
```

- [ ] **步骤 3：编译 ir-diff**

运行：
```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")
cargo build --release --manifest-path tests/audit/ir-diff/Cargo.toml
```
预期：编译成功，二进制在 `tests/audit/ir-diff/target/release/ir-diff.exe`。

- [ ] **步骤 4：写 diff-ir.ps1 驱动脚本**

`tests/audit/diff-ir.ps1`：

```powershell
#requires -Version 5.1
<#
.SYNOPSIS
    Differential IR testing: TS reference vs Rust compiler.
#>
$ErrorActionPreference = "Continue"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

$root = (Get-Item $PSScriptRoot).Parent.Parent.FullName
Set-Location $root

$irDiffBin = Join-Path $PSScriptRoot "ir-diff\target\release\ir-diff.exe"
if (-not (Test-Path $irDiffBin)) {
    Write-Host "Building ir-diff..."
    cargo build --release --manifest-path (Join-Path $PSScriptRoot "ir-diff\Cargo.toml") | Out-Null
}

# Ensure TS reference is built
$tsEntry = Join-Path $root "reference\dist\src\cli\main.js"
if (-not (Test-Path $tsEntry)) {
    Write-Host "Building TS reference..."
    Push-Location (Join-Path $root "reference")
    npm run build 2>&1 | Out-Null
    Pop-Location
}

$fixtures = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName

$workDir = Join-Path $env:TEMP "tangle-diff-ir"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

$matchCount = 0
$diffCount = 0
$skipCount = 0

foreach ($fixture in $fixtures) {
    $name = [System.IO.Path]::GetFileNameWithoutExtension($fixture)
    $tsIr = Join-Path $workDir "$name.ts.json"
    $rsIr = Join-Path $workDir "$name.rs.json"

    # TS reference
    $tsProc = Start-Process -FilePath "node" -ArgumentList @($tsEntry, "run", $fixture, "--emit-ir") -NoNewWindow -PassThru -Wait -RedirectStandardOutput $tsIr -RedirectStandardError (Join-Path $workDir "$name.ts.err")
    if ($tsProc.ExitCode -ne 0) {
        Write-Host "[SKIPPED] $name — TS reference failed"
        $skipCount++
        continue
    }

    # Rust
    $rsProc = Start-Process -FilePath "cargo" -ArgumentList @("run","--quiet","--","build",$fixture,"--emit-ir") -NoNewWindow -PassThru -Wait -RedirectStandardOutput $rsIr -RedirectStandardError (Join-Path $workDir "$name.rs.err")
    if ($rsProc.ExitCode -ne 0 -and -not (Test-Path $rsIr)) {
        Write-Host "[DIFF] $name — Rust failed to emit IR"
        $diffCount++
        continue
    }

    $cmpProc = Start-Process -FilePath $irDiffBin -ArgumentList @($tsIr, $rsIr) -NoNewWindow -PassThru -Wait -RedirectStandardOutput (Join-Path $workDir "$name.cmp.out") -RedirectStandardError (Join-Path $workDir "$name.cmp.err")
    $cmpOut = Get-Content (Join-Path $workDir "$name.cmp.out") -Raw
    if ($cmpOut -match "MATCH") {
        Write-Host "[MATCH] $name"
        $matchCount++
    } else {
        Write-Host "[DIFF] $name"
        $diffCount++
    }
}

Write-Host ""
Write-Host "Diff-IR complete: $matchCount MATCH, $diffCount DIFF, $skipCount SKIPPED"
exit $(if ($diffCount -gt 0) { 1 } else { 0 })
```

- [ ] **步骤 5：跑一遍验证**

运行：
```powershell
.\tests\audit\diff-ir.ps1
```
预期：输出每个 fixture 的 `[MATCH]` / `[DIFF]` / `[SKIPPED]`，最后一行总结。

**注：** 当前 TS reference 可能与 Rust 实现有真实差异（reference 是 0.x 版本）。任何 `[DIFF]` 都进 findings.md（任务 11），不阻断本任务完成。

- [ ] **步骤 6：Commit**

```powershell
git add tests/audit/ir-diff tests/audit/diff-ir.ps1
git commit -m "feat(audit): add ir-diff tool + diff-ir.ps1 differential harness"
```

---

## 任务 11：跑全量审计，写 findings.md

**文件：**
- 创建：`docs/audit/findings.md`

- [ ] **步骤 1：跑 run-audit.ps1 与 diff-ir.ps1**

```powershell
.\tests\audit\run-audit.ps1
.\tests\audit\diff-ir.ps1
```
记录两者的输出目录与总结。

- [ ] **步骤 2：跑 cargo clippy**

```powershell
cargo clippy --workspace -- -D warnings 2>&1 | Tee-Object -Variable clippyOut
```

- [ ] **步骤 3：创建 docs/audit/ 目录**

```powershell
New-Item -ItemType Directory -Force -Path docs/audit
```

- [ ] **步骤 4：写 findings.md（基于审计结果）**

`docs/audit/findings.md` 模板（实际内容根据审计输出填）：

```markdown
# Tangle v0.2.1 Quality Audit Findings

> Audit run: <timestamp>
> Audit harness: tests/audit/run-audit.ps1 + tests/audit/diff-ir.ps1 + cargo clippy

## Summary

- Total cells: <N>
- Failing cells (with diagnostics): <M>
- Differential tests: <X> MATCH / <Y> DIFF / <Z> SKIPPED
- Clippy warnings: <W>

## F-001: <fixture> 在 <surface>/<target>/<mode> 下误报 <code>

- Cell: <surface> / <target> / <mode> / <fixture>
- 现象: <N 条 TANGLE_* 诊断>
- 期望: 零诊断（或白名单内诊断）
- 分类: false-positive | real-bug | gap | rust-warning | doc-drift
- 根因: <具体到文件:行号>
- 修复方案: <TDD 引用对应 G* 测试>
- 影响范围: <其他受影响 cell>
- 优先级: P0 | P1 | P2 | P3

## F-002: ...
```

- [ ] **步骤 5：Commit findings.md**

```powershell
git add docs/audit/findings.md
git commit -m "docs(audit): add findings report from initial audit run"
```

---

## 任务 12：根据 findings 修复 G5/G6 发现

**文件：**
- 视 finding 内容而定
- 修改：`compiler/tangle-cli/tests/audit_regression/G5_doc_drift.rs` 和 `G6_platform_diff.rs`

**背景：** 这是 discovery-driven 任务。每条 P0/P1/P2 级 G5/G6 finding 走 TDD：写测试 → 修代码 → 验证。

- [ ] **步骤 1：对每条 G5 finding（doc-drift）**

对 findings.md 中每条 G5 finding：

a. 在 `G5_doc_drift.rs` 添加测试：

```rust
#[test]
fn g5_<short_name>() {
    let run = run_collecting_diagnostics("<fixture>");
    // 或：对 docgen 输出做断言
    let _html = tangle_cli::docgen::generate_doc_html(
        &tangle_cli::frontend::compile_module::compile_module(
            tangle_cli::frontend::compile_module::CompileModuleInput {
                file: "<fixture>".into(),
                source: std::fs::read_to_string("<fixture>").unwrap(),
            }
        ),
        "<fixture>"
    );
    // 具体断言依据 finding 内容
    assert!(/* 期望条件 */);
}
```

b. 运行验证失败：`cargo test -p tangle-cli --test G5_doc_drift g5_<short_name>`

c. 修对应代码（docgen 模块、examples 文件、或 README）

d. 运行验证通过

e. Commit: `fix(audit): G5 <短描述> (refs F-NNN)`

- [ ] **步骤 2：对每条 G6 finding（platform-diff）**

对 findings.md 中每条 G6 finding：

a. 在 `G6_platform_diff.rs` 添加测试，断言 py/go target 与 js target 诊断一致
b. 验证失败
c. 修 codegen/py_emitter.rs 或 codegen/go_emitter.rs
d. 验证通过
e. Commit: `fix(audit): G6 <短描述> (refs F-NNN)`

- [ ] **步骤 3：对每条 P3 finding（gap）**

不修，但在 findings.md 该 finding 下加注：
```
- 后续行动: 进入 v0.3.0 backlog，由对应 Phase 处理
```

- [ ] **步骤 4：更新 expected_diagnostics.yaml**

如果审计发现 `errors/payment.tangle.md` 等故意错误用例产生预期诊断，在 `expected_diagnostics.yaml` 中声明：

```yaml
payment.tangle.md:
  run/js/normal:
    - TANGLE_PANIC_REACHED  # 或实际预期代码
```

- [ ] **步骤 5：Commit**

```powershell
git add docs/audit/findings.md tests/audit/expected_diagnostics.yaml compiler/tangle-cli/tests/audit_regression/G5_doc_drift.rs compiler/tangle-cli/tests/audit_regression/G6_platform_diff.rs
git commit -m "fix(audit): G5/G6 address findings from initial audit"
```

---

## 任务 13：创建 LSP 探测器

**文件：**
- 创建：`tests/audit/lsp-probe/Cargo.toml`
- 创建：`tests/audit/lsp-probe/src/main.rs`

**背景：** LSP 表面不走 `run-audit.ps1`（其协议是 stdio JSON-RPC，不是命令行）。探测器对每个 example 跑固定 LSP 协议序列并收集 `publishDiagnostics`。

- [ ] **步骤 1：创建 lsp-probe crate**

`tests/audit/lsp-probe/Cargo.toml`：

```toml
[package]
name = "lsp-probe"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "lsp-probe"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **步骤 2：写 lsp-probe 主体**

`tests/audit/lsp-probe/src/main.rs`：

```rust
//! LSP probe: runs a fixed LSP protocol sequence against `tangle lsp`
//! for each example file and reports all publishDiagnostics notifications.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio, Child};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lsp-probe <tangle-cli-target-debug> [example.md ...]");
        std::process::exit(2);
    }
    let tangle_bin = &args[1];
    let examples: Vec<String> = if args.len() > 2 {
        args[2..].iter().cloned().collect()
    } else {
        std::fs::read_dir("examples").expect("read examples/")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
            .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
            .collect()
    };

    for example in &examples {
        eprintln!("--- Probing {} ---", example);
        let mut child = match spawn_lsp(tangle_bin) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ERROR] spawn failed: {}", e);
                continue;
            }
        };
        let stdin = child.stdin.as_mut().unwrap();
        let stdout = child.stdout.as_mut().unwrap();
        let mut reader = BufReader::new(stdout);

        // initialize
        send_msg(&mut *stdin, json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "processId": null, "rootUri": null, "capabilities": {} }
        }));
        let _init_resp = read_msg(&mut reader);

        // initialized notification
        send_msg(&mut *stdin, json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }));

        // didOpen
        let uri = format!("file://{}", std::fs::canonicalize(example).unwrap_or_default().display());
        let text = std::fs::read_to_string(example).unwrap_or_default();
        send_msg(&mut *stdin, json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": { "uri": uri, "languageId": "tangle", "version": 1, "text": text }
            }
        }));

        // Collect diagnostics for 1 second
        let mut diag_count = 0;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            if let Some(msg) = try_read_msg(&mut reader) {
                if msg.get("method").and_then(|m| m.as_str()) == Some("textDocument/publishDiagnostics") {
                    if let Some(diags) = msg.pointer("/params/diagnostics").and_then(|d| d.as_array()) {
                        diag_count += diags.len();
                        for d in diags {
                            eprintln!("  [diag] {}", d);
                        }
                    }
                }
            }
        }

        eprintln!("  -> {} diagnostics", diag_count);

        // shutdown
        send_msg(&mut *stdin, json!({ "jsonrpc": "2.0", "id": 99, "method": "shutdown" }));
        let _ = read_msg(&mut reader);
        send_msg(&mut *stdin, json!({ "jsonrpc": "2.0", "method": "exit" }));
        let _ = child.wait();
    }
}

fn spawn_lsp(tangle_bin: &str) -> std::io::Result<Child> {
    Command::new(tangle_bin)
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn send_msg(stdin: &mut dyn Write, msg: Value) {
    let s = serde_json::to_string(&msg).unwrap();
    let _ = write!(stdin, "Content-Length: {}\r\n\r\n{}", s.len(), s);
    let _ = stdin.flush();
}

fn read_msg(reader: &mut dyn BufRead) -> Option<Value> {
    let mut content_length = None;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).ok()? == 0 { return None; }
        let trimmed = line.trim_end();
        if trimmed.is_empty() { break; }
        if let Some(v) = trimmed.strip_prefix("Content-Length: ") {
            content_length = v.parse::<usize>().ok();
        }
    }
    let len = content_length?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn try_read_msg(reader: &mut BufReader<&mut std::process::ChildStdout>) -> Option<Value> {
    // Non-blocking-ish: read with very short timeout. For simplicity, blocking
    // read but with a tight deadline handled by caller.
    read_msg(reader)
}
```

- [ ] **步骤 3：编译 lsp-probe**

```powershell
cargo build --release --manifest-path tests/audit/lsp-probe/Cargo.toml
```

- [ ] **步骤 4：跑一遍验证**

```powershell
.\tests\audit\lsp-probe\target\release\lsp-probe.exe (Resolve-Path .\target\debug\tangle.exe)
```
预期：对每个 example 输出诊断数。审计后预期全为 0（account.tangle.md 在 G1/G2 修复后应干净）。

- [ ] **步骤 5：Commit**

```powershell
git add tests/audit/lsp-probe
git commit -m "feat(audit): add lsp-probe for LSP surface diagnostics collection"
```

---

## 任务 14：跑出口闸验证

**文件：** 无（验证任务）

- [ ] **步骤 1：跑出口闸**

```powershell
.\tests\audit\verify-exit-gate.ps1
```
预期：最后一行输出 `EXIT GATE: PASS`。

- [ ] **步骤 2：若失败，定位失败步**

脚本会在失败步退出并打印 `EXIT GATE: FAIL — <step>`。根据 `<step>` 定位：
- `cargo test` → 跑 `cargo test --workspace` 看哪个测试失败
- `cargo clippy` → 跑 `cargo clippy --workspace -- -D warnings` 看告警
- `run-audit.ps1` → 看 `tests/audit/output/<最新时间戳>/summary.md`
- `diff-ir.ps1` → 看哪些 fixture `[DIFF]`
- `audit_regression tests` → 跑具体测试看断言

修复后回到步骤 1。

- [ ] **步骤 3：记录通过证据**

把 `EXIT GATE: PASS` 输出截图或复制到 commit message。

- [ ] **步骤 4：Commit（如有修复）**

```powershell
git add -A
git commit -m "chore(audit): exit gate PASS — all 5 gates green"
```

---

## 任务 15：切 v0.2.1 版本

**文件：**
- 修改：`compiler/tangle-cli/Cargo.toml`
- 修改：`Cargo.lock`（自动）
- 创建：`CHANGELOG.md`
- 修改：`README.md`、`README.zh.md`

- [ ] **步骤 1：bump Cargo.toml 版本**

[compiler/tangle-cli/Cargo.toml:3](../../compiler/tangle-cli/Cargo.toml#L3) 改：

```toml
version = "0.2.1"
```

- [ ] **步骤 2：更新 Cargo.lock**

```powershell
cargo build --workspace
```
预期：`Cargo.lock` 自动更新 `tangle-cli` 版本到 0.2.1。

- [ ] **步骤 3：创建 CHANGELOG.md**

```markdown
# Changelog

## v0.2.1 — Quality Audit

- Fix: struct/type symbol resolution false positives (G1) — `resolve_types` now recurses into heading children
- Fix: method parameters not injected into method body scope (G2) — `block_env.variables` seeded from heading params
- Fix: cascade "Member access on non-struct type" errors (G3) — automatically resolved by G1
- Chore: clear all `cargo clippy` warnings (G4) — unused variable in `cli/run.rs`
- Docs: align examples with actual behavior (G5) — per audit findings
- Test: add `audit_regression` suite + audit matrix baseline + ir-diff differential harness
```

- [ ] **步骤 4：更新 README Roadmap**

[README.md](../../README.md) 中 Roadmap 区段，在 Post-B5 v0.3.0 表前插入：

```markdown
### ✅ v0.2.1 — Quality Audit — Complete

| Gate | Status |
|------|--------|
| Audit matrix (~250 cells) zero false diagnostics | ✅ |
| `cargo test --workspace` (99 + audit_regression) | ✅ |
| `cargo clippy --workspace -- -D warnings` | ✅ |
| Differential IR test against TS reference | ✅ |

See [docs/audit/findings.md](docs/audit/findings.md) for audit details.
```

[README.zh.md](../../README.zh.md) 同步翻译。

- [ ] **步骤 5：验证版本号生效**

```powershell
cargo run -- --version
```
预期：输出 `tangle 0.2.1`。

- [ ] **步骤 6：最终出口闸确认**

```powershell
.\tests\audit\verify-exit-gate.ps1
```
预期：`EXIT GATE: PASS`。

- [ ] **步骤 7：Commit 版本**

```powershell
git add compiler/tangle-cli/Cargo.toml Cargo.lock CHANGELOG.md README.md README.zh.md
git commit -m "$(Get-Content -Path <<commit-msg-file>> -Raw)"
```

commit message（写到临时文件再 `-F`）：

```
release: v0.2.1 Quality Audit

5 exit gates green:
- audit matrix: zero false diagnostics across ~250 cells
- cargo test --workspace: 99 + audit_regression tests pass
- cargo clippy -D warnings: clean
- diff-ir: MATCH/SKIPPED only
- audit_regression suite: green

Fixes: G1 (struct symbol resolution), G2 (method param scope),
G3 (member access cascade), G4 (rust warnings), G5/G6 per findings.
```

- [ ] **步骤 8：打 tag（不 push）**

```powershell
git tag -a v0.2.1 -m "v0.2.1 — Quality Audit"
```

**注意：** 不执行 `git push` 或 `git push --tags`。由用户决定何时推送。

---

## 自检

### 1. 规格覆盖度

| 规格章节 | 实现任务 |
|---------|---------|
| §2 审计范围与矩阵 | 任务 8（run-audit.ps1 实现 matrix.csv + summary.md）、任务 13（LSP 表面）、任务 10（emit-ir 单独 cell） |
| §3.1 矩阵驱动脚本 | 任务 8 |
| §3.2 LSP 探测器 | 任务 13 |
| §3.3 差分测试 harness | 任务 10 |
| §3.4 findings.md | 任务 11 |
| §4.1 性质分类 | 任务 11（findings 写入分类） |
| §4.2 根因组 G1-G6 | 任务 4 (G1)、任务 5 (G2)、任务 6 (G3)、任务 1 (G4)、任务 12 (G5/G6) |
| §4.3 优先级规则 | 任务 11（在 findings 中标注 P0-P3）、任务 12 步骤 3（P3 进 backlog） |
| §5.1 TDD 五步 | 任务 3-6、任务 12 严格遵循 |
| §5.2 测试组织 | 任务 3、5、6、7 创建 5 个 audit_regression 文件（G4 例外） |
| §5.3 测试 helper | 任务 2 创建 `run_collecting_diagnostics` + `TestRun` |
| §5.4 修复顺序 G4→G1/G2/G3→G5/G6 | 任务顺序 1 (G4) → 3-6 (G1/G2/G3) → 12 (G5/G6) 一致 |
| §5.5 YAGNI | 全程遵守：未引入新框架、未重构、未 fuzz、未加新诊断 code |
| §6.1 出口闸 5 条 | 任务 14 验证；任务 9 创建 verify-exit-gate.ps1 实现 5 条 |
| §6.2 验收脚本 | 任务 9 |
| §6.3 版本与发布 | 任务 15（v0.2.1 bump + CHANGELOG + tag，不 push） |
| §6.4 不做的事 | 任务 15 步骤 8 明确不 push；全程未引入 semver 承诺 |
| §7 工作分解与依赖 | 任务顺序与依赖图一致：G4 先、G1-G3 中、G5/G6 后、出口闸最后 |
| §8 成功标准 | 任务 14 出口闸 PASS 即覆盖所有成功标准 |

✅ 覆盖完整。

### 2. 占位符扫描

- ❌ "待定"、"TODO"、"后续实现" — 已搜索，无遗留
- ❌ "添加适当的错误处理" — 无；G5/G6 任务 12 步骤 1/2 给出具体测试代码模式
- ⚠️ 任务 12 是 discovery-driven，但步骤 1a 给出具体测试代码骨架（不是"为以上代码编写测试"占位），步骤 4 给出具体 yaml 示例。可接受。
- ⚠️ 任务 11 findings.md 模板含 `<placeholder>` 占位符——这是模板字段（运行时填入实际值），不是计划缺陷。可接受。

### 3. 类型一致性

- `TestRun` 在任务 2 定义，在任务 3、5、6、7、12 一致使用
- `run_collecting_diagnostics(file: &str) -> TestRun` 签名一致
- `Diagnostic` 类型引用一致（`crate::model::TangleDiagnostic`，包含 `code: String`、`message: String`、`span: SourceSpan`）
- G1 测试文件名 `G1_struct_symbol_resolution.rs` 在任务 3 创建、任务 9 verify-exit-gate.ps1 中引用一致
- G2 测试文件名 `G2_method_param_scope.rs`（修正命名，与 spec 预测的 `G2_let_binding_scope.rs` 不同，因为根因实际是方法参数而非 let 作用域）—— spec 允许"待审计后调整"

✅ 类型一致。

### 4. 范围检查

- 15 个任务，1-2 周可完成
- 每个 task 边界清晰、可独立 commit
- 不覆盖 v0.3.0 Phase 1-4（明确在 spec §9 声明）

✅ 范围适当。

---

## 执行交接

计划已完成并保存到 `docs/superpowers/plans/2026-07-11-quality-audit.md`。

两种执行方式：

**1. 子代理驱动（推荐）** - 每个任务调度一个新的子代理，任务间进行审查，快速迭代

**2. 内联执行** - 在当前会话中使用 executing-plans 执行任务，批量执行并设有检查点

**选哪种方式？**
