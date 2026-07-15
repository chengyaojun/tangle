# Phase 5: 差分测试闭合 + 工程性推迟项 设计规格

## 1. 背景与目标

### 1.1 背景

Phase 4 (v0.4.0) 闭合了 Phase 3 的 4 项推迟，但留下 2 个差分测试缺口 + 2 个工程性推迟项：

| 推迟项 | 来源 | 现状 |
|--------|------|------|
| payment.tangle KNOWN_DIFF | Phase 4 §4.4 注释 | diff-ir 报 1 KNOWN_DIFF（Rust dual-entry: 顶层 merged + functions[] 双存） |
| F-010 TS 参考实现 rule lowering | Phase 4 §9 | diff-ir 报 7 SKIPPED（6 rule fixture + order-service 失败） |
| 递归深度限制 | Phase 4 §3.5 | 三宿主 emit_branch_body 仅有 visited 集合，无 depth 计数 |
| 跨 toggle 块 group/style 继承 | Phase 4 §5.8 | 语义未定义，当前"缓存遇非注释非 checkbox 行清空" |

### 1.2 目标

1. **A1 — Rust IR 结构自洽**：修复 dual-entry，让 Rust IR 在有 main 函数时顶层不再重复 `functions[]` 内容；ir-diff 支持多 function 比较；Py/Go 补 `emit_multi_function`
2. **A2 — TS 参考实现 rule lowering**：忠实 port Rust 端 4 个 lower_rule_*.rs 到 TS，让 6 个 rule fixture 输出与 Rust MATCH 的 IR
3. **B2 — 递归深度限制**：三宿主 codegen 加 depth 计数器，阈值 100，超限发注释停止递归
4. **B4 — 跨 toggle 块 group/style 继承语义**：明确文档化"不继承"，加测试覆盖

### 1.3 成功标准

- `tests/audit/diff-ir.ps1` 报告 **0 KNOWN_DIFF + 0 SKIPPED + 全 MATCH**（3 → 10 MATCH，含 payment + 6 rule fixture + order-service）
- payment.tangle Rust IR 顶层 `nodes[]` 在 `functions[]` 非空时不含 Callable 代码块节点
- Py/Go emitter 在 `functions[]` 非空时发射多函数代码（与 JS 一致）
- 三宿主 `emit_branch_body` 含 `depth: usize` 参数，超 100 发 `// max depth reached` 注释
- 跨 toggle 块 group/style 不继承的行为有测试覆盖
- 出口闸门 8 项全部 PASS

## 2. 架构概览

### 2.1 工作流划分

4 个工作流，A1 内部有 3 个紧耦合子任务（必须一起改才能通过编译/测试），A2/B2/B4 相互独立：

```
工作流 A1: Rust dual-entry 修复（紧耦合，必须一起改）
    ├── compile_to_ir.rs: 当 functions 非空时，不合并 @tangle 块到顶层
    ├── ir-diff/main.rs: lift_functions → compare_functions（多 function 比较）
    ├── py_emitter.rs: 新增 emit_multi_function_py
    ├── go_emitter.rs: 新增 emit_multi_function_go
    └── js_emitter.rs: 无改动（已支持 multi-function）

工作流 A2: TS 参考实现 rule lowering（独立，与 A1 无交叉）
    ├── reference/src/ir/compileToIR.ts: 接入 4 个 rule lower 调用
    ├── reference/src/ir/ruleTree.ts: 忠实 port lower_rule_tree.rs（DNF 语义）
    ├── reference/src/ir/ruleTable.ts: 忠实 port lower_rule_table.rs（priority + overlap）
    ├── reference/src/ir/ruleFlow.ts: 忠实 port lower_rule_flow.rs（subgraph + multi-edge）
    └── reference/src/ir/ruleToggle.ts: 忠实 port lower_rule_toggle.rs（group + style + diagnostics）

工作流 B2: 递归深度限制（独立，三宿主对称改动）
    ├── js_emitter.rs: emit_branch_body 加 depth 参数
    ├── py_emitter.rs: 同
    └── go_emitter.rs: 同

工作流 B4: 跨 toggle 块 group/style 不继承语义（独立）
    └── lower_rule_toggle.rs: 加注释明确语义 + 测试覆盖
```

