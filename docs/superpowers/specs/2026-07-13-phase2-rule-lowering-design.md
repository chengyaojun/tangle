# Phase 2 — Rule Lowering 设计规格

> **版本**: v0.3.0 Phase 2
> **日期**: 2026-07-13
> **作者**: brainstorming session
> **状态**: 待实现
> **前置**: Phase 1（Call 表达式类型检查）已完成，v0.3.0 tag @ `42cf181` on `audit/v0.2.1`

## 1. 背景与目标

### 1.1 Roadmap 定位

v0.3.0 分四个阶段，Phase 2 聚焦 **Rule Lowering 完整性**：

| Phase | 状态 | 焦点 |
|-------|------|------|
| 1 — Stdlib Signatures | ✅ 完成 | Call 表达式类型检查、19 模块签名注册表 |
| **2 — Rule Lowering** | **本文档** | 嵌套列表 AND/OR 结合性、表格行优先级+重叠检测、Mermaid graph/subgraph 解构 |
| 3 — Typed Codegen | ⬜ | AST 化类型代码生成 |
| 4 — IR Interpreter | ⬜ | 原生 IR 树遍历解释器 |

### 1.2 现状缺陷

现有 4 个 rule lowering 模块均为**扁平化、最小化实现**：

| 模块 | 现状 | Phase 2 缺口 |
|------|------|-------------|
| `lower_rule_tree.rs` (53 行) | 把 `* `/`- ` 列表项串成线性链，忽略缩进 | 无嵌套、无 AND/OR 语义、无结合性、无 leaf/action 区分 |
| `lower_rule_table.rs` (88 行) | 每行→一个 Action 节点，条件用 `" AND "` 字符串拼接，全部从 entry 扁平连出 | 无行优先级排序、无重叠检测、无优先级编码 |
| `lower_rule_flow.rs` (156 行) | 解析 Mermaid `graph` 语法（节点声明 + `-->` 边 + `\|guard\|`），首节点作 entry | 无 `subgraph` 解析、无环检测、仅 `-->` 单一边类型 |
| `lower_rule_toggle.rs` (59 行) | `- [x]` 复选框→Compute 节点 | Roadmap 未提及，不在 Phase 2 范围 |

### 1.3 相关审计遗留项

| Finding | 状态 | Phase 2 处理 |
|---------|------|-------------|
| F-007~F-012 (IR schema 差异 TS vs Rust) | KNOWN_DIFF, 推迟到 v0.3.0 | **不动** — Phase 2 仅扩展 Rust 端，不更新 TS 参考 |
| F-010 (TS 端 rule lowering 完全缺失) | 4 rule fixture diff SKIPPED | **不动** — TS 端不实现，diff 保持 SKIPPED |

### 1.4 成功标准

- 三个子主题（tree/table/flow）的 rule lowering 完整实现
- IR schema 最小扩展，向后兼容现有 lowering
- 快照测试覆盖所有新特性
- 出口闸门 6 项全部 PASS

## 2. 架构概览

### 2.1 模块结构

Phase 2 三个子主题作为**独立单元**实现，各自有边界清晰的 lowering 模块，通过共享的 IR schema 扩展汇合：

```
compile_to_ir.rs (dispatch，不动)
    ├── lower_rule_tree.rs   (重写: DNF lowering)
    ├── lower_rule_table.rs  (扩展: 优先级 + 重叠检测)
    ├── lower_rule_flow.rs   (扩展: subgraph + 多边类型 + 样式)
    └── lower_rule_toggle.rs (不动)
```

三个模块各自独立可测试，通过 `compile_to_ir.rs` 的 `collect_rule_graphs` 统一调度（现有架构不变）。快照测试按模块组织。

### 2.2 IR 扩展方案：最小拓扑扩展

语义由图拓扑隐式编码，IR 结构改动最小：

