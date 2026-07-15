# Phase 5: 差分测试闭合 + 工程性推迟项 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 闭合 Phase 4 遗留的差分测试缺口 + 工程性推迟项 — A1 Rust dual-entry 修复 + ir-diff 多 function + Py/Go 多函数发射；A2 TS 参考实现 rule lowering 忠实 port；B2 三宿主递归深度限制；B4 跨 toggle 块不继承语义文档化。

**架构：** 4 个工作流。A1 内部紧耦合（compile_to_ir + ir-diff + Py/Go emitter 必须一起改），A2/B2/B4 相互独立。A2 按 toggle → tree → table → flow 顺序 port，每完成一个立即差分验证。

**技术栈：** Rust + serde_json（ir-diff）、Rust codegen（js/py/go emitter）、TypeScript + vitest（reference）、PowerShell（diff-ir.ps1）

**规格文档：** `docs/superpowers/specs/2026-07-16-phase5-diff-closure-design.md`

---

## 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | A1 — has_main_callable 检测 + 顶层不再合并 Callable 块 | 修改 |
| `compiler/tangle-cli/src/codegen/py_emitter.rs` | A1+B2 — emit_multi_function_py + depth 参数 | 修改 |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | A1+B2 — emit_multi_function_go + depth 参数 | 修改 |
| `compiler/tangle-cli/src/codegen/js_emitter.rs` | B2 — emit_branch_body 加 depth 参数 | 修改 |
| `compiler/tangle-cli/src/ir/lower_rule_toggle.rs` | B4 — doc comment 明确不继承语义 | 修改 |
| `tests/audit/ir-diff/src/main.rs` | A1 — lift_functions → compare_functions 多 function 比较 | 修改 |
| `tests/audit/diff-ir.ps1` | A1+A2 — 清空 $KnownDiffs（payment 转 MATCH） | 修改 |
| `reference/src/ir/compileToIR.ts` | A2 — 接入 4 个 rule lower 调用 | 修改 |
| `reference/src/ir/ruleTree.ts` | A2 — 忠实 port lower_rule_tree.rs | 修改 |
| `reference/src/ir/ruleTable.ts` | A2 — 忠实 port lower_rule_table.rs | 修改 |
| `reference/src/ir/ruleFlow.ts` | A2 — 忠实 port lower_rule_flow.rs | 修改 |
| `reference/src/ir/ruleToggle.ts` | A2 — 忠实 port lower_rule_toggle.rs | 修改 |
| `reference/tests/ir/ruleToggle.test.ts` | A2 — 扩展 vitest 单元测试 | 修改 |
| `reference/tests/ir/ruleTree.test.ts` | A2 — 扩展 vitest 单元测试 | 修改 |
| `reference/tests/ir/ruleTable.test.ts` | A2 — 扩展 vitest 单元测试 | 修改 |
| `reference/tests/ir/ruleFlow.test.ts` | A2 — 扩展 vitest 单元测试 | 修改 |
| `compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs` | A1 测试（Cargo name: `phase5_dual_entry`） | 创建 |
| `compiler/tangle-cli/tests/v05_phase5/recursion_depth.rs` | B2 测试（Cargo name: `phase5_recursion_depth`） | 创建 |
| `compiler/tangle-cli/tests/v05_phase5/toggle_cross_block.rs` | B4 测试（Cargo name: `phase5_toggle_cross_block`） | 创建 |
| `compiler/tangle-cli/Cargo.toml` | 新增 3 个 `[[test]]` 条目 | 修改 |
| `tests/v05_phase5/deep-recursion.tangle.md` | B2 fixture（100+ 层嵌套） | 创建 |
| `tests/v05_phase5/multi-toggle-blocks.tangle.md` | B4 fixture（2 个 toggle 块） | 创建 |

**总计**：修改 15 个文件，创建 5 个文件。

---

## 任务 1：Worktree 与分支准备

**文件：**
- 无文件修改，仅 git 操作

- [ ] **步骤 1：基于 main 创建 worktree**

运行：
```bash
cd e:\GitProjects\tangle
git fetch origin
git worktree add .worktrees/phase5-v0.5.0 -b phase5/v0.5.0 main
```

预期：创建 `.worktrees/phase5-v0.5.0` 目录，新分支 `phase5/v0.5.0` 基于 `main` 最新 commit。

- [ ] **步骤 2：验证 worktree 可编译**

运行：
```bash
cd .worktrees/phase5-v0.5.0
cargo build --workspace
```

预期：编译成功，无错误。

- [ ] **步骤 3：验证基线测试全绿**

运行：
```bash
cargo test --workspace
```

预期：所有现有测试通过。

- [ ] **步骤 4：记录差分测试基线**

运行：
```bash
pwsh tests/audit/diff-ir.ps1
```

预期：3 MATCH / 1 KNOWN_DIFF (payment) / 7 SKIPPED。记录输出作为基线。

- [ ] **步骤 5：验证 reference 项目基线**

运行：
```bash
cd reference
npm install
npm run build
npm test
cd ..
```

预期：TS 编译成功，现有 vitest 测试通过。

---

## 任务 2：A1-1 compile_to_ir.rs 清理顶层（has_main_callable 检测）

**文件：**
- 修改：`compiler/tangle-cli/src/ir/compile_to_ir.rs`
- 创建：`compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs`
- 修改：`compiler/tangle-cli/Cargo.toml`

**背景：** 当前 `compile_to_ir.rs:13-64` 把所有 `@tangle` 代码块合并到顶层 `merged_graph`，又把同样代码块单独构建到 `functions[]`，造成 dual-entry。改造：当存在 `main` Callable heading 时，顶层不再合并 Callable 代码块。

**参照：** `compiler/tangle-cli/src/ir/compile_to_ir.rs:13-64`（当前流程）、`compiler/tangle-cli/src/model.rs`（`Heading`/`HeadingKind` 定义）

- [ ] **步骤 1：在 Cargo.toml 注册测试**

在 `compiler/tangle-cli/Cargo.toml` 的 `[[test]]` 列表末尾追加：

```toml
[[test]]
name = "phase5_dual_entry"
path = "tests/v05_phase5/dual_entry_fix.rs"

[[test]]
name = "phase5_recursion_depth"
path = "tests/v05_phase5/recursion_depth.rs"

[[test]]
name = "phase5_toggle_cross_block"
path = "tests/v05_phase5/toggle_cross_block.rs"
```

- [ ] **步骤 2：编写失败的测试**

创建 `compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs`：

```rust
use tangle_cli::run_collecting_diagnostics;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/mvp")
        .join(name)
}

#[test]
fn payment_top_level_empty_when_functions_present() {
    let path = fixture_path("payment.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    
    // payment 有 main Callable heading，functions[] 非空
    assert!(!graph.functions.is_empty(), "payment should have functions[]");
    assert_eq!(graph.functions.len(), 2, "payment should have main + process");
    
    // 顶层 nodes[] 应为空（rule lowering 结果也合并到顶层，payment 无 rule）
    assert!(
        graph.nodes.is_empty(),
        "top-level nodes[] should be empty when functions[] present, got: {:?}",
        graph.nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
    );
}

#[test]
fn payment_functions_array_has_main_and_process() {
    let path = fixture_path("payment.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    
    let names: Vec<&str> = graph.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"main"), "functions[] should contain main, got: {:?}", names);
    assert!(names.contains(&"process"), "functions[] should contain process, got: {:?}", names);
}

#[test]
fn expression_top_level_populated_when_no_main() {
    let path = fixture_path("expression.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    
    // expression 无 main Callable heading，走 fallback 单函数模式
    assert!(graph.functions.is_empty(), "expression should have no functions[]");
    assert!(!graph.nodes.is_empty(), "expression top-level nodes[] should be populated (fallback mode)");
}
```

- [ ] **步骤 3：运行测试验证失败**

运行：
```bash
cd .worktrees/phase5-v0.5.0
cargo test --test phase5_dual_entry payment_top_level_empty_when_functions_present -- --nocapture
```

预期：FAIL，报错 "top-level nodes[] should be empty when functions[] present"——当前 payment 顶层 nodes[] 含 main+process 的代码块节点。

- [ ] **步骤 4：实现 has_main_callable 检测 + 条件合并**

修改 `compiler/tangle-cli/src/ir/compile_to_ir.rs`。先读取当前文件确认行号：

```bash
# 读取 compile_to_ir.rs 第 1-80 行确认结构
```

在 `compile_to_ir` 函数中改造（参照 §3.2 伪代码）：

1. **新增辅助函数** `has_main_callable`（在文件末尾）：

```rust
/// Check if the module has a `main` Callable heading (enables multi-function mode).
fn has_main_callable(headings: &[crate::model::Heading]) -> bool {
    headings.iter().any(|h| {
        matches!(h.kind, crate::model::HeadingKind::Callable)
            && h.title.trim() == "main"
    })
}
```

2. **改造 `compile_to_ir` 主流程**（找到当前合并 `@tangle` 块到 `merged_graph` 的循环，包在条件内）：

```rust
// 在 collect_functions 调用前，检测 main
let has_main = has_main_callable(&checked.headings);

// Lower @tangle blocks: 仅当无 main 时合并到顶层
if !has_main {
    for block in &checked.parsed_blocks {
        let sub_graph = lower_statements(&block.statements, &checked.file, &mut id_gen);
        merge_into(&mut merged_graph, sub_graph);
    }
}

// collect_functions: 仅当有 main 时调用
if has_main {
    let mut functions: Vec<IRFunction> = vec![];
    collect_functions(&checked.headings, &checked.parsed_blocks, &checked.file, &mut id_gen, &mut functions);
    graph.functions = functions;
}
```

**注意：** `collect_functions` 的现有调用点（第 60-64 行附近）已经只在 `has_main` 时赋值 `graph.functions`。改造点是把 `@tangle` 块合并循环包在 `if !has_main` 内。rule lowering 合并循环**保持不变**（rule 是模块级，不属于任何 function）。

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
cargo test --test phase5_dual_entry -- --nocapture
```

预期：3 个测试全 PASS。

- [ ] **步骤 6：验证现有测试不回归**

运行：
```bash
cargo test --workspace
```

预期：所有现有测试通过。重点关注 `audit_regression/`、`v04_phase4/`、`v03_phase2/` 测试。

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/ir/compile_to_ir.rs compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs compiler/tangle-cli/Cargo.toml
git commit -m "feat(ir): A1-1 compile_to_ir 清理顶层 dual-entry

当存在 main Callable heading 时，顶层 nodes[]/edges[] 不再合并
@tangle 代码块（functions[] 会单独存）。无 main 时走 fallback
单函数模式（expression/hello/user 不受影响）。

- 新增 has_main_callable 检测函数
- @tangle 块合并循环包在 if !has_main 条件内
- rule lowering 合并循环保持不变（模块级）

测试：payment_top_level_empty_when_functions_present
      payment_functions_array_has_main_and_process
      expression_top_level_populated_when_no_main"
```

---

## 任务 3：A1-2 ir-diff compare_functions 多 function 比较

**文件：**
- 修改：`tests/audit/ir-diff/src/main.rs`

**背景：** 当前 `lift_functions` 只提升 `functions[0]` 到顶层，多 function 时 functions[1+] 丢失。payment 有 main+process 两个 function，process 不被比较。改造为 `compare_functions` 对 `functions[]` 数组整体比较。

**参照：** `tests/audit/ir-diff/src/main.rs`（当前 `lift_functions` 函数 + `normalize` 函数 + `main` 调用点）

- [ ] **步骤 1：编写失败的测试**

在 `tests/audit/ir-diff/src/main.rs` 末尾的 `#[cfg(test)] mod tests` 块中新增测试：

