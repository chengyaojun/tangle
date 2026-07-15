# Phase 3: Codegen IR 字段消费 + Span 跟踪修复 设计规格

## 1. 背景与目标

### 1.1 背景

Phase 2 扩展了 IR schema，新增以下字段用于 rule lowering：

| 字段 | 位置 | 来源 | Phase 2 状态 |
|------|------|------|-------------|
| `group: Option<String>` | `IRNode` | Mermaid subgraph | lower 完成，codegen 未消费 |
| `style: Option<String>` | `IRNode` | Mermaid classDef/class | lower 完成，codegen 未消费 |
| `priority: Option<u32>` | `IREdge` | 决策表行序 | lower 完成，codegen 未消费 |
| `style: Option<String>` | `IREdge` | Mermaid linkStyle | lower 完成，codegen 未消费 |
| `IREdgeKind::Dashed` | `IREdge` | Mermaid `-.->` | lower 完成，codegen 未消费 |
| `IREdgeKind::Thick` | `IREdge` | Mermaid `==>` | lower 完成，codegen 未消费 |
| `IREdgeKind::Crossed` | `IREdge` | Mermaid `--x` | lower 完成，codegen 未消费 |

当前 codegen（`emit_js`/`emit_py`/`emit_go`）通过 BFS 遍历 RuleGraph，**完全忽略**上述字段。Decision 节点不发射 if/else 分支，`edge.guard` 也被忽略——所有节点按 BFS 线性化为顺序语句。

此外，Phase 2 的 rule lowering 遗留 5 项 span 跟踪 TODO（`lower_rule_tree.rs` x2 + `lower_rule_table.rs` x3），诊断 span 全为 `0`。

### 1.2 目标

1. **JS emitter 完整消费新 IR 字段**：Decision 节点发射 if/else-if 分支（按 priority 排序），元数据字段作为注释发射
2. **Py/Go emitter 注释消费**：group/style/Dashed/Thick/Crossed 作为注释发射，不发射 if/else
3. **修复 5 项 span 跟踪 TODO**：诊断 span 使用实际行号而非 `0`
4. **Crossed 边语义**：codegen 跳过 Crossed 边（禁用路径），发射注释标记

### 1.3 成功标准

- JS 生成的 Decision 分支代码包含 if/else-if，按 priority 排序
- JS/Py/Go 输出包含 group/style/edge-type 注释
- 5 项 span TODO 修复后诊断行号 ≠ 0
- 出口闸门 6 项全部 PASS

## 2. 字段语义决策

| 字段 | 语义 | codegen 行为 |
|------|------|-------------|
| `IREdge.priority` | **语义化** | JS: if/else-if 分支按 priority 升序排列（lower = higher precedence） |
| `IREdgeKind::Crossed` | **语义化** | JS: 跳过该分支，发射 `// skipped: crossed edge` |
| `IREdgeKind::Dashed` | 元数据 | 注释 `// edge: dashed` |
| `IREdgeKind::Thick` | 元数据 | 注释 `// edge: thick` |
| `IRNode.group` | 元数据 | 注释 `// group: <name>` |
| `IRNode.style` | 元数据 | 注释 `// style: <name>` |
| `IREdge.style` | 元数据 | 注释 `// edge-style: <style>` |

**priority 排序规则**：
- 有 priority 的边按 priority 升序排列（0 = 最高优先级）
- 无 priority 的边排最后（保持原有顺序）
- 相同 priority 的边保持稳定排序（不交换）

## 3. 架构概览

### 3.1 工作流划分

Phase 3 包含两个无交叉依赖的工作流：

```
工作流 A: Codegen 消费新 IR 字段
    ├── JS emitter: if/else 分支 + 注释
    ├── Py emitter: 仅注释
    └── Go emitter: 仅注释

工作流 B: Span 跟踪修复
    ├── lower_rule_tree.rs: ListNode 加 line 字段
    └── lower_rule_table.rs: 保留原始行索引
```

两个工作流可并行实现，通过出口闸门统一验证。

### 3.2 JS if/else 分支发射设计

当前 `emit_js_function_body` 的 BFS 逻辑：

```text
// 当前（简化）：
while let Some(node) = queue.pop_front() {
    emit_node_statement(node);       // 所有节点都作为顺序语句
    for edge in edges_from(node) {
        queue.push_back(edge.target); // 所有后继入队，忽略 guard/kind
    }
}
```

改造后逻辑：

