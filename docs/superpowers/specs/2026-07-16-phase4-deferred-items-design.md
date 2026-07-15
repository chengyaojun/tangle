# Phase 4: Phase 3 推迟项闭合 设计规格

## 1. 背景与目标

### 1.1 背景

Phase 3 (v0.3.1) 完成了 JS codegen IR 字段消费 + span 跟踪修复，但显式推迟了 4 项工作（见 `2026-07-15-phase3-codegen-ir-consumption-design.md` §7）：

| 推迟项 | 来源 | 现状 |
|--------|------|------|
| Py/Go if/else 分支发射 | Phase 3 §7 | 仅 JS 实现 if/else-if，Py/Go 仅发注释 |
| TS 参考实现同步 (F-010) | 审计 findings | diff-ir.ps1 标记为 SKIPPED |
| IR schema TS/Rust 对齐 (F-007~F-012) | 审计 findings | 4 fixture 标记为 KNOWN_DIFF |
| `lower_rule_toggle.rs` 改动 | Phase 2/3 | 62 行基础实现，无 span/诊断/group/style |

### 1.2 目标

1. **A 组 — Codegen 对等**：Py/Go emitter 移植 JS 的 Decision if/else-if 分支发射，三宿主语义一致
2. **B 组 — TS/Rust 差分对等**：ir-diff 归一化闭合 F-007~F-012，4 个 KNOWN_DIFF fixture 转为 MATCH
3. **C 组 — Toggle lowering 一致性**：lower_rule_toggle.rs 升级到与 tree/table 一致的工程质量

### 1.3 成功标准

- Py/Go 生成的 Decision 分支代码包含 if/elif(Py) 或 if/else if(Go)，按 priority 升序
- `tests/audit/diff-ir.ps1` 报告 0 KNOWN_DIFF + 0 unexpected DIFF（4 fixture 转 MATCH）
- `lower_rule_toggle` 返回 `(RuleGraph, Vec<TangleDiagnostic>)`，checkbox 节点 span 行号 ≠ 0
- 出口闸门 7 项全部 PASS

## 2. 架构概览

### 2.1 工作流划分

3 个独立工作流，无交叉依赖，可并行：

```
工作流 A: Py/Go if/else 分支发射（Codegen 对等）
    ├── py_emitter.rs: emit_decision_branch + emit_branch_body（缩进式）
    └── go_emitter.rs: emit_decision_branch + emit_branch_body（大括号式）

工作流 B: ir-diff 归一化（TS/Rust 差分对等）
    └── tests/audit/ir-diff/src/main.rs: normalize() 4 阶段流水线
        ├── F-007: 节点 ID 重映射
        ├── F-008: strip null guard
        ├── F-009: 提升 functions[0] 为顶层
        └── F-011: label "return" → "exit"

工作流 C: lower_rule_toggle.rs 增强（IR lowering 一致性）
    ├── 签名改为 (RuleGraph, Vec<TangleDiagnostic>)
    ├── Span 跟踪：checkbox 行号 → IRNode.source_span
    ├── 名称提取：支持 `name: value` 格式
    └── group/style：`<!-- group: X -->` HTML 注释语法
```

### 2.2 前提

基于 `main` 分支（commit `99cb0e6`）创建 worktree：
```bash
git worktree add .worktrees/phase4-v0.4.0 -b phase4/v0.4.0 main
```

### 2.3 版本

完成后 merge `phase4/v0.4.0` 到 `main`，打 tag `v0.4.0`。

## 3. A 组：Py/Go if/else 分支发射

### 3.1 移植参照

JS 的 Phase 3 实现（`compiler/tangle-cli/src/codegen/js_emitter.rs:242-383`）核心 3 函数：

- `sort_edges_by_priority(edges: &[&IREdge]) -> Vec<&IREdge>` — 按 priority 升序稳定排序，无 priority 排最后
- `emit_branch_body(target_id, nodes, edges, visited, indent) -> String` — 递归发射分支目标节点及非 Crossed 后继
- `emit_decision_branch(node, nodes, edges, visited, indent) -> String` — 发射 if/else-if 链 + crossed 边跳过注释