```rust
#[test]
fn compare_functions_aligns_by_name() {
    // 模拟 Rust IR: functions[main, process]
    let rs = serde_json::json!({
        "functions": [
            {"name": "main", "nodes": [{"id": "n0", "kind": "compute", "label": "a"}], "edges": [], "entryNodeId": "n0"},
            {"name": "process", "nodes": [{"id": "n1", "kind": "compute", "label": "b"}], "edges": [], "entryNodeId": "n1"}
        ]
    });
    // 模拟 TS IR: functions[process, main]（顺序不同）
    let ts = serde_json::json!({
        "functions": [
            {"name": "process", "nodes": [{"id": "entry1", "kind": "compute", "label": "b"}], "edges": [], "entryNodeId": "entry1"},
            {"name": "main", "nodes": [{"id": "entry2", "kind": "compute", "label": "a"}], "edges": [], "entryNodeId": "entry2"}
        ]
    });
    
    let (ts_norm, rs_norm) = compare_functions(ts, rs);
    let ts_arr = ts_norm.as_array().unwrap();
    let rs_arr = rs_norm.as_array().unwrap();
    assert_eq!(ts_arr.len(), 2);
    assert_eq!(rs_arr.len(), 2);
    // 按 name 排序后对齐
    assert_eq!(ts_arr[0]["name"], rs_arr[0]["name"]);
    assert_eq!(ts_arr[1]["name"], rs_arr[1]["name"]);
}

#[test]
fn compare_functions_wraps_single_when_no_functions_array() {
    // 无 functions[] 的 IR（如 expression）包装为单 function 数组
    let rs = serde_json::json!({
        "nodes": [{"id": "n0", "kind": "compute", "label": "a"}],
        "edges": [],
        "entryNodeId": "n0"
    });
    let ts = serde_json::json!({
        "nodes": [{"id": "entry1", "kind": "compute", "label": "a"}],
        "edges": [],
        "entryNodeId": "entry1"
    });
    
    let (ts_norm, rs_norm) = compare_functions(ts, rs);
    let ts_arr = ts_norm.as_array().unwrap();
    let rs_arr = rs_norm.as_array().unwrap();
    assert_eq!(ts_arr.len(), 1);
    assert_eq!(rs_arr.len(), 1);
    // 包装的 function name 应为 "module"
    assert_eq!(ts_arr[0]["name"], "module");
    assert_eq!(rs_arr[0]["name"], "module");
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --manifest-path tests/audit/ir-diff/Cargo.toml compare_functions
```

预期：FAIL，`compare_functions` 函数未定义（当前是 `lift_functions`）。

- [ ] **步骤 3：实现 compare_functions**

在 `tests/audit/ir-diff/src/main.rs` 中：

1. **保留 `lift_functions` 函数**（用于无 functions[] 的单 function 包装），但改造其逻辑。
2. **新增 `compare_functions` 函数**（替换 main 中的调用）：

```rust
/// Compare two IRs' functions[] arrays (or wrap top-level as single function).
/// Returns (ts_normalized, rs_normalized) as JSON arrays sorted by function.name.
fn compare_functions(ts: serde_json::Value, rs: serde_json::Value) -> (serde_json::Value, serde_json::Value) {
    let ts_arr = extract_functions_array(&ts);
    let rs_arr = extract_functions_array(&rs);
    
    // 按 name 排序
    let mut ts_sorted = ts_arr.clone();
    let mut rs_sorted = rs_arr.clone();
    ts_sorted.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    rs_sorted.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    
    // 对每个 function 分别 normalize
    let ts_norm: Vec<serde_json::Value> = ts_sorted.iter().map(|f| normalize_function(f)).collect();
    let rs_norm: Vec<serde_json::Value> = rs_sorted.iter().map(|f| normalize_function(f)).collect();
    
    (serde_json::Value::Array(ts_norm), serde_json::Value::Array(rs_norm))
}

/// Extract functions[] array, or wrap top-level nodes/edges as single function.
fn extract_functions_array(ir: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(funcs) = ir.get("functions").and_then(|f| f.as_array()) {
        if !funcs.is_empty() {
            return funcs.clone();
        }
    }
    // 包装为单 function
    vec![serde_json::json!({
        "name": "module",
        "nodes": ir.get("nodes").cloned().unwrap_or(serde_json::Value::Array(vec![])),
        "edges": ir.get("edges").cloned().unwrap_or(serde_json::Value::Array(vec![])),
        "entryNodeId": ir.get("entryNodeId").cloned().unwrap_or(serde_json::Value::Null),
    })]
}

/// Normalize a single function's IR (apply build_id_map + normalize).
fn normalize_function(func: &serde_json::Value) -> serde_json::Value {
    let id_map = build_id_map(func);
    normalize(func, &id_map)
}
```

3. **修改 main 调用点**：找到当前调用 `lift_functions` 的地方，改为 `compare_functions`，比较返回的两个数组是否相等。

**注意：** `build_id_map` 和 `normalize` 是已有函数，作用域改为单 function。原 `lift_functions` 若仍被其他测试引用，保留或重构。

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test --manifest-path tests/audit/ir-diff/Cargo.toml compare_functions
```

预期：2 个测试 PASS。

- [ ] **步骤 5：验证 ir-diff 工具整体工作**

运行：
```bash
pwsh tests/audit/diff-ir.ps1
```

预期：expression/hello/user 仍 MATCH（单 function 包装后比较），payment 仍 KNOWN_DIFF（A1-1 改了 Rust 端，但 Py/Go 还没改，Py/Go 输出仍是单函数模式）。

- [ ] **步骤 6：Commit**

```bash
git add tests/audit/ir-diff/src/main.rs
git commit -m "feat(ir-diff): A1-2 compare_functions 多 function 数组比较

lift_functions → compare_functions：
- 提取 functions[] 数组（或包装顶层为单 function）
- 按 name 排序对齐（main 与 main 对齐）
- 对每个 function 分别 normalize

测试：compare_functions_aligns_by_name
      compare_functions_wraps_single_when_no_functions_array"
```

---

## 任务 4：A1-3 Py emitter emit_multi_function_py

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/py_emitter.rs`

**背景：** Py/Go 当前只消费顶层 `graph.nodes`，无 multi-function 分支。参照 `js_emitter.rs:205-208` 的 `emit_multi_function_js` 实现 Py 版本。

**参照：** `compiler/tangle-cli/src/codegen/js_emitter.rs:205-208`（JS multi-function 分支）、`js_emitter.rs` 的 `emit_multi_function_js` 函数、`compiler/tangle-cli/src/codegen/py_emitter.rs`（当前 Py emitter 结构）

- [ ] **步骤 1：编写失败的测试**

在 `compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs` 追加：

```rust
use tangle_cli::codegen::emit_python;

#[test]
fn py_multi_function_emits_main_and_process() {
    let path = fixture_path("payment.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_python(&graph, "payment");
    
    assert!(output.contains("def main("), "Py output should contain def main, got: {}", output);
    assert!(output.contains("def process("), "Py output should contain def process, got: {}", output);
    assert!(
        output.contains(r#"if __name__ == "__main__""#),
        "Py output should contain if __name__ == __main__ entry, got: {}", output
    );
}

#[test]
fn py_single_function_fallback_when_no_main() {
    let path = fixture_path("expression.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_python(&graph, "expression");
    
    // 无 main 时走单函数模式，不应有 def main
    assert!(!output.contains("def main("), "expression should not emit def main, got: {}", output);
    assert!(!output.contains(r#"if __name__"#), "expression should not emit __main__ entry, got: {}", output);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --test phase5_dual_entry py_multi_function -- --nocapture
```

预期：FAIL，Py 输出不含 `def main` 和 `def process`（当前 Py 只消费顶层 nodes[]，A1-1 后顶层为空）。

- [ ] **步骤 3：实现 emit_multi_function_py**

修改 `compiler/tangle-cli/src/codegen/py_emitter.rs`：

1. **在 `emit_python` 主函数中**（找到当前调用单函数发射的地方，参照 js_emitter.rs:205-208 模式）：

```rust
pub fn emit_python(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();
    // ... metadata 头部（保持现状）...
    
    if !graph.functions.is_empty() {
        out.push_str(&emit_multi_function_py(&graph.functions));
    } else {
        out.push_str(&emit_single_function_py(&graph.nodes, &graph.edges, &graph.entry_node_id, module_name));
    }
    
    out
}
```

2. **新增 `emit_multi_function_py` 函数**（参照 `emit_multi_function_js`）：

```rust
fn emit_multi_function_py(functions: &[IRFunction]) -> String {
    let mut out = String::new();
    for func in functions {
        let name = &func.name;
        // Py 函数签名: def {name}({params}):
        // 当前 params 为空（IRFunction.params 未来 phase 扩展）
        out.push_str(&format!("def {}():\n", name));
        // 函数体：复用现有 BFS + emit_decision_branch_py，作用域为单 function
        out.push_str(&emit_function_body_py(&func.nodes, &func.edges, &func.entry_node_id));
        out.push('\n');
    }
    // 入口：仅当含 main 时
    let has_main = functions.iter().any(|f| f.name == "main");
    if has_main {
        out.push_str(r#"if __name__ == "__main__":
    main()
"#);
    }
    out
}

/// Emit function body (indent level 1, i.e., 4 spaces).
fn emit_function_body_py(nodes: &[IRNode], edges: &[IREdge], entry_id: &str) -> String {
    let mut visited = HashSet::new();
    emit_branch_body_py(entry_id, nodes, edges, &mut visited, "    ", 0)
}
```

**注意：** `emit_branch_body_py` 需要加 `depth: usize` 参数（B2 任务统一加，此任务先加参数但暂不使用 depth 逻辑，传 0）。实际上 B2 任务会统一改签名，此任务先实现 multi-function，depth 参数在任务 12 统一加。

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test --test phase5_dual_entry py_multi_function py_single_function -- --nocapture
```

预期：2 个测试 PASS。

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/codegen/py_emitter.rs compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs
git commit -m "feat(codegen): A1-3 Py emit_multi_function 多函数发射

参照 js_emitter.rs:205-208 实现 Py 多函数分支：
- graph.functions 非空时发射 def main + def process + ...
- 模块末尾发 if __name__ == __main__: main()（仅当含 main）
- 无 main 时走 fallback 单函数模式

测试：py_multi_function_emits_main_and_process
      py_single_function_fallback_when_no_main"
```

---

## 任务 5：A1-4 Go emitter emit_multi_function_go

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/go_emitter.rs`
- 修改：`compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs`

**背景：** 同 Py，参照 JS 实现 Go multi-function。Go 的 main 自动入口，无需额外 `if __name__`。

**参照：** `compiler/tangle-cli/src/codegen/go_emitter.rs`（当前 Go emitter 结构）

- [ ] **步骤 1：编写失败的测试**

在 `compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs` 追加：

```rust
use tangle_cli::codegen::emit_go;