### 2.2 前提

基于 `main` 分支（commit `8a3df7e` v0.4.0）创建 worktree：
```bash
git worktree add .worktrees/phase5-v0.5.0 -b phase5/v0.5.0 main
```

### 2.3 版本

完成后 merge `phase5/v0.5.0` 到 `main`，打 tag `v0.5.0`。

## 3. A1：Rust dual-entry 修复 + ir-diff 多 function + Py/Go 多函数

### 3.1 现状分析

`compile_to_ir.rs:13-64` 当前流程：

```
1. 遍历 parsed_blocks → 合并到顶层 merged_graph          ← 第 19-29 行
2. 遍历 headings 收集 rule_graphs → 合并到顶层 merged_graph ← 第 32-43 行
3. collect_functions：每个 Callable heading 单独构建 functions[]  ← 第 60-64 行
   （仅当含 main 时赋值给 graph.functions）
```

**dual-entry 根因**：步骤 1 把所有 `@tangle` 代码块（含 main/process 的代码块）合并到顶层 `merged_graph`，步骤 3 又把同样的代码块单独构建到 `functions[]`。顶层 nodes[] 与 functions[] 重复存了同样的语义内容。

**codegen 消费现状**：
- JS `js_emitter.rs:205-208`：`if !graph.functions.is_empty() { emit_multi_function_js } else { emit_single_function_js }` ✓
- Py/Go：**只消费顶层 `graph.nodes`**，没有 multi-function 分支 ✗

### 3.2 改动 1：Rust compile_to_ir.rs 清理顶层

**核心规则**：当 `functions[]` 非空时，顶层 `nodes[]`/`edges[]` 不含 Callable 代码块内容；rule lowering 结果仍合并到顶层（rule 是模块级，不属于任何 function）。

```rust
// compile_to_ir.rs 改造后流程：
let mut merged_graph: Option<RuleGraph> = None;
let mut has_main_callable = false;

// 1. 检测是否有 main Callable heading（决定是否启用 multi-function 模式）
collect_function_names(&checked.headings, &mut has_main_callable);

// 2. Lower rule blocks（always 合并到顶层）
let mut rule_graphs: Vec<RuleGraph> = vec![];
collect_rule_graphs(...);
for sub_graph in rule_graphs { merge_into(&mut merged_graph, sub_graph); }

// 3. Lower @tangle blocks
//    - has_main_callable=true: 不合并到顶层（functions[] 会单独存）
//    - has_main_callable=false: 合并到顶层（fallback 单函数模式）
if !has_main_callable {
    for block in &checked.parsed_blocks {
        let sub_graph = lower_statements(...);
        merge_into(&mut merged_graph, sub_graph);
    }
}

// 4. collect_functions：仅 has_main_callable=true 时调用
if has_main_callable {
    let mut functions: Vec<IRFunction> = vec![];
    collect_functions(...);
    graph.functions = functions;
}
```

**payment.tangle 改造后预期**：
- 顶层 `nodes[]`：空（payment 无 rule lowering 结果）
- `functions[]`：[main (4 节点), process (3 节点)]
- 与 TS 端结构对齐（A2 让 TS 也生成 functions[]）

**expression/hello/user 不受影响**：这些 fixture 无 main Callable heading，走 fallback 单函数模式，顶层 nodes[] 仍含代码块节点。

### 3.3 改动 2：ir-diff 多 function 比较

**现状**：`lift_functions` 只提升 `functions[0]` 到顶层，多 function 时 functions[1+] 丢失。payment 有 main+process 两个 function，process 不被比较。

**改造**：`lift_functions` 改为 `compare_functions`，对 `functions[]` 数组整体比较：

```rust
fn compare_functions(ts: Value, rs: Value) -> (Value, Value) {
    // 1. 提取 functions[] 数组（若空则用顶层 nodes/edges/entryNodeId 包装为单 function）
    // 2. 数组顺序对齐（按 function.name 排序，确保 main 与 main 对齐）
    // 3. 对每个 function 分别 normalize（已有逻辑）
    // 4. 返回归一化后的 functions[] 数组用于比较
}
```

