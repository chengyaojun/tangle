# Tangle v0.2.1 Quality Audit Findings

> Audit run: 2026-07-11 14:18:22 (local)
> Worktree: `.worktrees/audit-v0.2.1` @ branch `audit/v0.2.1`
> Audit harness: `tests/audit/run-audit.ps1` + `tests/audit/diff-ir.ps1` + `cargo clippy`
> Findings author: Task 11 (initial audit pass)

## Summary

| Metric | Value |
|--------|-------|
| Total audit cells (matrix) | 210 |
| Failing cells (`diag_count > 0`) | 64 |
| Fixtures with diagnostics | 8 / 15 |
| Differential tests | 0 MATCH / 7 DIFF / 2 SKIPPED |
| `cargo clippy --workspace -- -D warnings` errors | 17 (across 12 files) |
| Go-target runtime failures (`exit_code=101`) | 30 cells (environmental, go toolchain not installed in worktree) |

### Failing cells distribution

By fixture (8 cells each = run×{js,py,go}×{normal,incremental} + build×{js,py,go,emit-ir}×normal — note `run/go/incremental` cells report `diag_count=0` because the pipeline short-circuits on cache hits):

| Fixture | Diag count per cell | Codes |
|---------|---------------------|-------|
| account.tangle.md | 5 | `TANGLE_SYMBOL_NOT_FOUND` × 3 + `TANGLE_TYPE_ERROR` × 2 |
| expression.tangle.md | 5 | `TANGLE_SYMBOL_NOT_FOUND` × 3 + `TANGLE_TYPE_ERROR` × 2 |
| payment.tangle.md | 7 | `TANGLE_HEADING_MULTI_WORD` × 4 + `TANGLE_SYMBOL_NOT_FOUND` × 3 |
| order-service.tangle.md | 6 | `TANGLE_HEADING_MULTI_WORD` × 6 |
| approval-flow.tangle.md | 2 | `TANGLE_HEADING_MULTI_WORD` × 2 |
| decision-table.tangle.md | 2 | `TANGLE_HEADING_MULTI_WORD` × 2 |
| decision-tree.tangle.md | 2 | `TANGLE_HEADING_MULTI_WORD` × 2 |
| feature-toggles.tangle.md | 2 | `TANGLE_HEADING_MULTI_WORD` × 2 |

### Fixtures with zero diagnostics (passing)

`collections`, `concurrency`, `crypto`, `hello`, `io-system`, `math-data`, `user` (7 fixtures).

### Known environmental noise (not classified as findings)

- **Go target runtime missing**: 30 cells across `run --target go` exit with code 101 because the Go toolchain is not on PATH in the audit worktree. Diagnostics are still collected (and counted above) but the runtime cannot execute the emitted Go. These are environmental and tracked separately; not real-bug findings.
- **Incremental cells cache-hit**: `*_incremental_*` cells reuse cached pipeline output and surface zero new diagnostics for cached fixtures. This is harness behavior, not a compiler property.
- **`doc` surface cells**: 0 diagnostics by design — the `doc` subcommand does not invoke `render_diagnostics`.

---

## Findings by Category

### Real-bug (诊断类)

#### F-001: Parser expression stop-list omits `Let` / `Const` (account.tangle.md)