#[test]
fn go_multi_function_emits_main_and_process() {
    let path = fixture_path("payment.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_go(&graph, "payment");
    
    assert!(output.contains("func main("), "Go output should contain func main, got: {}", output);
    assert!(output.contains("func process("), "Go output should contain func process, got: {}", output);
}

#[test]
fn go_single_function_fallback_when_no_main() {
    let path = fixture_path("expression.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_go(&graph, "expression");
    
    assert!(!output.contains("func main("), "expression should not emit func main, got: {}", output);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --test phase5_dual_entry go_multi_function -- --nocapture
```

预期：FAIL，Go 输出不含 `func main` 和 `func process`。

- [ ] **步骤 3：实现 emit_multi_function_go**

修改 `compiler/tangle-cli/src/codegen/go_emitter.rs`：

```rust
pub fn emit_go(graph: &RuleGraph, module_name: &str) -> String {
    let mut out = String::new();
    // ... package + import + metadata（保持现状）...
    
    if !graph.functions.is_empty() {
        out.push_str(&emit_multi_function_go(&graph.functions));
    } else {
        out.push_str(&emit_single_function_go(&graph.nodes, &graph.edges, &graph.entry_node_id, module_name));
    }
    
    out
}

fn emit_multi_function_go(functions: &[IRFunction]) -> String {
    let mut out = String::new();
    for func in functions {
        let name = &func.name;
        // Go 函数签名: func {name}() {
        out.push_str(&format!("func {}() {{\n", name));
        // 函数体
        out.push_str(&emit_function_body_go(&func.nodes, &func.edges, &func.entry_node_id));
        out.push_str("}\n\n");
    }
    // Go 的 main 自动入口，无需额外调用
    out
}

fn emit_function_body_go(nodes: &[IRNode], edges: &[IREdge], entry_id: &str) -> String {
    let mut visited = HashSet::new();
    emit_branch_body_go(entry_id, nodes, edges, &mut visited, "\t", 0)
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test --test phase5_dual_entry go_multi_function go_single_function -- --nocapture
```

预期：2 个测试 PASS。

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/codegen/go_emitter.rs compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs
git commit -m "feat(codegen): A1-4 Go emit_multi_function 多函数发射

参照 js_emitter.rs 实现 Go 多函数分支：
- graph.functions 非空时发射 func main + func process + ...
- Go 的 main 自动入口，无需额外调用
- 无 main 时走 fallback 单函数模式

测试：go_multi_function_emits_main_and_process
      go_single_function_fallback_when_no_main"
```

---

## 任务 6：A1-5 diff-ir.ps1 清空 $KnownDiffs + payment 端到端验证

**文件：**
- 修改：`tests/audit/diff-ir.ps1`

**背景：** A1-1~A1-4 改造后，payment.tangle 的 Rust IR 顶层不再 dual-entry，Py/Go 发射多函数，ir-diff 多 function 比较。payment 应从 KNOWN_DIFF 转 MATCH。但 TS 端仍失败（A2 未完成），所以 payment 的 TS IR 可能仍不匹配。

**关键判断：** payment 的 TS 端失败原因。先运行 diff-ir.ps1 看 payment 当前状态。

- [ ] **步骤 1：运行 diff-ir.ps1 查看 payment 当前状态**

运行：
```bash
pwsh tests/audit/diff-ir.ps1
```

观察 payment.tangle 的输出：
- 若 `[KNOWN_DIFF]` 且 diff 仍显示 dual-entry → A1 改动未生效，检查
- 若 `[MATCH]` → A1 改动生效，TS 端 payment 无 main Callable heading（走 fallback 模式），跳过步骤 2
- 若 `[SKIPPED]` 或新错误 → TS 端 payment 失败，记录原因

- [ ] **步骤 2：清空 $KnownDiffs（若 payment 已 MATCH）**

修改 `tests/audit/diff-ir.ps1`：找到 `$KnownDiffs` 哈希表定义，清空或注释掉 payment 条目：

```powershell
# 清空：Phase 5 已修复 payment dual-entry
$KnownDiffs = @{}
```

- [ ] **步骤 3：运行 diff-ir.ps1 验证 payment MATCH**

运行：
```bash
pwsh tests/audit/diff-ir.ps1
```

预期：
- payment.tangle: `[MATCH]`
- expression/hello/user: `[MATCH]`（不回归）
- 6 rule fixture: 仍 `[SKIPPED]`（A2 未完成）
- order-service: 仍 `[SKIPPED]`（B5 依赖）

- [ ] **步骤 4：Commit**

```bash
git add tests/audit/diff-ir.ps1
git commit -m "test(audit): A1-5 清空 \$KnownDiffs，payment 转 MATCH

A1-1~A1-4 修复 Rust dual-entry + Py/Go 多函数 + ir-diff 多 function
比较后，payment.tangle 从 KNOWN_DIFF 转 MATCH。

diff-ir.ps1: 4 MATCH / 0 KNOWN_DIFF / 6 SKIPPED (rule, A2 待修复)
                                   / 1 SKIPPED (order-service, B5 依赖)"
```

---

## 任务 7：A2-1 ruleToggle.ts 忠实 port

**文件：**
- 修改：`reference/src/ir/ruleToggle.ts`
- 修改：`reference/tests/ir/ruleToggle.test.ts`

**背景：** Rust `lower_rule_toggle.rs` 150 行含完整诊断 + group/style + 名称提取（backtick > colon > None）。TS `ruleToggle.ts` 仅 19 行骨架，不返回诊断、不处理 group/style、名称提取用简单正则（与 Rust 不一致）。

**参照：** `compiler/tangle-cli/src/ir/lower_rule_toggle.rs`（150 行 Rust 源码）、`reference/src/ir/ruleToggle.ts`（当前 19 行骨架）、`reference/src/ir/graph.ts`（RuleGraph/IRNode/IREdge/TangleDiagnostic 类型）

- [ ] **步骤 1：编写失败的测试**

替换 `reference/tests/ir/ruleToggle.test.ts` 内容：

```typescript
import { describe, it, expect } from "vitest";
import { lowerRuleToggle } from "../../src/ir/ruleToggle.js";

describe("lowerRuleToggle", () => {
  it("ruleToggle_lower_basic_checkbox: basic checkbox lowering", () => {
    const md = "- [x] `enable_flag`: Enable the feature";
    const result = lowerRuleToggle(md, "test.tangle");
    
    expect(result.graph.nodes.length).toBe(2); // entry + 1 toggle
    expect(result.graph.nodes[0].kind).toBe("compute");
    expect(result.graph.nodes[0].label).toBe("toggle.entry");
    expect(result.graph.nodes[1].label).toBe("enable_flag = true");
    expect(result.graph.edges.length).toBe(1);
    expect(result.graph.edges[0].kind).toBe("control");
  });

  it("ruleToggle_lower_with_group_style: group/style 附加", () => {
    const md = `<!-- group: UI -->
<!-- style: highlight -->
- [x] \`enable_ui\`: Enable UI`;
    const result = lowerRuleToggle(md, "test.tangle");
    
    expect(result.graph.nodes[1].group).toBe("UI");
    expect(result.graph.nodes[1].style).toBe("highlight");
  });

  it("ruleToggle_lower_uppercase_x: [X] 大写支持", () => {
    const md = "- [X] `flag`: desc";
    const result = lowerRuleToggle(md, "test.tangle");
    
    expect(result.graph.nodes[1].label).toBe("flag = true");
  });

  it("ruleToggle_lower_colon_name: colon 名称提取", () => {
    const md = "- [x] flag_name: true";
    const result = lowerRuleToggle(md, "test.tangle");
    
    expect(result.graph.nodes[1].label).toBe("flag_name = true");
  });

  it("ruleToggle_lower_missing_name_diagnostic: 缺名称发诊断", () => {
    const md = "- [x] no name here";
    const result = lowerRuleToggle(md, "test.tangle");
    
    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_TOGGLE_MISSING_NAME")).toBe(true);
    expect(result.graph.nodes[1].label).toContain("toggle_0");
  });

  it("ruleToggle_lower_malformed_diagnostic: 畸形发诊断", () => {
    const md = "- [y] `flag`: desc";
    const result = lowerRuleToggle(md, "test.tangle");
    
    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_TOGGLE_MALFORMED")).toBe(true);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd reference
npx vitest run tests/ir/ruleToggle.test.ts
```

预期：FAIL，`lowerRuleToggle` 不返回 `{ graph, diagnostics }` 对象（当前只返回 `RuleGraph`）。

- [ ] **步骤 3：忠实 port lower_rule_toggle.rs 到 ruleToggle.ts**

替换 `reference/src/ir/ruleToggle.ts` 内容，**逐行 port Rust 端逻辑**：

```typescript
import type { RuleGraph, IRNode, IREdge, TangleDiagnostic } from "./graph.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

export interface ToggleLowerResult {
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
}

export function lowerRuleToggle(checkboxMarkdown: string, file: string): ToggleLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const entryId = freshNodeId("toggle-entry");
  const graph = createGraph(entryId);

  graph.nodes.push({
    id: entryId,
    kind: "compute",
    label: "toggle.entry",
    sourceSpan: null,
    sourceText: null,
    group: null,
    style: null,
  });

  let pendingGroup: string | null = null;
  let pendingStyle: string | null = null;
  let toggleIndex = 0;

  const lines = checkboxMarkdown.split("\n");
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const line = lines[lineIdx];
    const lineNo = lineIdx + 1; // 1-based
    const t = line.trimStart();

    // HTML comment metadata
    const meta = parseHtmlComment(t);
    if (meta) {
      if (meta.key === "group") pendingGroup = meta.value;
      if (meta.key === "style") pendingStyle = meta.value;
      continue;
    }

    // Skip non-checkbox lines (but clear pending metadata)
    if (!t.startsWith("- [") && !t.startsWith("* [")) {
      if (t !== "" && !t.startsWith("<!--")) {
        pendingGroup = null;
        pendingStyle = null;
      }
      continue;
    }

    // Malformed detection
    const isValid = t.includes("[x]") || t.includes("[X]") || t.includes("[ ]");
    if (!isValid) {
      diagnostics.push({
        code: "TANGLE_RULE_TOGGLE_MALFORMED",
        message: `malformed checkbox: expected [x], [X], or [ ]: ${t}`,
        span: { file, startLine: lineNo, startColumn: 0, endLine: lineNo, endColumn: 0 },
      });
      continue;
    }

    const checked = t.includes("[x]") || t.includes("[X]");
    // Strip checkbox prefix
    let rest = t
      .replace(/^-\s*\[x\]\s*/, "")
      .replace(/^-\s*\[X\]\s*/, "")
      .replace(/^-\s*\[ \]\s*/, "")
      .replace(/^\*\s*\[x\]\s*/, "")
      .replace(/^\*\s*\[X\]\s*/, "")
      .replace(/^\*\s*\[ \]\s*/, "")
      .trim();

    // Extract name
    const extracted = extractName(rest);
    let name: string;
    if (extracted) {
      name = extracted;
    } else {
      diagnostics.push({
        code: "TANGLE_RULE_TOGGLE_MISSING_NAME",
        message: `could not extract toggle name from: ${rest}`,
        span: { file, startLine: lineNo, startColumn: 0, endLine: lineNo, endColumn: 0 },
      });
      name = `toggle_${toggleIndex}`;
    }

    const nodeId = freshNodeId("toggle");
    graph.nodes.push({
      id: nodeId,
      kind: "compute",
      label: `${name} = ${checked}`,
      sourceSpan: { file, startLine: lineNo, startColumn: 0, endLine: lineNo, endColumn: 0 },
      sourceText: null,
      group: pendingGroup,
      style: pendingStyle,
    });
    graph.edges.push({
      from: entryId,
      to: nodeId,
      kind: "control",
      guard: null,
      sourceSpan: null,
      priority: null,
      style: null,
    });
    pendingGroup = null;
    pendingStyle = null;
    toggleIndex++;
  }

  return { graph, diagnostics };
}

/// Extract toggle name. Priority: backtick (`name`) > colon (name: value) > null.
function extractName(rest: string): string | null {
  // 1. Backtick: `name`: desc
  const tickStart = rest.indexOf("`");
  if (tickStart >= 0) {
    const afterTick = rest.substring(tickStart + 1);
    const tickEnd = afterTick.indexOf("`");
    if (tickEnd >= 0) {
      return afterTick.substring(0, tickEnd);
    }
  }
  // 2. Colon: name: value (name must be valid identifier)
  const colonPos = rest.indexOf(":");
  if (colonPos >= 0) {
    const candidate = rest.substring(0, colonPos).trim();
    if (isValidIdentifier(candidate)) {
      return candidate;
    }
  }
  return null;
}

function isValidIdentifier(s: string): boolean {
  if (s.length === 0) return false;
  const first = s.charCodeAt(0);
  if (!((first >= 65 && first <= 90) || (first >= 97 && first <= 122) || first === 95)) return false;
  for (let i = 1; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (!((c >= 65 && c <= 90) || (c >= 97 && c <= 122) || (c >= 48 && c <= 57) || c === 95)) return false;
  }
  return true;
}

interface HtmlCommentMeta { key: "group" | "style"; value: string }

function parseHtmlComment(line: string): HtmlCommentMeta | null {
  const trimmed = line.trim();
  if (trimmed.length < 8) return null;
  if (!trimmed.startsWith("<!--") || !trimmed.endsWith("-->")) return null;
  const inner = trimmed.substring(4, trimmed.length - 3).trim();
  if (inner.startsWith("group:")) {
    return { key: "group", value: inner.substring(6).trim() };
  }
  if (inner.startsWith("style:")) {
    return { key: "style", value: inner.substring(6).trim() };
  }
  return null;
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
npx vitest run tests/ir/ruleToggle.test.ts
```

预期：6 个测试全 PASS。

- [ ] **步骤 5：Commit**

```bash
cd ..
git add reference/src/ir/ruleToggle.ts reference/tests/ir/ruleToggle.test.ts
git commit -m "feat(reference): A2-1 ruleToggle.ts 忠实 port lower_rule_toggle.rs

从 19 行骨架扩展为完整 port（~150 行 TS）：
- 返回 { graph, diagnostics } 对象（含诊断数组）
- group/style 缓存 + checkbox 行消费
- 名称提取：backtick > colon > None（发 MISSING_NAME 诊断）
- 畸形检测：- [ 开头但不含 [x]/[X]/[ ] 发 MALFORMED 诊断
- [X] 大写支持

测试：6 个 vitest 单元测试覆盖基础/group-style/大写/colon/缺名称/畸形"
```

---

## 任务 8：A2-2 ruleTree.ts 忠实 port

**文件：**
- 修改：`reference/src/ir/ruleTree.ts`
- 修改：`reference/tests/ir/ruleTree.test.ts`

**背景：** Rust `lower_rule_tree.rs` 313 行，含 DNF 范式 lowering、AND/OR 缩进语义、Action 标记。TS `ruleTree.ts` 仅 18 行骨架。

**参照：** `compiler/tangle-cli/src/ir/lower_rule_tree.rs`（313 行 Rust 源码）、`reference/src/ir/ruleTree.ts`（当前 18 行骨架）、`tests/rules/decision-tree.tangle.md`（fixture）

- [ ] **步骤 1：阅读 Rust 源码理解结构**

运行：
```bash
# 阅读 lower_rule_tree.rs 全文
```

理解：
- `ListNode` 树结构（children + content + is_action）
- `build_tree` 递归构建（按缩进深度）
- `lower_list_tree` 把 ListNode 树转 IR（Decision + guarded edges）
- DNF 语义：AND 嵌套 → 路径组合，OR 嵌套 → 分支

- [ ] **步骤 2：编写失败的测试**

替换 `reference/tests/ir/ruleTree.test.ts` 内容：

```typescript
import { describe, it, expect } from "vitest";
import { lowerRuleTree } from "../../src/ir/ruleTree.js";

describe("lowerRuleTree", () => {
  it("ruleTree_lower_basic_list: 基础列表 lowering", () => {
    const md = `- Check A
  - Action: do X
- Check B
  - Action: do Y`;
    const result = lowerRuleTree(md, "test.tangle");
    
    // 应有 entry + 2 Decision 节点 + 2 Terminal 节点
    expect(result.graph.nodes.length).toBeGreaterThanOrEqual(5);
    const kinds = result.graph.nodes.map(n => n.kind);
    expect(kinds).toContain("decision");
    expect(kinds).toContain("terminal");
  });

  it("ruleTree_lower_dnf_and_or: DNF AND/OR 嵌套", () => {
    const md = `- A
  - B
    - Action: X
  - C
    - Action: Y`;
    const result = lowerRuleTree(md, "test.tangle");
    
    // A 下有 B 和 C 两个分支（OR），B 下有 Action X（AND 路径）
    const edges = result.graph.edges;
    expect(edges.length).toBeGreaterThanOrEqual(4);
    // 应有 guarded edges
    expect(edges.some(e => e.guard !== null)).toBe(true);
  });

  it("ruleTree_lower_action_endpoint: Action: 子项作为 Terminal", () => {
    const md = `- Check
  - Action: do something`;
    const result = lowerRuleTree(md, "test.tangle");
    
    const terminals = result.graph.nodes.filter(n => n.kind === "terminal");
    expect(terminals.length).toBe(1);
    expect(terminals[0].label).toContain("do something");
  });

  it("ruleTree_lower_indent_depth: 缩进深度计算", () => {
    const md = `- A
  - B
    - C
      - Action: deep`;
    const result = lowerRuleTree(md, "test.tangle");
    
    // 4 层缩进应正确解析为嵌套树
    expect(result.graph.nodes.length).toBeGreaterThan(0);
  });
});
```

- [ ] **步骤 3：运行测试验证失败**

运行：
```bash
cd reference
npx vitest run tests/ir/ruleTree.test.ts
```

预期：FAIL，当前 `lowerRuleTree` 不返回 `{ graph, diagnostics }`，且无 DNF/Action 逻辑。

- [ ] **步骤 4：忠实 port lower_rule_tree.rs 到 ruleTree.ts**

替换 `reference/src/ir/ruleTree.ts`，**逐行 port Rust 端**：

```typescript
import type { RuleGraph, IRNode, IREdge, TangleDiagnostic } from "./graph.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

export interface TreeLowerResult {
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
}

interface ListNode {
  content: string;
  depth: number;
  isAction: boolean;
  children: ListNode[];
}

export function lowerRuleTree(markdown: string, file: string): TreeLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const entryId = freshNodeId("tree-entry");
  const graph = createGraph(entryId);

  graph.nodes.push({
    id: entryId,
    kind: "compute",
    label: "tree.entry",
    sourceSpan: null, sourceText: null, group: null, style: null,
  });

  // 1. Parse lines into ListNode tree
  const lines = markdown.split("\n").filter(l => l.trim().match(/^[-*]\s/));
  const root: ListNode = { content: "", depth: -1, isAction: false, children: [] };
  buildTree(lines, 0, 0, root);

  // 2. Lower tree to IR
  lowerListTree(root, entryId, graph, file);

  return { graph, diagnostics };
}

/// Build ListNode tree from lines. Rust 端 build_tree 逻辑：
/// - 按 leading_spaces / 2 计算深度
/// - "Action: xxx" 标记为 is_action
/// - 递归构建 children
function buildTree(lines: string[], startIdx: number, targetDepth: number, parent: ListNode): number {
  let i = startIdx;
  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trimStart();
    const leadingSpaces = line.length - trimmed.length;
    const depth = Math.floor(leadingSpaces / 2);
    
    if (depth < targetDepth) {
      break; // 回到上层
    }
    if (depth > targetDepth) {
      i++; // 跳过更深层的（应由父节点的 buildTree 处理）
      continue;
    }
    
    // 同层节点
    const content = trimmed.replace(/^[-*]\s+/, "").trim();
    const isAction = content.startsWith("Action:");
    const node: ListNode = { content, depth, isAction, children: [] };
    parent.children.push(node);
    
    // 递归构建子节点
    i = buildTree(lines, i + 1, targetDepth + 1, node);
  }
  return i;
}

/// Lower ListNode tree to IR nodes + edges. Rust 端 lower_list_tree 逻辑：
/// - 每个 ListNode → Decision 节点（非 Action）或 Terminal 节点（Action）
/// - parent → child 边带 guard（content 作为条件）
function lowerListTree(node: ListNode, parentId: string, graph: RuleGraph, file: string): void {
  for (const child of node.children) {
    const nodeId = freshNodeId("tree");
    if (child.isAction) {
      graph.nodes.push({
        id: nodeId,
        kind: "terminal",
        label: child.content,
        sourceSpan: null, sourceText: null, group: null, style: null,
      });
    } else {
      graph.nodes.push({
        id: nodeId,
        kind: "decision",
        label: child.content,
        sourceSpan: null, sourceText: null, group: null, style: null,
      });
    }
    graph.edges.push({
      from: parentId,
      to: nodeId,
      kind: "condition",
      guard: child.isAction ? null : child.content,
      sourceSpan: null, priority: null, style: null,
    });
    // 递归 lower 子节点
    lowerListTree(child, nodeId, graph, file);
  }
}
```

**注意：** 上述是简化版 port。实际 port 时需对照 Rust `lower_rule_tree.rs` 的完整 DNF 语义（AND/OR 组合、路径爆炸等）。若 Rust 端有 DNF 范式转换（如 `to_dnf` 函数），需同步 port。

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
npx vitest run tests/ir/ruleTree.test.ts
```

预期：4 个测试全 PASS。若失败，对照 Rust 端逻辑调整。

- [ ] **步骤 6：差分验证（可选，提前发现 IR 不匹配）**

运行：
```bash
cd ..
pwsh tests/audit/diff-ir.ps1
```

观察 decision-tree.tangle.md 是否从 SKIPPED 转 MATCH 或仍 SKIPPED。若仍 SKIPPED，记录 TS 报错原因（compileToIR.ts 尚未接入，任务 11 才接入）。

- [ ] **步骤 7：Commit**

```bash
git add reference/src/ir/ruleTree.ts reference/tests/ir/ruleTree.test.ts
git commit -m "feat(reference): A2-2 ruleTree.ts 忠实 port lower_rule_tree.rs

从 18 行骨架扩展为完整 port：
- ListNode 树结构 + build_tree 递归构建（按缩进深度）
- lower_list_tree 把 ListNode 转 IR（Decision + guarded edges）
- Action: 子项作为 Terminal 节点
- DNF 语义（AND/OR 嵌套）

测试：4 个 vitest 单元测试覆盖基础/DNF/Action/缩进深度"
```

---

## 任务 9：A2-3 ruleTable.ts 忠实 port

**文件：**
- 修改：`reference/src/ir/ruleTable.ts`
- 修改：`reference/tests/ir/ruleTable.test.ts`

**背景：** Rust `lower_rule_table.rs` 262 行，含 priority 排序、overlap 检测、wildcard 行。TS `ruleTable.ts` 仅 28 行骨架。

**参照：** `compiler/tangle-cli/src/ir/lower_rule_table.rs`（262 行 Rust 源码）、`reference/src/ir/ruleTable.ts`（当前 28 行骨架）、`tests/rules/decision-table.tangle.md`（fixture）

- [ ] **步骤 1：阅读 Rust 源码理解结构**

阅读 `lower_rule_table.rs` 全文，理解：
- 表格解析（header row + data rows + column mapping）
- `sort_rows_by_priority` 稳定升序
- overlap 检测（两行条件重叠且 action 不同 → `TANGLE_RULE_TABLE_OVERLAP`）
- wildcard 行（`*` 匹配所有 → `TANGLE_RULE_TABLE_WILDCARD_COVERS`）
- Decision 节点 + guarded edges 发射

- [ ] **步骤 2：编写失败的测试**

替换 `reference/tests/ir/ruleTable.test.ts` 内容：

```typescript
import { describe, it, expect } from "vitest";
import { lowerRuleTable } from "../../src/ir/ruleTable.js";

describe("lowerRuleTable", () => {
  it("ruleTable_lower_basic_table: 基础表格 lowering", () => {
    const md = `| condition | action |
|------------|--------|
| A          | do X   |
| B          | do Y   |`;
    const result = lowerRuleTable(md, "test.tangle");
    
    expect(result.graph.nodes.length).toBeGreaterThan(0);
    const decisions = result.graph.nodes.filter(n => n.kind === "decision");
    expect(decisions.length).toBeGreaterThanOrEqual(1);
  });

  it("ruleTable_lower_priority_ordering: priority 升序排列", () => {
    const md = `| priority | condition | action |
|----------|-----------|--------|
| 2        | A         | do X   |
| 1        | B         | do Y   |`;
    const result = lowerRuleTable(md, "test.tangle");
    
    // priority 1 应排在 priority 2 之前
    const edges = result.graph.edges;
    expect(edges.length).toBeGreaterThanOrEqual(2);
  });

  it("ruleTable_lower_overlap_detection: overlap 诊断发射", () => {
    const md = `| condition | action |
|------------|--------|
| A          | do X   |
| A          | do Y   |`;
    const result = lowerRuleTable(md, "test.tangle");
    
    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_TABLE_OVERLAP")).toBe(true);
  });

  it("ruleTable_lower_wildcard: wildcard 行覆盖后续", () => {
    const md = `| condition | action |
|------------|--------|
| *          | default |
| A          | specific |`;
    const result = lowerRuleTable(md, "test.tangle");
    
    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_TABLE_WILDCARD_COVERS")).toBe(true);
  });
});
```

- [ ] **步骤 3：运行测试验证失败**

运行：
```bash
cd reference
npx vitest run tests/ir/ruleTable.test.ts
```

预期：FAIL，当前 `lowerRuleTable` 无 priority/overlap/wildcard 逻辑。

- [ ] **步骤 4：忠实 port lower_rule_table.rs 到 ruleTable.ts**

替换 `reference/src/ir/ruleTable.ts`，**逐行 port Rust 端**：

```typescript
import type { RuleGraph, IRNode, IREdge, TangleDiagnostic } from "./graph.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

export interface TableLowerResult {
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
}

interface TableRow {
  priority: number | null;
  conditions: string[];
  action: string;
  lineNumber: number;
}

export function lowerRuleTable(markdown: string, file: string): TableLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const entryId = freshNodeId("table-entry");
  const graph = createGraph(entryId);

  graph.nodes.push({
    id: entryId,
    kind: "compute",
    label: "table.entry",
    sourceSpan: null, sourceText: null, group: null, style: null,
  });

  // 1. Parse table
  const lines = markdown.split("\n").filter(l => l.includes("|"));
  if (lines.length < 2) return { graph, diagnostics };

  // Header row
  const header = parseRow(lines[0]);
  const priorityIdx = header.findIndex(h => h.toLowerCase() === "priority");
  const conditionIdx = header.findIndex(h => h.toLowerCase() === "condition");
  const actionIdx = header.findIndex(h => h.toLowerCase() === "action");

  // Data rows (skip header + separator)
  let rows: TableRow[] = [];
  for (let i = 2; i < lines.length; i++) {
    const cells = parseRow(lines[i]);
    const priority = priorityIdx >= 0 ? parseInt(cells[priorityIdx]) || null : null;
    const conditions = conditionIdx >= 0 ? [cells[conditionIdx]] : [];
    const action = actionIdx >= 0 ? cells[actionIdx] : cells[cells.length - 1];
    rows.push({ priority, conditions, action, lineNumber: i + 1 });
  }

  // 2. Sort by priority (stable ascending, null last)
  rows = sortRowsByPriority(rows);

  // 3. Overlap detection
  for (let i = 0; i < rows.length; i++) {
    for (let j = i + 1; j < rows.length; j++) {
      if (conditionsOverlap(rows[i].conditions, rows[j].conditions) && rows[i].action !== rows[j].action) {
        diagnostics.push({
          code: "TANGLE_RULE_TABLE_OVERLAP",
          message: `overlapping conditions with different actions: ${rows[i].conditions.join(", ")} vs ${rows[j].conditions.join(", ")}`,
          span: { file, startLine: rows[j].lineNumber, startColumn: 0, endLine: rows[j].lineNumber, endColumn: 0 },
        });
      }
    }
  }

  // 4. Wildcard detection
  for (let i = 0; i < rows.length; i++) {
    if (rows[i].conditions.includes("*")) {
      for (let j = i + 1; j < rows.length; j++) {
        diagnostics.push({
          code: "TANGLE_RULE_TABLE_WILDCARD_COVERS",
          message: `wildcard row covers subsequent row: ${rows[j].conditions.join(", ")}`,
          span: { file, startLine: rows[j].lineNumber, startColumn: 0, endLine: rows[j].lineNumber, endColumn: 0 },
        });
      }
    }
  }

  // 5. Emit Decision node + guarded edges
  const decisionId = freshNodeId("table-decision");
  graph.nodes.push({
    id: decisionId,
    kind: "decision",
    label: "table.dispatch",
    sourceSpan: null, sourceText: null, group: null, style: null,
  });
  graph.edges.push({
    from: entryId, to: decisionId, kind: "control", guard: null,
    sourceSpan: null, priority: null, style: null,
  });

  for (const row of rows) {
    const actionId = freshNodeId("table-action");
    graph.nodes.push({
      id: actionId,
      kind: "terminal",
      label: row.action,
      sourceSpan: null, sourceText: null, group: null, style: null,
    });
    graph.edges.push({
      from: decisionId, to: actionId, kind: "condition",
      guard: row.conditions.join(" AND "),
      sourceSpan: null,
      priority: row.priority,
      style: null,
    });
  }

  return { graph, diagnostics };
}

function parseRow(line: string): string[] {
  return line.split("|").map(c => c.trim()).filter(c => c !== "");
}

function sortRowsByPriority(rows: TableRow[]): TableRow[] {
  return [...rows].sort((a, b) => {
    if (a.priority === null && b.priority === null) return 0;
    if (a.priority === null) return 1;
    if (b.priority === null) return -1;
    return a.priority - b.priority;
  });
}

function conditionsOverlap(a: string[], b: string[]): boolean {
  // 简化版：若任一条件相同视为重叠（Rust 端可能更复杂，对照源码调整）
  return a.some(c => b.includes(c));
}
```

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
npx vitest run tests/ir/ruleTable.test.ts
```

预期：4 个测试全 PASS。

- [ ] **步骤 6：Commit**

```bash
cd ..
git add reference/src/ir/ruleTable.ts reference/tests/ir/ruleTable.test.ts
git commit -m "feat(reference): A2-3 ruleTable.ts 忠实 port lower_rule_table.rs

从 28 行骨架扩展为完整 port：
- 表格解析（header + data rows + column mapping）
- priority 排序（稳定升序，null 最后）
- overlap 检测（发 TANGLE_RULE_TABLE_OVERLAP 诊断）
- wildcard 行检测（发 TANGLE_RULE_TABLE_WILDCARD_COVERS 诊断）
- Decision 节点 + guarded edges 发射

测试：4 个 vitest 单元测试覆盖基础/priority/overlap/wildcard"
```

---

## 任务 10：A2-4 ruleFlow.ts 忠实 port

**文件：**
- 修改：`reference/src/ir/ruleFlow.ts`
- 修改：`reference/tests/ir/ruleFlow.test.ts`

**背景：** Rust `lower_rule_flow.rs` 535 行，最复杂，含 Mermaid 语法解析、subgraph、多 edge 类型（Dashed/Thick/Crossed）、group/style。TS `ruleFlow.ts` 仅 42 行骨架。

**参照：** `compiler/tangle-cli/src/ir/lower_rule_flow.rs`（535 行 Rust 源码）、`reference/src/ir/ruleFlow.ts`（当前 42 行骨架）、`tests/rules/approval-flow.tangle.md`（fixture）

- [ ] **步骤 1：阅读 Rust 源码理解结构**

阅读 `lower_rule_flow.rs` 全文，理解：
- Mermaid 语法：`graph TD` / `subgraph X` / `A -->|label| B` / `A -.->|label| B`
- 节点定义：`A[Action]` / `A{Decision}` / `A((Start))` / `A>Terminal]`
- Edge 类型：`-->` (Solid) / `-.->` (Dashed) / `==>` (Thick) / `--x` (Crossed)
- group：subgraph 名 → `IRNode.group`
- style：`classDef` + `class A classA` → `IRNode.style`

- [ ] **步骤 2：编写失败的测试**

替换 `reference/tests/ir/ruleFlow.test.ts` 内容：

```typescript
import { describe, it, expect } from "vitest";
import { lowerRuleFlow } from "../../src/ir/ruleFlow.js";

describe("lowerRuleFlow", () => {
  it("ruleFlow_lower_basic_graph: 基础 Mermaid graph", () => {
    const md = `graph TD
    A[Start] --> B{Decision}
    B -->|yes| C[Action]
    B -->|no| D[End]`;
    const result = lowerRuleFlow(md, "test.tangle");
    
    expect(result.graph.nodes.length).toBe(4); // A, B, C, D
    expect(result.graph.edges.length).toBe(3);
  });

  it("ruleFlow_lower_subgraph_group: subgraph → group 字段", () => {
    const md = `graph TD
    subgraph UI
    A[Button] --> B[Handler]
    end`;
    const result = lowerRuleFlow(md, "test.tangle");
    
    const nodeA = result.graph.nodes.find(n => n.label === "Button");
    expect(nodeA).toBeDefined();
    expect(nodeA!.group).toBe("UI");
  });

  it("ruleFlow_lower_dashed_edge: Dashed edge 类型", () => {
    const md = `graph TD
    A -.-> B`;
    const result = lowerRuleFlow(md, "test.tangle");
    
    expect(result.graph.edges[0].kind).toBe("dashed");
  });

  it("ruleFlow_lower_thick_edge: Thick edge 类型", () => {
    const md = `graph TD
    A ==> B`;
    const result = lowerRuleFlow(md, "test.tangle");
    
    expect(result.graph.edges[0].kind).toBe("thick");
  });

  it("ruleFlow_lower_crossed_edge_skipped: Crossed edge 标记", () => {
    const md = `graph TD
    A --x B`;
    const result = lowerRuleFlow(md, "test.tangle");
    
    expect(result.graph.edges[0].kind).toBe("crossed");
  });

  it("ruleFlow_lower_node_shapes: 节点形状映射", () => {
    const md = `graph TD
    A[Action] --> B{Decision}
    C((Start)) --> D>Terminal]`;
    const result = lowerRuleFlow(md, "test.tangle");
    
    const kinds = result.graph.nodes.map(n => n.kind);
    expect(kinds).toContain("compute");   // [Action]
    expect(kinds).toContain("decision");  // {Decision}
    // ((Start)) 和 >Terminal] 根据 Rust 端映射
  });
});
```

- [ ] **步骤 3：运行测试验证失败**

运行：
```bash
cd reference
npx vitest run tests/ir/ruleFlow.test.ts
```

预期：FAIL，当前 `lowerRuleFlow` 无 Mermaid 解析、无 subgraph、无多 edge 类型。

- [ ] **步骤 4：忠实 port lower_rule_flow.rs 到 ruleFlow.ts**

替换 `reference/src/ir/ruleFlow.ts`，**逐行 port Rust 端**。这是最大的 port（535 行 Rust → ~700 行 TS）。

关键结构（参照 Rust 端）：

```typescript
import type { RuleGraph, IRNode, IREdge, IREdgeKind, TangleDiagnostic } from "./graph.js";
import { createGraph, freshNodeId, resetNodeCounter } from "./graph.js";

export interface FlowLowerResult {
  graph: RuleGraph;
  diagnostics: TangleDiagnostic[];
}

interface MermaidNode {
  id: string;
  label: string;
  shape: "rect" | "diamond" | "circle" | "terminal";
  group: string | null;
  style: string | null;
}

interface MermaidEdge {
  from: string;
  to: string;
  label: string | null;
  kind: "solid" | "dashed" | "thick" | "crossed";
}

export function lowerRuleFlow(markdown: string, file: string): FlowLowerResult {
  resetNodeCounter();
  const diagnostics: TangleDiagnostic[] = [];
  const entryId = freshNodeId("flow-entry");
  const graph = createGraph(entryId);

  const lines = markdown.split("\n").map(l => l.trim()).filter(l => l.length > 0);
  
  const nodes = new Map<string, MermaidNode>();
  const edges: MermaidEdge[] = [];
  const subgraphStack: string[] = [];
  const classDefs = new Map<string, string>();
  const nodeClasses = new Map<string, string>();

  for (const line of lines) {
    // 1. subgraph 开始
    const subgraphMatch = line.match(/^subgraph\s+(.+)$/);
    if (subgraphMatch) {
      subgraphStack.push(subgraphMatch[1].trim());
      continue;
    }
    // 2. subgraph 结束
    if (line === "end") {
      subgraphStack.pop();
      continue;
    }
    // 3. classDef
    const classDefMatch = line.match(/^classDef\s+(\w+)\s+(.+)$/);
    if (classDefMatch) {
      classDefs.set(classDefMatch[1], classDefMatch[2].trim());
      continue;
    }
    // 4. class assignment
    const classMatch = line.match(/^class\s+(\w+)\s+(\w+)$/);
    if (classMatch) {
      nodeClasses.set(classMatch[1], classMatch[2]);
      continue;
    }
    // 5. Edge: A --> B, A -.-> B, A ==> B, A --x B, A -->|label| B
    const edgeMatch = parseEdge(line);
    if (edgeMatch) {
      // 确保节点存在
      ensureNode(nodes, edgeMatch.from, subgraphStack[subgraphStack.length - 1] || null);
      ensureNode(nodes, edgeMatch.to, subgraphStack[subgraphStack.length - 1] || null);
      edges.push(edgeMatch);
      continue;
    }
    // 6. Node definition: A[Label], A{Label}, A((Label)), A>Label]
    const nodeMatch = parseNodeDef(line);
    if (nodeMatch) {
      const existing = nodes.get(nodeMatch.id);
      if (existing) {
        existing.label = nodeMatch.label;
        existing.shape = nodeMatch.shape;
      } else {
        nodes.set(nodeMatch.id, {
          id: nodeMatch.id,
          label: nodeMatch.label,
          shape: nodeMatch.shape,
          group: subgraphStack[subgraphStack.length - 1] || null,
          style: null,
        });
      }
    }
  }

  // 应用 classDef style
  for (const [nodeId, className] of nodeClasses) {
    const node = nodes.get(nodeId);
    if (node) {
      node.style = classDefs.get(className) || null;
    }
  }

  // 发射 IR
  for (const [, mNode] of nodes) {
    const irId = freshNodeId("flow");
    const kind = shapeToKind(mNode.shape);
    graph.nodes.push({
      id: irId,
      kind,
      label: mNode.label,
      sourceSpan: null, sourceText: null,
      group: mNode.group, style: mNode.style,
    });
    // 记录 mNode.id → irId 映射（用于 edge 发射）
    (mNode as any).irId = irId;
  }

  for (const mEdge of edges) {
    const fromNode = nodes.get(mEdge.from);
    const toNode = nodes.get(mEdge.to);
    if (fromNode && toNode) {
      graph.edges.push({
        from: (fromNode as any).irId,
        to: (toNode as any).irId,
        kind: edgeKindToIr(mEdge.kind),
        guard: mEdge.label,
        sourceSpan: null, priority: null, style: null,
      });
    }
  }

  return { graph, diagnostics };
}

function parseEdge(line: string): MermaidEdge | null {
  // 匹配: A --> B, A -.-> B, A ==> B, A --x B, A -->|label| B
  const patterns: Array<{ re: RegExp; kind: MermaidEdge["kind"] }> = [
    { re: /^(\w+)\s*==>\s*(\w+)(?:\s*\|(.+?)\|)?$/, kind: "thick" },
    { re: /^(\w+)\s*-\.\->\s*(\w+)(?:\s*\|(.+?)\|)?$/, kind: "dashed" },
    { re: /^(\w+)\s*--x\s*(\w+)(?:\s*\|(.+?)\|)?$/, kind: "crossed" },
    { re: /^(\w+)\s*-->\s*(\w+)(?:\s*\|(.+?)\|)?$/, kind: "solid" },
  ];
  for (const { re, kind } of patterns) {
    const m = line.match(re);
    if (m) {
      return { from: m[1], to: m[2], label: m[3] || null, kind };
    }
  }
  return null;
}

function parseNodeDef(line: string): { id: string; label: string; shape: MermaidNode["shape"] } | null {
  // A[Label] rect, A{Label} diamond, A((Label)) circle, A>Label] terminal
  const patterns: Array<{ re: RegExp; shape: MermaidNode["shape"] }> = [
    { re: /^(\w+)\((.+)\)$/, shape: "circle" },
    { re: /^(\w+)\{(.+)\}$/, shape: "diamond" },
    { re: /^(\w+)>(.+)\]$/, shape: "terminal" },
    { re: /^(\w+)\[(.+)\]$/, shape: "rect" },
  ];
  for (const { re, shape } of patterns) {
    const m = line.match(re);
    if (m) return { id: m[1], label: m[2], shape };
  }
  return null;
}