**关键设计**：
- `functions[]` 数组按 `name` 字段排序后比较（避免顺序差异）
- 每个 function 内部仍用 `build_id_map` + `normalize`（已有逻辑，作用域为单 function）
- 无 `functions[]` 的 IR（如 expression/hello/user）包装为 `[{name: "module", nodes: ..., edges: ..., entryNodeId: ...}]` 后比较

### 3.4 改动 3：Py/Go emit_multi_function

**参照**：`js_emitter.rs` 的 `emit_multi_function_js`（已实现）。

**Py 版本**：
```python
def main(params):
    # body...

def process(params):
    # body...
```

- 每个函数独立发射，函数名 = `IRFunction.name`
- 函数签名：`def {name}({params}):`（params 来自 `IRFunction.params`，当前 payment 的 params 为空）
- 函数体：复用现有 BFS + `emit_decision_branch` 逻辑，作用域为单 function 的 `nodes[]`/`edges[]`/`entryNodeId`
- 入口：模块末尾发 `if __name__ == "__main__": main()`（仅当含 main 时）

**Go 版本**：
```go
func main(params) {
    // body...
}

func process(params) {
    // body...
}
```

- 函数签名：`func {name}({params}) {`（Go 大括号同行）
- 函数体：复用现有 BFS + `emit_decision_branch`
- 入口：Go 的 `main` 自动入口，无需额外调用

**关键差异**：Py/Go 的 `emit_branch_body` 接收单 function 的 `nodes[]`/`edges[]` 切片，而非整个 graph。BFS 主循环改为遍历 `graph.functions`，对每个 function 单独发射。

### 3.5 不在范围

- JS codegen 改动（已支持 multi-function，不动）
- `IRFunction` 结构扩展（params/receiver 当前为空/null，未来 phase 处理）
- codegen 的 `source_text` 翻译（B1 范围，不在 Phase 5）

### 3.6 A1 测试

`compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs`（Cargo name: `phase5_dual_entry`）：

| 测试 | 验证内容 |
|------|---------|
| `payment_top_level_empty_when_functions_present` | payment IR 顶层 nodes[] 为空 |
| `payment_functions_array_has_main_and_process` | functions[] 含 2 个 function |
| `expression_top_level_populated_when_no_main` | expression 仍走 fallback 模式 |
| `py_multi_function_emits_main_and_process` | Py 输出含 `def main` 和 `def process` |
| `go_multi_function_emits_main_and_process` | Go 输出含 `func main` 和 `func process` |
| `py_single_function_fallback_when_no_main` | 无 main 时 Py 走单函数模式 |
| `ir_diff_compares_multi_function_arrays` | ir-diff 对多 function 数组整体比较 |
| `ir_diff_aligns_functions_by_name` | ir-diff 按 name 排序对齐 |

加上 `diff-ir.ps1` 端到端：payment.tangle 输出 `[MATCH]`。

## 4. A2：TS 参考实现 rule lowering 忠实 port

### 4.1 现状分析

| 文件 | Rust 行数 | TS 行数 | 缺失能力 |
|------|-----------|---------|---------|
| `lower_rule_tree.rs` | 313 | 18 | DNF 范式 lowering、AND/OR 缩进语义、Action 标记 |
| `lower_rule_table.rs` | 262 | 28 | priority 排序、overlap 检测、wildcard 行 |
| `lower_rule_flow.rs` | 535 | 42 | Mermaid subgraph 解构、多 edge 类型（Dashed/Thick/Crossed）、group/style |
| `lower_rule_toggle.rs` | 150 | 19 | group/style 注释、名称提取、诊断发射 |
| **总计** | **1260** | **107** | — |

TS 端 `compileToIR.ts` 第 13-27 行只 lower `@tangle` 代码块，**完全没调用 4 个 rule lower 函数**。这是 6 个 rule fixture 输出空 IR 的直接原因。