- **Tree**: OR = 从 entry 分出多条 Condition 边（首匹配胜出）；AND = 路径内顺序 Decision 节点链；Action = 路径末端 Action 节点
- **Table**: 优先级 = entry 出边的 `priority` 字段（第一条边 priority=0，最高优先级）；重叠 = 编译期诊断
- **Mermaid**: `IRNode` 加 `group` 字段保留 subgraph 分组；新增 `IREdgeKind` 变体映射多边类型；样式作为节点/边的可选元数据

**不引入**显式 `Or`/`And`/`Priority` 节点类型（YAGNI——codegen 消费前是冗余的）；**不引入**分层 IR（无第二层消费者）。

## 3. IR Schema 变更

### 3.1 `IRNode` 新增字段

```rust
pub struct IRNode {
    pub id: String,
    pub kind: IRNodeKind,
    pub label: String,
    pub source_span: Option<SourceSpan>,
    pub source_text: Option<String>,
    pub group: Option<String>,      // 新增: Mermaid subgraph 名称
    pub style: Option<String>,      // 新增: Mermaid classDef 样式类名或内联描述
}
```

### 3.2 `IREdge` 新增字段

```rust
pub struct IREdge {
    pub from: String,
    pub to: String,
    pub kind: IREdgeKind,
    pub guard: Option<String>,
    pub source_span: Option<SourceSpan>,
    pub priority: Option<u32>,      // 新增: table 行优先级（仅 table lowering 填充）
    pub style: Option<String>,      // 新增: Mermaid linkStyle 描述
}
```

### 3.3 `IREdgeKind` 新增变体

```rust
pub enum IREdgeKind {
    Control,
    Condition,
    Error,
    Dashed,   // 新增: Mermaid -.-> 映射
    Thick,    // 新增: Mermaid ==> 映射
    Crossed,  // 新增: Mermaid --x 映射
}
```

### 3.4 `IRNodeKind` 不变

AND/OR/优先级由拓扑编码，不加新节点类型。

### 3.5 `RuleGraph` 不变

subgraph 扁平化进 `nodes`/`edges`，分组信息在节点的 `group` 字段。

### 3.6 向后兼容

- 现有 `lower_statements`/`lower_function_body`：新字段填 `None`
- `validate.rs`：可选增加 `group` 一致性检查（同一 group 的节点应连续）
- codegen：新 `IREdgeKind` 变体需映射到各宿主语言（JS/Py/Go），但 Phase 2 **仅 lower 到 IR**，codegen 消费留待 Phase 3

## 4. Tree Lowering 设计

`lower_rule_tree.rs` 完全重写，从 53 行的线性链实现改为 DNF 感知的嵌套树 lowering。

### 4.1 输入解析

解析嵌套列表为缩进感知的树结构：

```rust
struct ListNode {
    text: String,
    depth: usize,
    children: Vec<ListNode>,
}

fn parse_list_to_tree(markdown: &str) -> Vec<ListNode>
```

按前导空格数计算 depth（每 4 空格或 1 tab = 1 级）。过滤非列表行（不以 `* ` 或 `- ` 开头的行）。

### 4.2 DNF 语义映射

| 结构 | 语义 | IR 编码 |
|------|------|---------|
| depth-1 项 | OR 分支标签（描述性，非条件） | entry 分出多条 Condition 边 |
| depth-2+ 项 | AND 条件（全部须匹配） | 路径内顺序 Decision 节点链 |
| `* Action: x` 子项 | 该分支的动作 | 路径末端 Action 节点 |

深度模型固定：**OR→AND→AND→AND**。depth-1 = OR 分支，depth-2 及以下全部 = AND。不交替，不支持嵌套 OR。AND 本身满足结合律，无歧义。

### 4.3 IR 拓扑示例

输入：
```
* Branch A
    * Income: high
    * Credit: good
    * Action: approve
* Branch B
    * Income: low
    * Action: reject
```

生成：
```
entry(Decision) ──[Income: high]──> n1(Decision) ──[Credit: good]──> n2(Decision) ──[]──> n3(Action: approve)
             └──[Income: low]──> n4(Decision) ──[]──> n5(Action: reject)
```

OR = 从 entry 分出多条边（codegen 生成 `if/else if`）；AND = 路径内链式（codegen 生成 `&&`）；Action = 路径终点。