三函数需移植到 `py_emitter.rs` 和 `go_emitter.rs`，语法差异是关键。

### 3.2 Python if/elif/else 设计

Python 用缩进（4 空格）而非大括号。分支发射格式：

```python
# edge: dashed
if (guard):
    # branch body — 递归 emit_branch_body，indent +4
elif (guard2):
    # branch body
else:
    # branch body
# skipped: crossed edge to nX
```

**emit_branch_body 语义**（Py 版本）：
- Action/Compute/Decision 节点：发注释 `# {kind}: {label}`（保持现有行为，不翻译 source_text）
- Terminal 节点：`return Ok(None)`
- ErrorTerminal 节点：`return Err('label')`
- 已访问节点：不递归（防环）
- 空分支体：发 `pass`（Python 语法要求非空块）

### 3.3 Go if/else if/else 设计

Go 用大括号，**要求 `} else` 同行**（分号自动插入规则）。分支发射格式：

```go
// edge: dashed
if guard {
    // branch body — 递归 emit_branch_body，indent +4
} else if guard2 {
    // branch body
} else {
    // branch body
}
// skipped: crossed edge to nX
```

**关键差异**：JS 的 `emit_decision_branch` 分行发 `}` 和 `else if`，Go 必须合并为 `} else if {`。

**emit_branch_body 语义**（Go 版本）：
- Action/Compute/Decision 节点：发注释 `// {kind}: {label}`（保持现有行为）
- Terminal 节点：`return Ok(nil)`
- ErrorTerminal 节点：`return Err("label")`
- 已访问节点：不递归（防环）

### 3.4 BFS 主循环改造

两个 emitter 的 BFS 循环改为：
- 检测 `Decision` 节点且有 `guarded` 出边（`edge.guard.is_some() && edge.kind != Crossed`）时，调用 `emit_decision_branch`
- 否则保持现有线性化行为
- guarded 边跳过入队（与 JS 一致，见 `js_emitter.rs:453-456`）

### 3.5 不在范围

- Py/Go 的 `source_text` 翻译（当前两 emitter 仅发注释，不翻译源码语句）— 保持现状
- 递归深度限制（JS 未实现，三宿主保持一致，不加）

### 3.6 A 组测试

`compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs`：

| 测试 | 验证内容 |
|------|---------|
| `py_decision_if_elif_emission` | Py 发射 if/elif |
| `py_priority_ordering` | Py 分支按 priority 升序 |
| `py_crossed_edge_skipped` | Py 跳过 crossed 边，发注释 |
| `py_branch_body_recursion` | Py 嵌套 Decision 递归发射 |
| `go_decision_if_else_emission` | Go 发射 if/else if（`} else if` 同行） |
| `go_priority_ordering` | Go 分支按 priority 升序 |
| `go_crossed_edge_skipped` | Go 跳过 crossed 边，发注释 |
| `go_branch_body_recursion` | Go 嵌套 Decision 递归发射 |

## 4. B 组：ir-diff 归一化

### 4.1 现状分析

当前 `tests/audit/ir-diff/src/main.rs` 的 `normalize(v: Value) -> Value` 仅做两件事：
- strip span 字段（含 `sourceText`，F-012 已闭合）
- 排序 object keys

`diff-ir.ps1` 已对空 TS IR 做 SKIPPED（F-010 已处理）。剩余 4 项分歧需归一化：F-007、F-008、F-009、F-011。

### 4.2 4 阶段归一化流水线

当前 `normalize(v)` 是无状态递归。改造为 4 阶段流水线：