### 4.2 port 策略

**忠实 port 原则**（策略 X）：
- TS 端生成的 IR JSON 与 Rust 端**字段完全一致**（相同 `kind`/`label`/`guard`/`from`/`to`/`group`/`style`/`priority` 值）
- 节点 ID 命名约定保持差异（TS 用 `entry1/bind2/ret3`，Rust 用 `n0/n1/n2`），ir-diff 的 `build_id_map` 已处理
- 诊断码字符串完全一致（如 `TANGLE_RULE_TABLE_OVERLAP`）
- 不引入 TS 特有优化（如 async/await、iterator helpers）

**port 顺序**（按依赖）：
1. `ruleToggle.ts`（最简单，150 行）—— 验证 port 模式可行
2. `ruleTree.ts`（313 行）—— DNF 语义核心
3. `ruleTable.ts`（262 行）—— priority + overlap
4. `ruleFlow.ts`（535 行）—— 最复杂，Mermaid 解析

### 4.3 改动 1：reference/src/ir/compileToIR.ts 接入 rule lower

```typescript
// compileToIR.ts 改造后：
import { lowerRuleTree } from "./ruleTree.js";
import { lowerRuleTable } from "./ruleTable.js";
import { lowerRuleFlow } from "./ruleFlow.js";
import { lowerRuleToggle } from "./ruleToggle.js";

export function compileToIR(checked: CheckedModule): { graph: RuleGraph; diagnostics: TangleDiagnostic[] } {
  const allDiagnostics: TangleDiagnostic[] = [...checked.diagnostics];
  let graph = createGraph("");
  
  // 1. Lower @tangle code blocks（保持现状）
  for (const parsed of checked.parsedBlocks) { ... }
  
  // 2. Lower rule blocks from headings（新增）
  const ruleDiags: TangleDiagnostic[] = [];
  collectRuleGraphs(checked.headings, checked.file, (ruleKind, markdown, heading) => {
    let subGraph: RuleGraph;
    switch (ruleKind) {
      case "tree":   subGraph = lowerRuleTree(markdown, checked.file); break;
      case "table":  subGraph = lowerRuleTable(markdown, checked.file, ruleDiags); break;
      case "flow":   subGraph = lowerRuleFlow(markdown, checked.file); break;
      case "toggle": subGraph = lowerRuleToggle(markdown, checked.file, ruleDiags); break;
    }
    mergeInto(&graph, subGraph);
  });
  allDiagnostics.push(...ruleDiags);
  
  // 3. Validate（保持现状）
  ...
}
```

### 4.4 改动 2：ruleToggle.ts（150 行 port）

**Rust 端核心结构**（`lower_rule_toggle.rs`）：
- 签名：`(checkbox_markdown: &str, file: &str, id_gen: &mut FreshNodeId) -> (RuleGraph, Vec<TangleDiagnostic>)`
- 行遍历：`lines().enumerate()` 跟踪 1-based 行号
- 名称提取：反引号 > 冒号分隔 > None（发 `TANGLE_RULE_TOGGLE_MISSING_NAME`）
- 畸形检测：`- [` 开头但不含 `[x]/[X]/[ ]` 发 `TANGLE_RULE_TOGGLE_MALFORMED`
- group/style 缓存：`pending_group: Option<String>`，遇非注释非 checkbox 行清空

**TS port 要点**：
- TS 没有 `&str` 切片，用 `string` + `split('\n')`
- ID 生成器用 module-level counter + `resetNodeCounter()`（已有）
- 诊断对象结构与 Rust `TangleDiagnostic` 对齐（`code`/`message`/`span`）
- 不用 `Option<String>`，用 `string | null`

### 4.5 改动 3：ruleTree.ts（313 行 port）

**Rust 端核心结构**（`lower_rule_tree.rs`）：
- 列表项解析：缩进深度 + 内容提取
- DNF 范式：AND/OR 嵌套 → 析取范式
- `build_tree(items, target_depth, idx)` 递归构建 `ListNode` 树
- `lower_list_tree(nodes, edges, id_gen)` 把 `ListNode` 树转 IR
- Action 标记：`Action: xxx` 子项作为路径终点（Terminal 节点）

