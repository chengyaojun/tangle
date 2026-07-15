# Phase 4: Phase 3 推迟项闭合 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 闭合 Phase 3 推迟的 4 项工作 — Py/Go if/else 分支发射、ir-diff 归一化（F-007~F-012）、lower_rule_toggle.rs 增强

**架构：** 3 个独立工作流（A/B/C），无交叉依赖。B 组在 ir-diff 工具侧增加 4 阶段归一化流水线；A 组把 JS 的 if/else 分支发射移植到 Py/Go emitter；C 组升级 toggle lowering 的签名/span/名称/元数据。

**技术栈：** Rust + serde_json（ir-diff）、Rust codegen（py/go emitter）、PowerShell（diff-ir.ps1）

**规格文档：** `docs/superpowers/specs/2026-07-16-phase4-deferred-items-design.md`

---

## 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `compiler/tangle-cli/src/codegen/py_emitter.rs` | Python codegen + if/elif 分支 | 修改 |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | Go codegen + if/else if 分支 | 修改 |
| `compiler/tangle-cli/src/ir/lower_rule_toggle.rs` | Toggle lowering 增强 | 修改 |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | Toggle 调用点签名同步 | 修改 |
| `compiler/tangle-cli/Cargo.toml` | 新增 2 个 `[[test]]` 条目 | 修改 |
| `tests/audit/ir-diff/src/main.rs` | ir-diff 4 阶段归一化 + 内联单元测试 | 修改 |
| `tests/audit/diff-ir.ps1` | 清空 $KnownDiffs | 修改 |
| `compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs` | A 组测试（Cargo name: `phase4_py_go_codegen`） | 创建 |
| `compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs` | C 组测试（Cargo name: `phase4_toggle_lowering`） | 创建 |
| `tests/rules/feature-toggles.tangle.md` | Toggle fixture 扩展 | 修改 |

---

## 任务 1：Worktree 与分支准备

**文件：**
- 无文件修改，仅 git 操作

- [ ] **步骤 1：基于 main 创建 worktree**

运行：
```bash
cd e:\GitProjects\tangle
git fetch origin
git worktree add .worktrees/phase4-v0.4.0 -b phase4/v0.4.0 main
```

预期：创建 `.worktrees/phase4-v0.4.0` 目录，新分支 `phase4/v0.4.0` 基于 `main` 最新 commit。

- [ ] **步骤 2：验证 worktree 可编译**

运行：
```bash
cd .worktrees/phase4-v0.4.0
cargo build --workspace
```
预期：编译成功，无错误。

- [ ] **步骤 3：验证基线测试全绿**

运行：
```bash
cargo test --workspace
```
预期：所有现有测试通过（记录测试数量作为基线）。

---

## 任务 2：B 组 — ir-diff lift_functions（F-009）

**文件：**
- 修改：`tests/audit/ir-diff/src/main.rs`

**背景：** Rust IR 顶层含 `functions: [{nodes, edges, entryNodeId, ...}]`，TS IR 顶层直接含 `nodes`/`edges`/`entryNodeId`。需在 normalize 前提升 `functions[0]` 到顶层。

- [ ] **步骤 1：编写失败的测试**