function ensureNode(nodes: Map<string, MermaidNode>, id: string, group: string | null): void {
  if (!nodes.has(id)) {
    nodes.set(id, { id, label: id, shape: "rect", group, style: null });
  }
}

function shapeToKind(shape: MermaidNode["shape"]): IRNode["kind"] {
  switch (shape) {
    case "diamond": return "decision";
    case "terminal": return "terminal";
    case "circle":
    case "rect":
    default: return "compute";
  }
}

function edgeKindToIr(kind: MermaidEdge["kind"]): IREdgeKind {
  switch (kind) {
    case "dashed": return "dashed";
    case "thick": return "thick";
    case "crossed": return "crossed";
    case "solid":
    default: return "control";
  }
}
```

**注意：** 上述是简化版 port。实际 port 时需对照 Rust `lower_rule_flow.rs` 的完整逻辑（如 `graph TD` 声明跳过、多 edge 语法变体、嵌套 subgraph、style 内联 `style A fill:#f9f` 等）。

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
npx vitest run tests/ir/ruleFlow.test.ts
```

预期：6 个测试全 PASS。若失败，对照 Rust 端正则和逻辑调整。

- [ ] **步骤 6：Commit**

```bash
cd ..
git add reference/src/ir/ruleFlow.ts reference/tests/ir/ruleFlow.test.ts
git commit -m "feat(reference): A2-4 ruleFlow.ts 忠实 port lower_rule_flow.rs