- **Cell**: `run|build` × `js|py|go|emit-ir` × `normal` + `run|js` × `incremental` — 8 cells, `examples/account.tangle.md`
- **现象**: 5 条诊断 — `TANGLE_SYMBOL_NOT_FOUND: Symbol 'acc2' not found`, `TANGLE_SYMBOL_NOT_FOUND: Symbol 'acc' not found`, `TANGLE_TYPE_ERROR: Type mismatch in binary operation`, `TANGLE_TYPE_ERROR: Member access on non-struct type`, `TANGLE_SYMBOL_NOT_FOUND: Symbol 'acc2' not found` (positions 1:2–1:4)
- **期望**: 0 诊断（fixture 仅含合法 `let` 绑定与字段访问）
- **分类**: real-bug
- **根因**: `compiler/tangle-cli/src/parser/parser.rs:94-106` 的表达式 stop-list `matches!(op.kind, Eof | RParen | RBrace | RBracket | Semicolon | Comma | Else | Return)` 缺少 `TokenKind::Let | TokenKind::Const`。当 `main` 块内两条无分号 `let` 语句相邻时，解析器在第一条 `let` 后不停止，把第二条 `let acc2 = ...` 的 `let` 当作中缀操作数消费，导致两条语句被合并为一个非法表达式，进而触发级联的 symbol/type 诊断。
- **修复方案**: 在 stop-list 中追加 `TokenKind::Let | TokenKind::Const`（并审计是否还需 `If | Match | For | While` 等块起始关键字）。引用 TDD 测试 `G5`（parser stop-list 回归）— 当前 `G5` 为占位结构，需在 G5/G6 阶段填充实际用例。
- **影响范围**: 同模式波及 `expression.tangle.md`（见 F-002）。任何无分号相邻 `let`/`const` 块都会触发。
- **优先级**: **P0**

#### F-002: Parser expression stop-list omits `Let` / `Const` (expression.tangle.md)

- **Cell**: `run|build` × `js|py|go|emit-ir` × `normal` + `run|js` × `incremental` — 8 cells, `tests/basic/expression.tangle.md`
- **现象**: 5 条诊断 — `TANGLE_SYMBOL_NOT_FOUND: Symbol 'y'/'x' not found`, `TANGLE_TYPE_ERROR: Type mismatch in binary operation` × 2
- **期望**: 0 诊断
- **分类**: real-bug
- **根因**: 与 F-001 同根因 — `compiler/tangle-cli/src/parser/parser.rs:94-106`。
- **修复方案**: 同 F-001（一次性修复 stop-list 即同时消除 F-001 与 F-002）。
- **影响范围**: 见 F-001。
- **优先级**: **P0**

#### F-003: Heading parser does not recognize `Prefix: Identifier` pattern (payment.tangle.md)

- **Cell**: `run|build` × `js|py|go|emit-ir` × `normal` + `run|js` × `incremental` — 8 cells, `tests/errors/payment.tangle.md`
- **现象**: 7 条诊断 — `TANGLE_HEADING_MULTI_WORD: Heading 'Error: PayFailed' has multiple words` × 2, `TANGLE_HEADING_MULTI_WORD: Heading 'Error: Timeout' has multiple words` × 2, `TANGLE_SYMBOL_NOT_FOUND: Symbol 'result' not found`, `TANGLE_SYMBOL_NOT_FOUND: Symbol 'process' not found`, `TANGLE_SYMBOL_NOT_FOUND: Symbol 'result' not found`
- **期望**: 0 诊断（`Error: PayFailed` 应解析为 error 类别 + 标识符 `PayFailed`）
- **分类**: real-bug
- **根因**: `compiler/tangle-cli/src/frontend/headings.rs:65` 的 multi-word 判定 `trimmed.chars().all(|c| c.is_ascii_graphic() || c == ' ') && trimmed.contains(' ')` 把 `Error: PayFailed`、`Error: Timeout` 当作多词标题。该规则不识别 `Error:` / `Type:` / `Rule:` / `Command:` 等 Tangle 内置前缀约定，未从前缀中提取冒号后的标识符。同时 `headings.rs:42-58` 的 ASCII 标识符分支因 `:` 不属于 `is_valid_identifier`（`headings.rs:80-87`）而提前落入 multi-word 分支。
- **修复方案**: 在 `headings.rs` 的 multi-word 检测前增加 `Prefix: Identifier` 识别 — 形如 `^(Error|Type|Rule|Command|Query|Command)\s*:\s*(<ident>)$` 的标题应提取 `<ident>` 作为符号名，跳过 multi-word 警告。引用 TDD 测试 — 建议新增 `G7` 覆盖前缀式标题解析。
- **影响范围**: 波及 `order-service.tangle.md`（F-004）、`approval-flow` / `decision-table` / `decision-tree` / `feature-toggles`（F-005）。
- **优先级**: **P1**