在 `tests/audit/ir-diff/src/main.rs` 末尾（`normalize` 函数之后）添加 `#[cfg(test)] mod tests` 块。先添加 `lift_functions` 测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lift_functions_promotes_first_function_to_top_level() {
        let input = json!({
            "functions": [{
                "nodes": [{"id": "n0", "kind": "action", "label": "do"}],
                "edges": [{"from": "n0", "to": "n1", "kind": "control"}],
                "entryNodeId": "n0"
            }],
            "importedStdlib": [],
            "stdlibImports": []
        });
        let result = lift_functions(input);
        assert!(result.get("functions").is_none(), "functions should be removed");
        assert!(result.get("importedStdlib").is_none(), "empty importedStdlib should be removed");
        assert!(result.get("stdlibImports").is_none(), "empty stdlibImports should be removed");
        assert_eq!(result["entryNodeId"], "n0");
        assert_eq!(result["nodes"][0]["id"], "n0");
        assert_eq!(result["edges"][0]["from"], "n0");
    }

    #[test]
    fn lift_functions_preserves_flat_ir_without_functions_key() {
        let input = json!({
            "nodes": [{"id": "entry", "kind": "action", "label": "do"}],
            "edges": [],
            "entryNodeId": "entry"
        });
        let result = lift_functions(input);
        assert_eq!(result["nodes"][0]["id"], "entry");
        assert_eq!(result["entryNodeId"], "entry");
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd tests/audit/ir-diff
cargo test lift_functions
```
预期：编译失败，`lift_functions` 未定义。

- [ ] **步骤 3：实现 lift_functions**

在 `normalize` 函数之前添加：

```rust
/// Phase 1 of normalization pipeline: lift functions[0] to top-level.
///
/// Rust IR wraps nodes/edges/entryNodeId inside `functions[0]`, while TS IR
/// places them at top level. This function promotes functions[0] to top level
/// and strips the `functions`, empty `importedStdlib`, and empty `stdlibImports`
/// keys. If `functions` is absent or empty, the input is returned unchanged
/// (minus empty stdlib arrays).
fn lift_functions(v: Value) -> Value {
    let mut map = match v {
        Value::Object(m) => m,
        other => return other,
    };

    // Lift functions[0] if present
    if let Some(functions_val) = map.remove("functions") {
        if let Value::Array(arr) = functions_val {
            if let Some(first) = arr.into_iter().next() {
                if let Value::Object(func_map) = first {
                    for (k, v) in func_map {
                        // Don't overwrite existing top-level keys
                        map.entry(k).or_insert(v);
                    }
                }
            }
        }
    }

    // Strip empty stdlib arrays
    if let Some(Value::Array(arr)) = map.get("importedStdlib") {
        if arr.is_empty() {
            map.remove("importedStdlib");
        }
    }
    if let Some(Value::Array(arr)) = map.get("stdlibImports") {
        if arr.is_empty() {
            map.remove("stdlibImports");
        }
    }

    Value::Object(map)
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test lift_functions
```
预期：2 个测试通过。

- [ ] **步骤 5：Commit**

```bash
git add tests/audit/ir-diff/src/main.rs
git commit -m "feat(ir-diff): lift functions[0] to top-level (F-009 normalization)"
```

---

## 任务 3：B 组 — ir-diff ID 重映射（F-007）

**文件：**
- 修改：`tests/audit/ir-diff/src/main.rs`

- [ ] **步骤 1：编写失败的测试**

在 `tests` mod 中添加：

```rust
    #[test]
    fn id_remap_normalizes_n0_to_node0() {
        let input = json!({
            "nodes": [{"id": "n0"}, {"id": "n1"}],
            "edges": [{"from": "n0", "to": "n1"}],
            "entryNodeId": "n0"
        });
        let id_map = build_id_map(&input);
        assert_eq!(id_map.get("n0"), Some(&"node0".to_string()));
        assert_eq!(id_map.get("n1"), Some(&"node1".to_string()));
    }

    #[test]
    fn id_remap_normalizes_entry1_to_node0() {
        let input = json!({
            "nodes": [{"id": "entry1"}, {"id": "bind2"}],
            "entryNodeId": "entry1"
        });
        let id_map = build_id_map(&input);
        assert_eq!(id_map.get("entry1"), Some(&"node0".to_string()));
        assert_eq!(id_map.get("bind2"), Some(&"node1".to_string()));
    }

    #[test]
    fn id_remap_applies_to_edges_and_entry() {
        let input = json!({
            "nodes": [{"id": "n0"}, {"id": "n1"}],
            "edges": [{"from": "n0", "to": "n1", "kind": "control"}],
            "entryNodeId": "n0"
        });
        let id_map = build_id_map(&input);
        let normalized = normalize(input, &id_map);
        assert_eq!(normalized["nodes"][0]["id"], "node0");
        assert_eq!(normalized["nodes"][1]["id"], "node1");
        assert_eq!(normalized["edges"][0]["from"], "node0");
        assert_eq!(normalized["edges"][0]["to"], "node1");
        assert_eq!(normalized["entryNodeId"], "node0");
    }
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test id_remap
```
预期：编译失败，`build_id_map` 和新版 `normalize` 签名未定义。

- [ ] **步骤 3：实现 build_id_map 并改造 normalize**

在 `lift_functions` 之后添加 `build_id_map`：

```rust
/// Phase 2 of normalization pipeline: build a mapping from original node IDs
/// to positional IDs ("node0", "node1", ...).
///
/// TS IR uses semantic IDs (entry1, bind2, ret4), Rust IR uses positional IDs
/// (n0, n1, n2). Both share the same node array order, so positional remapping
/// produces identical IDs on both sides.
fn build_id_map(v: &Value) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
        for (i, node) in nodes.iter().enumerate() {
            if let Some(id) = node.get("id").and_then(|id| id.as_str()) {
                map.insert(id.to_string(), format!("node{}", i));
            }
        }
    }
    map
}
```

改造 `normalize` 函数签名为 `normalize(v: Value, id_map: &std::collections::HashMap<String, String>) -> Value`，在现有 strip span + sort keys 基础上增加 ID 重映射：

```rust
fn normalize(v: Value, id_map: &std::collections::HashMap<String, String>) -> Value {
    match v {
        Value::Object(map) => {
            let mut filtered: Vec<(String, Value)> = Vec::with_capacity(map.len());
            for (k, v) in map {
                if SPAN_FIELDS.contains(&k.as_str()) {
                    continue;
                }
                // F-008: strip null guard
                if k == "guard" && v == Value::Null {
                    continue;
                }
                // F-011: normalize label "return" → "exit"
                if k == "label" {
                    if let Some(s) = v.as_str() {
                        if s == "return" {
                            filtered.push((k, Value::String("exit".into())));
                            continue;
                        }
                    }
                }
                // F-007: remap node IDs
                if k == "id" || k == "from" || k == "to" || k == "entryNodeId" {
                    if let Some(s) = v.as_str() {
                        if let Some(remapped) = id_map.get(s) {
                            filtered.push((k, Value::String(remapped.clone())));
                            continue;
                        }
                    }
                }
                filtered.push((k, normalize(v, id_map)));
            }
            filtered.sort_by(|a, b| a.0.cmp(&b.0));
            let collected: Map<String, Value> = filtered.into_iter().collect();
            Value::Object(collected)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(|v| normalize(v, id_map)).collect()),
        other => other,
    }
}
```

更新 `main` 函数调用点：

```rust
    let ts_lifted = lift_functions(ts);
    let rs_lifted = lift_functions(rs);
    let ts_id_map = build_id_map(&ts_lifted);
    let rs_id_map = build_id_map(&rs_lifted);
    let ts_normalized = normalize(ts_lifted, &ts_id_map);
    let rs_normalized = normalize(rs_lifted, &rs_id_map);
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test id_remap
```
预期：3 个测试通过。

- [ ] **步骤 5：Commit**

```bash
git add tests/audit/ir-diff/src/main.rs
git commit -m "feat(ir-diff): node ID positional remapping + null guard strip + label normalize (F-007/F-008/F-011)"
```

---

## 任务 4：B 组 — ir-diff 端到端测试与 KnownDiffs 清空

**文件：**
- 修改：`tests/audit/ir-diff/src/main.rs`（添加端到端测试）
- 修改：`tests/audit/diff-ir.ps1`（清空 $KnownDiffs）

- [ ] **步骤 1：编写端到端归一化测试**

在 `tests` mod 中添加：

```rust
    #[test]
    fn end_to_end_expression_style_fixture_matches() {
        // Simulate TS IR (semantic IDs, flat structure, return label)
        let ts = json!({
            "nodes": [
                {"id": "entry1", "kind": "action", "label": "main"},
                {"id": "ret4", "kind": "terminal", "label": "return"}
            ],
            "edges": [
                {"from": "entry1", "to": "ret4", "kind": "control", "guard": null}
            ],
            "entryNodeId": "entry1"
        });
        // Simulate Rust IR (positional IDs, functions wrapper, exit label)
        let rs = json!({
            "functions": [{
                "nodes": [
                    {"id": "n0", "kind": "action", "label": "main"},
                    {"id": "n1", "kind": "terminal", "label": "exit"}
                ],
                "edges": [
                    {"from": "n0", "to": "n1", "kind": "control", "guard": null}
                ],
                "entryNodeId": "n0"
            }],
            "importedStdlib": [],
            "stdlibImports": []
        });

        let ts_lifted = lift_functions(ts);
        let rs_lifted = lift_functions(rs);
        let ts_id_map = build_id_map(&ts_lifted);
        let rs_id_map = build_id_map(&rs_lifted);
        let ts_norm = normalize(ts_lifted, &ts_id_map);
        let rs_norm = normalize(rs_lifted, &rs_id_map);
        assert_eq!(ts_norm, rs_norm, "TS and Rust IR should match after normalization");
    }

    #[test]
    fn null_guard_stripped() {
        let input = json!({
            "edges": [{"from": "n0", "to": "n1", "kind": "control", "guard": null}]
        });
        let id_map = std::collections::HashMap::new();
        let result = normalize(input, &id_map);
        assert!(result["edges"][0].get("guard").is_none(), "null guard should be stripped");
    }

    #[test]
    fn non_null_guard_preserved() {
        let input = json!({
            "edges": [{"from": "n0", "to": "n1", "kind": "condition", "guard": "x > 0"}]
        });
        let id_map = std::collections::HashMap::new();
        let result = normalize(input, &id_map);
        assert_eq!(result["edges"][0]["guard"], "x > 0");
    }

    #[test]
    fn return_label_normalized_to_exit() {
        let input = json!({
            "nodes": [{"id": "n0", "kind": "terminal", "label": "return"}]
        });
        let id_map = std::collections::HashMap::new();
        let result = normalize(input, &id_map);
        assert_eq!(result["nodes"][0]["label"], "exit");
    }