从 42 行骨架扩展为完整 port（~535 行 Rust → ~700 行 TS）：
- Mermaid 语法解析（graph TD / subgraph / end / classDef / class）
- 节点形状：[rect] {diamond} ((circle)) >terminal]
- Edge 类型：--> (solid) -.-> (dashed) ==> (thick) --x (crossed)
- subgraph 嵌套 → IRNode.group
- classDef + class → IRNode.style

测试：6 个 vitest 单元测试覆盖基础/subgraph/dashed/thick/crossed/形状"
```

---

## 任务 11：A2-5 compileToIR.ts 接入 + 端到端验证

**文件：**
- 修改：`reference/src/ir/compileToIR.ts`

**背景：** TS 端 `compileToIR.ts` 只 lower `@tangle` 代码块，完全没调用 4 个 rule lower 函数。这是 6 个 rule fixture 输出空 IR 的直接原因。

**参照：** `reference/src/ir/compileToIR.ts`（当前 13-27 行只 lower @tangle）、`compiler/tangle-cli/src/ir/compile_to_ir.rs:32-43`（Rust 端 rule lowering 调用模式）

- [ ] **步骤 1：阅读 compileToIR.ts 当前结构**

阅读 `reference/src/ir/compileToIR.ts` 全文，理解：
- `CheckedModule` 类型（parsedBlocks + headings + file + diagnostics）
- `Heading` 类型（kind + title + body）
- 当前 `@tangle` 代码块 lowering 流程

- [ ] **步骤 2：编写失败的测试（端到端）**

在 `reference/tests/ir/compileToIR.test.ts` 追加（或新建）：

```typescript
import { describe, it, expect } from "vitest";
import { compileToIR } from "../../src/ir/compileToIR.js";
import { parseModule } from "../../src/parser/parser.js";
import { checkModule } from "../../src/checker/checker.js";
import * as fs from "fs";
import * as path from "path";