#### F-004: Heading parser does not recognize `Prefix: Identifier` pattern (order-service.tangle.md)

- **Cell**: `run|build` × `js|py|go|emit-ir` × `normal` + `run|js` × `incremental` — 8 cells, `tests/mvp/order-service.tangle.md`
- **现象**: 6 条诊断 — `TANGLE_HEADING_MULTI_WORD: Heading 'Order Service' has multiple words` × 2, `TANGLE_HEADING_MULTI_WORD: Heading 'Error: PayFailed' has multiple words` × 2, `TANGLE_HEADING_MULTI_WORD: Heading 'Error: Timeout' has multiple words` × 2
- **期望**: 0 诊断（fixture 使用 `# Order Service` 作为示例标题；`## Error: PayFailed` 等使用前缀约定）
- **分类**: real-bug
- **根因**: 双重 — (a) 顶层标题 `# Order Service` 确为多词，fixture 应改写为 `# OrderService` 或 `# Order (order-service)`；(b) `##### Error: PayFailed` 等与 F-003 同根因（`headings.rs:65`）。
- **修复方案**: F-003 修复后 (b) 部分自动消除；(a) 部分需更新 fixture 文档约定或显式加括号标识符。
- **影响范围**: 单一 fixture。
- **优先级**: **P2**

#### F-005: Heading parser does not recognize `Rule:` prefix (rule-based fixtures)

- **Cell**: 8 cells × 4 fixtures = 32 cells — `tests/rules/approval-flow.tangle.md`, `tests/rules/decision-table.tangle.md`, `tests/rules/decision-tree.tangle.md`, `tests/rules/feature-toggles.tangle.md`
- **现象**: 每个 fixture 2 条 `TANGLE_HEADING_MULTI_WORD: Heading 'Rule: Approval' / 'Rule: ScoreCard' / ... has multiple words`
- **期望**: 0 诊断（`Rule: Approval` 应解析为 rule 类别 + 标识符 `Approval`）
- **分类**: real-bug
- **根因**: 与 F-003 同根因 — `compiler/tangle-cli/src/frontend/headings.rs:65`。
- **修复方案**: F-003 修复后自动消除。
- **影响范围**: 4 个 rule-based fixture，全部位于 `tests/rules/`。
- **优先级**: **P2**

### False-positive (级联诊断类)

#### F-006: Cascading `TANGLE_SYMBOL_NOT_FOUND` from heading parse failure (payment.tangle.md)

- **Cell**: 与 F-003 相同的 8 cells
- **现象**: 7 条诊断中的 3 条 `TANGLE_SYMBOL_NOT_FOUND: Symbol 'result'/'process' not found` — 这些符号在 fixture 的 `main` 与 `process` 块内已合法绑定
- **期望**: 0 诊断
- **分类**: false-positive
- **根因**: 由 F-003 的标题解析失败级联产生 — 标题 `Error: PayFailed` 未提取出符号名导致后续 `process`/`result` 引用解析时上下文环境不完整，触发误报。
- **修复方案**: 修复 F-003 后这 3 条级联诊断自动消失。无需独立测试。
- **影响范围**: 仅 `payment.tangle.md`。
- **优先级**: **P1**（依赖 F-003）

### Gap / Doc-drift (差分类)

#### F-007: TS/Rust IR node ID naming convention divergence

