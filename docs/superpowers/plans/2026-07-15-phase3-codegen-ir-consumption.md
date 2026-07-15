# Phase 3: Codegen IR 字段消费 + Span 跟踪修复 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让 codegen 消费 Phase 2 新增的 IR 字段（JS 完整 if/else + 注释，Py/Go 仅注释），并修复 5 项 span 跟踪 TODO。

**架构：** 两个独立工作流并行推进——工作流 A 改造三个 codegen emitter（js_emitter.rs 增加 Decision if/else 分支发射 + 元数据注释；py/go_emitter.rs 仅加注释），工作流 B 修复 lower_rule_tree.rs 和 lower_rule_table.rs 的 span 跟踪（ListNode 加 line 字段 + table 保留原始行索引）。

**技术栈：** Rust，insta 快照测试，cargo test/clippy

**规格：** `docs/superpowers/specs/2026-07-15-phase3-codegen-ir-consumption-design.md`

**前提：** 在 `audit/v0.2.1` 分支基础上创建 `phase3/v0.3.1` worktree：
```bash
git worktree add .worktrees/phase3-v0.3.1 -b phase3/v0.3.1 audit/v0.2.1
```

所有文件路径相对于 worktree 根目录 `e:\GitProjects\tangle\.worktrees\phase3-v0.3.1\`。

---

## 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `compiler/tangle-cli/src/ir/lower_rule_tree.rs` | 规则树 lowering + span 修复 | 修改 |
| `compiler/tangle-cli/src/ir/lower_rule_table.rs` | 决策表 lowering + span 修复 | 修改 |
| `compiler/tangle-cli/src/codegen/js_emitter.rs` | JS codegen + if/else + 注释 | 修改 |
| `compiler/tangle-cli/src/codegen/py_emitter.rs` | Python codegen + 注释 | 修改 |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | Go codegen + 注释 | 修改 |
| `compiler/tangle-cli/tests/v03_phase3/span_tracking.rs` | Span 准确性测试 | 创建 |
| `compiler/tangle-cli/tests/v03_phase3/js_codegen.rs` | JS if/else + 注释测试 | 创建 |
| `compiler/tangle-cli/tests/v03_phase3/py_go_codegen.rs` | Py/Go 注释测试 | 创建 |
| `compiler/tangle-cli/tests/v03_phase3/snapshots.rs` | 快照测试 | 创建 |

---

## 任务 1：Span 跟踪修复 — lower_rule_tree.rs（2 处 TODO）

**文件：**
- 修改：`compiler/tangle-cli/src/ir/lower_rule_tree.rs`
- 测试：`compiler/tangle-cli/tests/v03_phase3/span_tracking.rs`

- [ ] **步骤 1：创建测试文件，编写失败的 tree span 测试**

创建 `compiler/tangle-cli/tests/v03_phase3/span_tracking.rs`：

```rust
use tangle_cli::ir::lower_rule_tree::lower_rule_tree;
use tangle_cli::ir::graph::FreshNodeId;

#[test]
fn span_tree_empty_branch_diagnostic_has_nonzero_line() {
    // 该列表的 branch 'no_children' 没有 children，应触发 TANGLE_RULE_EMPTY_BRANCH
    // 第 1 行是 'no_children'（1-based），诊断 span.start_line 应为 1
    let md = "\
- no_children
- has_action
    - condition
    - Action: do_something
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_tree(md, "test.md", &mut id_gen);
    let empty_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_EMPTY_BRANCH");
    assert!(empty_diag.is_some(), "should emit TANGLE_RULE_EMPTY_BRANCH");
    let diag = empty_diag.unwrap();
    assert!(diag.span.start_line != 0, "span.start_line should be nonzero, got {}", diag.span.start_line);
    assert_eq!(diag.span.file, "test.md");
}