```

- [ ] **步骤 2：运行测试验证通过**

运行：
```bash
cargo test
```
预期：所有 8 个 ir-diff 单元测试通过。

- [ ] **步骤 3：清空 diff-ir.ps1 的 $KnownDiffs**

编辑 `tests/audit/diff-ir.ps1` 第 32-37 行：

```powershell
$KnownDiffs = @()  # F-007~F-012 closed in Phase 4 via ir-diff normalization
```

- [ ] **步骤 4：重新构建 ir-diff**

运行：
```bash
cd tests/audit/ir-diff
cargo build --release
```
预期：编译成功。

- [ ] **步骤 5：运行差分测试验证 4 fixture 转 MATCH**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase4-v0.4.0
powershell -ExecutionPolicy Bypass -File tests/audit/diff-ir.ps1
```
预期：expression/hello/user/payment 4 fixture 输出 `[MATCH]`，0 KNOWN_DIFF，0 unexpected DIFF，exit code 0。

- [ ] **步骤 6：Commit**

```bash
git add tests/audit/ir-diff/src/main.rs tests/audit/diff-ir.ps1
git commit -m "feat(ir-diff): end-to-end normalization tests + clear KnownDiffs (F-007~F-012 closed)"
```

---

## 任务 5：A 组 — Python if/elif 分支发射

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/py_emitter.rs`
- 创建：`compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs`
- 修改：`compiler/tangle-cli/Cargo.toml`

- [ ] **步骤 1：在 Cargo.toml 注册测试**

在 `compiler/tangle-cli/Cargo.toml` 最后一个 `[[test]]` 条目后添加：

```toml
[[test]]
name = "phase4_py_go_codegen"
path = "tests/v04_phase4/py_go_codegen.rs"

[[test]]
name = "phase4_toggle_lowering"
path = "tests/v04_phase4/toggle_lowering.rs"
```

- [ ] **步骤 2：编写失败的测试**

创建 `compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs`：

```rust
use tangle_cli::codegen::py_emitter::emit_python;
use tangle_cli::codegen::go_emitter::emit_go;
use tangle_cli::ir::graph::*;

fn make_decision_graph() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode { id: "entry".into(), kind: IRNodeKind::Decision, label: "check".into(),
                source_span: None, source_text: None, group: None, style: None },
            IRNode { id: "auto".into(), kind: IRNodeKind::Action, label: "auto_approve".into(),
                source_span: None, source_text: None, group: None, style: None },
            IRNode { id: "manual".into(), kind: IRNodeKind::Action, label: "manual_review".into(),
                source_span: None, source_text: None, group: None, style: None },
            IRNode { id: "end".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None },
        ],
        edges: vec![
            IREdge { from: "entry".into(), to: "auto".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 100".into()), source_span: None,
                priority: Some(0), style: None },
            IREdge { from: "entry".into(), to: "manual".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 1000".into()), source_span: None,
                priority: Some(1), style: None },
            IREdge { from: "auto".into(), to: "end".into(), kind: IREdgeKind::Dashed,
                guard: None, source_span: None, priority: None, style: None },
            IREdge { from: "manual".into(), to: "end".into(), kind: IREdgeKind::Thick,
                guard: None, source_span: None, priority: None, style: None },
        ],
        error_edges: vec![], entry_node_id: "entry".into(),
        imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
    }
}

#[test]
fn py_decision_if_elif_emission() {
    let graph = make_decision_graph();
    let py = emit_python(&graph, "ApprovalFlow");
    assert!(py.contains("if (amount < 100):"), "Py should emit if with guard, got:\n{}", py);
    assert!(py.contains("elif (amount < 1000):"), "Py should emit elif, got:\n{}", py);
}

#[test]
fn py_priority_ordering() {
    let graph = make_decision_graph();
    let py = emit_python(&graph, "ApprovalFlow");
    let if_pos = py.find("if (amount < 100)").unwrap();
    let elif_pos = py.find("elif (amount < 1000)").unwrap();
    assert!(if_pos < elif_pos, "priority 0 guard should come before priority 1");
}

#[test]
fn py_branch_body_recursion() {
    let graph = make_decision_graph();
    let py = emit_python(&graph, "ApprovalFlow");
    // Branch body should contain action comments for auto/manual
    assert!(py.contains("# action: auto_approve"), "Py branch body should recurse, got:\n{}", py);
    assert!(py.contains("# action: manual_review"), "Py branch body should recurse");
}
```

- [ ] **步骤 3：运行测试验证失败**

运行：
```bash
cargo test --test phase4_py_go_codegen py_decision_if_elif
```
预期：FAIL，Py 输出不含 `if (` 和 `elif (`。

- [ ] **步骤 4：实现 Python if/elif 分支发射**

在 `py_emitter.rs` 中添加 3 个辅助函数（`emit_node_comments`、`emit_edge_comments`、`sort_edges_by_priority`、`emit_branch_body`、`emit_decision_branch`），然后改造 BFS 循环。

在 `emit_python` 函数之前添加：

```rust
/// Emit metadata comments (group, style) for a node. Returns comment lines with given indent.
fn emit_node_comments(node: &IRNode, indent: &str) -> String {
    let mut out = String::new();
    if let Some(ref group) = node.group {
        out.push_str(&format!("{}# group: {}\n", indent, group));
    }
    if let Some(ref style) = node.style {
        out.push_str(&format!("{}# style: {}\n", indent, style));
    }
    out
}

/// Emit metadata comments for an edge kind and style.
fn emit_edge_comments(edge: &IREdge, indent: &str) -> String {
    let mut out = String::new();
    match edge.kind {
        IREdgeKind::Dashed => out.push_str(&format!("{}# edge: dashed\n", indent)),
        IREdgeKind::Thick => out.push_str(&format!("{}# edge: thick\n", indent)),
        IREdgeKind::Crossed => out.push_str(&format!("{}# edge: crossed\n", indent)),
        IREdgeKind::Control | IREdgeKind::Condition | IREdgeKind::Error => {}
    }
    if let Some(ref style) = edge.style {
        out.push_str(&format!("{}# edge-style: {}\n", indent, style));
    }
    out
}

/// Sort edges by priority (lower = higher precedence). Edges without priority sort last.
fn sort_edges_by_priority<'a>(edges: &[&'a IREdge]) -> Vec<&'a IREdge> {
    let mut sorted = edges.to_vec();
    sorted.sort_by(|a, b| {
        match (a.priority, b.priority) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    sorted
}

/// Recursively emit the body of a branch target inside an if/elif block (Python).
fn emit_branch_body<'a>(
    target_id: &'a str,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
) -> String {
    let mut out = String::new();
    if visited.contains(target_id) {
        return out;
    }
    visited.insert(target_id);

    let node = match nodes.iter().find(|n| n.id == target_id) {
        Some(n) => n,
        None => return out,
    };

    out.push_str(&emit_node_comments(node, indent));

    match node.kind {
        IRNodeKind::Action | IRNodeKind::Compute | IRNodeKind::Decision => {
            let label = node.label.replace('\'', "\\'");
            out.push_str(&format!("{}# {}: {}\n", indent,
                match node.kind {
                    IRNodeKind::Action => "action",
                    IRNodeKind::Compute => "compute",
                    IRNodeKind::Decision => "decision",
                    _ => "step",
                },
                label
            ));
        }
        IRNodeKind::Terminal => {
            out.push_str(&format!("{}return Ok(None)\n", indent));
        }
        IRNodeKind::ErrorTerminal => {
            let label = node.label.replace('\'', "\\'");
            out.push_str(&format!("{}return Err('{}')\n", indent, label));
        }
    }

    // Recurse into non-Crossed successors
    for edge in edges {
        if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, visited, indent));
        }
    }
    out
}