```
阶段 1: lift_functions(v) -> Value
    若顶层有 functions[0]，提升其 nodes/edges/entryNodeId 到顶层
    删除 functions / importedStdlib（空时）/ stdlibImports（空时）
    → 闭合 F-009

阶段 2: build_id_map(v) -> HashMap<String, String>
    遍历 nodes[]，按数组顺序构建 {原 ID → "node{index}"} 映射
    → 为 F-007 准备

阶段 3: normalize(v, id_map) -> Value
    递归处理，在现有 strip span + sort keys 基础上增加：
    a) 遇 "guard" 键且值为 null → 跳过（闭合 F-008）
    b) 遇 "id"/"from"/"to"/"entryNodeId" 键 → 用 id_map 重映射（闭合 F-007）
    c) 遇 "label" 键且值为 "return" → 替换为 "exit"（闭合 F-011）

阶段 4: 比较 normalized TS vs normalized RS
```

### 4.3 各规则细节

#### F-009（lift functions[0]）

Rust IR 顶层含 `functions: [{nodes, edges, entryNodeId, ...}]`，TS IR 顶层直接含 `nodes`/`edges`/`entryNodeId`。

归一化：若 `functions` 数组非空，取 `functions[0]` 的 `nodes`/`edges`/`entryNodeId` 提升为顶层，删除 `functions`。同时删除空的 `importedStdlib`/`stdlibImports` 数组。

#### F-007（节点 ID 重映射）

- TS: `entry1, bind2, ret4, end5` → `node0, node1, node2, node3`
- Rust: `n0, n1, n2, n3` → `node0, node1, node2, node3`
- 前提：两端 `nodes[]` 数组顺序一致（审计确认顺序相同，仅命名约定不同）
- 重映射作用于 `nodes[].id`、`edges[].from`、`edges[].to`、`entryNodeId`

#### F-008（strip null guard）

Rust `IREdge.guard: Option<String>` 无 `skip_serializing_if`，`None` 序列化为 `guard: null`。TS 省略该字段。

归一化：遇 `guard` 键且值为 `null` 时删除；非 null 的 guard 保留（语义 meaningful）。

#### F-011（label "return" → "exit"）

TS Terminal 节点 `label: "return"`，Rust 为 `label: "exit"`。

归一化：遇 `label` 键且值为 `"return"` 时替换为 `"exit"`。安全（"return" 仅出现在 Terminal 节点 label）。

### 4.4 diff-ir.ps1 改动

`$KnownDiffs` 数组清空（4 个 fixture 现在应 MATCH）：

```powershell
$KnownDiffs = @()  # F-007~F-012 closed in Phase 4 via ir-diff normalization
```

### 4.5 不在范围

- TS 参考实现任何改动（`reference/` 目录不动）
- Rust IR 序列化改动（不加 `#[serde(skip_serializing_if)]`，所有归一化在 ir-diff 侧）

### 4.6 B 组测试

`tests/audit/ir-diff/tests/` 新增 ir-diff 自身的单元测试：

| 测试 | 验证内容 |
|------|---------|
| `lift_functions_promotes_first_function_to_top_level` | functions[0] 提升 |
| `id_remap_normalizes_n0_to_node0` | Rust ID 重映射 |
| `id_remap_normalizes_entry1_to_node0` | TS ID 重映射 |
| `id_remap_applies_to_edges_and_entry` | from/to/entryNodeId 重映射 |
| `null_guard_stripped` | null guard 删除 |
| `non_null_guard_preserved` | 非 null guard 保留 |
| `return_label_normalized_to_exit` | label 归一化 |
| `end_to_end_expression_fixture_matches` | 端到端 MATCH |

加上 `diff-ir.ps1` 端到端验证：expression/hello/user/payment 4 fixture 输出 `[MATCH]`。

## 5. C 组：lower_rule_toggle.rs 增强

### 5.1 现状

当前 `lower_rule_toggle.rs`（62 行）：
- 签名 `pub fn lower_rule_toggle(checkbox_markdown: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph`
- 返回 `RuleGraph`（不带诊断），与 tree/table 的 `(RuleGraph, Vec<TangleDiagnostic>)` 不一致
- 无 span 跟踪（所有 `source_span: None`）
- 无诊断发射
- 名称提取仅支持反引号格式，fixture 实际用 `name: value` 格式导致全部回退为 `"flag"`
- 无 group/style 支持