#[test]
fn span_tree_no_action_diagnostic_has_nonzero_line() {
    // branch 'has_conditions' 有 children 但无 Action: 标记
    // 第 2 行是 'has_conditions'
    let md = "\
- first_branch
    - Action: ok
- has_conditions
    - cond_a
    - cond_b
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_tree(md, "test.md", &mut id_gen);
    let no_action_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_NO_ACTION");
    assert!(no_action_diag.is_some(), "should emit TANGLE_RULE_NO_ACTION");
    let diag = no_action_diag.unwrap();
    assert!(diag.span.start_line != 0, "span.start_line should be nonzero, got {}", diag.span.start_line);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --test span_tracking -v`（在 worktree 目录）
预期：编译失败或测试失败（`lower_rule_tree` 模块可能不公开，且 span 为 0）

注意：如果 `lower_rule_tree` 模块不公开，需先在 `compiler/tangle-cli/src/lib.rs` 中确认或添加 `pub mod ir;` 及子模块可见性。检查 `lib.rs` 是否已导出 `ir::lower_rule_tree`。如未导出，在 `lib.rs` 中添加 `pub mod ir;` 下必要的 `pub use` 语句。

- [ ] **步骤 3：修改 `ListNode` 增加 `line` 字段**

在 `compiler/tangle-cli/src/ir/lower_rule_tree.rs:135` 修改：

```rust
// 原：
#[derive(Debug, Clone)]
pub struct ListNode {
    pub text: String,
    pub depth: usize,
    pub children: Vec<ListNode>,
}

// 改为：
#[derive(Debug, Clone)]
pub struct ListNode {
    pub text: String,
    pub depth: usize,
    pub line: usize,
    pub children: Vec<ListNode>,
}
```

- [ ] **步骤 4：修改 `parse_list_to_tree` 跟踪行号**

在 `compiler/tangle-cli/src/ir/lower_rule_tree.rs:143` 修改：

```rust
// 原：
pub fn parse_list_to_tree(markdown: &str) -> Vec<ListNode> {
    let mut items: Vec<(usize, String)> = vec![];
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("* ") && !trimmed.starts_with("- ") {
            continue;
        }
        let leading = &line[..line.len() - trimmed.len()];
        let depth = compute_depth_from_str(leading);
        let text = trimmed
            .trim_start_matches("* ")
            .trim_start_matches("- ")
            .trim()
            .to_string();
        items.push((depth, text));
    }
    let mut idx = 0;
    build_tree(&items, 0, &mut idx)
}

// 改为：
pub fn parse_list_to_tree(markdown: &str) -> Vec<ListNode> {
    let mut items: Vec<(usize, String, usize)> = vec![]; // (depth, text, line)
    for (line_no, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("* ") && !trimmed.starts_with("- ") {
            continue;
        }
        let leading = &line[..line.len() - trimmed.len()];
        let depth = compute_depth_from_str(leading);
        let text = trimmed
            .trim_start_matches("* ")
            .trim_start_matches("- ")
            .trim()
            .to_string();
        items.push((depth, text, line_no + 1)); // 1-based
    }
    let mut idx = 0;
    build_tree(&items, 0, &mut idx)
}
```

- [ ] **步骤 5：修改 `build_tree` 传递行号**

在 `compiler/tangle-cli/src/ir/lower_rule_tree.rs:185` 修改：

```rust
// 原：
fn build_tree(items: &[(usize, String)], target_depth: usize, idx: &mut usize) -> Vec<ListNode> {
    let mut nodes = vec![];
    while *idx < items.len() {
        let (depth, ref text) = items[*idx];
        if depth < target_depth {
            break;
        }
        if depth == target_depth {
            *idx += 1;
            let children = build_tree(items, target_depth + 1, idx);
            nodes.push(ListNode {
                text: text.clone(),
                depth: target_depth,
                children,
            });
        } else {
            *idx += 1;
        }
    }
    nodes
}

// 改为：
fn build_tree(items: &[(usize, String, usize)], target_depth: usize, idx: &mut usize) -> Vec<ListNode> {
    let mut nodes = vec![];
    while *idx < items.len() {
        let (depth, ref text, line) = items[*idx];
        if depth < target_depth {
            break;
        }
        if depth == target_depth {
            *idx += 1;
            let children = build_tree(items, target_depth + 1, idx);
            nodes.push(ListNode {
                text: text.clone(),
                depth: target_depth,
                line: *line,
                children,
            });
        } else {
            *idx += 1;
        }
    }
    nodes
}
```

- [ ] **步骤 6：修改诊断使用实际行号**

在 `compiler/tangle-cli/src/ir/lower_rule_tree.rs:27-32` 和 `38-43` 修改两处诊断：

```rust
// 第一处 TANGLE_RULE_EMPTY_BRANCH（原 line 27-32）：
// 原：
diagnostics.push(TangleDiagnostic {
    code: "TANGLE_RULE_EMPTY_BRANCH".into(),
    message: format!("branch '{}' has no conditions or action", branch.text),
    // TODO: track line numbers through parse_list_to_tree to provide accurate spans
    span: SourceSpan { file: _file.into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 },
});

// 改为：
diagnostics.push(TangleDiagnostic {
    code: "TANGLE_RULE_EMPTY_BRANCH".into(),
    message: format!("branch '{}' has no conditions or action", branch.text),
    span: SourceSpan { file: _file.into(), start_line: branch.line, start_column: 0, end_line: branch.line, end_column: 0 },
});

// 第二处 TANGLE_RULE_NO_ACTION（原 line 38-43）：
// 原：
diagnostics.push(TangleDiagnostic {
    code: "TANGLE_RULE_NO_ACTION".into(),
    message: format!("branch '{}' has no Action: marker", branch.text),
    // TODO: track line numbers through parse_list_to_tree to provide accurate spans
    span: SourceSpan { file: _file.into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 },
});