/// Emit if/elif/else chain for a Decision node with guarded outgoing edges (Python).
fn emit_decision_branch<'a>(
    node: &IRNode,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
) -> String {
    let mut out = String::new();
    let out_edges: Vec<&IREdge> = edges.iter().filter(|e| e.from == node.id).collect();

    let mut guarded: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.guard.is_some() && e.kind != IREdgeKind::Crossed)
        .copied()
        .collect();
    let unguarded: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.guard.is_none() && e.kind != IREdgeKind::Crossed)
        .copied()
        .collect();
    let crossed: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.kind == IREdgeKind::Crossed)
        .copied()
        .collect();

    guarded = sort_edges_by_priority(&guarded);

    if guarded.is_empty() && unguarded.is_empty() {
        out.push_str(&format!("{}# decision: {} (no guarded branches)\n", indent, node.label));
        for e in &crossed {
            out.push_str(&format!("{}# skipped: crossed edge to {}\n", indent, e.to));
        }
        return out;
    }

    let inner_indent = format!("{}    ", indent);
    let mut all_branch_visited: HashSet<&str> = HashSet::new();
    let mut has_body = false;

    for (i, edge) in guarded.iter().enumerate() {
        out.push_str(&emit_edge_comments(edge, indent));
        let guard = edge.guard.as_ref().unwrap();
        if i == 0 {
            out.push_str(&format!("{}if ({}):\n", indent, guard));
        } else {
            out.push_str(&format!("{}elif ({}):\n", indent, guard));
        }
        let mut branch_visited = visited.clone();
        let body = emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent);
        if body.is_empty() {
            out.push_str(&format!("{}pass\n", inner_indent));
        } else {
            out.push_str(&body);
        }
        all_branch_visited.extend(branch_visited.iter().copied());
        has_body = true;
    }

    if !unguarded.is_empty() {
        out.push_str(&format!("{}else:\n", indent));
        for edge in &unguarded {
            out.push_str(&emit_edge_comments(edge, &inner_indent));
            let mut branch_visited = visited.clone();
            let body = emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent);
            if body.is_empty() {
                out.push_str(&format!("{}pass\n", inner_indent));
            } else {
                out.push_str(&body);
            }
            all_branch_visited.extend(branch_visited.iter().copied());
        }
        has_body = true;
    }

    if !has_body {
        out.push_str(&format!("{}pass\n", indent));
    }

    visited.extend(all_branch_visited.iter().copied());

    for edge in &crossed {
        out.push_str(&format!("{}# skipped: crossed edge to {}\n", indent, edge.to));
    }

    out
}
```

- [ ] **步骤 5：改造 BFS 循环**

在 `emit_python` 的 `while let Some(node) = queue.pop_front()` 循环中，将 `IRNodeKind::Action | IRNodeKind::Compute | IRNodeKind::Decision` 分支拆分，Decision 节点检测 guarded 边：

将第 65-86 行的 match 块替换为：

```rust
        match node.kind {
            IRNodeKind::Action | IRNodeKind::Compute => {
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!(
                    "    # {}: {}\n",
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        _ => "step",
                    },
                    label
                ));
            }
            IRNodeKind::Decision => {
                let has_guarded = graph.edges.iter().any(|e| {
                    e.from == node.id && e.guard.is_some() && e.kind != IREdgeKind::Crossed
                });
                if has_guarded {
                    out.push_str(&emit_decision_branch(node, &graph.nodes, &graph.edges, &mut visited, "    "));
                } else {
                    let label = node.label.replace('\'', "\\'");
                    out.push_str(&format!("    # decision: {}\n", label));
                }
            }
            IRNodeKind::Terminal => {
                out.push_str("    return Ok(None)\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('\'', "\\'");
                out.push_str(&format!("    return Err('{}')\n", label));
            }
        }
```

并将边遍历部分（第 88-103 行）改为跳过 Decision 节点的 guarded 边入队：

```rust
        for edge in &graph.edges {
            if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
                // Decision 节点的 guarded 边已在 if/elif 中处理，跳过入队
                if node.kind == IRNodeKind::Decision && edge.guard.is_some() {
                    continue;
                }
                match edge.kind {
                    IREdgeKind::Dashed => out.push_str("    # edge: dashed\n"),
                    IREdgeKind::Thick => out.push_str("    # edge: thick\n"),
                    IREdgeKind::Crossed => out.push_str("    # edge: crossed\n"),
                    IREdgeKind::Control | IREdgeKind::Condition | IREdgeKind::Error => {}
                }
                if let Some(ref style) = edge.style {
                    out.push_str(&format!("    # edge-style: {}\n", style));
                }
                if let Some(target) = graph.nodes.iter().find(|n| n.id == edge.to) {
                    queue.push_back(target);
                }
            }
        }
```

- [ ] **步骤 6：运行测试验证通过**

运行：
```bash
cargo test --test phase4_py_go_codegen py_decision py_priority py_branch_body
```
预期：3 个 Py 测试通过。

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/codegen/py_emitter.rs compiler/tangle-cli/Cargo.toml compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs
git commit -m "feat(py-emitter): if/elif/else branch emission for Decision nodes (Phase 4 A组)"
```

---

## 任务 6：A 组 — Go if/else if 分支发射

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/go_emitter.rs`
- 修改：`compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs`

- [ ] **步骤 1：编写失败的测试**

在 `compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs` 末尾追加 Go 测试：

```rust
#[test]
fn go_decision_if_else_emission() {
    let graph = make_decision_graph();
    let go = emit_go(&graph, "ApprovalFlow");
    // Go requires } else if on the same line
    assert!(go.contains("if amount < 100 {"), "Go should emit if with guard, got:\n{}", go);
    assert!(go.contains("} else if amount < 1000 {"), "Go should emit } else if, got:\n{}", go);
}

