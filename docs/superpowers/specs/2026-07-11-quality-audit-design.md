# Tangle 质量审计阶段设计规格

> **范围：** 仅覆盖质量审计与修复阶段。v0.3.0 Phase 1-4 后续各自走独立的 brainstorm → spec → plan 流程。
>
> **目标：** 通过穷举式审计 + 按根因批量修复，交付一个零虚假诊断的稳定基线，作为 v0.2.1 补丁版本发布，为 v0.3.0 路线图铺路。

---

## 1. 背景与动机

### 1.1 当前状态

- Tangle v0.2.0 已完成：Track A（TypeScript reference，冻结）+ Track B（Rust 权威编译器）
- 全 workspace 99 个既有测试全部通过（95 in tangle-cli + 4 in tangle-std）
- 22 个 stdlib 模块、多宿主 codegen（JS/Python/Go）、LSP、docgen、增量编译均已落地
- README 已规划 Post-B5 v0.3.0 四个阶段，但尚未启动实施

### 1.2 触发本规格的具体问题

运行 [examples/account.tangle.md](../../../examples/account.tangle.md) 时，编译器输出虚假诊断：

```
error[TANGLE_SYMBOL_NOT_FOUND]: Symbol 'Account' not found
error[TANGLE_SYMBOL_NOT_FOUND]: Symbol 'account' not found
error[TANGLE_SYMBOL_NOT_FOUND]: Symbol 'Account' not found
error[TANGLE_TYPE_ERROR]: Member access on non-struct type
```

但执行结果 `{ balance: 150 }` **正确**——即类型检查器对 struct 与隐式方法绑定的符号解析存在 bug，而 codegen 未受影响。