- **Differential cells**: `expression`, `hello`, `user` — 3 fixtures where TS emits non-empty IR
- **现象**: diff-ir 报告 DIFF；TS IR 节点 ID 为语义化命名（`entry1`, `bind2`, `ret4`, `end5`），Rust IR 节点 ID 为位置化命名（`n0`, `n1`, `n2`, `n3`）
- **期望**: 节点 ID 命名约定一致（或 ir-diff 工具对 ID 进行规范化归一）
- **分类**: gap
- **根因**: Rust 端 `compiler/tangle-cli/src/ir/graph.rs:91-95` 的 `FreshNodeId::next()` 返回 `format!("n{}", self.counter)`；TS 端按节点角色（entry/bind/ret/end）+ 递增序号命名。两端无统一约定。
- **修复方案**: 二选一 — (a) 统一采用语义化命名（Rust 端为 `FreshNodeId` 增加 role 参数）；(b) 在 `tests/audit/ir-diff/` 的归一化阶段把所有节点 ID 重映射为 `node0, node1, ...` 后再比较。建议 (b)，对编译器侵入更小。引用 TDD 测试 `G8`（ir-diff 归一化）。
- **影响范围**: 所有非空 IR 的 fixture。
- **优先级**: **P2**

#### F-008: Rust IR edge carries `guard: null` field; TS IR omits it

- **Differential cells**: `expression`, `hello`, `user`
- **现象**: diff-ir 报告 DIFF；Rust IR 每条 edge 序列化为 `{from, to, kind, guard: null, sourceSpan: null}`，TS IR edge 序列化为 `{from, to, kind, sourceSpan}`（无 `guard` 字段）
- **期望**: edge schema 一致
- **分类**: gap
- **根因**: Rust 端 `compiler/tangle-cli/src/ir/graph.rs` 的 `Edge` 结构体把 `guard` 作为非 `Option` 字段或总是序列化的字段；TS 端在 `guard` 为空时直接省略。ir-diff 当前未对 `guard: null` 与 `guard` 缺省做等价归一。
- **修复方案**: 在 ir-diff 的归一化阶段，删除值为 `null` 的 `guard` 字段后再比较；或在 Rust 端使用 `#[serde(skip_serializing_if = "Option::is_none")]`。
- **影响范围**: 所有 edge。
- **优先级**: **P2**

#### F-009: Rust IR includes top-level `functions` / `importedStdlib` / `stdlibImports`; TS IR does not

- **Differential cells**: `expression`, `hello`, `user`
- **现象**: diff-ir 报告 DIFF；Rust IR 顶层包含 `functions: [{name, receiver, params, nodes, edges, entryNodeId, errorEdges}]`、`importedStdlib: []`、`stdlibImports: []`，TS IR 顶层只有 `nodes` / `edges` / `errorEdges` / `entryNodeId`
- **期望**: 顶层 schema 一致
- **分类**: gap
- **根因**: Rust 编译器的 IR 设计已演进到支持函数级 IR（`RuleGraph.functions`），TS 参考实现仍停留在扁平 graph。两端 IR schema 版本不一致。
- **修复方案**: 中期路线 — 统一 IR schema 版本（建议 TS 参考实现对齐 Rust 的 `functions` 结构）；短期 — ir-diff 在归一化阶段把 Rust 的 `functions[0]` 提升为顶层 `nodes`/`edges` 后再比较，并忽略 `importedStdlib` / `stdlibImports` 空数组。
- **影响范围**: 所有非空 IR 的 fixture。
- **优先级**: **P2**

#### F-010: TS reference emits empty IR for rule-based fixtures

- **Differential cells**: `approval-flow`, `decision-table`, `decision-tree`, `feature-toggles` — 4 fixtures
- **现象**: diff-ir 报告 DIFF；TS IR 为 `{nodes: [], edges: [], errorEdges: [], entryNodeId: "entry"}`（空），Rust IR 含完整的 rule graph 节点与边
- **期望**: 两端要么都为空，要么都含 rule graph
- **分类**: gap
- **根因**: TS 参考实现尚未实现 rule-based fixture 的 IR lower（mermaid / decision-table / decision-tree / feature-toggle 节点不进入 IR）；Rust 端已在 `compiler/tangle-cli/src/ir/lower_rule_table.rs` 等模块实现。
- **修复方案**: 短期 — ir-diff 在 TS 端 IR 为空时跳过比较（标记为 SKIPPED 而非 DIFF）；长期 — TS 参考实现对齐 rule lowering。
- **影响范围**: 4 个 rule-based fixture。
- **优先级**: **P1**