#[test]
fn go_priority_ordering() {
    let graph = make_decision_graph();
    let go = emit_go(&graph, "ApprovalFlow");
    let if_pos = go.find("if amount < 100 {").unwrap();
    let else_if_pos = go.find("} else if amount < 1000 {").unwrap();
    assert!(if_pos < else_if_pos, "priority 0 guard should come before priority 1");
}

#[test]
fn go_branch_body_recursion() {
    let graph = make_decision_graph();
    let go = emit_go(&graph, "ApprovalFlow");
    assert!(go.contains("// action: auto_approve"), "Go branch body should recurse, got:\n{}", go);
    assert!(go.contains("// action: manual_review"), "Go branch body should recurse");
}

#[test]
fn py_go_crossed_edge_skipped() {
    let mut graph = make_decision_graph();
    // Add a crossed edge from entry to a reject node
    graph.nodes.push(IRNode {
        id: "reject".into(), kind: IRNodeKind::Action, label: "reject".into(),
        source_span: None, source_text: None, group: None, style: None,
    });
    graph.edges.push(IREdge {
        from: "entry".into(), to: "reject".into(), kind: IREdgeKind::Crossed,
        guard: None, source_span: None, priority: None, style: None,
    });
    let py = emit_python(&graph, "TestFlow");
    let go = emit_go(&graph, "TestFlow");
    assert!(py.contains("# skipped: crossed edge to reject"), "Py should emit crossed skip comment, got:\n{}", py);
    assert!(go.contains("// skipped: crossed edge to reject"), "Go should emit crossed skip comment, got:\n{}", go);
    // Crossed target should NOT appear as action comment in branch body
    assert!(!py.contains("# action: reject"), "Py should not emit crossed target as action");
    assert!(!go.contains("// action: reject"), "Go should not emit crossed target as action");
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --test phase4_py_go_codegen go_decision go_priority go_branch_body py_go_crossed
```
预期：FAIL，Go 输出不含 `if amount` 和 `} else if`。

- [ ] **步骤 3：实现 Go if/else if 分支发射**

在 `go_emitter.rs` 中添加与 Py 类似的辅助函数，但用 Go 语法（`//` 注释、`return Ok(nil)`、`} else if` 同行）。

在 `emit_go` 函数之前添加：

```rust
/// Emit metadata comments (group, style) for a node. Returns comment lines with given indent.
fn emit_node_comments(node: &IRNode, indent: &str) -> String {
    let mut out = String::new();
    if let Some(ref group) = node.group {
        out.push_str(&format!("{}// group: {}\n", indent, group));
    }
    if let Some(ref style) = node.style {
        out.push_str(&format!("{}// style: {}\n", indent, style));
    }
    out
}

/// Emit metadata comments for an edge kind and style.
fn emit_edge_comments(edge: &IREdge, indent: &str) -> String {
    let mut out = String::new();
    match edge.kind {
        IREdgeKind::Dashed => out.push_str(&format!("{}// edge: dashed\n", indent)),
        IREdgeKind::Thick => out.push_str(&format!("{}// edge: thick\n", indent)),
        IREdgeKind::Crossed => out.push_str(&format!("{}// edge: crossed\n", indent)),
        IREdgeKind::Control | IREdgeKind::Condition | IREdgeKind::Error => {}
    }
    if let Some(ref style) = edge.style {
        out.push_str(&format!("{}// edge-style: {}\n", indent, style));
    }
    out
}

/// Sort edges by priority (lower = higher precedence). Edges without priority sort last.
fn sort_edges_by_priority<'a>(edges: &[&'a IREdge]) -> Vec<&'a IREdge> {
    let mut sorted = edges.to_vec();
    sorted.sort_by(|a, b| {
        match (a.priority, b.priority) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    sorted
}

/// Recursively emit the body of a branch target inside an if/else block (Go).
fn emit_branch_body<'a>(
    target_id: &'a str,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
) -> String {
    let mut out = String::new();
    if visited.contains(target_id) {
        return out;
    }
    visited.insert(target_id);

    let node = match nodes.iter().find(|n| n.id == target_id) {
        Some(n) => n,
        None => return out,
    };

    out.push_str(&emit_node_comments(node, indent));

    match node.kind {
        IRNodeKind::Action | IRNodeKind::Compute | IRNodeKind::Decision => {
            out.push_str(&format!("{}// {}: {}\n", indent,
                match node.kind {
                    IRNodeKind::Action => "action",
                    IRNodeKind::Compute => "compute",
                    IRNodeKind::Decision => "decision",
                    _ => "step",
                },
                node.label
            ));
        }
        IRNodeKind::Terminal => {
            out.push_str(&format!("{}return Ok(nil)\n", indent));
        }
        IRNodeKind::ErrorTerminal => {
            let label = node.label.replace('"', "\\\"");
            out.push_str(&format!("{}return Err(\"{}\")\n", indent, label));
        }
    }

    for edge in edges {
        if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, visited, indent));
        }
    }
    out
}

/// Emit if/else if/else chain for a Decision node with guarded outgoing edges (Go).
/// Key difference from JS/Py: Go requires `} else if` on the same line.
fn emit_decision_branch<'a>(
    node: &IRNode,
    nodes: &'a [IRNode],
    edges: &'a [IREdge],
    visited: &mut HashSet<&'a str>,
    indent: &str,
) -> String {
    let mut out = String::new();
    let out_edges: Vec<&IREdge> = edges.iter().filter(|e| e.from == node.id).collect();

    let mut guarded: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.guard.is_some() && e.kind != IREdgeKind::Crossed)
        .copied()
        .collect();
    let unguarded: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.guard.is_none() && e.kind != IREdgeKind::Crossed)
        .copied()
        .collect();
    let crossed: Vec<&IREdge> = out_edges.iter()
        .filter(|e| e.kind == IREdgeKind::Crossed)
        .copied()
        .collect();

    guarded = sort_edges_by_priority(&guarded);

    if guarded.is_empty() && unguarded.is_empty() {
        out.push_str(&format!("{}// decision: {} (no guarded branches)\n", indent, node.label));
        for e in &crossed {
            out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, e.to));
        }
        return out;
    }

    let inner_indent = format!("{}    ", indent);
    let mut all_branch_visited: HashSet<&str> = HashSet::new();

    for (i, edge) in guarded.iter().enumerate() {
        out.push_str(&emit_edge_comments(edge, indent));
        let guard = edge.guard.as_ref().unwrap();
        if i == 0 {
            out.push_str(&format!("{}if {} {{\n", indent, guard));
        } else {
            // Go requires } else if on the same line
            out.push_str(&format!("{}}} else if {} {{\n", indent, guard));
        }
        let mut branch_visited = visited.clone();
        out.push_str(&emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent));
        all_branch_visited.extend(branch_visited.iter().copied());
    }

    if !unguarded.is_empty() {
        // Go requires } else on the same line
        out.push_str(&format!("{}}} else {{\n", indent));
        for edge in &unguarded {
            out.push_str(&emit_edge_comments(edge, &inner_indent));
            let mut branch_visited = visited.clone();
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, &mut branch_visited, &inner_indent));
            all_branch_visited.extend(branch_visited.iter().copied());
        }
        out.push_str(&format!("{}}}\n", indent));
    } else if !guarded.is_empty() {
        // Close the last if/else-if block
        out.push_str(&format!("{}}}\n", indent));
    }

    visited.extend(all_branch_visited.iter().copied());

    for edge in &crossed {
        out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, edge.to));
    }

    out
}
```