**TS port 要点**：
- 缩进检测：用 `line.indexOf('*')` 或 `line.indexOf('-')` 计算深度（与 Rust `leading_spaces / 2` 一致）
- DNF 算法直接 port（无外部依赖）
- `ListNode` 类型定义在 `ruleTree.ts` 内部

### 4.6 改动 4：ruleTable.ts（262 行 port）

**Rust 端核心结构**（`lower_rule_table.rs`）：
- 表格解析：header row + data rows + column mapping
- priority 排序：`sort_rows_by_priority` 稳定升序
- overlap 检测：两行条件重叠且 action 不同 → 发 `TANGLE_RULE_TABLE_OVERLAP`
- wildcard 行：`*` 匹配所有，覆盖后续行 → 发 `TANGLE_RULE_TABLE_WILDCARD_COVERS`
- Decision 节点 + guarded edges 发射

**TS port 要点**：
- Markdown 表格解析：split `|` 字符，trim 空格
- overlap 检测算法直接 port（无外部依赖）

### 4.7 改动 5：ruleFlow.ts（535 行 port）

**Rust 端核心结构**（`lower_rule_flow.rs`）：
- Mermaid 语法解析：`graph TD` / `subgraph X` / `A -->|label| B` / `A -.->|label| B`
- 节点定义：`A[Action]` / `A{Decision}` / `A((Start))` / `A>Terminal]`
- Edge 类型：`-->` (Solid) / `-.->` (Dashed) / `==>` (Thick) / `--x` (Crossed)
- group：subgraph 名 → `IRNode.group`
- style：`classDef` + `class A classA` → `IRNode.style`（简化版，只支持内联 `style A fill:#f9f`）

**TS port 要点**：
- 不引入 Mermaid 解析库（保持 Rust 端的"自实现 parser"风格）
- 正则表达式直接 port（Rust `regex` crate 与 JS RegExp 语法兼容）
- subgraph 栈：用数组维护嵌套（与 Rust `Vec<String>` 一致）

### 4.8 不在范围

- TS 端 `@tangle` 代码块 lowering 改动（保持现状）
- TS 端 functions[] 概念引入（payment 这种函数式 fixture 由 A1 的 ir-diff 归一化处理，TS 端继续用顶层 merged_graph 模式）
- TS 端 codegen 改动（reference 仅用于差分测试 IR，不参与 codegen 差分）

### 4.9 A2 测试

**策略**：端到端差分测试为主，单元测试为辅。

`reference/src/ir/__tests__/ruleLowering.test.ts`（新文件，用 vitest）：

| 测试 | 验证内容 |
|------|---------|
| `ruleToggle_lower_basic_checkbox` | toggle 基础 lowering |
| `ruleToggle_lower_with_group_style` | group/style 附加 |
| `ruleTree_lower_dnf_and_or` | DNF AND/OR 嵌套 |
| `ruleTree_lower_action_endpoint` | Action: 子项作为 Terminal |
| `ruleTable_lower_priority_ordering` | priority 升序排列 |
| `ruleTable_lower_overlap_detection` | overlap 诊断发射 |
| `ruleFlow_lower_subgraph_group` | subgraph → group 字段 |
| `ruleFlow_lower_dashed_edge` | Dashed edge 类型 |
| `ruleFlow_lower_crossed_edge_skipped` | Crossed edge 标记 |

加上 `diff-ir.ps1` 端到端：6 个 rule fixture 输出 `[MATCH]`。

## 5. B2：三宿主递归深度限制

### 5.1 现状

三宿主 `emit_branch_body` 签名（以 JS 为例，`js_emitter.rs:242-383`）：
```rust
fn emit_branch_body(
    target_id: &str,
    nodes: &[IRNode],
    edges: &[IREdge],
    visited: &mut HashSet<String>,
    indent: &str,
) -> String
```

防环靠 `visited: HashSet<String>`，但无深度计数。恶意/错误 fixture 可能导致深度递归（如 `A --> B --> A` 不被 visited 捕获的边类型）。