全 examples 跑查发现其余 5 个 example（collections/concurrency/crypto/io-system/math-data）输出干净，唯一有问题的恰是 struct + 方法绑定场景。外加 [compiler/tangle-cli/src/cli/run.rs:17](../../../compiler/tangle-cli/src/cli/run.rs#L17) 的 `unused variable: source` 告警。

### 1.3 路径选择

用户在头脑风暴中明确选择"先修复可见 bug，再执行现有 v0.3.0 路线图"，并进一步选定**全面质量审计**范围与**仅质量审计阶段**的规格边界。本规格是后续 v0.3.0 各阶段 brainstorm 的前置依赖。

---

## 2. 审计范围与矩阵

### 2.1 矩阵维度

审计覆盖 4 个维度的笛卡尔积，每个 cell 执行一次：

| 维度 | 取值 |
|------|------|
| **CLI 表面** | `run`、`build`、`test`、`doc`、`lsp` |
| **Codegen 目标** | `js`、`py`、`go`、`--emit-ir`（独立一格） |
| **执行模式** | `normal`、`--incremental`、`--interp`（仅 `run` 适用） |
| **Fixture 集合** | 6 个 examples + 9 个 tests/ fixture（basic ×2、errors ×1、mvp ×1、rules ×4、structs ×1） |

合计约 250 个 cell。

### 2.2 每 cell 记录项

- 退出码
- stdout/stderr 全量
- 诊断条目（按 `TANGLE_*` code 分类计数）

### 2.3 特殊处理

- **LSP 表面**：不走 fixture 路径，而是用固定 LSP 协议脚本序列（`initialize` → `initialized` → `didOpen` → `hover` → `completion` → `definition` → 收集 `publishDiagnostics`）对每个 example 文件执行
- **doc 表面**：输出 HTML，记录是否成功生成 + 是否含错误标记
- **cargo clippy**：单独一格，`cargo clippy --workspace -- -D warnings` 零告警才放行

### 2.4 已知跳过的 cell

- TS reference 没有 `--interp` 模式，相关差分测试 cell 标 `N/A`
- 任何 fixture + CLI 组合在 reference 端不支持的，差分测试标 `SKIPPED`

---

## 3. 审计工具与脚本

新增 `tests/audit/` 目录（不进 Cargo workspace，避免污染正式测试套件）。

### 3.1 审计矩阵驱动脚本

**文件：** `tests/audit/run-audit.ps1`（PowerShell，跨平台可移植）

遍历 §2 矩阵，对每个 cell 执行 `cargo run --quiet -- <cli-surface> <fixture> [--target X] [--mode Y]`，捕获退出码、stdout、stderr，写入：

```
tests/audit/output/<timestamp>/
  ├── matrix.csv              # 一行一 cell
  ├── cells/<cell-id>.out     # 每 cell 完整 stdout+stderr
  └── summary.md              # 按 surface/target/mode 聚合的失败 cell 清单
```

`matrix.csv` 字段：`surface,target,mode,fixture,exit_code,diag_count,diag_codes`。

### 3.2 LSP 协议探测脚本

**目录：** `tests/audit/lsp-probe/`（独立 Cargo.toml 的 Rust 二进制）

通过 LSP JSON-RPC over stdio 跑固定协议序列：

1. `initialize` → `initialized`
2. `textDocument/didOpen` for each example
3. `textDocument/hover` at 每个 symbol 位置
4. `textDocument/completion` at 每个调用点
5. `textDocument/definition` at `Account`、`fmt.println` 等典型符号
6. 收集全量 `textDocument/publishDiagnostics` 推送

用 Rust 而非 TS 编写，与项目主语言栈对齐。

### 3.3 差分测试 harness（吸收方案 C）

**文件：** `tests/audit/diff-ir.ps1`

对每个 fixture：

1. 跑 TS reference：`node reference/dist/src/cli/main.js run <fixture> --emit-ir > ts-ir.json`
2. 跑 Rust：`cargo run --quiet -- build <fixture> --emit-ir > rs-ir.json`
3. 调用一个 Rust 写的 `ir-diff` 工具做语义比较（忽略键序、忽略 source span，只比 Rule Graph 结构）
4. 任何语义差异即一条 finding

**限制：** Track A 是 0.x 版本，可能不覆盖所有新功能。差分测试只对**两边都支持的 fixture** 跑；reference 不支持的 fixture 在报告里标 `SKIPPED`，不算失败。

### 3.4 审计报告文档

**文件：** `docs/audit/findings.md`

每条 finding 一节：

```
## F-001: account.tangle.md 在 run --target js 下误报 Account 符号未找到
- Cell: run / js / normal / examples/account.tangle.md
- 现象: 4 条 TANGLE_SYMBOL_NOT_FOUND + 1 条 TANGLE_TYPE_ERROR
- 期望: 零诊断；执行结果 { balance: 150 } 正确
- 分类: false-positive
- 根因假设: checker 未把 ### Account 注册为类型符号供 #### main 中 Account.open() 查找
- 影响范围: 所有含 struct + 隐式方法绑定的 fixture
- 修复优先级: P0
```

---

## 4. Findings 分类与根因分组

### 4.1 性质分类

| 分类 | 含义 | 处理路径 |
|------|------|---------|
| `false-positive` | 诊断误报，执行结果正确 | 修 checker，加回归测试 |
| `real-bug` | 真实功能错误，执行结果错或崩溃 | 修对应模块，加回归测试 |
| `gap` | 缺失功能（如某 fixture 在 `--interp` 下未实现） | 评估是否纳入本期；若否则记入 backlog |
| `rust-warning` | `cargo clippy`/`cargo build` 告警 | 直接修 |
| `doc-drift` | `doc` 输出与实际行为不符 | 修 docgen 或文档 |

### 4.2 根因分组

修复按根因分组批量执行，同一根因的所有 cell 一次性修。预计根因组（待审计后可能调整）：

| 根因组 | 典型 finding | 影响模块 |
|--------|--------------|---------|
| **G1: Struct/类型符号注册** | `Account` 在 `#### main` 中查不到 | checker 符号表 |
| **G2: 局部 `let` 绑定作用域** | `let acc = ...` 后续引用误报未找到 | checker scope resolver |
| **G3: Member access 类型推断** | "Member access on non-struct type" 误报 | checker 类型推断 |
| **G4: Rust 工程告警** | `unused variable: source` | cli/run.rs 等 |
| **G5: 文档/示例漂移** | example 行为与 README 描述不符 | examples/、docs/ |
| **G6: 平台/宿主差异** | 某诊断只在 py/go 目标下出现 | codegen/python、codegen/go |

每个根因组在 `findings.md` 里有独立的"批次修复"章节，包含：

- 涉及的 finding 列表
- 根因分析（具体到代码文件 + 行号）
- 修复方案
- 回归测试清单
- 验证步骤（重跑该组所有 cell）

### 4.3 优先级规则

| 优先级 | 含义 | 处理 |
|--------|------|------|
| **P0** | 阻断 examples 干净跑通 | 必修 |
| **P1** | 阻断某个 CLI 表面正常工作 | 必修 |
| **P2** | 告警、文档漂移、平台差异 | 必修 |
| **P3** | `gap` 类，属于新功能缺失 | 进 backlog，不在本期修 |

**本期原则：** P0/P1/P2 必修，P3 进 backlog。

---

## 5. 修复工作流（TDD）

### 5.1 单条根因组修复流程

每个根因组严格走 TDD 五步：

1. **写失败测试**
   在 `compiler/tangle-cli/tests/audit_regression/<group-id>_<short-name>.rs`
   断言：跑某 fixture 经某 CLI 路径，诊断输出不应包含某 `TANGLE_*` code
   运行 → 红

2. **定位根因**
   阅读对应模块代码，找到为什么产生误报
   在 `findings.md` 补全"根因（具体到文件:行号）"

3. **最小修复**
   只改这个根因，不顺手做无关重构
   运行该组测试 → 绿

4. **回归验证**
   重跑该根因组所有 cell → 应全部干净
   重跑 workspace 全量测试 → 既有测试不能变红

5. **提交**
   commit message: `fix(audit): <group-id> <短描述>`
   引用 finding ID（如 `F-001`）

### 5.2 测试组织

新建目录：`compiler/tangle-cli/tests/audit_regression/`

```
audit_regression/
├── G1_struct_symbol_resolution.rs
├── G2_let_binding_scope.rs
├── G3_member_access.rs
├── G5_doc_drift.rs
└── G6_platform_diff.rs
```

一根因组一文件。**G4 例外**：G4 是 Rust clippy 告警，无可写的 Rust 单元测试，其"回归测试"即出口闸 #4 本身（`cargo clippy --workspace -- -D warnings`），不需要独立的 `.rs` 文件。

### 5.3 测试 helper

在 `compiler/tangle-cli/src/lib.rs` 暴露测试用入口：

```rust
#[doc(hidden)]
pub fn run_collecting_diagnostics(args: &[&str]) -> TestRun;

pub struct TestRun {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub diagnostics: Vec<Diagnostic>,  // 已解析的 TANGLE_* 诊断
}
```

这样测试可以精确断言"诊断列表为空"或"诊断列表恰好包含 X"，而不是 grep 字符串。

### 5.4 修复顺序约束

- **G4 先修**：无依赖的工程清理，最快赢
- **G1/G2/G3 并行可能**：都在 checker，但根因不同，可并行 subagent
- **G5/G6 后修**：依赖前面 fix 后的稳定行为作基线
- 每组修完立即 commit，不积压

### 5.5 YAGNI 清单

本期**不做**：

- ❌ 不引入新的测试框架（用标准 `cargo test` + `assert_eq!`）
- ❌ 不重构现有 checker 架构（即使发现可改进点，记 backlog）
- ❌ 不写 fuzzing harness（本期是审计 + 修复，不是探索性测试）
- ❌ 不增加新的诊断 code（只消除误报，不引入新分类）

---

## 6. 出口闸与版本发布

### 6.1 出口闸（5 条全部满足才算审计阶段完成）

1. **审计矩阵重跑零虚假诊断**
   `tests/audit/run-audit.ps1` 重跑所有 ~250 个 cell，每个 cell 的 diagnostics 列表为空，或仅含**预期诊断白名单**中预声明的诊断

   **预期诊断白名单**：审计阶段为每个 fixture 文件维护一份 `tests/audit/expected_diagnostics.yaml`，声明该 fixture 在每个 CLI 路径下**应当**产生的诊断（如 `errors/payment.tangle.md` 故意触发 `PayFailed` 错误传播，应在 `run` 路径下产生一条对应诊断）。任何未在白名单中的诊断视为虚假诊断。白名单随审计进展逐步建立，初版为空（即所有 fixture 在所有路径下都期望零诊断），遇到故意错误用例时再添加。

2. **既有测试全过**
   `cargo test --workspace` —— 99 个既有测试 0 失败

3. **新增回归测试全过**
   `cargo test --workspace` 包含新增的 `audit_regression/*` 测试，0 失败

4. **`cargo clippy --workspace -- -D warnings` 零告警**
   `-D warnings` 把告警升级为错误，确保 G4 真的清干净

5. **差分测试通过**
   `tests/audit/diff-ir.ps1` 对所有"两边都支持"的 fixture 跑出 `MATCH` 或 `SKIPPED`，零 `DIFF`

### 6.2 验收脚本

一条命令跑完全部出口闸：

```powershell
# tests/audit/verify-exit-gate.ps1
# 依次：cargo test → cargo clippy → run-audit → diff-ir
# 任何一步失败立即退出非零，打印失败摘要
```

最后一行输出：`EXIT GATE: PASS` 或 `EXIT GATE: FAIL — <step>`。

### 6.3 版本与发布

- 切 **v0.2.1** 补丁版本（README + Cargo.toml + Cargo.lock）
- CHANGELOG 条目：

  ```
  ## v0.2.1 — Quality Audit
  - Fix: struct/method symbol resolution false positives (G1)
  - Fix: local `let` binding scope errors (G2)
  - Fix: member access type inference (G3)
  - Chore: clear all cargo clippy warnings (G4)
  - Docs: align examples with actual behavior (G5)
  - Test: add audit_regression suite + audit matrix baseline
  ```

- 创建 tag `v0.2.1`，**但不 push 远程**（由用户决定何时发）
- README 的 Roadmap 表新增 `✅ v0.2.1 — Quality Audit — Complete` 行

### 6.4 不做的事

- ❌ 不引入语义化版本兼容性承诺（仍是 0.x，breaking change 不需要 bump major）
- ❌ 不发 binaries release（只是源码 tag）
- ❌ 不写迁移指南（用户面尚小，CHANGELOG 足够）

---

## 7. 工作分解与依赖

```
G4 (Rust 告警)  ──┐
                  ├──> 出口闸 ──> v0.2.1
G1/G2/G3 (checker) ┤
                  │
G5 (doc drift)  ───┤  (依赖 G1-G3 稳定)
                  │
G6 (平台差异)    ──┘
```

- G4 立即可启
- G1/G2/G3 在审计完成后并行启
- G5/G6 等 G1-G3 落定后启
- 出口闸脚本本身可在修复早期就准备好，作为持续验证工具

---

## 8. 成功标准

| 标准 | 验证方法 |
|------|---------|
| `examples/account.tangle.md` 运行零诊断 | `cargo run -- run examples/account.tangle.md` 输出仅含 `{ balance: 150 }` |
| 全部 6 个 examples 零诊断 | 跑 `run-audit.ps1` 过滤 examples 行，diag_count=0 |
| 全部 9 个 tests/ fixture 仅产生预期诊断 | 跑 `run-audit.ps1` 过滤 tests 行，对照预期诊断白名单 |
| `cargo clippy --workspace -- -D warnings` 通过 | 退出码 0 |
| `cargo test --workspace` 通过 | 99 + 新增回归测试全过 |
| 差分测试零 DIFF | `diff-ir.ps1` 全 `MATCH` 或 `SKIPPED` |
| `verify-exit-gate.ps1` 输出 `EXIT GATE: PASS` | 一条命令验证 |

---

## 9. 后续衔接

本规格完成后，下一步调用 `writing-plans` 技能创建详细实现计划。本规格**不**覆盖 v0.3.0 Phase 1-4；这些阶段在本期审计交付 v0.2.1 后，各自走独立的 brainstorm → spec → plan 流程。