```text
// 改造后：
while let Some(node) = queue.pop_front() {
    if node.kind == Decision && has_guarded_edges(node) {
        emit_decision_branch(node);   // 发射 if/else-if 链
    } else {
        emit_node_statement(node);
        for edge in edges_from(node) {
            if edge.kind != Crossed {
                queue.push_back(edge.target);
            }
        }
    }
}

fn emit_decision_branch(node):
    guarded_edges = sort_by_priority(edges_from(node).filter(has_guard))
    unguarded_edges = edges_from(node).filter(no_guard)
    crossed_edges = edges_from(node).filter(kind == Crossed)

    // 发射注释（group/style/edge-type）
    emit_node_comments(node)

    for (i, edge) in guarded_edges.enumerate() {
        if i == 0 { emit "if ({edge.guard}) {{" }
        else { emit "else if ({edge.guard}) {{" }
        emit_branch_body(edge.target)  // 递归发射目标节点
        emit "}}"
    }
    // 无 guard 的边作为 else 分支
    if !unguarded_edges.is_empty() {
        emit "else {"
        for edge in unguarded_edges { emit_branch_body(edge.target) }
        emit "}"
    }
    // Crossed 边发射跳过注释
    for edge in crossed_edges {
        emit "// skipped: crossed edge to {edge.to}"
    }
```

### 3.3 分支体递归发射

`emit_branch_body(target)` 递归发射目标节点及其后继（在 if/else 块内）：

- Action/Compute 节点：发射语句后，继续递归其非 Crossed 后继
- Decision 节点：嵌套 if/else-if
- Terminal 节点：发射 `return Ok(undefined);`
- ErrorTerminal 节点：发射 `return Err('...');`
- 已访问节点：不再递归（防止环）

## 4. 详细设计

### 4.1 JS Emitter 改造（`codegen/js_emitter.rs`）

#### 4.1.1 新增函数

- `emit_decision_branch(node, nodes, edges, visited, indent) -> String`：发射 if/else-if 链
- `emit_branch_body(target_id, nodes, edges, visited, indent) -> String`：递归发射分支体
- `sort_edges_by_priority(edges: &[IREdge]) -> Vec<&IREdge>`：按 priority 稳定排序
- `emit_node_comments(node: &IRNode, indent) -> String`：发射 group/style 注释
- `emit_edge_comments(edge: &IREdge, indent) -> String`：发射 edge-type/style 注释

#### 4.1.2 改造 `emit_js_function_body`

- BFS 主循环中检测 Decision 节点
- 有 guarded 出边时调用 `emit_decision_branch`
- 无 guarded 出边时保持现有逻辑（顺序语句 + 入队）
- 所有出边为 Crossed 时不入队（跳过）

#### 4.1.3 注释发射规则

节点注释（在节点语句前）：
```js
// group: Approval
// style: highlight
<node statement>;
```

边注释（在 if/else 分支前）：
```js
// edge: dashed
// edge-style: stroke:#ff3
if (guard) { ... }
```

### 4.2 Py/Go Emitter 改造

#### 4.2.1 Python（`codegen/py_emitter.rs`）

- 节点前：`# group: Approval` / `# style: highlight`
- 边：`# edge: dashed` / `# edge: thick` / `# edge: crossed` / `# edge-style: ...`
- **不发射 if/else**，保持现有线性化
- Crossed 边不跳过（仅注释标记）

#### 4.2.2 Go（`codegen/go_emitter.rs`）

- 节点前：`// group: Approval` / `// style: highlight`
- 边：`// edge: dashed` / `// edge: thick` / `// edge: crossed` / `// edge-style: ...`
- **不发射 if/else**，保持现有线性化
- Crossed 边不跳过（仅注释标记）

### 4.3 Span 跟踪修复

#### 4.3.1 `lower_rule_tree.rs`（2 处 TODO）

**改动 1：`ListNode` 增加行号字段**

```rust
pub struct ListNode {
    pub text: String,
    pub depth: usize,
    pub line: usize,      // 新增：1-based 行号
    pub children: Vec<ListNode>,
}
```

**改动 2：`parse_list_to_tree` 跟踪行号**

```rust
pub fn parse_list_to_tree(markdown: &str) -> Vec<ListNode> {
    let mut items: Vec<(usize, String, usize)> = vec![]; // (depth, text, line)
    for (line_no, line) in markdown.lines().enumerate() {
        // ... existing parsing ...
        items.push((depth, text, line_no + 1)); // 1-based
    }
    // build_tree 传递 line_no 到 ListNode
}
```