### 5.2 改动

**三宿主对称加 `depth: usize` 参数**：

```rust
const MAX_BRANCH_DEPTH: usize = 100;

fn emit_branch_body(
    target_id: &str,
    nodes: &[IRNode],
    edges: &[IREdge],
    visited: &mut HashSet<String>,
    indent: &str,
    depth: usize,           // 新增
) -> String {
    if depth >= MAX_BRANCH_DEPTH {
        return format!("{}// max depth reached\n", indent);  // 三宿主注释格式不同
    }
    // ... 现有逻辑 ...
    // 递归调用：depth + 1
}
```

**三宿主注释格式**：
- JS: `// max depth reached`
- Py: `# max depth reached`
- Go: `// max depth reached`

**调用点**：`emit_decision_branch` 调用 `emit_branch_body` 时初始 `depth = 0`，递归时 `depth + 1`。

### 5.3 阈值选择

`MAX_BRANCH_DEPTH = 100`。

**理由**：
- 现有 fixture 最大嵌套深度 ≤ 5（approval-flow-subgraph）
- 100 给业务用例留充足余量
- 防止栈溢出（Rust 默认栈 8MB，每帧 ~几百字节，100 层远低于栈上限）
- 三宿主一致，便于差分测试

### 5.4 不在范围

- 运行时递归深度限制（需解释器，Phase 5 无）
- IR 生成阶段循环检测（B2 仅在 codegen 层）
- 可配置阈值（YAGNI，硬编码 100）

### 5.5 B2 测试

`compiler/tangle-cli/tests/v05_phase5/recursion_depth.rs`（Cargo name: `phase5_recursion_depth`）：

| 测试 | 验证内容 |
|------|---------|
| `js_branch_body_depth_limit_emits_comment` | JS 超 100 层发 `// max depth reached` |
| `py_branch_body_depth_limit_emits_comment` | Py 超 100 层发 `# max depth reached` |
| `go_branch_body_depth_limit_emits_comment` | Go 超 100 层发 `// max depth reached` |
| `branch_body_normal_depth_no_comment` | 正常深度（≤5）无注释 |
| `three_hosts_consistent_depth_threshold` | 三宿主阈值一致（100） |

Fixture：`tests/v05_phase5/deep-recursion.tangle.md`（构造 100+ 层嵌套 Decision 图）。

## 6. B4：跨 toggle 块 group/style 不继承语义

### 6.1 现状

`lower_rule_toggle.rs`（Phase 4 §5.6）当前语义：
- `pending_group: Option<String>` 缓存
- 遇 `<!-- group: X -->` 行 → 缓存
- 遇 checkbox 行 → 消费缓存，清空
- **遇非注释非 checkbox 行 → 清空缓存**

每次 `lower_rule_toggle` 调用是独立的（每个 `@rule.toggle` 块单独 lower），跨块之间无状态共享。

### 6.2 期望行为（明确文档化）

**跨 `@rule.toggle` 块不继承**：每个 `@rule.toggle` 块独立 lower，前一块的 `pending_group`/`pending_style` 不流入下一块。

**示例**：
```markdown
@rule.toggle

<!-- group: UI -->
- [x] enable_new_ui: true

@rule.toggle

- [ ] enable_crypto: false   ← 不继承 UI group，group 为 None
```

### 6.3 改动

**无代码改动**——当前实现已是"不继承"语义（每次调用独立）。

**改动内容**：
1. `lower_rule_toggle.rs` 顶部加 doc comment 明确语义
2. 加测试覆盖"跨块不继承"行为

```rust
/// Lower a single `@rule.toggle` block to IR.
///
/// # 跨块语义
///
/// 每次调用独立处理单个 toggle 块。跨 `@rule.toggle` 块的 group/style
/// 不继承——前一块的 pending_group/pending_style 不会流入下一块。
/// 如需为多个块设置统一 group，必须在每个块内显式声明。
pub fn lower_rule_toggle(...) -> (RuleGraph, Vec<TangleDiagnostic>) { ... }
```