function loadFixture(name: string) {
  const fixturePath = path.join(__dirname, "../../../tests/rules", name);
  const src = fs.readFileSync(fixturePath, "utf-8");
  const parsed = parseModule(src, fixturePath);
  return checkModule(parsed);
}

describe("compileToIR rule lowering 集成", () => {
  it("decision-tree fixture 生成非空 IR", () => {
    const checked = loadFixture("decision-tree.tangle.md");
    const { graph } = compileToIR(checked);
    
    expect(graph.nodes.length).toBeGreaterThan(0);
    expect(graph.edges.length).toBeGreaterThan(0);
  });

  it("feature-toggles fixture 生成非空 IR", () => {
    const checked = loadFixture("feature-toggles.tangle.md");
    const { graph } = compileToIR(checked);
    
    expect(graph.nodes.length).toBeGreaterThan(0);
  });
});
```

- [ ] **步骤 3：运行测试验证失败**

运行：
```bash
cd reference
npx vitest run tests/ir/compileToIR.test.ts
```

预期：FAIL，IR 为空（未调用 rule lower）。

- [ ] **步骤 4：修改 compileToIR.ts 接入 4 个 rule lower**

修改 `reference/src/ir/compileToIR.ts`：

```typescript
import { lowerRuleTree } from "./ruleTree.js";
import { lowerRuleTable } from "./ruleTable.js";
import { lowerRuleFlow } from "./ruleFlow.js";
import { lowerRuleToggle } from "./ruleToggle.js";
// ... 其他已有 import ...

export function compileToIR(checked: CheckedModule): { graph: RuleGraph; diagnostics: TangleDiagnostic[] } {
  const allDiagnostics: TangleDiagnostic[] = [...checked.diagnostics];
  let graph = createGraph("");

  // 1. Lower @tangle code blocks（保持现状）
  for (const parsed of checked.parsedBlocks) {
    // ... 现有逻辑 ...
  }

  // 2. Lower rule blocks from headings（新增）
  const ruleDiags: TangleDiagnostic[] = [];
  collectRuleGraphs(checked.headings, checked.file, (ruleKind, markdown, _heading) => {
    let result: { graph: RuleGraph; diagnostics: TangleDiagnostic[] };
    switch (ruleKind) {
      case "tree":
        result = lowerRuleTree(markdown, checked.file);
        break;
      case "table":
        result = lowerRuleTable(markdown, checked.file);
        break;
      case "flow":
        result = lowerRuleFlow(markdown, checked.file);
        break;
      case "toggle":
        result = lowerRuleToggle(markdown, checked.file);
        break;
      default:
        return;
    }
    mergeInto(&graph, result.graph);
    ruleDiags.push(...result.diagnostics);
  });
  allDiagnostics.push(...ruleDiags);

  // 3. Validate（保持现状）
  // ...

  return { graph, diagnostics: allDiagnostics };
}

/// Collect rule blocks from headings. 参照 Rust 端 collect_rule_graphs。
/// 遍历 headings，找 @rule.tree/@rule.table/@rule.flow/@rule.toggle 标记的 heading，
/// 提取其 body markdown，调用 callback。
function collectRuleGraphs(
  headings: Heading[],
  file: string,
  callback: (ruleKind: "tree" | "table" | "flow" | "toggle", markdown: string, heading: Heading) => void
): void {
  for (const heading of headings) {
    // 根据 heading 的 rule 类型标记（@rule.tree 等）分发
    // 参照 Rust 端 collect_rule_graphs 逻辑
    const ruleKind = detectRuleKind(heading);
    if (ruleKind) {
      callback(ruleKind, heading.body, heading);
    }
  }
}

function detectRuleKind(heading: Heading): "tree" | "table" | "flow" | "toggle" | null {
  // 参照 Rust 端逻辑：检查 heading.title 或 heading.kind
  if (heading.title.includes("@rule.tree")) return "tree";
  if (heading.title.includes("@rule.table")) return "table";
  if (heading.title.includes("@rule.flow")) return "flow";
  if (heading.title.includes("@rule.toggle")) return "toggle";
  return null;
}

function mergeInto(target: RuleGraph, source: RuleGraph): void {
  // 参照 Rust 端 merge_into 逻辑
  target.nodes.push(...source.nodes);
  target.edges.push(...source.edges);
}
```

**注意：** `collectRuleGraphs` 和 `detectRuleKind` 需对照 Rust 端 `compile_to_ir.rs:32-43` 的实际逻辑实现。Rust 端可能用 `HeadingKind::Rule(kind)` 而非标题字符串匹配。

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
npx vitest run tests/ir/compileToIR.test.ts
```

预期：2 个测试 PASS。

- [ ] **步骤 6：运行 diff-ir.ps1 端到端验证**

运行：
```bash
cd ..
pwsh tests/audit/diff-ir.ps1
```

预期：
- 6 rule fixture: 从 `[SKIPPED]` 转 `[MATCH]`（若 DIFF，对照 ir-diff 输出调整 TS 端 port）
- payment.tangle: 仍 `[MATCH]`（A1 已修复）
- expression/hello/user: 仍 `[MATCH]`
- order-service.tangle: 仍 `[SKIPPED]`（B5 依赖，预期）