### 4.4 诊断

| 诊断码 | 级别 | 触发条件 |
|--------|------|---------|
| `TANGLE_RULE_NO_ACTION` | 警告 | OR 分支无 `Action:` 子项 |
| `TANGLE_RULE_EMPTY_BRANCH` | 警告 | depth-1 项无任何子项 |
| `TANGLE_RULE_DEEP_NESTING` | 信息 | depth > 4（不阻塞） |

### 4.5 Fixture 变更

现有 `tests/rules/decision-tree.tangle.md` 不符合新模型（`Result: true` 是 depth-1 叶子，无 `Action:` 标记）。重写为：

```
* Approve path
    * Income check: true
    * Credit check: true
    * Collateral: true
    * Action: approve
* Reject path
    * Action: reject
```

## 5. Table Lowering 设计

`lower_rule_table.rs` 扩展，保留现有解析逻辑，新增优先级编码和重叠检测。

### 5.1 优先级编码

行顺序 = 优先级（first-match-wins）。IR 中通过 entry 出边的 `priority` 字段编码：第一行 = priority 0，第二行 = priority 1，以此类推。

```
| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| low | - | review |
| - | poor | reject |
```

生成：
```
entry(Decision) ──[priority:0, Income=high AND Credit=good]──> n1(Action: approve)
              ──[priority:1, Income=low]──────────────────> n2(Action: review)
              ──[priority:2, Credit=poor]─────────────────> n3(Action: reject)
```

`IREdge.priority` 显式记录优先级，codegen 按 priority 排序生成 `if/else if` 链。

### 5.2 重叠检测算法

两行重叠 = 所有列的值集相交。`-` = 全集（通配）。

```rust
fn rows_overlap(row_a: &[String], row_b: &[String]) -> bool {
    row_a.iter().zip(row_b.iter())
        .all(|(a, b)| a == "-" || b == "-" || a == b)
}
```

对每对行 `(i, j)` 且 `i < j`（i 优先级更高）执行检测。若重叠，发出 `TANGLE_RULE_OVERLAP` 警告，指明行号和重叠单元。

### 5.3 诊断

| 诊断码 | 级别 | 触发条件 |
|--------|------|---------|
| `TANGLE_RULE_OVERLAP` | 警告 | 两行条件相交，行 {i} 优先级高于 {j} |
| `TANGLE_RULE_DUPLICATE` | 警告 | 两行完全相同（所有列一致），冗余行 |
| `TANGLE_RULE_UNREACHABLE` | 信息 | 某行被更高优先级的通配行完全覆盖 |

### 5.4 Fixture 变更

- 现有 `tests/rules/decision-table.tangle.md` 保留作为"无重叠"基准
- **新增** `tests/rules/decision-table-overlap.tangle.md`：含通配符和重叠，验证 `TANGLE_RULE_OVERLAP` 诊断

## 6. Mermaid Lowering 设计

`lower_rule_flow.rs` 扩展。这是范围最大的子主题（完整 subgraph + 多边类型 + 样式）。

### 6.1 Subgraph 解析

跟踪 `subgraph <name>` ... `end` 块的嵌套栈。每个在 subgraph 内声明的节点填充 `group: Some(<name>)`。跨 subgraph 的边保留，端点的 `group` 各自独立。

```
graph TD
    A[Start] --> B{Decision}
    subgraph Approval
        B -->|yes| C[Approve]
    end
    subgraph Rejection
        B -->|no| E[Reject]
    end
```

结果：`A.group=None`, `B.group=None`, `C.group=Some("Approval")`, `E.group=Some("Rejection")`。

### 6.2 边类型映射

| Mermaid | IREdgeKind | 语义（由 codegen 解释） |
|---------|-----------|----------------------|
| `-->` | `Control`（默认） | 正常控制流 |
| `-.->` | `Dashed` | 异步/可选流 |
| `==>` | `Thick` | 高优先级/强制流 |
| `--x` | `Crossed` | 错误/失败路径 |