### 5.2 改动 1：签名一致性

```rust
// 改前：
pub fn lower_rule_toggle(checkbox_markdown: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph

// 改后：
pub fn lower_rule_toggle(checkbox_markdown: &str, file: &str, id_gen: &mut FreshNodeId) -> (RuleGraph, Vec<TangleDiagnostic>)
```

`compile_to_ir.rs` 调用点（第 99-101 行）同步改为直接调用（去掉 `vec![]` 占位）。

### 5.3 改动 2：Span 跟踪

`checkbox_markdown.lines().enumerate()` 跟踪 1-based 行号。每个 checkbox 节点的 `source_span` 填充：

```rust
source_span: Some(SourceSpan {
    file: file.to_string(),
    start_line: line_no,
    start_column: 0,
    end_line: line_no,
    end_column: 0,
}),
```

entry 节点 span 为 `None`（无对应源行）。

### 5.4 改动 3：名称提取增强

新增 `extract_name(rest: &str) -> Option<String>` 函数，按优先级尝试：

1. **反引号**：`` `enable_new_ui`: desc `` → `enable_new_ui`（现有逻辑）
2. **冒号分隔**：`enable_new_ui: true` → `enable_new_ui`（取 `:` 前的标识符，需匹配 `[a-zA-Z_][a-zA-Z0-9_]*`）
3. **返回 None**：无法提取 → 发射 `TANGLE_RULE_TOGGLE_MISSING_NAME` 诊断，label 回退为 `toggle_{index}`

### 5.5 改动 4：诊断发射

新增 2 个诊断码：

| 诊断码 | 触发条件 | span |
|--------|---------|------|
| `TANGLE_RULE_TOGGLE_MALFORMED` | 行以 `- [` 或 `* [` 开头但不含 `[x]`/`[X]`/`[ ]`（如 `- [?]`） | 当前行 |
| `TANGLE_RULE_TOGGLE_MISSING_NAME` | checkbox 行无法提取名称（无反引号且无冒号分隔） | 当前行 |

诊断 span 使用改动的行号，与 Phase 3 的 tree/table span 跟踪一致。

### 5.6 改动 5：group/style 元数据

**语法**：HTML 注释行，作用于下一个 checkbox 节点：

```markdown
@rule.toggle

<!-- group: Approval -->
- [x] enable_new_ui: true

<!-- style: highlight -->
- [ ] enable_crypto: false
```

**解析逻辑**：
- 遇 `<!-- group: X -->` 行 → 缓存 `pending_group = Some("X")`
- 遇 `<!-- style: Y -->` 行 → 缓存 `pending_style = Some("Y")`
- 遇 checkbox 行 → 创建节点时消费缓存的 `pending_group`/`pending_style`，清空缓存
- 缓存跨空行保留，遇非注释非 checkbox 行清空

### 5.7 节点结构

checkbox 节点 label 保持 `"{name} = {checked}"` 格式（codegen 已依赖此 label 格式发注释）。节点 kind 保持 `IRNodeKind::Compute`。

### 5.8 不在范围

- toggle 节点的 codegen 改动（现有 codegen 已通过 label 注释处理 toggle 节点）
- toggle 的 Decision 语义（保持 Compute 节点类型）
- 跨多个 `@rule.toggle` 块的 group/style 继承

### 5.9 C 组测试

`compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs`：

| 测试 | 验证内容 |
|------|---------|
| `toggle_span_tracking_populates_line_numbers` | 节点 span 行号 ≠ 0 |
| `toggle_name_extraction_from_backtick` | 反引号格式名称提取 |
| `toggle_name_extraction_from_colon` | 冒号格式名称提取 |
| `toggle_missing_name_emits_diagnostic` | 无名称时发 TANGLE_RULE_TOGGLE_MISSING_NAME |
| `toggle_malformed_checkbox_emits_diagnostic` | 畸形 checkbox 发 TANGLE_RULE_TOGGLE_MALFORMED |
| `toggle_group_style_metadata_attached` | group/style 附加到节点 |
| `toggle_group_style_pending_cleared_on_non_checkbox` | 缓存清空 |
| `toggle_signature_returns_diagnostics_vec` | 签名一致性 |