#### F-011: Terminal node label divergence (`exit` vs `return`)

- **Differential cells**: `expression`, `hello`, `user`
- **现象**: Rust IR 终结节点 `label: "exit"`，TS IR 终结节点 `label: "return"`
- **期望**: 终结节点 label 一致
- **分类**: doc-drift
- **根因**: Rust 端在 IR lower 时硬编码 `label: "exit"`（如 `compiler/tangle-cli/src/ir/` 相关 lower 模块），TS 端沿用 `return`。
- **修复方案**: 统一为 `exit`（语义更准确 — 终结节点不一定源自 return 语句，也可能是块尾隐式返回）。
- **影响范围**: 所有含 `return` 的 fixture。
- **优先级**: **P3**

#### F-012: Rust IR node carries `sourceText` field; TS IR does not

- **Differential cells**: `expression`, `hello`, `user`
- **现象**: Rust IR 节点包含 `sourceText` 字段（如 `"sourceText": "return \"hello\""`），TS IR 节点只有 `sourceSpan`，无 `sourceText`
- **期望**: 节点 schema 一致
- **分类**: gap
- **根因**: Rust 端 `Node` 结构体在序列化时包含 `sourceText`（用于调试），TS 端未携带。
- **修复方案**: ir-diff 已声明会 strip source spans — 同时 strip `sourceText` 字段（当前实现可能遗漏）。
- **影响范围**: 所有非空 IR 的 fixture。
- **优先级**: **P3**

### Gap (环境类)

#### F-013: Go toolchain not installed in audit worktree

- **Cell**: 30 cells — `run --target go` × 所有 fixture × {normal, incremental}
- **现象**: `exit_code=101`，stderr 报告 go 命令未找到
- **期望**: `exit_code=0` 且无诊断（或在 CI 中显式标记为 SKIPPED）
- **分类**: gap
- **根因**: 审计 worktree 环境未安装 Go 工具链；`run-audit.ps1` 未在运行前检查 `go` 是否可用，也未将缺失工具链的 cell 标记为 SKIPPED。
- **修复方案**: (a) 在 `run-audit.ps1` 中预检 `go` / `node` / `python` 是否可用，缺失时把对应 cells 标记为 SKIPPED 而非 FAIL；(b) CI 环境补齐 Go 工具链。
- **影响范围**: 所有 `run --target go` cells。
- **优先级**: **P3**

### Rust-warning (clippy 类)

> 以下发现来自 `cargo clippy --workspace -- -D warnings`，共 **17 个错误**，跨 12 个文件。
> 全部为 P3（不影响功能正确性，但阻碍 `-D warnings` 通过的 CI 闸门）。
> 修复时建议按 lint 类别批量处理，每个类别引用一个 TDD 测试（如 `G9` clippy 修复回归）。

#### F-014: clippy `new_without_default` — 4 处

- **位置**:
  - `compiler/tangle-cli/src/checker/env.rs:21` — `TypeEnv::new`
  - `compiler/tangle-cli/src/checker/errors.rs:17` — `ErrorRegistry::new`
  - `compiler/tangle-cli/src/ir/graph.rs:89` — `FreshNodeId::new`
  - `compiler/tangle-cli/src/lsp/server.rs:10` — `LspServer::new`
- **分类**: rust-warning
- **修复方案**: 为每个类型添加 `impl Default for T { fn default() -> Self { Self::new() } }`，或加 `#[derive(Default)]` 并把 `new` 改为调用 `Default::default()`。
- **优先级**: **P3**

#### F-015: clippy `unnecessary_map_or` — 3 处

- **位置**:
  - `compiler/tangle-cli/src/checker/types.rs:74` — `.map_or(false, |ms| callable_sigs_match(...))`
  - `compiler/tangle-cli/src/codegen/js_emitter.rs:124` — `.map_or(false, |c| c.is_uppercase())`
  - `compiler/tangle-cli/src/codegen/js_emitter.rs:219` — `.map_or(false, |s| statement_uses_propagation(s))`