解析器用正则识别 4 种边操作符。guard 语法 `-->|label|` 对所有类型通用（如 `-.->|async| B`）。

### 6.3 节点形状 → IRNodeKind

| Mermaid 形状 | IRNodeKind |
|-------------|-----------|
| `id[Label]` | `Action` |
| `id{Label}` | `Decision` |
| `id(Label)` | `Action` |
| `id((Label))` | `Terminal` |

label 含 `error:`/`错误:` 前缀 → `ErrorTerminal`（保留现有逻辑）。

### 6.4 样式处理

| Mermaid 语法 | IR 映射 |
|-------------|---------|
| `classDef className fill:#f9f` | 注册到 `HashMap<String, String>`（类名→样式描述） |
| `class A className` | `IRNode.style = Some("className")` |
| `style A fill:#f9f` | `IRNode.style = Some("fill:#f9f")`（内联描述） |
| `linkStyle 0 stroke:#ff3` | `IREdge.style = Some("stroke:#ff3")` |

样式描述是**不透明字符串**——lowering 只保留，不解析颜色/笔触。codegen/docgen 决定是否消费。

### 6.5 入口节点检测

改为"无入边的首个节点 = entry"（当前是"首个声明的节点"，脆弱）。若无入边节点 > 1，发 `TANGLE_RULE_MULTI_ENTRY` 信息。若无无入边节点，回退到首个声明节点并发 `TANGLE_RULE_NO_ENTRY` 警告。

### 6.6 注释

`%%` 开头的行跳过。

### 6.7 诊断

| 诊断码 | 级别 | 触发条件 |
|--------|------|---------|
| `TANGLE_RULE_MULTI_ENTRY` | 信息 | 多个无入边节点 |
| `TANGLE_RULE_NO_ENTRY` | 警告 | 无无入边节点，回退到首声明 |
| `TANGLE_RULE_UNREACHABLE` | 信息 | 有入边但无出边的非 Terminal 节点 |
| `TANGLE_RULE_DANGLING_SUBGRAPH` | 警告 | `end` 无匹配 `subgraph` 或反之 |
| `TANGLE_RULE_UNKNOWN_EDGE` | 警告 | 未识别的边操作符（如 `o--o`），降级为 `Control` |

### 6.8 Fixture 变更

- 现有 `tests/rules/approval-flow.tangle.md` 保留作为基础
- **新增** `tests/rules/approval-flow-subgraph.tangle.md`：含 subgraph + 多边类型 + 样式，覆盖完整特性

## 7. 测试策略

### 7.1 测试分层

| 层级 | 内容 | 位置 |
|------|------|------|
| 单元测试 | 解析逻辑（缩进树/表格/Mermaid）、重叠检测算法、边类型映射、subgraph 栈 | 各模块 `#[cfg(test)]` |
| 快照测试 | 输入 markdown → lowered RuleGraph → JSON 序列化 → 与 golden file 对比 | `compiler/tangle-cli/tests/v03_phase2/snapshots/` |
| 诊断测试 | 各 `TANGLE_RULE_*` 诊断码的触发与抑制 | `compiler/tangle-cli/tests/v03_phase2/diagnostics_*.rs` |
| 回归测试 | 确保 Phase 1 + audit 修复无回归 | 现有 `tests/audit_regression/` + `tests/v03_phase1/` 不变 |

### 7.2 快照测试机制