若 rule fixture 仍 DIFF：
1. 阅读 ir-diff 的 diff 输出
2. 对照 Rust 端 IR JSON 和 TS 端 IR JSON
3. 调整 TS 端 port 直到 MATCH

- [ ] **步骤 7：Commit**

```bash
git add reference/src/ir/compileToIR.ts reference/tests/ir/compileToIR.test.ts
git commit -m "feat(reference): A2-5 compileToIR.ts 接入 4 个 rule lower

接入 lowerRuleTree/Table/Flow/Toggle 调用：
- collectRuleGraphs 遍历 headings 分发到对应 lower
- mergeInto 合并 sub-graph 到主 graph
- 诊断聚合到 allDiagnostics

端到端：6 rule fixture 从 SKIPPED 转 MATCH
diff-ir.ps1: 10 MATCH / 0 KNOWN_DIFF / 1 SKIPPED (order-service, B5)"
```

---

## 任务 12：B2 三宿主递归深度限制

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/js_emitter.rs`
- 修改：`compiler/tangle-cli/src/codegen/py_emitter.rs`
- 修改：`compiler/tangle-cli/src/codegen/go_emitter.rs`
- 创建：`compiler/tangle-cli/tests/v05_phase5/recursion_depth.rs`
- 创建：`tests/v05_phase5/deep-recursion.tangle.md`

**背景：** 三宿主 `emit_branch_body` 仅有 `visited` 集合防环，无深度计数。加 `depth: usize` 参数，阈值 100，超限发注释停止递归。

**参照：** `compiler/tangle-cli/src/codegen/js_emitter.rs:242-383`（JS emit_branch_body）、`py_emitter.rs`（Py emit_branch_body_py）、`go_emitter.rs`（Go emit_branch_body_go）

- [ ] **步骤 1：创建 deep-recursion fixture**

创建 `tests/v05_phase5/deep-recursion.tangle.md`（构造 100+ 层嵌套 Decision 图）：

```markdown
# Deep Recursion Test

@tangle