- **分类**: rust-warning
- **修复方案**: 替换为 `.is_some_and(...)`。
- **优先级**: **P3**

#### F-016: clippy `bind_instead_of_map` — 2 处

- **位置**:
  - `compiler/tangle-cli/src/checker/check_module.rs:134` — `.and_then(|tn| match tn.as_str() { ... Some(...) })`
  - `compiler/tangle-cli/src/checker/check_module.rs:155` — 同上模式
- **分类**: rust-warning
- **修复方案**: 把 `.and_then(|x| Some(y))` 改为 `.map(|x| y)`。
- **优先级**: **P3**

#### F-017: clippy `collapsible_if` / `collapsible_match` — 2 处

- **位置**:
  - `compiler/tangle-cli/src/checker/resolve.rs:126` — `collapsible_if`：`if h.role == ... { if h.children.iter().any(...) { ... } }`
  - `compiler/tangle-cli/src/frontend/headings.rs:51` — `collapsible_match`：`match depth { 4..=6 => { if !first_char.is_lowercase() { ... } } }`
- **分类**: rust-warning
- **修复方案**: 用 `&&` 合并嵌套 `if`；用 match guard `4..=6 if !first_char.is_lowercase() =>` 折叠。
- **优先级**: **P3**

#### F-018: clippy `module_inception` — 1 处

- **位置**: `compiler/tangle-cli/src/parser/mod.rs:2` — `pub mod parser;`（`parser` 模块内含 `parser` 子模块）
- **分类**: rust-warning
- **修复方案**: 重命名子模块为 `parse` 或 `pratt`，或加 `#[allow(clippy::module_inception)]` 并加注释说明。
- **优先级**: **P3**

#### F-019: clippy `ptr_arg` — 1 处

- **位置**: `compiler/tangle-cli/src/markdown/parse_markdown.rs:166` — `fn flush_text(..., stack: &mut Vec<MarkdownNode>)`
- **分类**: rust-warning
- **修复方案**: 把签名改为 `stack: &mut [MarkdownNode]`。
- **优先级**: **P3**

#### F-020: clippy `derivable_impls` — 1 处

- **位置**: `compiler/tangle-cli/src/ir/graph.rs:100` — 手写 `impl Default for RuleGraph`
- **分类**: rust-warning
- **修复方案**: 删除手写 impl，在 `RuleGraph` 结构体上加 `#[derive(Default)]`。
- **优先级**: **P3**

#### F-021: clippy `should_implement_trait` — 1 处

- **位置**: `compiler/tangle-cli/src/ir/graph.rs:91` — `pub fn next(&mut self) -> String`（易与 `Iterator::next` 混淆）
- **分类**: rust-warning
- **修复方案**: 重命名为 `fresh()` / `alloc()`，或实现 `std::iter::Iterator` trait。
- **优先级**: **P3**

#### F-022: clippy `needless_range_loop` — 1 处

- **位置**: `compiler/tangle-cli/src/ir/lower_rule_table.rs:46` — `for i in 0..condition_count.min(cells.len().saturating_sub(1))`
- **分类**: rust-warning
- **修复方案**: 改为 `for (i, <item>) in cells.iter().enumerate().take(condition_count.min(cells.len().saturating_sub(1)))`。
- **优先级**: **P3**

#### F-023: clippy `empty_line_after_doc_comments` — 1 处

- **位置**: `compiler/tangle-cli/src/stdlib/bindings.rs:2` — 文档注释后跟空行
- **分类**: rust-warning
- **修复方案**: 把外层 `///` 文档注释改为模块级 `//!` 内部文档注释，或删除空行。
- **优先级**: **P3**

---

## Findings index