使用 [`insta`](https://crates.io/crates/insta) crate（Rust 生态标准快照测试工具）：

- 输入：各 fixture 的 rule 段落
- 输出：`RuleGraph` 的 `serde_json::to_string_pretty` 结果
- 快照文件：`tests/v03_phase2/snapshots/<test_name>.snap`
- `cargo insta review` 审查变更，`cargo insta accept` 确认更新

快照测试用例：

| 用例 | 子主题 | 覆盖 |
|------|--------|------|
| `tree_basic` | tree | 单分支 + 单条件 + Action |
| `tree_dnf` | tree | 多 OR 分支 + 多 AND 条件 |
| `tree_no_action` | tree | 分支无 Action（触发警告） |
| `table_basic` | table | 无重叠基准 |
| `table_overlap` | table | 通配符 + 重叠 |
| `table_unreachable` | table | 通配行覆盖后续行 |
| `flow_basic` | flow | 基础 `-->` + 节点形状 |
| `flow_subgraph` | flow | subgraph 分组 |
| `flow_multi_edge` | flow | 4 种边类型 |
| `flow_styles` | flow | classDef + style + linkStyle |

### 7.3 Fixture 清单

| Fixture | 动作 | 用途 |
|---------|------|------|
| `tests/rules/decision-tree.tangle.md` | **重写** | DNF 模型 + `Action:` 标记 |
| `tests/rules/decision-table.tangle.md` | 保留 | 无重叠基准 |
| `tests/rules/decision-table-overlap.tangle.md` | **新增** | 通配符 + 重叠 + 不可达行 |
| `tests/rules/approval-flow.tangle.md` | 保留 | 基础 Mermaid |
| `tests/rules/approval-flow-subgraph.tangle.md` | **新增** | subgraph + 多边类型 + 样式 |

## 8. 出口闸门

| 闸门 | 标准 |
|------|------|
| 1. 单元+集成测试 | `cargo test --workspace` 全绿（现有 + Phase 2 新增） |
| 2. Clippy | `cargo clippy --workspace -- -D warnings` 0 警告 |
| 3. 审计回归 | `run-audit.ps1` 210 cells 0 failing（现有 fixture 无回归） |
| 4. 差分测试 | `diff-ir.ps1` — F-010 仍 SKIPPED，F-007~F-012 仍 KNOWN_DIFF（不动 TS schema） |
| 5. 快照测试 | 所有 Phase 2 快照匹配，`cargo insta test` 全绿 |
| 6. 诊断测试 | 所有 `TANGLE_RULE_*` 诊断码按预期触发/抑制 |

## 9. 不在 Phase 2 范围内

- TS 参考实现的 rule lowering（F-010 保持 SKIPPED）
- IR schema 与 TS 对齐（F-007~F-012 保持 KNOWN_DIFF）
- `lower_rule_toggle.rs` 改动（Roadmap 未提及）
- codegen 消费新 `IREdgeKind` 变体（`Dashed`/`Thick`/`Crossed`）——仅 lower 到 IR，codegen 消费留待 Phase 3
- codegen 消费 `group`/`style`/`priority` 字段——同上，Phase 3 处理

## 10. 依赖与风险

### 10.1 依赖

- Phase 1 已完成的 IR 基础结构（`IRNode`/`IREdge`/`RuleGraph`）
- `insta` crate（需添加到 `[dev-dependencies]`）

### 10.2 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| Mermaid 样式范围最大（用户选择"完整：+样式"） | 解析复杂度高，可能需多轮迭代 | 优先实现 subgraph + 多边类型，样式作为最后子任务 |
| `insta` 引入新依赖 | 构建/CI 环境需联网拉取 | 确认 crates.io 可达；备选方案：手写 golden file 比较 |
| fixture 重写可能影响审计 | `run-audit.ps1` 依赖现有 fixture | 重写后立即运行审计，确保 0 failing |
| 新 `IREdgeKind` 变体未被 codegen 消费 | codegen 可能 panic 或忽略 | Phase 2 仅 lower，codegen 改动留待 Phase 3；确保 codegen 对未知 kind 有 fallback |

## 11. 实现顺序建议

1. **IR schema 变更**（`graph.rs`）— 加字段 + enum 变体，确保现有测试不回归
2. **Tree lowering**（`lower_rule_tree.rs` 重写）— 独立，无跨模块依赖
3. **Table lowering**（`lower_rule_table.rs` 扩展）— 独立
4. **Mermaid lowering**（`lower_rule_flow.rs` 扩展）— 范围最大，最后做
5. **快照测试 + 诊断测试** — 随各模块实现同步编写（TDD）
6. **Fixture 变更** — 各模块实现时同步更新
7. **出口闸门验证** — 全部实现完成后运行 6 项闸门