### 6.4 B4 测试

`compiler/tangle-cli/tests/v05_phase5/toggle_cross_block.rs`（Cargo name: `phase5_toggle_cross_block`）：

| 测试 | 验证内容 |
|------|---------|
| `toggle_block_isolation_no_group_inheritance` | 跨块不继承 group |
| `toggle_block_isolation_no_style_inheritance` | 跨块不继承 style |
| `toggle_explicit_group_per_block_works` | 每块显式声明 group 生效 |
| `toggle_pending_cleared_on_non_checkbox_line` | 单块内缓存遇非注释非 checkbox 行清空（回归） |

Fixture：`tests/v05_phase5/multi-toggle-blocks.tangle.md`（含 2 个 `@rule.toggle` 块，第一块带 group，第二块不带）。

### 6.5 不在范围

- 跨块继承功能本身（YAGNI，无明确用例）
- 父级标题作用域继承（B4-a 方案，已排除）
- 显式 `<!-- inherit: ... -->` 语法（B4-b 方案，已排除）

## 7. 文件结构

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
| `reference/src/ir/ruleTree.ts` | A2 — 忠实 port lower_rule_tree.rs（313 行 → ~400 行 TS） | 修改 |
| `reference/src/ir/ruleTable.ts` | A2 — 忠实 port lower_rule_table.rs（262 行 → ~350 行 TS） | 修改 |
| `reference/src/ir/ruleFlow.ts` | A2 — 忠实 port lower_rule_flow.rs（535 行 → ~700 行 TS） | 修改 |
| `reference/src/ir/ruleToggle.ts` | A2 — 忠实 port lower_rule_toggle.rs（150 行 → ~200 行 TS） | 修改 |
| `reference/src/ir/__tests__/ruleLowering.test.ts` | A2 — vitest 单元测试 | 创建 |
| `compiler/tangle-cli/tests/v05_phase5/dual_entry_fix.rs` | A1 测试（Cargo name: `phase5_dual_entry`） | 创建 |
| `compiler/tangle-cli/tests/v05_phase5/recursion_depth.rs` | B2 测试（Cargo name: `phase5_recursion_depth`） | 创建 |
| `compiler/tangle-cli/tests/v05_phase5/toggle_cross_block.rs` | B4 测试（Cargo name: `phase5_toggle_cross_block`） | 创建 |
| `compiler/tangle-cli/Cargo.toml` | 新增 3 个 `[[test]]` 条目 | 修改 |
| `tests/v05_phase5/deep-recursion.tangle.md` | B2 fixture（100+ 层嵌套） | 创建 |
| `tests/v05_phase5/multi-toggle-blocks.tangle.md` | B4 fixture（2 个 toggle 块） | 创建 |

**总计**：修改 11 个文件，创建 6 个文件。

## 8. 测试策略

### 8.1 新增测试位置

- `compiler/tangle-cli/tests/v05_phase5/`（A1/B2/B4 集成测试）
- `tests/audit/ir-diff/src/main.rs` 内 `#[cfg(test)] mod tests`（A1 ir-diff 单元测试扩展）
- `reference/src/ir/__tests__/ruleLowering.test.ts`（A2 TS 单元测试）

### 8.2 回归测试

- 现有 `audit_regression/` 测试全绿（codegen 输出变化不影响 IR 级诊断）
- 现有 Phase 3 `js_codegen.rs` / `py_go_codegen.rs` / `span_tracking.rs` 测试全绿（JS emitter 不变；Py/Go 仅新增 multi-function 分支，单函数路径不变）
- **Phase 3 `snapshots.rs` 快照无需更新**：B2 改动后 depth 仅在超 100 时发注释，正常用例输出不变。
- **payment.tangle 的 Py/Go codegen 输出变化**：从单函数（main+process 合并）变为多函数（main 和 process 分开）。审计确认 Phase 3 snapshots 不含 payment，无影响。
- 现有 Phase 2 快照测试匹配（IR 层无变化）
- `diff-ir.ps1` 从 3 MATCH/1 KNOWN_DIFF/7 SKIPPED 变为 10 MATCH/0 KNOWN_DIFF/0 SKIPPED