- [ ] **步骤 4：改造 BFS 循环**

在 `emit_go` 的 `while let Some(node) = queue.pop_front()` 循环中，将 `IRNodeKind::Action | IRNodeKind::Compute | IRNodeKind::Decision` 分支拆分：

将第 63-83 行的 match 块替换为：

```rust
        match node.kind {
            IRNodeKind::Action | IRNodeKind::Compute => {
                out.push_str(&format!(
                    "    // {}: {}\n",
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        _ => "step",
                    },
                    node.label
                ));
            }
            IRNodeKind::Decision => {
                let has_guarded = graph.edges.iter().any(|e| {
                    e.from == node.id && e.guard.is_some() && e.kind != IREdgeKind::Crossed
                });
                if has_guarded {
                    out.push_str(&emit_decision_branch(node, &graph.nodes, &graph.edges, &mut visited, "    "));
                } else {
                    out.push_str(&format!("    // decision: {}\n", node.label));
                }
            }
            IRNodeKind::Terminal => {
                out.push_str("    return Ok(nil)\n");
            }
            IRNodeKind::ErrorTerminal => {
                let label = node.label.replace('"', "\\\"");
                out.push_str(&format!("    return Err(\"{}\")\n", label));
            }
        }
```

并将边遍历部分（第 85-100 行）改为跳过 Decision 节点的 guarded 边入队：