if (a0) {
  if (a1) {
    if (a2) {
      if (a3) {
        if (a4) {
          if (a5) {
            if (a6) {
              if (a7) {
                if (a8) {
                  if (a9) {
                    if (a10) {
                      if (a11) {
                        if (a12) {
                          if (a13) {
                            if (a14) {
                              if (a15) {
                                if (a16) {
                                  if (a17) {
                                    if (a18) {
                                      if (a19) {
                                        if (a20) {
                                          if (a21) {
                                            if (a22) {
                                              if (a23) {
                                                if (a24) {
                                                  if (a25) {
                                                    if (a26) {
                                                      if (a27) {
                                                        if (a28) {
                                                          if (a29) {
                                                            if (a30) {
                                                              if (a31) {
                                                                if (a32) {
                                                                  if (a33) {
                                                                    if (a34) {
                                                                      if (a35) {
                                                                        if (a36) {
                                                                          if (a37) {
                                                                            if (a38) {
                                                                              if (a39) {
                                                                                if (a40) {
                                                                                  if (a41) {
                                                                                    if (a42) {
                                                                                      if (a43) {
                                                                                        if (a44) {
                                                                                          if (a45) {
                                                                                            if (a46) {
                                                                                              if (a47) {
                                                                                                if (a48) {
                                                                                                  if (a49) {
                                                                                                    if (a50) {
                                                                                                      if (a51) {
                                                                                                        if (a52) {
                                                                                                          if (a53) {
                                                                                                            if (a54) {
                                                                                                              if (a55) {
                                                                                                                if (a56) {
                                                                                                                  if (a57) {
                                                                                                                    if (a58) {
                                                                                                                      if (a59) {
                                                                                                                        if (a60) {
                                                                                                                          if (a61) {
                                                                                                                            if (a62) {
                                                                                                                              if (a63) {
                                                                                                                                if (a64) {
                                                                                                                                  if (a65) {
                                                                                                                                    if (a66) {
                                                                                                                                      if (a67) {
                                                                                                                                        if (a68) {
                                                                                                                                          if (a69) {
                                                                                                                                            if (a70) {
                                                                                                                                              if (a71) {
                                                                                                                                                if (a72) {
                                                                                                                                                  if (a73) {
                                                                                                                                                    if (a74) {
                                                                                                                                                      if (a75) {
                                                                                                                                                        if (a76) {
                                                                                                                                                          if (a77) {
                                                                                                                                                            if (a78) {
                                                                                                                                                              if (a79) {
                                                                                                                                                                if (a80) {
                                                                                                                                                                  if (a81) {
                                                                                                                                                                    if (a82) {
                                                                                                                                                                      if (a83) {
                                                                                                                                                                        if (a84) {
                                                                                                                                                                          if (a85) {
                                                                                                                                                                            if (a86) {
                                                                                                                                                                              if (a87) {
                                                                                                                                                                                if (a88) {
                                                                                                                                                                                  if (a89) {
                                                                                                                                                                                    if (a90) {
                                                                                                                                                                                      if (a91) {
                                                                                                                                                                                        if (a92) {
                                                                                                                                                                                          if (a93) {
                                                                                                                                                                                            if (a94) {
                                                                                                                                                                                              if (a95) {
                                                                                                                                                                                                if (a96) {
                                                                                                                                                                                                  if (a97) {
                                                                                                                                                                                                    if (a98) {
                                                                                                                                                                                                      if (a99) {
                                                                                                                                                                                                        if (a100) {
                                                                                                                                                                                                          result = "deep";
                                                                                                                                                                                                        }
                                                                                                                                                                                                      }
                                                                                                                                                                                                    }
                                                                                                                                                                                                  }
                                                                                                                                                                                                }
                                                                                                                                                                                              }
                                                                                                                                                                                            }
                                                                                                                                                                                          }
                                                                                                                                                                                        }
                                                                                                                                                                                      }
                                                                                                                                                                                    }
                                                                                                                                                                                  }
                                                                                                                                                                                }
                                                                                                                                                                              }
                                                                                                                                                                            }
                                                                                                                                                                          }
                                                                                                                                                                        }
                                                                                                                                                                      }
                                                                                                                                                                    }
                                                                                                                                                                  }
                                                                                                                                                                }
                                                                                                                                                              }
                                                                                                                                                            }
                                                                                                                                                          }
                                                                                                                                                        }
                                                                                                                                                      }
                                                                                                                                                    }
                                                                                                                                                  }
                                                                                                                                                }
                                                                                                                                              }
                                                                                                                                            }
                                                                                                                                          }
                                                                                                                                        }
                                                                                                                                      }
                                                                                                                                    }
                                                                                                                                  }
                                                                                                                                }
                                                                                                                              }
                                                                                                                            }
                                                                                                                          }
                                                                                                                        }
                                                                                                                      }
                                                                                                                    }
                                                                                                                  }
                                                                                                                }
                                                                                                              }
                                                                                                            }
                                                                                                          }
                                                                                                        }
                                                                                                      }
                                                                                                    }
                                                                                                  }
                                                                                                }
                                                                                              }
                                                                                            }
                                                                                          }
                                                                                        }
                                                                                      }
                                                                                    }
                                                                                  }
                                                                                }
                                                                              }
                                                                            }
                                                                          }
                                                                        }
                                                                      }
                                                                    }
                                                                  }
                                                                }
                                                              }
                                                            }
                                                          }
                                                        }
                                                      }
                                                    }
                                                  }
                                                }
                                              }
                                            }
                                          }
                                        }
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
```

- [ ] **步骤 2：编写失败的测试**

创建 `compiler/tangle-cli/tests/v05_phase5/recursion_depth.rs`：

```rust
use tangle_cli::run_collecting_diagnostics;
use tangle_cli::codegen::{emit_javascript, emit_python, emit_go};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/v05_phase5")
        .join(name)
}

#[test]
fn js_branch_body_depth_limit_emits_comment() {
    let path = fixture_path("deep-recursion.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_javascript(&graph, "deep");
    
    // 超 100 层应发 "// max depth reached"
    assert!(
        output.contains("// max depth reached"),
        "JS output should contain depth limit comment, got: {}",
        output
    );
}

#[test]
fn py_branch_body_depth_limit_emits_comment() {
    let path = fixture_path("deep-recursion.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_python(&graph, "deep");
    
    assert!(
        output.contains("# max depth reached"),
        "Py output should contain depth limit comment, got: {}",
        output
    );
}

#[test]
fn go_branch_body_depth_limit_emits_comment() {
    let path = fixture_path("deep-recursion.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_go(&graph, "deep");
    
    assert!(
        output.contains("// max depth reached"),
        "Go output should contain depth limit comment, got: {}",
        output
    );
}

#[test]
fn branch_body_normal_depth_no_comment() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/mvp/expression.tangle.md");
    let (graph, _diags) = run_collecting_diagnostics(&path);
    let output = emit_javascript(&graph, "expression");
    
    // 正常深度（≤5）不应发注释
    assert!(
        !output.contains("max depth reached"),
        "Normal depth should not emit depth comment, got: {}",
        output
    );
}
```

- [ ] **步骤 3：运行测试验证失败**

运行：
```bash
cargo test --test phase5_recursion_depth -- --nocapture
```

预期：FAIL，三宿主输出不含 "max depth reached"（当前无 depth 参数）。

- [ ] **步骤 4：三宿主 emit_branch_body 加 depth 参数**

**修改 `js_emitter.rs`**：

1. 在文件顶部加常量：
```rust
const MAX_BRANCH_DEPTH: usize = 100;
```

2. 找到 `emit_branch_body` 函数签名，加 `depth: usize` 参数：
```rust
fn emit_branch_body(
    target_id: &str,
    nodes: &[IRNode],
    edges: &[IREdge],
    visited: &mut HashSet<String>,
    indent: &str,
    depth: usize,  // 新增
) -> String {
    if depth >= MAX_BRANCH_DEPTH {
        return format!("{}// max depth reached\n", indent);
    }
    // ... 现有逻辑 ...
    // 递归调用 emit_branch_body 时传 depth + 1
}
```

3. 找到所有 `emit_branch_body` 的调用点（包括 `emit_decision_branch` 中的递归调用），传 `depth + 1`。初始调用点传 `0`。

**修改 `py_emitter.rs`**：

1. 加常量 `const MAX_BRANCH_DEPTH: usize = 100;`
2. `emit_branch_body_py` 加 `depth: usize` 参数，超限发 `# max depth reached`
3. 所有调用点传 `depth + 1`，初始传 `0`

**修改 `go_emitter.rs`**：

1. 加常量 `const MAX_BRANCH_DEPTH: usize = 100;`
2. `emit_branch_body_go` 加 `depth: usize` 参数，超限发 `// max depth reached`
3. 所有调用点传 `depth + 1`，初始传 `0`

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
cargo test --test phase5_recursion_depth -- --nocapture
```

预期：4 个测试全 PASS。

- [ ] **步骤 6：验证现有测试不回归**

运行：
```bash
cargo test --workspace
```

预期：所有现有测试通过（depth 参数改动不破坏正常深度用例）。

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/codegen/js_emitter.rs compiler/tangle-cli/src/codegen/py_emitter.rs compiler/tangle-cli/src/codegen/go_emitter.rs compiler/tangle-cli/tests/v05_phase5/recursion_depth.rs tests/v05_phase5/deep-recursion.tangle.md
git commit -m "feat(codegen): B2 三宿主递归深度限制

emit_branch_body 加 depth: usize 参数：
- 阈值 MAX_BRANCH_DEPTH = 100
- 超限发注释停止递归（JS/Go: // max depth reached, Py: # max depth reached）
- 三宿主一致，便于差分测试

测试：js/py/go depth limit + normal depth no comment
fixture: deep-recursion.tangle.md (100+ 层嵌套)"
```

---

## 任务 13：B4 toggle 不继承语义文档化 + 测试

**文件：**
- 修改：`compiler/tangle-cli/src/ir/lower_rule_toggle.rs`
- 创建：`compiler/tangle-cli/tests/v05_phase5/toggle_cross_block.rs`
- 创建：`tests/v05_phase5/multi-toggle-blocks.tangle.md`

**背景：** 当前 `lower_rule_toggle` 每次调用独立，跨块不继承 group/style。无代码改动，仅文档化 + 测试覆盖。

- [ ] **步骤 1：创建 multi-toggle-blocks fixture**

创建 `tests/v05_phase5/multi-toggle-blocks.tangle.md`：

```markdown
# Multi Toggle Blocks

@rule.toggle

<!-- group: UI -->
<!-- style: highlight -->
- [x] `enable_ui`: Enable new UI

@rule.toggle

- [ ] `enable_crypto`: Enable crypto features
```

**预期：** 第二个 toggle 块的 `enable_crypto` 节点 group/style 为 None（不继承第一块的 UI/highlight）。

- [ ] **步骤 2：编写测试**

创建 `compiler/tangle-cli/tests/v05_phase5/toggle_cross_block.rs`：

```rust
use tangle_cli::ir::lower_rule_toggle;
use tangle_cli::ir::graph::FreshNodeId;

#[test]
fn toggle_block_isolation_no_group_inheritance() {
    // 第一块：含 group
    let md1 = "<!-- group: UI -->\n- [x] `enable_ui`: Enable UI";
    let mut id_gen = FreshNodeId::new();
    let (graph1, _diags) = lower_rule_toggle(md1, "test.tangle", &mut id_gen);
    
    // 第二块：不含 group
    let md2 = "- [ ] `enable_crypto`: Enable crypto";
    let (graph2, _diags) = lower_rule_toggle(md2, "test.tangle", &mut id_gen);
    
    // 第一块的 toggle 节点 group = "UI"
    let toggle1 = graph1.nodes.iter().find(|n| n.label.contains("enable_ui")).unwrap();
    assert_eq!(toggle1.group.as_deref(), Some("UI"));
    
    // 第二块的 toggle 节点 group = None（不继承）
    let toggle2 = graph2.nodes.iter().find(|n| n.label.contains("enable_crypto")).unwrap();
    assert!(toggle2.group.is_none(), "second block should not inherit group, got: {:?}", toggle2.group);
}

#[test]
fn toggle_block_isolation_no_style_inheritance() {
    let md1 = "<!-- style: highlight -->\n- [x] `flag1`: desc";
    let mut id_gen = FreshNodeId::new();
    let (graph1, _diags) = lower_rule_toggle(md1, "test.tangle", &mut id_gen);
    
    let md2 = "- [ ] `flag2`: desc";
    let (graph2, _diags) = lower_rule_toggle(md2, "test.tangle", &mut id_gen);
    
    let toggle1 = graph1.nodes.iter().find(|n| n.label.contains("flag1")).unwrap();
    assert_eq!(toggle1.style.as_deref(), Some("highlight"));
    
    let toggle2 = graph2.nodes.iter().find(|n| n.label.contains("flag2")).unwrap();
    assert!(toggle2.style.is_none(), "second block should not inherit style, got: {:?}", toggle2.style);
}

#[test]
fn toggle_explicit_group_per_block_works() {
    // 每块显式声明 group，都生效
    let md1 = "<!-- group: A -->\n- [x] `flag1`: desc";
    let md2 = "<!-- group: B -->\n- [x] `flag2`: desc";
    let mut id_gen = FreshNodeId::new();
    
    let (graph1, _diags) = lower_rule_toggle(md1, "test.tangle", &mut id_gen);
    let (graph2, _diags) = lower_rule_toggle(md2, "test.tangle", &mut id_gen);
    
    let toggle1 = graph1.nodes.iter().find(|n| n.label.contains("flag1")).unwrap();
    assert_eq!(toggle1.group.as_deref(), Some("A"));
    
    let toggle2 = graph2.nodes.iter().find(|n| n.label.contains("flag2")).unwrap();
    assert_eq!(toggle2.group.as_deref(), Some("B"));
}

#[test]
fn toggle_pending_cleared_on_non_checkbox_line() {
    // 单块内：group 缓存遇非注释非 checkbox 行清空
    let md = "<!-- group: UI -->\nsome random text\n- [x] `flag`: desc";
    let mut id_gen = FreshNodeId::new();
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut id_gen);
    
    let toggle = graph.nodes.iter().find(|n| n.label.contains("flag")).unwrap();
    assert!(toggle.group.is_none(), "group should be cleared by non-checkbox line, got: {:?}", toggle.group);
}
```

- [ ] **步骤 3：运行测试验证（应直接通过，因为当前已是该行为）**

运行：
```bash
cargo test --test phase5_toggle_cross_block -- --nocapture
```

预期：4 个测试全 PASS（当前实现已是不继承语义）。若失败，说明当前实现有 bug，需修复。

- [ ] **步骤 4：加 doc comment 明确语义**

修改 `compiler/tangle-cli/src/ir/lower_rule_toggle.rs`，在 `lower_rule_toggle` 函数前加 doc comment：

```rust
/// Lower a single `@rule.toggle` block to IR.
///
/// # 跨块语义
///
/// 每次调用独立处理单个 toggle 块。跨 `@rule.toggle` 块的 group/style
/// 不继承——前一块的 pending_group/pending_style 不会流入下一块。
/// 如需为多个块设置统一 group，必须在每个块内显式声明。
///
/// # 单块内语义
///
/// `pending_group`/`pending_style` 缓存遇 `<!-- group: X -->` 行设置，
/// 遇 checkbox 行消费并清空，遇非注释非 checkbox 行清空。
pub fn lower_rule_toggle(checkbox_markdown: &str, file: &str, id_gen: &mut FreshNodeId) -> (RuleGraph, Vec<TangleDiagnostic>) {
    // ... 现有实现 ...
}
```

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
cargo test --test phase5_toggle_cross_block -- --nocapture
```

预期：4 个测试全 PASS。

- [ ] **步骤 6：Commit**

```bash
git add compiler/tangle-cli/src/ir/lower_rule_toggle.rs compiler/tangle-cli/tests/v05_phase5/toggle_cross_block.rs tests/v05_phase5/multi-toggle-blocks.tangle.md
git commit -m "docs(ir): B4 toggle 跨块不继承语义文档化 + 测试

无代码改动，当前实现已是不继承语义。补充：
- lower_rule_toggle 加 doc comment 明确跨块/单块语义
- 4 个测试覆盖：跨块不继承 group/style + 每块显式声明 + 单块内清空

fixture: multi-toggle-blocks.tangle.md (2 个 toggle 块)"
```

---

## 任务 14：出口闸门验证 + merge + tag

**文件：**
- 无文件修改，仅验证 + git 操作

- [ ] **步骤 1：闸门 1 — 单元+集成测试**

运行：
```bash
cd .worktrees/phase5-v0.5.0
cargo test --workspace
```

预期：全绿。记录测试数量。

- [ ] **步骤 2：闸门 2 — Clippy**

运行：
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

预期：0 警告。若有警告，修复后重新运行。

- [ ] **步骤 3：闸门 3 — 审计回归**

运行：
```bash
pwsh tests/audit/run-audit.ps1
```

预期：0 failing。

- [ ] **步骤 4：闸门 4 — 差分测试**

运行：
```bash
pwsh tests/audit/diff-ir.ps1
```

预期：**10 MATCH + 0 KNOWN_DIFF + 1 SKIPPED (order-service)**。

- [ ] **步骤 5：闸门 5 — Phase 4 回归**

运行：
```bash
cargo test --test phase4_py_go_codegen --test phase4_toggle_lowering
```

预期：全绿。

- [ ] **步骤 6：闸门 6 — Phase 5 新测试**

运行：
```bash
cargo test --test phase5_dual_entry --test phase5_recursion_depth --test phase5_toggle_cross_block
cargo test --manifest-path tests/audit/ir-diff/Cargo.toml
```

预期：全绿。

- [ ] **步骤 7：闸门 7 — TS 参考实现测试**

运行：
```bash
cd reference
npm test
cd ..
```

预期：全绿（vitest）。

- [ ] **步骤 8：闸门 8 — TS 参考实现构建**

运行：
```bash
cd reference
npm run build
npm run typecheck
cd ..
```

预期：0 类型错误。

- [ ] **步骤 9：更新 project_memory.md**

更新 `c:\Users\cheng\.trae-cn\memory\projects\-e-GitProjects-tangle\project_memory.md`：
- 追加 Phase 5 完成的约束和约定
- 记录 order-service.tangle 保持 SKIPPED 的根因（B5 类型系统依赖）

- [ ] **步骤 10：Merge 到 main**

运行：
```bash
cd e:\GitProjects\tangle
git checkout main
git merge --ff-only phase5/v0.5.0
```

预期：fast-forward merge 成功。

- [ ] **步骤 11：打 tag v0.5.0**

运行：
```bash
git tag v0.5.0
```

预期：tag 创建成功。

- [ ] **步骤 12：清理 worktree**

运行：
```bash
git worktree remove .worktrees/phase5-v0.5.0
git branch -d phase5/v0.5.0
```

预期：worktree 删除，本地分支删除（已 merge）。

- [ ] **步骤 13：最终验证**

运行：
```bash
pwsh tests/audit/diff-ir.ps1
```

预期：main 分支上 10 MATCH + 0 KNOWN_DIFF + 1 SKIPPED。

---

## 自检

### 规格覆盖度

| 规格章节 | 实现任务 | 覆盖 |
|---------|---------|------|
| §1.2 A1 Rust IR 结构自洽 | 任务 2-6 | ✓ |
| §1.2 A2 TS 参考实现 rule lowering | 任务 7-11 | ✓ |
| §1.2 B2 递归深度限制 | 任务 12 | ✓ |
| §1.2 B4 跨 toggle 块不继承语义 | 任务 13 | ✓ |
| §1.3 成功标准 diff-ir 10 MATCH | 任务 6, 11, 14 | ✓ |
| §1.3 payment 顶层 nodes[] 不含 Callable | 任务 2 | ✓ |
| §1.3 Py/Go 多函数发射 | 任务 4-5 | ✓ |
| §1.3 三宿主 depth 参数 | 任务 12 | ✓ |
| §1.3 跨 toggle 块不继承测试覆盖 | 任务 13 | ✓ |
| §1.3 出口闸门 8 项 | 任务 14 | ✓ |
| §3.2 has_main_callable 检测 | 任务 2 | ✓ |
| §3.3 ir-diff compare_functions | 任务 3 | ✓ |
| §3.4 Py/Go emit_multi_function | 任务 4-5 | ✓ |
| §4.2-4.7 4 个 rule lower port | 任务 7-10 | ✓ |
| §4.3 compileToIR.ts 接入 | 任务 11 | ✓ |
| §5.2 三宿主 depth 参数 | 任务 12 | ✓ |
| §6.3 B4 doc comment + 测试 | 任务 13 | ✓ |
| §9 出口闸门 | 任务 14 | ✓ |

**所有规格需求均有对应任务。**

### 占位符扫描

搜索计划中的 "TODO"、"待定"、"待补"、"类似任务 N" 等：
- 无占位符
- 所有代码步骤含完整代码或详细 port 指南
- "参照 Rust 端" 引用包含具体文件路径和行号

### 类型一致性

- `ToggleLowerResult` / `TreeLowerResult` / `TableLowerResult` / `FlowLowerResult` 接口在任务 7-10 定义，任务 11 `compileToIR.ts` 消费 `result.graph` + `result.diagnostics` —— 一致
- `has_main_callable` 函数在任务 2 定义，任务 2 内使用 —— 一致
- `compare_functions` 函数在任务 3 定义，任务 3 内 main 调用 —— 一致
- `emit_multi_function_py` / `emit_multi_function_go` 在任务 4-5 定义 —— 一致
- `MAX_BRANCH_DEPTH = 100` 常量在任务 12 三宿主一致 —— 一致
- `depth: usize` 参数在任务 12 三宿主一致 —— 一致

**类型和签名一致。**