| ID | 分类 | 优先级 | 根因文件:行 | 影响范围 |
|----|------|--------|-------------|----------|
| F-001 | real-bug | P0 | parser/parser.rs:94-106 | account.tangle.md (8 cells) |
| F-002 | real-bug | P0 | parser/parser.rs:94-106 | expression.tangle.md (8 cells) |
| F-003 | real-bug | P1 | frontend/headings.rs:65 | payment.tangle.md (8 cells) |
| F-004 | real-bug | P2 | frontend/headings.rs:65 + fixture | order-service.tangle.md (8 cells) |
| F-005 | real-bug | P2 | frontend/headings.rs:65 | 4 rule fixtures (32 cells) |
| F-006 | false-positive | P1 | (级联自 F-003) | payment.tangle.md (8 cells) |
| F-007 | gap | P2 | ir/graph.rs:91-95 | 3 diff cells |
| F-008 | gap | P2 | ir/graph.rs (Edge 序列化) | 3 diff cells |
| F-009 | gap | P2 | ir/graph.rs (RuleGraph schema) | 3 diff cells |
| F-010 | gap | P1 | TS reference 缺 rule lower | 4 diff cells |
| F-011 | doc-drift | P3 | ir/ lower 模块 | 3 diff cells |
| F-012 | gap | P3 | ir/graph.rs (Node 序列化) | 3 diff cells |
| F-013 | gap | P3 | run-audit.ps1 环境 | 30 cells |
| F-014 | rust-warning | P3 | checker/env.rs:21 + 3 处 | 4 clippy errors |
| F-015 | rust-warning | P3 | checker/types.rs:74 + 2 处 | 3 clippy errors |
| F-016 | rust-warning | P3 | checker/check_module.rs:134, 155 | 2 clippy errors |
| F-017 | rust-warning | P3 | checker/resolve.rs:126 + headings.rs:51 | 2 clippy errors |
| F-018 | rust-warning | P3 | parser/mod.rs:2 | 1 clippy error |
| F-019 | rust-warning | P3 | markdown/parse_markdown.rs:166 | 1 clippy error |
| F-020 | rust-warning | P3 | ir/graph.rs:100 | 1 clippy error |
| F-021 | rust-warning | P3 | ir/graph.rs:91 | 1 clippy error |
| F-022 | rust-warning | P3 | ir/lower_rule_table.rs:46 | 1 clippy error |
| F-023 | rust-warning | P3 | stdlib/bindings.rs:2 | 1 clippy error |

## 分类汇总

| 分类 | 数量 | 影响 cells |
|------|------|------------|
| real-bug | 5 | 64 (含级联) |
| false-positive | 1 | 8 (级联自 F-003) |
| gap | 6 | 10 diff cells + 30 env cells |
| doc-drift | 1 | 3 diff cells |
| rust-warning | 10 | 17 clippy errors |
| **合计** | **23** | — |

## 修复优先级建议

1. **P0（先修）**: F-001 + F-002 — parser stop-list 一行改动，立即消除 16 个 failing cells。
2. **P1（次修）**: F-003 + F-006 — heading 前缀识别，消除 payment 的 8 cells；F-010 — ir-diff 短期跳过空 IR。
3. **P2（中期）**: F-004 + F-005 — heading 修复后自动消除；F-007/F-008/F-009 — ir-diff 归一化策略。
4. **P3（清理）**: F-011/F-012/F-013 — 环境与文档漂移；F-014 ~ F-023 — clippy 批量清理。

## 自审

- 本报告基于单次审计运行（`tests/audit/output/20260711-141822/`），未重新运行 `run-audit.ps1`（数据未变，避免 2-3 分钟重复成本）。
- `cargo clippy` 本次实跑得到 **17 个错误**（任务上下文预估 16 个，实际多 1 个 — 已全部记录）。
- `diff-ir` 数据来自 `%TEMP%\tangle-diff-ir\` 上次运行产物（2026-07-11 17:42-17:43），未重新运行。
- 未编造任何发现 — 所有诊断码、行号、文件路径均来自实际审计输出。
- 本任务仅创建 `docs/audit/findings.md`，未修改任何 src / 测试 / Cargo.toml 文件。