```rust
        for edge in &graph.edges {
            if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
                if node.kind == IRNodeKind::Decision && edge.guard.is_some() {
                    continue;
                }
                match edge.kind {
                    IREdgeKind::Dashed => out.push_str("    // edge: dashed\n"),
                    IREdgeKind::Thick => out.push_str("    // edge: thick\n"),
                    IREdgeKind::Crossed => out.push_str("    // edge: crossed\n"),
                    IREdgeKind::Control | IREdgeKind::Condition | IREdgeKind::Error => {}
                }
                if let Some(ref style) = edge.style {
                    out.push_str(&format!("    // edge-style: {}\n", style));
                }
                if let Some(target) = graph.nodes.iter().find(|n| n.id == edge.to) {
                    queue.push_back(target);
                }
            }
        }
```

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
cargo test --test phase4_py_go_codegen
```
预期：全部 8 个 A 组测试通过（3 Py + 4 Go + 1 crossed）。

- [ ] **步骤 6：Commit**

```bash
git add compiler/tangle-cli/src/codegen/go_emitter.rs compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs
git commit -m "feat(go-emitter): if/else if/else branch emission for Decision nodes (Phase 4 A组)"
```

---

## 任务 7：A 组 — 更新 Phase 3 快照

**文件：**
- 修改：`compiler/tangle-cli/tests/v03_phase3/snapshots/`（insta 自动生成）

- [ ] **步骤 1：运行快照测试查看失败**

运行：
```bash
cargo test --test snapshots
```
预期：`snapshot_py_metadata_comments` 和 `snapshot_go_metadata_comments` 失败（输出变化）。

- [ ] **步骤 2：重新生成快照**

运行：
```bash
$env:INSTA_UPDATE="always"
cargo test --test snapshots
Remove-Item Env:\INSTA_UPDATE
```
预期：快照文件更新。

- [ ] **步骤 3：审查新快照**

运行：
```bash
cargo test --test snapshots -v
```
检查 `snapshots.rs` 同目录下 `.snap` 文件，确认 Py 快照包含 `if (` / `elif (`，Go 快照包含 `if ` / `} else if`。

- [ ] **步骤 4：验证快照测试通过**

运行：
```bash
cargo test --test snapshots
```
预期：全部快照测试通过。

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/tests/v03_phase3/snapshots/
git commit -m "test(snapshots): update Py/Go snapshots for if/else branch emission (Phase 4 A组)"
```

---

## 任务 8：C 组 — Toggle 签名变更 + compile_to_ir 同步

**文件：**
- 修改：`compiler/tangle-cli/src/ir/lower_rule_toggle.rs`
- 修改：`compiler/tangle-cli/src/ir/compile_to_ir.rs`

- [ ] **步骤 1：修改 lower_rule_toggle 签名**

在 `compiler/tangle-cli/src/ir/lower_rule_toggle.rs` 中：

第 1 行添加 import：
```rust
use crate::ir::graph::*;
use crate::model::{SourceSpan, TangleDiagnostic};
```

第 3 行改为：
```rust
pub fn lower_rule_toggle(checkbox_markdown: &str, file: &str, id_gen: &mut FreshNodeId) -> (RuleGraph, Vec<TangleDiagnostic>) {
```

第 4 行后添加：
```rust
    let mut diagnostics = vec![];
```

第 61 行（`graph` 返回）改为：
```rust
    (graph, diagnostics)
```

- [ ] **步骤 2：修改 compile_to_ir 调用点**

在 `compiler/tangle-cli/src/ir/compile_to_ir.rs` 第 99-101 行改为：

```rust
                RuleKind::Toggle => lower_rule_toggle(&rule.source, file, id_gen),
```

（去掉 `( ... , vec![])` 包装，直接调用）

- [ ] **步骤 3：验证编译通过**

运行：
```bash
cargo build --workspace
```
预期：编译成功。

- [ ] **步骤 4：Commit**

```bash
git add compiler/tangle-cli/src/ir/lower_rule_toggle.rs compiler/tangle-cli/src/ir/compile_to_ir.rs
git commit -m "refactor(toggle): change signature to (RuleGraph, Vec<TangleDiagnostic>) for consistency"
```

---

## 任务 9：C 组 — Toggle Span 跟踪 + 名称提取增强 + 诊断

**文件：**
- 修改：`compiler/tangle-cli/src/ir/lower_rule_toggle.rs`
- 创建：`compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs`

- [ ] **步骤 1：编写失败的测试**

创建 `compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs`：

```rust
use tangle_cli::ir::graph::*;
use tangle_cli::ir::lower_rule_toggle::lower_rule_toggle;
use tangle_cli::model::TangleDiagnostic;

fn fresh_id_gen() -> FreshNodeId {
    FreshNodeId::new()
}

#[test]
fn toggle_span_tracking_populates_line_numbers() {
    let md = "- [x] enable_new_ui: true\n- [ ] enable_crypto: false";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    // nodes[0] is entry (span None), nodes[1] and nodes[2] are checkboxes
    assert!(graph.nodes[1].source_span.is_some(), "checkbox node should have span");
    let span = graph.nodes[1].source_span.as_ref().unwrap();
    assert_eq!(span.start_line, 1, "first checkbox on line 1");
    assert_eq!(span.file, "test.tangle");
    let span2 = graph.nodes[2].source_span.as_ref().unwrap();
    assert_eq!(span2.start_line, 2, "second checkbox on line 2");
}

#[test]
fn toggle_name_extraction_from_colon() {
    let md = "- [x] enable_new_ui: true";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert_eq!(graph.nodes[1].label, "enable_new_ui = true");
}

#[test]
fn toggle_name_extraction_from_backtick() {
    let md = "- [x] `enable_new_ui`: some description";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert_eq!(graph.nodes[1].label, "enable_new_ui = true");
}

#[test]
fn toggle_missing_name_emits_diagnostic() {
    let md = "- [x] no name here";
    let (_graph, diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_TOGGLE_MISSING_NAME"),
        "should emit MISSING_NAME diagnostic, got: {:?}", diags);
}

#[test]
fn toggle_malformed_checkbox_emits_diagnostic() {
    let md = "- [?] invalid checkbox";
    let (_graph, diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_TOGGLE_MALFORMED"),
        "should emit MALFORMED diagnostic, got: {:?}", diags);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --test phase4_toggle_lowering
```
预期：FAIL（span 全为 None、冒号格式名称回退为 "flag"）。

- [ ] **步骤 3：实现 span 跟踪 + 名称提取 + 诊断**

将 `lower_rule_toggle.rs` 的整个函数体替换为：

```rust
pub fn lower_rule_toggle(checkbox_markdown: &str, file: &str, id_gen: &mut FreshNodeId) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
    let entry_id = id_gen.fresh();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(),
        kind: IRNodeKind::Compute,
        label: "toggle.entry".into(),
        source_span: None, source_text: None,
        group: None, style: None,
    });

    let mut pending_group: Option<String> = None;
    let mut pending_style: Option<String> = None;
    let mut toggle_index = 0u32;

    for (line_idx, line) in checkbox_markdown.lines().enumerate() {
        let line_no = line_idx + 1; // 1-based
        let t = line.trim_start();

        // Check for HTML comment metadata: <!-- group: X --> or <!-- style: Y -->
        if let Some(meta) = parse_html_comment(t) {
            match meta {
                ("group", value) => pending_group = Some(value),
                ("style", value) => pending_style = Some(value),
                _ => {}
            }
            continue;
        }

        // Skip non-checkbox lines (but clear pending metadata)
        if !t.starts_with("- [") && !t.starts_with("* [") {
            if !t.is_empty() && !t.starts_with("<!--") {
                pending_group = None;
                pending_style = None;
            }
            continue;
        }

        // Detect malformed checkbox: starts with - [ or * [ but doesn't contain [x]/[X]/[ ]
        let is_valid = t.contains("[x]") || t.contains("[X]") || t.contains("[ ]");
        if !is_valid {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_RULE_TOGGLE_MALFORMED".into(),
                message: format!("malformed checkbox: expected [x], [X], or [ ]: {}", t),
                span: SourceSpan {
                    file: file.into(),
                    start_line: line_no, start_column: 0,
                    end_line: line_no, end_column: 0,
                },
            });
            continue;
        }

        let checked = t.contains("[x]") || t.contains("[X]");
        let rest = t
            .trim_start_matches("- [x]")
            .trim_start_matches("- [X]")
            .trim_start_matches("- [ ]")
            .trim_start_matches("* [x]")
            .trim_start_matches("* [X]")
            .trim_start_matches("* [ ]")
            .trim();

        // Extract name: backtick first, then colon, then fallback
        let name = extract_name(rest);
        let name = match name {
            Some(n) => n,
            None => {
                diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_RULE_TOGGLE_MISSING_NAME".into(),
                    message: format!("could not extract toggle name from: {}", rest),
                    span: SourceSpan {
                        file: file.into(),
                        start_line: line_no, start_column: 0,
                        end_line: line_no, end_column: 0,
                    },
                });
                format!("toggle_{}", toggle_index)
            }
        };

        let node_id = id_gen.fresh();
        graph.nodes.push(IRNode {
            id: node_id.clone(),
            kind: IRNodeKind::Compute,
            label: format!("{} = {}", name, checked),
            source_span: Some(SourceSpan {
                file: file.into(),
                start_line: line_no, start_column: 0,
                end_line: line_no, end_column: 0,
            }),
            source_text: None,
            group: pending_group.take(),
            style: pending_style.take(),
        });
        graph.edges.push(IREdge {
            from: entry_id.clone(),
            to: node_id,
            kind: IREdgeKind::Control,
            guard: None,
            source_span: None,
            priority: None, style: None,
        });
        toggle_index += 1;
    }

    (graph, diagnostics)
}

/// Extract toggle name from the rest of a checkbox line.
/// Priority: backtick (`name`) > colon (name: value) > None.
fn extract_name(rest: &str) -> Option<String> {
    // 1. Backtick: `name`: desc
    if let Some(tick_start) = rest.find('`') {
        let after_tick = &rest[tick_start + 1..];
        if let Some(tick_end) = after_tick.find('`') {
            return Some(after_tick[..tick_end].to_string());
        }
    }
    // 2. Colon: name: value (name must be a valid identifier)
    if let Some(colon_pos) = rest.find(':') {
        let candidate = rest[..colon_pos].trim();
        if is_valid_identifier(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Check if a string is a valid identifier: [a-zA-Z_][a-zA-Z0-9_]*
fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse an HTML comment line like `<!-- group: X -->` or `<!-- style: Y -->`.
/// Returns (key, value) if the comment matches the metadata pattern.
fn parse_html_comment(line: &str) -> Option<(&'static str, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("<!--") || !trimmed.ends_with("-->") {
        return None;
    }
    let inner = trimmed[4..trimmed.len() - 3].trim();
    if let Some(rest) = inner.strip_prefix("group:") {
        return Some(("group", rest.trim().to_string()));
    }
    if let Some(rest) = inner.strip_prefix("style:") {
        return Some(("style", rest.trim().to_string()));
    }
    None
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test --test phase4_toggle_lowering
```
预期：5 个测试通过。

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/ir/lower_rule_toggle.rs compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs
git commit -m "feat(toggle): span tracking + name extraction + diagnostics (Phase 4 C组)"
```

---

## 任务 10：C 组 — Toggle group/style 元数据 + Fixture 更新

**文件：**
- 修改：`compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs`
- 修改：`tests/rules/feature-toggles.tangle.md`

- [ ] **步骤 1：编写 group/style 测试**

在 `compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs` 末尾追加：

```rust
#[test]
fn toggle_group_style_metadata_attached() {
    let md = "<!-- group: Approval -->\n<!-- style: highlight -->\n- [x] enable_new_ui: true";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    // nodes[0] is entry, nodes[1] is the checkbox
    assert_eq!(graph.nodes[1].group.as_deref(), Some("Approval"), "group should be attached");
    assert_eq!(graph.nodes[1].style.as_deref(), Some("highlight"), "style should be attached");
}

#[test]
fn toggle_group_style_pending_cleared_on_non_checkbox() {
    let md = "<!-- group: Approval -->\nSome text\n- [x] enable_new_ui: true";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    // The "Some text" line should clear the pending group
    assert!(graph.nodes[1].group.is_none(), "group should be cleared by non-checkbox line");
}

#[test]
fn toggle_group_style_survives_blank_lines() {
    let md = "<!-- group: Approval -->\n\n- [x] enable_new_ui: true";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert_eq!(graph.nodes[1].group.as_deref(), Some("Approval"), "group should survive blank line");
}

#[test]
fn toggle_signature_returns_diagnostics_vec() {
    let md = "- [x] ok: true";
    let result = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    // Verify the return type is a tuple
    let (_graph, _diags): (RuleGraph, Vec<TangleDiagnostic>) = result;
    // If this compiles, the signature is correct
}
```

- [ ] **步骤 2：运行测试验证通过**

运行：
```bash
cargo test --test phase4_toggle_lowering
```
预期：全部 9 个 C 组测试通过（含 group/style 4 个）。

- [ ] **步骤 3：更新 feature-toggles fixture**

将 `tests/rules/feature-toggles.tangle.md` 内容替换为：

```markdown
# ToggleTest

##### Rule: Features

<!-- group: UI -->
<!-- style: highlight -->
- [x] enable_new_ui: true

<!-- group: Security -->
- [ ] enable_crypto: false
- [x] enable_ai: true
```

- [ ] **步骤 4：验证 fixture 端到端**

运行：
```bash
cargo run -- build tests/rules/feature-toggles.tangle.md --emit-ir
```
预期：输出的 IR JSON 中，checkbox 节点含 `sourceSpan`（行号 ≠ 0）、正确名称（`enable_new_ui` 而非 `flag`）、`group`/`style` 字段。

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs tests/rules/feature-toggles.tangle.md
git commit -m "feat(toggle): group/style metadata + fixture update (Phase 4 C组)"
```

---

## 任务 11：出口闸门验证

**文件：**
- 无文件修改，仅验证

- [ ] **步骤 1：闸门 1 — 全工作区测试**

运行：
```bash
cargo test --workspace
```
预期：所有测试通过。

- [ ] **步骤 2：闸门 2 — Clippy 零警告**

运行：
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
预期：0 警告。如有警告，修复后重新运行。

- [ ] **步骤 3：闸门 3 — 审计回归**

运行：
```bash
powershell -ExecutionPolicy Bypass -File tests/audit/run-audit.ps1
```
预期：0 failing。

- [ ] **步骤 4：闸门 4 — 差分测试**

运行：
```bash
powershell -ExecutionPolicy Bypass -File tests/audit/diff-ir.ps1
```
预期：0 KNOWN_DIFF + 0 unexpected DIFF，exit code 0。

- [ ] **步骤 5：闸门 5 — Phase 3 回归**

运行：
```bash
cargo test --test js_codegen --test py_go_codegen --test span_tracking
```
预期：全绿。

- [ ] **步骤 6：闸门 5b — Phase 3 快照**

运行：
```bash
cargo test --test snapshots
```
预期：全绿（已在任务 7 更新）。

- [ ] **步骤 7：闸门 6 — Phase 4 新测试**

运行：
```bash
cargo test --test phase4_py_go_codegen --test phase4_toggle_lowering
cargo test --manifest-path tests/audit/ir-diff/Cargo.toml
```
预期：全绿。

- [ ] **步骤 8：闸门 7 — Toggle fixture IR 验证**

运行：
```bash
cargo run -- build tests/rules/feature-toggles.tangle.md --emit-ir
```
验证输出：
- checkbox 节点含 `sourceSpan`，`startLine` 分别为 5、8、9
- 节点 label 为 `enable_new_ui = true`、`enable_crypto = false`、`enable_ai = true`
- `enable_new_ui` 节点 `group` 为 `"UI"`，`style` 为 `"highlight"`
- `enable_crypto` 和 `enable_ai` 节点 `group` 为 `"Security"`

- [ ] **步骤 9：合并到 main 并打 tag**

运行：
```bash
cd e:\GitProjects\tangle
git checkout main
git merge --no-ff phase4/v0.4.0 -m "merge: Phase 4 — Py/Go if/else + ir-diff normalization + toggle enhancement (v0.4.0)"
git tag v0.4.0
```

- [ ] **步骤 10：最终验证**

运行：
```bash
git log --oneline -5
git tag --list "v0.4*"
```
预期：main 分支含 Phase 4 merge commit，tag `v0.4.0` 存在。

**注意：** 不要 push 到远程，除非用户明确要求。

---

## 自检

### 规格覆盖度

| 规格章节 | 对应任务 |
|---------|---------|
| §3 A 组 Py/Go if/else | 任务 5 (Py) + 任务 6 (Go) |
| §3.6 A 组测试 | 任务 5/6 测试 + 任务 7 快照 |
| §4 B 组 ir-diff 归一化 | 任务 2 (F-009) + 任务 3 (F-007/008/011) + 任务 4 (端到端) |
| §4.6 B 组测试 | 任务 2/3/4 内联测试 |
| §5 C 组 toggle 签名 | 任务 8 |
| §5.3 C 组 span 跟踪 | 任务 9 |
| §5.4 C 组 名称提取 | 任务 9 |
| §5.5 C 组 诊断 | 任务 9 |
| §5.6 C 组 group/style | 任务 10 |
| §5.9 C 组测试 | 任务 9 + 任务 10 |
| §7.2 回归测试 | 任务 11 闸门 1-6 |
| §8 出口闸门 | 任务 11 全部步骤 |

无遗漏。

### 占位符扫描

无 TODO、待定、FIXME。所有步骤含完整代码块。

### 类型一致性

- `lower_rule_toggle` 签名：任务 8 定义为 `(RuleGraph, Vec<TangleDiagnostic>)`，任务 9/10 测试中使用相同类型
- `lift_functions` / `build_id_map` / `normalize`：任务 2 定义，任务 3/4 使用相同签名
- `emit_decision_branch` / `emit_branch_body`：任务 5 (Py) 和任务 6 (Go) 各自定义，签名一致（仅注释语法不同）
- 诊断码：`TANGLE_RULE_TOGGLE_MALFORMED` 和 `TANGLE_RULE_TOGGLE_MISSING_NAME` 在任务 9 定义，任务 9/10 测试中使用相同字符串