**改动 3：诊断使用实际行号**

```rust
// 原：span: SourceSpan { start_line: 0, ... }
// 改：span: SourceSpan { start_line: branch.line, end_line: branch.line, ... }
```

#### 4.3.2 `lower_rule_table.rs`（3 处 TODO）

**改动 1：保留原始行索引**

```rust
// 原：
let lines: Vec<&str> = table_markdown.lines().filter(...).collect();

// 改：
let lines: Vec<(usize, &str)> = table_markdown.lines()
    .enumerate()
    .filter(|(_, l)| l.contains('|'))
    .filter(|(_, l)| !is_separator_row(l))
    .collect();
```

**改动 2：`parsed_rows` 携带行号**

```rust
let mut parsed_rows: Vec<(usize, Vec<String>)> = vec![]; // (line_no, conds)
for (line_no, line) in &lines[1..] {
    // ... existing parsing ...
    parsed_rows.push((*line_no, conds));
}
```

**改动 3：诊断使用实际行号**

```rust
// 原：span: SourceSpan { start_line: 0, ... }
// 改：span: SourceSpan { start_line: line_no, end_line: line_no, ... }
```

## 5. 测试策略

### 5.1 新增测试目录

`compiler/tangle-cli/tests/v03_phase3/`

### 5.2 单元测试

| 测试 | 验证内容 |
|------|---------|
| `js_decision_if_else_emission` | Decision 节点发射 if/else-if |
| `js_priority_ordering` | 分支按 priority 升序排列 |
| `js_crossed_edge_skipped` | Crossed 边被跳过，发射注释 |
| `js_group_style_comments` | group/style 作为注释发射 |
| `js_dashed_thick_comments` | Dashed/Thick 作为注释发射 |
| `py_comments_emission` | Python 注释格式正确 |
| `go_comments_emission` | Go 注释格式正确 |
| `span_tree_diagnostic_line` | tree 诊断 span 行号 ≠ 0 |
| `span_table_diagnostic_line` | table 诊断 span 行号 ≠ 0 |

### 5.3 快照测试

`compiler/tangle-cli/tests/v03_phase3/snapshots/` 目录，新增：
- `snapshot_js_decision_branches.snap` — JS if/else 输出
- `snapshot_js_metadata_comments.snap` — JS 注释输出
- `snapshot_py_comments.snap` — Python 注释输出
- `snapshot_go_comments.snap` — Go 注释输出

### 5.4 回归测试

- 现有 `audit_regression/` 测试必须全绿（codegen 输出变化不影响 IR 级诊断）
- 现有 Phase 2 快照测试必须匹配（IR 层无变化）
- `diff-ir.ps1` 保持 0 unexpected DIFF（IR schema 不变）

## 6. 出口闸门

| 闸门 | 命令 | 标准 |
|------|------|------|
| 1. 单元+集成测试 | `cargo test --workspace` | 全绿 |
| 2. Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 警告 |
| 3. 审计回归 | `run-audit.ps1` | 0 failing |
| 4. 差分测试 | `diff-ir.ps1` | 0 unexpected DIFF |
| 5. 快照测试 | Phase 3 新增快照 | 全部匹配 |
| 6. Span 诊断测试 | `span_tree_diagnostic_line` + `span_table_diagnostic_line` | 行号 ≠ 0 |

## 7. 不在 Phase 3 范围内

- Py/Go if/else 分支发射（留待后续 phase）
- TS 参考实现同步（F-010 保持 SKIPPED）
- IR schema TS/Rust 对齐（F-007~F-012 保持 KNOWN_DIFF）
- `lower_rule_toggle.rs` 改动
- `lower_rule_flow.rs` 改动（Phase 2 已完成）

## 8. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| if/else 发射改变现有 codegen 输出 | 现有测试可能失败 | 更新受影响测试；IR 级测试不受影响（仅 codegen 输出变化） |
| 递归分支发射可能导致栈溢出 | 深度嵌套的 Decision 链 | 设置最大递归深度（如 32），超限回退到线性化 + 警告 |
| priority 排序改变语义 | 同一 fixture 的行为变化 | 快照测试捕获；文档化 priority 语义 |
| Span 修复改变诊断输出 | 审计脚本可能误报 | 更新 `expected_diagnostics.yaml`；审计脚本只检查诊断码不检查行号 |