Fixture 更新：`tests/rules/feature-toggles.tangle.md` 扩展为包含 group/style 注释 + 冒号格式名称的完整测试 fixture。

## 6. 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `compiler/tangle-cli/src/codegen/py_emitter.rs` | Python codegen + if/elif 分支 | 修改 |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | Go codegen + if/else if 分支 | 修改 |
| `compiler/tangle-cli/src/ir/lower_rule_toggle.rs` | Toggle lowering 增强 | 修改 |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | Toggle 调用点签名同步 | 修改 |
| `tests/audit/ir-diff/src/main.rs` | ir-diff 4 阶段归一化 | 修改 |
| `tests/audit/diff-ir.ps1` | 清空 $KnownDiffs | 修改 |
| `compiler/tangle-cli/tests/v04_phase4/py_go_codegen.rs` | A 组测试 | 创建 |
| `compiler/tangle-cli/tests/v04_phase4/toggle_lowering.rs` | C 组测试 | 创建 |
| `tests/audit/ir-diff/tests/normalize_tests.rs` | B 组 ir-diff 单元测试 | 创建 |
| `tests/rules/feature-toggles.tangle.md` | Toggle fixture 扩展 | 修改 |

## 7. 测试策略

### 7.1 新增测试目录

`compiler/tangle-cli/tests/v04_phase4/`（A 组 + C 组）
`tests/audit/ir-diff/tests/`（B 组 ir-diff 自身测试）

### 7.2 回归测试

- 现有 `audit_regression/` 测试全绿（codegen 输出变化不影响 IR 级诊断）
- 现有 Phase 3 测试全绿（`v03_phase3/` 目录）
- 现有 Phase 2 快照测试匹配（IR 层无变化）
- `diff-ir.ps1` 保持 0 unexpected DIFF（4 KNOWN_DIFF 转 MATCH）

## 8. 出口闸门

| 闸门 | 命令 | 标准 |
|------|------|------|
| 1. 单元+集成测试 | `cargo test --workspace` | 全绿 |
| 2. Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 警告 |
| 3. 审计回归 | `tests/audit/run-audit.ps1` | 0 failing |
| 4. 差分测试 | `tests/audit/diff-ir.ps1` | 0 KNOWN_DIFF + 0 unexpected DIFF |
| 5. Phase 3 回归 | `cargo test --test js_codegen --test py_go_codegen --test span_tracking --test snapshots` | 全绿 |
| 6. Phase 4 新测试 | `cargo test --test v04_phase4` | 全绿 |
| 7. Toggle fixture | `cargo run -- build tests/rules/feature-toggles.tangle.md --emit-ir` | IR 含 span + 正确名称 |

## 9. 不在 Phase 4 范围内

- Py/Go 的 `source_text` 翻译（保持注释模式）
- TS 参考实现的 rule lowering 实现（F-010 保持 SKIPPED，未来项目）
- 递归深度限制（三宿主一致，不加）
- toggle 的 Decision 语义（保持 Compute）
- 跨 `@rule.toggle` 块的 group/style 继承
- Phase 1 推迟项（泛型类型系统、返回类型推导等）

## 10. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Py/Go if/else 发射改变现有 codegen 输出 | 现有测试可能失败 | 更新受影响测试；IR 级测试不受影响 |
| ir-diff ID 重映射前提（nodes 顺序一致）不成立 | 归一化后仍 DIFF | 审计已确认顺序一致；端到端测试验证 4 fixture |
| toggle 签名改动影响 compile_to_ir 调用 | 编译失败 | 同步修改调用点，编译验证 |
| group/style HTML 注释语法与现有 fixture 冲突 | 现有 fixture 行为变化 | 现有 fixture 不含 HTML 注释，无冲突 |