### 8.3 差分测试增强

`diff-ir.ps1` 端到端验证：
- payment.tangle：`[KNOWN_DIFF]` → `[MATCH]`（A1 + A2 联合）
- 6 个 rule fixture：`[SKIPPED]` → `[MATCH]`（A2）
- order-service.tangle：`[SKIPPED]`（TS 失败）→ 调查并修复（A2 顺带，若 TS 端 rule lower 实现后 order-service 仍失败，单独调查）

## 9. 出口闸门

| 闸门 | 命令 | 标准 |
|------|------|------|
| 1. 单元+集成测试 | `cargo test --workspace` | 全绿 |
| 2. Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 警告 |
| 3. 审计回归 | `tests/audit/run-audit.ps1` | 0 failing |
| 4. 差分测试 | `tests/audit/diff-ir.ps1` | **0 KNOWN_DIFF + 0 SKIPPED + 10 MATCH** |
| 5. Phase 4 回归 | `cargo test --test phase4_py_go_codegen --test phase4_toggle_lowering` | 全绿 |
| 6. Phase 5 新测试 | `cargo test --test phase5_dual_entry --test phase5_recursion_depth --test phase5_toggle_cross_block` + `cargo test --manifest-path tests/audit/ir-diff/Cargo.toml` | 全绿 |
| 7. TS 参考实现测试 | `cd reference && npm test` | 全绿（vitest） |
| 8. TS 参考实现构建 | `cd reference && npm run build && npm run typecheck` | 0 类型错误 |

## 10. 不在 Phase 5 范围内

- **B1**：Py/Go `source_text` 翻译（保持注释模式，拆 Phase 6）
- **B3**：toggle 的 Decision 语义（保持 Compute，拆 Phase 6）
- **B5**：Phase 1 推迟项（泛型/返回类型推导/跨模块解析/类型收窄，各自独立 phase）
- **C**：IR Interpreter（Track B 阶段四，独立大 phase）
- Py/Go codegen 的 `source_text` 翻译（与 B1 重叠）
- `IRFunction.params`/`receiver` 结构扩展（当前为空/null，未来 phase 处理）
- TS 参考实现的 functions[] 概念引入（payment 由 ir-diff 归一化处理，TS 端保持顶层 merged_graph 模式）
- order-service.tangle 若 A2 后仍失败的深度调查（若 TS 端 rule lower 实现后仍失败，作为 Phase 5 的已知遗留项报告）

## 11. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| A1 改 compile_to_ir.rs 影响所有 fixture 的 IR 生成 | 现有 3 MATCH fixture 可能 break | expression/hello/user 无 main Callable，走 fallback 模式，顶层 nodes[] 不变；测试闸门 4 验证 |
| A1 ir-diff 多 function 比较重构破坏现有 MATCH | expression/hello/user 从 MATCH 转 DIFF | 无 functions[] 的 IR 包装为单 function 数组后比较，归一化逻辑不变；闸门 4 验证 |
| A2 TS 端忠实 port 工作量大（~1650 行 TS） | Phase 5 周期长 | port 顺序 toggle → tree → table → flow，每完成一个 fixture 立即差分验证，早失败早修复 |
| A2 TS 端 Mermaid 解析与 Rust 端正则不一致 | ruleFlow fixture DIFF | 直接 port Rust 正则（regex crate 与 JS RegExp 语法兼容），逐个 fixture 验证 |
| B2 depth 参数改动三宿主 emit_branch_body 签名 | 调用点编译失败 | 同步改所有调用点，编译验证 |
| B4 "无代码改动"闭合方式被质疑不严谨 | 闭合方式争议 | doc comment 明确语义 + 4 个测试覆盖（含 2 个回归测试）= 工程闭合 |
| order-service.tangle A2 后仍失败 | diff-ir 仍有 1 SKIPPED | 调查失败原因；若是 TS 端 import 解析问题，作为 Phase 5 遗留项报告，不阻塞 v0.5.0 |