// 改为：
diagnostics.push(TangleDiagnostic {
    code: "TANGLE_RULE_NO_ACTION".into(),
    message: format!("branch '{}' has no Action: marker", branch.text),
    span: SourceSpan { file: _file.into(), start_line: branch.line, start_column: 0, end_line: branch.line, end_column: 0 },
});
```

- [ ] **步骤 7：修复 `lower_rule_tree.rs` 内其他 `ListNode` 构造点**

搜索文件中所有 `ListNode {` 构造点，确保都添加了 `line` 字段。可能在测试代码中有构造 `ListNode` 的地方，需要添加 `line: 0`（测试中不关心行号）。

运行：`cargo build` 验证编译通过。

- [ ] **步骤 8：运行测试验证通过**

运行：`cargo test --test span_tracking span_tree -v`
预期：两个 span_tree 测试 PASS

- [ ] **步骤 9：Commit**

```bash
git add compiler/tangle-cli/src/ir/lower_rule_tree.rs compiler/tangle-cli/tests/v03_phase3/span_tracking.rs
git commit -m "fix(ir): track line numbers in rule tree lowering for accurate diagnostic spans"
```

---

## 任务 2：Span 跟踪修复 — lower_rule_table.rs（3 处 TODO）

**文件：**
- 修改：`compiler/tangle-cli/src/ir/lower_rule_table.rs`
- 测试：`compiler/tangle-cli/tests/v03_phase3/span_tracking.rs`（追加）

- [ ] **步骤 1：编写失败的 table span 测试**

在 `compiler/tangle-cli/tests/v03_phase3/span_tracking.rs` 追加：

```rust
use tangle_cli::ir::lower_rule_table::lower_rule_table_with_diagnostics;

#[test]
fn span_table_overlap_diagnostic_has_nonzero_line() {
    // 表格第 2、3 行重叠（通配符 + 具体值）
    // markdown 中第 1 行是表头，第 2 行是 separator，第 3 行是 row1，第 4 行是 row2
    let md = "\
| status | action |
|--------|--------|
| -      | ok     |
| -      | ok     |
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
    let overlap_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_DUPLICATE");
    assert!(overlap_diag.is_some(), "should emit TANGLE_RULE_DUPLICATE for identical rows");
    let diag = overlap_diag.unwrap();
    assert!(diag.span.start_line != 0, "span.start_line should be nonzero, got {}", diag.span.start_line);
    assert_eq!(diag.span.file, "test.md");
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --test span_tracking span_table -v`
预期：测试失败（span.start_line == 0）

- [ ] **步骤 3：修改表格行解析保留原始行号**

在 `compiler/tangle-cli/src/ir/lower_rule_table.rs:29-37` 修改：

```rust
// 原：
let lines: Vec<&str> = table_markdown
    .lines()
    .filter(|l| l.contains('|'))
    .filter(|l| {
        !l.trim()
            .chars()
            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
    })
    .collect();

// 改为：
let lines: Vec<(usize, &str)> = table_markdown
    .lines()
    .enumerate()
    .filter(|(_, l)| l.contains('|'))
    .filter(|(_, l)| {
        !l.trim()
            .chars()
            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
    })
    .collect();
```

- [ ] **步骤 4：修改 header 解析使用元组**

在 `compiler/tangle-cli/src/ir/lower_rule_table.rs:44` 修改：

```rust
// 原：
let headers: Vec<String> = split_table_row(lines[0]);

// 改为：
let headers: Vec<String> = split_table_row(lines[0].1);
```

- [ ] **步骤 5：修改数据行解析携带行号**

在 `compiler/tangle-cli/src/ir/lower_rule_table.rs:52-73` 修改：

```rust
// 原：
let mut parsed_rows: Vec<Vec<String>> = vec![];
let mut parsed_actions: Vec<String> = vec![];
for line in &lines[1..] {
    let cells = split_table_row(line);
    // ... existing parsing ...
    parsed_rows.push(conds);
    parsed_actions.push(cells.last().unwrap().clone());
}

// 改为：
let mut parsed_rows: Vec<(usize, Vec<String>)> = vec![]; // (line_no, conds)
let mut parsed_actions: Vec<String> = vec![];
for (line_no, line) in &lines[1..] {
    let cells = split_table_row(line);
    // ... existing parsing (conds 变量不变) ...
    parsed_rows.push((*line_no, conds));
    parsed_actions.push(cells.last().unwrap().clone());
}
```

- [ ] **步骤 6：修改重叠检测使用实际行号**

在 `compiler/tangle-cli/src/ir/lower_rule_table.rs:76-138` 修改三处诊断。将 `parsed_rows[i]` 改为 `parsed_rows[i].1`，并使用 `parsed_rows[i].0` 作为行号：

```rust
// 原：
for i in 0..parsed_rows.len() {
    for j in (i + 1)..parsed_rows.len() {
        if rows_overlap(&parsed_rows[i], &parsed_rows[j]) {
            if parsed_rows[i] == parsed_rows[j] {
                diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_RULE_DUPLICATE".into(),
                    message: format!("rows {} and {} are identical", i + 1, j + 1),
                    // TODO: track line numbers through table parsing to provide accurate spans
                    span: SourceSpan { file: file.into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 },
                });

// 改为：
for i in 0..parsed_rows.len() {
    for j in (i + 1)..parsed_rows.len() {
        if rows_overlap(&parsed_rows[i].1, &parsed_rows[j].1) {
            if parsed_rows[i].1 == parsed_rows[j].1 {
                diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_RULE_DUPLICATE".into(),
                    message: format!("rows {} and {} are identical", i + 1, j + 1),
                    span: SourceSpan { file: file.into(), start_line: parsed_rows[j].0, start_column: 0, end_line: parsed_rows[j].0, end_column: 0 },
                });
```

同样修改 `TANGLE_RULE_UNREACHABLE` 和 `TANGLE_RULE_OVERLAP` 两处诊断：
- 将 `parsed_rows[i]` 改为 `parsed_rows[i].1`，`parsed_rows[j]` 改为 `parsed_rows[j].1`
- 将 `span: SourceSpan { ..., start_line: 0, ... }` 改为 `span: SourceSpan { ..., start_line: parsed_rows[j].0, ... }`

- [ ] **步骤 7：修复后续使用 `parsed_rows` 的代码**

搜索文件中所有使用 `parsed_rows[i]` 的地方（在 IR 节点生成部分），将 `parsed_rows[i]` 改为 `parsed_rows[i].1`。运行 `cargo build` 验证编译。

- [ ] **步骤 8：运行测试验证通过**

运行：`cargo test --test span_tracking -v`
预期：所有 span 测试 PASS

- [ ] **步骤 9：Commit**

```bash
git add compiler/tangle-cli/src/ir/lower_rule_table.rs compiler/tangle-cli/tests/v03_phase3/span_tracking.rs
git commit -m "fix(ir): track line numbers in rule table lowering for accurate diagnostic spans"
```

---

## 任务 3：JS Emitter — 元数据注释发射

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/js_emitter.rs`
- 测试：`compiler/tangle-cli/tests/v03_phase3/js_codegen.rs`

- [ ] **步骤 1：创建测试文件，编写失败的注释测试**

创建 `compiler/tangle-cli/tests/v03_phase3/js_codegen.rs`：

```rust
use tangle_cli::codegen::js_emitter::emit_js;
use tangle_cli::ir::graph::*;

fn make_graph_with_group_style() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode {
                id: "n1".into(),
                kind: IRNodeKind::Action,
                label: "do_work".into(),
                source_span: None,
                source_text: Some("let x = 1".into()),
                group: Some("Approval".into()),
                style: Some("highlight".into()),
            },
            IRNode {
                id: "n2".into(),
                kind: IRNodeKind::Terminal,
                label: "done".into(),
                source_span: None,
                source_text: None,
                group: None,
                style: None,
            },
        ],
        edges: vec![IREdge {
            from: "n1".into(),
            to: "n2".into(),
            kind: IREdgeKind::Control,
            guard: None,
            source_span: None,
            priority: None,
            style: None,
        }],
        error_edges: vec![],
        entry_node_id: "n1".into(),
        imported_stdlib: vec![],
        stdlib_imports: vec![],
        functions: vec![],
    }
}

#[test]
fn js_emits_group_comment_for_node() {
    let graph = make_graph_with_group_style();
    let js = emit_js(&graph, "TestModule");
    assert!(js.contains("// group: Approval"), "JS output should contain group comment, got:\n{}", js);
}

#[test]
fn js_emits_style_comment_for_node() {
    let graph = make_graph_with_group_style();
    let js = emit_js(&graph, "TestModule");
    assert!(js.contains("// style: highlight"), "JS output should contain style comment, got:\n{}", js);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --test js_codegen js_emits_group -v`
预期：测试失败（当前不发射注释）

- [ ] **步骤 3：添加 `emit_node_comments` 函数**

在 `compiler/tangle-cli/src/codegen/js_emitter.rs` 的 `emit_js_function_body` 之前添加：

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
```

- [ ] **步骤 4：在 `emit_js_function_body` 中集成注释发射**

在 `compiler/tangle-cli/src/codegen/js_emitter.rs:238` 的 match 分支前，添加注释发射：

```rust
// 在 match node.kind { 之前添加：
out.push_str(&emit_node_comments(node, "  "));

match node.kind {
    // ... existing code ...
}
```

- [ ] **步骤 5：运行测试验证通过**

运行：`cargo test --test js_codegen js_emits -v`
预期：两个注释测试 PASS

- [ ] **步骤 6：Commit**

```bash
git add compiler/tangle-cli/src/codegen/js_emitter.rs compiler/tangle-cli/tests/v03_phase3/js_codegen.rs
git commit -m "feat(codegen): emit group/style metadata comments in JS emitter"
```

---

## 任务 4：JS Emitter — Decision if/else 分支发射

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/js_emitter.rs`
- 测试：`compiler/tangle-cli/tests/v03_phase3/js_codegen.rs`（追加）

- [ ] **步骤 1：编写失败的 if/else 测试**

在 `compiler/tangle-cli/tests/v03_phase3/js_codegen.rs` 追加：

```rust
fn make_graph_with_decision_branches() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode {
                id: "entry".into(), kind: IRNodeKind::Decision, label: "check".into(),
                source_span: None, source_text: None, group: None, style: None,
            },
            IRNode {
                id: "approve".into(), kind: IRNodeKind::Action, label: "approve".into(),
                source_span: None, source_text: Some("status = \"approved\"".into()),
                group: None, style: None,
            },
            IRNode {
                id: "reject".into(), kind: IRNodeKind::Action, label: "reject".into(),
                source_span: None, source_text: Some("status = \"rejected\"".into()),
                group: None, style: None,
            },
            IRNode {
                id: "end".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None,
            },
        ],
        edges: vec![
            IREdge { from: "entry".into(), to: "approve".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 1000".into()), source_span: None, priority: Some(0), style: None },
            IREdge { from: "entry".into(), to: "reject".into(), kind: IREdgeKind::Condition,
                guard: Some("amount >= 1000".into()), source_span: None, priority: Some(1), style: None },
            IREdge { from: "approve".into(), to: "end".into(), kind: IREdgeKind::Control,
                guard: None, source_span: None, priority: None, style: None },
            IREdge { from: "reject".into(), to: "end".into(), kind: IREdgeKind::Control,
                guard: None, source_span: None, priority: None, style: None },
        ],
        error_edges: vec![], entry_node_id: "entry".into(),
        imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
    }
}

#[test]
fn js_decision_emits_if_else_chain() {
    let graph = make_graph_with_decision_branches();
    let js = emit_js(&graph, "DecisionTest");
    assert!(js.contains("if (amount < 1000)"), "should emit if branch, got:\n{}", js);
    assert!(js.contains("else if (amount >= 1000)"), "should emit else-if branch, got:\n{}", js);
}

#[test]
fn js_decision_priority_orders_branches() {
    let graph = make_graph_with_decision_branches();
    let js = emit_js(&graph, "DecisionTest");
    let if_pos = js.find("if (amount < 1000)").unwrap();
    let elseif_pos = js.find("else if (amount >= 1000)").unwrap();
    assert!(if_pos < elseif_pos, "priority 0 branch should come before priority 1");
}

#[test]
fn js_crossed_edge_is_skipped() {
    let mut graph = make_graph_with_decision_branches();
    // 将 reject 边改为 Crossed
    graph.edges[1].kind = IREdgeKind::Crossed;
    graph.edges[1].guard = None;
    let js = emit_js(&graph, "CrossedTest");
    assert!(js.contains("// skipped: crossed edge"), "should emit skipped comment, got:\n{}", js);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --test js_codegen js_decision -v`
预期：测试失败（当前不发射 if/else）

- [ ] **步骤 3：添加 `sort_edges_by_priority` 函数**

在 `compiler/tangle-cli/src/codegen/js_emitter.rs` 添加：

```rust
/// Sort edges by priority (lower = higher precedence). Edges without priority sort last.
fn sort_edges_by_priority(edges: &[&IREdge]) -> Vec<&IREdge> {
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
```

- [ ] **步骤 4：添加 `emit_decision_branch` 和 `emit_branch_body` 函数**

在 `compiler/tangle-cli/src/codegen/js_emitter.rs` 添加：

```rust
/// Recursively emit the body of a branch target inside an if/else block.
fn emit_branch_body(
    target_id: &str,
    nodes: &[IRNode],
    edges: &[IREdge],
    visited: &mut HashSet<&str>,
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
            if let Some(ref src) = node.source_text {
                out.push_str(&format!("{}{};\n", indent, translate_stmt_to_js(src)));
            } else {
                out.push_str(&format!("{}// {}: {}\n", indent,
                    match node.kind {
                        IRNodeKind::Action => "action",
                        IRNodeKind::Compute => "compute",
                        IRNodeKind::Decision => "decision",
                        _ => "unknown",
                    },
                    node.label
                ));
            }
        }
        IRNodeKind::Terminal => {
            out.push_str(&format!("{}return Ok(undefined);\n", indent));
        }
        IRNodeKind::ErrorTerminal => {
            let label = node.label.replace('\'', "\\'");
            out.push_str(&format!("{}return Err('{}');\n", indent, label));
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

/// Emit if/else-if chain for a Decision node with guarded outgoing edges.
fn emit_decision_branch(
    node: &IRNode,
    nodes: &[IRNode],
    edges: &[IREdge],
    visited: &mut HashSet<&str>,
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
        // No emitable edges; fall back to comment
        out.push_str(&format!("{}// decision: {} (no guarded branches)\n", indent, node.label));
        for e in &crossed {
            out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, e.to));
        }
        return out;
    }

    let inner_indent = format!("{}  ", indent);

    for (i, edge) in guarded.iter().enumerate() {
        out.push_str(&emit_edge_comments(edge, indent));
        let guard = edge.guard.as_ref().unwrap();
        if i == 0 {
            out.push_str(&format!("{}if ({}) {{\n", indent, guard));
        } else {
            out.push_str(&format!("{}else if ({}) {{\n", indent, guard));
        }
        out.push_str(&emit_branch_body(&edge.to, nodes, edges, visited, &inner_indent));
        out.push_str(&format!("{}}}\n", indent));
    }

    if !unguarded.is_empty() {
        out.push_str(&format!("{}else {{\n", indent));
        for edge in &unguarded {
            out.push_str(&emit_edge_comments(edge, &inner_indent));
            out.push_str(&emit_branch_body(&edge.to, nodes, edges, visited, &inner_indent));
        }
        out.push_str(&format!("{}}}\n", indent));
    }

    for edge in &crossed {
        out.push_str(&format!("{}// skipped: crossed edge to {}\n", indent, edge.to));
    }

    out
}
```

- [ ] **步骤 5：在 `emit_js_function_body` 中集成 Decision 分支发射**

修改 `compiler/tangle-cli/src/codegen/js_emitter.rs:238` 的 match 分支：

```rust
// 原 match node.kind { ... } 中的 Decision 处理：
// Decision 与 Action/Compute 一起处理

// 改为：Decision 单独处理
match node.kind {
    IRNodeKind::Action | IRNodeKind::Compute => {
        // ... existing code ...
    }
    IRNodeKind::Decision => {
        // Check if this Decision has guarded outgoing edges
        let has_guarded = edges.iter().any(|e| {
            e.from == node.id && e.guard.is_some() && e.kind != IREdgeKind::Crossed
        });
        if has_guarded {
            out.push_str(&emit_decision_branch(node, nodes, edges, &mut visited, "  "));
        } else {
            // Fall back to existing linear behavior
            if let Some(ref src) = node.source_text {
                out.push_str(&format!("  {};\n", translate_stmt_to_js(src)));
            } else {
                out.push_str(&format!("  // decision: {}\n", node.label));
            }
        }
    }
    IRNodeKind::Terminal => {
        // ... existing code ...
    }
    IRNodeKind::ErrorTerminal => {
        // ... existing code ...
    }
}
```

然后修改 BFS 后继入队逻辑，跳过 Decision 节点的 guarded 出边（已在 if/else 中发射）和 Crossed 边：

```rust
// 在 match 块之后，修改 for edge in edges { ... } 循环：
for edge in edges {
    if edge.from == node.id && edge.kind != IREdgeKind::Crossed {
        // Decision 节点的 guarded 边已在 if/else 中处理，跳过入队
        if node.kind == IRNodeKind::Decision && edge.guard.is_some() {
            continue;
        }
        if let Some(target) = nodes.iter().find(|n| n.id == edge.to) {
            queue.push_back(target);
        }
    }
}
```

- [ ] **步骤 6：运行测试验证通过**

运行：`cargo test --test js_codegen -v`
预期：所有 js_codegen 测试 PASS

- [ ] **步骤 7：运行现有测试确认无回归**

运行：`cargo test --workspace -v 2>&1 | tail -20`
预期：全绿（可能有少量现有 codegen 测试因输出格式变化需要更新快照）

如有现有快照测试失败，使用 `cargo insta review` 更新快照。

- [ ] **步骤 8：Commit**

```bash
git add compiler/tangle-cli/src/codegen/js_emitter.rs compiler/tangle-cli/tests/v03_phase3/js_codegen.rs
git commit -m "feat(codegen): emit if/else-if branches for Decision nodes ordered by priority in JS"
```

---

## 任务 5：Py Emitter — 注释消费

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/py_emitter.rs`
- 测试：`compiler/tangle-cli/tests/v03_phase3/py_go_codegen.rs`

- [ ] **步骤 1：创建测试文件，编写失败的 Py 注释测试**

创建 `compiler/tangle-cli/tests/v03_phase3/py_go_codegen.rs`：

```rust
use tangle_cli::codegen::py_emitter::emit_python;
use tangle_cli::codegen::go_emitter::emit_go;
use tangle_cli::ir::graph::*;

fn make_graph_with_metadata() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode {
                id: "n1".into(), kind: IRNodeKind::Action, label: "do_work".into(),
                source_span: None, source_text: None,
                group: Some("Approval".into()), style: Some("highlight".into()),
            },
            IRNode {
                id: "n2".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None,
            },
        ],
        edges: vec![
            IREdge { from: "n1".into(), to: "n2".into(), kind: IREdgeKind::Dashed,
                guard: None, source_span: None, priority: None, style: Some("stroke:#f00".into()) },
        ],
        error_edges: vec![], entry_node_id: "n1".into(),
        imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
    }
}

#[test]
fn py_emits_group_style_comments() {
    let graph = make_graph_with_metadata();
    let py = emit_python(&graph, "TestModule");
    assert!(py.contains("# group: Approval"), "Python should emit group comment, got:\n{}", py);
    assert!(py.contains("# style: highlight"), "Python should emit style comment, got:\n{}", py);
}

#[test]
fn py_emits_edge_type_comments() {
    let graph = make_graph_with_metadata();
    let py = emit_python(&graph, "TestModule");
    assert!(py.contains("# edge: dashed"), "Python should emit edge type comment, got:\n{}", py);
    assert!(py.contains("# edge-style: stroke:#f00"), "Python should emit edge style comment, got:\n{}", py);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --test py_go_codegen py_emits -v`
预期：测试失败

- [ ] **步骤 3：在 Py emitter BFS 循环中添加注释发射**

在 `compiler/tangle-cli/src/codegen/py_emitter.rs:58` 的 match 分支前添加：

```rust
// 在 match node.kind { 之前添加：
if node.group.is_some() {
    out.push_str(&format!("    # group: {}\n", node.group.as_ref().unwrap()));
}
if node.style.is_some() {
    out.push_str(&format!("    # style: {}\n", node.style.as_ref().unwrap()));
}
```

然后在 `for edge in &graph.edges { ... }` 循环中添加边注释（在 `if edge.from == node.id` 块内）：

```rust
for edge in &graph.edges {
    if edge.from == node.id {
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

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test --test py_go_codegen py_emits -v`
预期：两个 Py 测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/codegen/py_emitter.rs compiler/tangle-cli/tests/v03_phase3/py_go_codegen.rs
git commit -m "feat(codegen): emit group/style/edge-type comments in Python emitter"
```

---

## 任务 6：Go Emitter — 注释消费

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/go_emitter.rs`
- 测试：`compiler/tangle-cli/tests/v03_phase3/py_go_codegen.rs`（追加）

- [ ] **步骤 1：编写失败的 Go 注释测试**

在 `compiler/tangle-cli/tests/v03_phase3/py_go_codegen.rs` 追加：

```rust
#[test]
fn go_emits_group_style_comments() {
    let graph = make_graph_with_metadata();
    let go = emit_go(&graph, "TestModule");
    assert!(go.contains("// group: Approval"), "Go should emit group comment, got:\n{}", go);
    assert!(go.contains("// style: highlight"), "Go should emit style comment, got:\n{}", go);
}

#[test]
fn go_emits_edge_type_comments() {
    let graph = make_graph_with_metadata();
    let go = emit_go(&graph, "TestModule");
    assert!(go.contains("// edge: dashed"), "Go should emit edge type comment, got:\n{}", go);
    assert!(go.contains("// edge-style: stroke:#f00"), "Go should emit edge style comment, got:\n{}", go);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：`cargo test --test py_go_codegen go_emits -v`
预期：测试失败

- [ ] **步骤 3：在 Go emitter BFS 循环中添加注释发射**

在 `compiler/tangle-cli/src/codegen/go_emitter.rs:56` 的 match 分支前添加（与 Py 相同模式，但用 `//` 注释）：

```rust
// 在 match node.kind { 之前添加：
if node.group.is_some() {
    out.push_str(&format!("    // group: {}\n", node.group.as_ref().unwrap()));
}
if node.style.is_some() {
    out.push_str(&format!("    // style: {}\n", node.style.as_ref().unwrap()));
}
```

然后在 `for edge in &graph.edges { ... }` 循环中添加边注释：

```rust
for edge in &graph.edges {
    if edge.from == node.id {
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

- [ ] **步骤 4：运行测试验证通过**

运行：`cargo test --test py_go_codegen -v`
预期：所有 Py/Go 测试 PASS

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/codegen/go_emitter.rs compiler/tangle-cli/tests/v03_phase3/py_go_codegen.rs
git commit -m "feat(codegen): emit group/style/edge-type comments in Go emitter"
```

---

## 任务 7：快照测试

**文件：**
- 测试：`compiler/tangle-cli/tests/v03_phase3/snapshots.rs`
- 快照：`compiler/tangle-cli/tests/v03_phase3/snapshots/*.snap`

- [ ] **步骤 1：创建快照测试文件**

创建 `compiler/tangle-cli/tests/v03_phase3/snapshots.rs`：

```rust
use insta::assert_snapshot;
use tangle_cli::codegen::js_emitter::emit_js;
use tangle_cli::codegen::py_emitter::emit_python;
use tangle_cli::codegen::go_emitter::emit_go;
use tangle_cli::ir::graph::*;

fn make_decision_graph_with_priority() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode { id: "entry".into(), kind: IRNodeKind::Decision, label: "check".into(),
                source_span: None, source_text: None,
                group: Some("Approval".into()), style: None },
            IRNode { id: "auto".into(), kind: IRNodeKind::Action, label: "auto_approve".into(),
                source_span: None, source_text: Some("status = \"auto\"".into()),
                group: Some("Approval".into()), style: Some("highlight".into()) },
            IRNode { id: "manual".into(), kind: IRNodeKind::Action, label: "manual_review".into(),
                source_span: None, source_text: Some("status = \"manual\"".into()),
                group: None, style: None },
            IRNode { id: "reject".into(), kind: IRNodeKind::Action, label: "reject".into(),
                source_span: None, source_text: Some("status = \"rejected\"".into()),
                group: None, style: None },
            IRNode { id: "end".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None },
        ],
        edges: vec![
            IREdge { from: "entry".into(), to: "auto".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 100".into()), source_span: None,
                priority: Some(0), style: None },
            IREdge { from: "entry".into(), to: "manual".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 1000".into()), source_span: None,
                priority: Some(1), style: Some("stroke:#f00".into()) },
            IREdge { from: "entry".into(), to: "reject".into(), kind: IREdgeKind::Crossed,
                guard: None, source_span: None, priority: None, style: None },
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
fn snapshot_js_decision_branches_with_priority() {
    let graph = make_decision_graph_with_priority();
    let js = emit_js(&graph, "ApprovalFlow");
    assert_snapshot!(js);
}

#[test]
fn snapshot_py_metadata_comments() {
    let graph = make_decision_graph_with_priority();
    let py = emit_python(&graph, "ApprovalFlow");
    assert_snapshot!(py);
}

#[test]
fn snapshot_go_metadata_comments() {
    let graph = make_decision_graph_with_priority();
    let go = emit_go(&graph, "ApprovalFlow");
    assert_snapshot!(go);
}
```

- [ ] **步骤 2：生成快照**

运行：`cargo test --test snapshots -v`
首次运行会生成 `.snap.new` 文件。运行 `cargo insta accept` 接受快照。

- [ ] **步骤 3：再次运行验证快照匹配**

运行：`cargo test --test snapshots -v`
预期：3 个快照全部匹配

- [ ] **步骤 4：Commit**

```bash
git add compiler/tangle-cli/tests/v03_phase3/snapshots.rs compiler/tangle-cli/tests/v03_phase3/snapshots/
git commit -m "test(phase3): add snapshot tests for JS if/else + Py/Go metadata comments"
```

---

## 任务 8：出口闸门验证

**文件：** 无（仅验证）

- [ ] **步骤 1：Gate 1 — cargo test**

运行：`cargo test --workspace 2>&1 | tail -30`
预期：全绿，0 失败

- [ ] **步骤 2：Gate 2 — cargo clippy**

运行：`cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10`
预期：0 警告

- [ ] **步骤 3：Gate 3 — 审计回归**

运行：`pwsh tests/audit/run-audit.ps1`（在 worktree 目录）
预期：0 failing（cell 数可能因 codegen 输出变化而不同，但诊断码不变）

如出现 failing cells，检查 `tests/audit/expected_diagnostics.yaml` 是否需要更新。

- [ ] **步骤 4：Gate 4 — 差分测试**

运行：`pwsh tests/audit/diff-ir.ps1`（在 worktree 目录）
预期：0 unexpected DIFF，F-007~F-012 保持 KNOWN_DIFF，F-010 保持 SKIPPED

- [ ] **步骤 5：Gate 5 — 快照测试**

运行：`cargo test --test snapshots -v`
预期：Phase 3 快照全部匹配

- [ ] **步骤 6：Gate 6 — Span 诊断测试**

运行：`cargo test --test span_tracking -v`
预期：span.start_line ≠ 0

- [ ] **步骤 7：Commit 闸门验证记录**

```bash
git commit --allow-empty -m "test(phase3): exit gate validation — all 6 gates PASS

Gate 1 (cargo test): N tests pass, 0 failures.
Gate 2 (cargo clippy): 0 warnings.
Gate 3 (run-audit.ps1): N cells, 0 failing.
Gate 4 (diff-ir.ps1): 0 unexpected DIFF.
Gate 5 (snapshots): N snapshots match.
Gate 6 (span_tracking): all span.start_line != 0."
```

- [ ] **步骤 8：打 tag 并推送（待用户批准）**

```bash
git tag -a v0.3.1 -m "v0.3.1: Phase 3 Codegen IR consumption + Span tracking fix"
# 推送待用户批准
```
