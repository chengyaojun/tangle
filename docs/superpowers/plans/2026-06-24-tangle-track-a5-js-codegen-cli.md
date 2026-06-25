# Tangle Track A5: JS Codegen & CLI 实现计划

> **语法精炼勘误（2026-06-25）：** (1) `with { }` → 无关键字大括号更新。(2) `Struct -> method` → 隐式方法绑定。(3) `=>` → `->`。(4) 新增 `|>` 管道。(5) 新增标题大小写对齐契约。(6) 移除 @export，改为下划线隐式私有。(7) 移除 @entry，改为 main 隐式入口契约。详见设计规格 §3.2。

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 A4（Rule Graph IR）的基础上，实现 JavaScript/TypeScript 代码生成器和 `tangle run` / `tangle test` CLI 命令。

**架构：** A4 输出 `RuleGraph`（统一的中间表示）。A5 新增：
1. **JS Codegen** (`src/codegen/jsEmitter.ts`)：IR → 可执行 JavaScript 代码
2. **Error Mapping** (`src/codegen/errorMapping.ts`)：错误结果映射策略（`{ok, value}` / `{ok, error}`）
3. **CLI** (`src/cli/run.ts`, `src/cli/test.ts`, `src/cli/main.ts`)：`tangle run` 和 `tangle test` 命令

**技术栈：** TypeScript ESM、Vitest。CLI 使用 Node.js `process.argv` 解析（或可加 `commander`）。

---

## 规格来源

- `docs/superpowers/specs/2026-06-24-tangle-language-design.md`
- A5 覆盖：§6.1（编译流水线）、§6.2（CLI）、§6.3（宿主映射）

---

## 文件结构

- 创建：`src/codegen/jsEmitter.ts` — IR → JS 代码生成
- 创建：`src/codegen/errorMapping.ts` — 错误结果映射
- 创建：`src/codegen/prelude.ts` — JS 运行时 prelude（辅助函数）
- 创建：`src/cli/run.ts` — `tangle run` 实现
- 创建：`src/cli/test.ts` — `tangle test` 实现
- 创建：`src/cli/main.ts` — CLI 入口 + 参数解析
- 修改：`src/index.ts` — barrel 导出
- 修改：`package.json` — 添加 `bin` 字段和 CLI 脚本

---

## 任务 1：定义 JS Codegen 基础框架

**文件：**
- 创建：`src/codegen/jsEmitter.ts`
- 创建：`tests/codegen/jsEmitter.test.ts`

- [ ] **步骤 1：编写基础 codegen 测试**

```ts
import { describe, expect, it } from "vitest";
import { emitJS } from "../../src/index";
import type { RuleGraph } from "../../src/index";

describe("emitJS", () => {
  it("emits a simple terminal node graph as a function", () => {
    const graph: RuleGraph = {
      nodes: [
        { kind: "terminal", id: "entry", label: "entry", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } }
      ],
      edges: [],
      errorEdges: [],
      entryNodeId: "entry"
    };
    const js = emitJS(graph);
    expect(js).toContain("function");
    expect(js).toContain("entry");
  });

  it("emits action nodes as function calls", () => {
    const graph: RuleGraph = {
      nodes: [
        { kind: "action", id: "n1", label: "greet", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 10 } },
        { kind: "terminal", id: "end", label: "end", sourceSpan: { file: "t.md", startLine: 2, startColumn: 1, endLine: 2, endColumn: 1 } }
      ],
      edges: [
        { from: "n1", to: "end", kind: "control", sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 2, endColumn: 1 } }
      ],
      errorEdges: [],
      entryNodeId: "n1"
    };
    const js = emitJS(graph);
    expect(js).toContain("greet");
  });
});
```

- [ ] **步骤 2：实现 `src/codegen/jsEmitter.ts`**

```ts
export function emitJS(graph: RuleGraph): string;
// 遍历 graph 的节点和边，生成 JS 代码
// 策略：拓扑排序节点，每个节点生成一个函数
// 边生成控制流（if/else、函数调用链）
```

---

## 任务 2：实现错误结果映射

**文件：**
- 创建：`src/codegen/errorMapping.ts`
- 创建：`tests/codegen/errorMapping.test.ts`

JS 目标的错误返回映射：`{ ok: true, value }` 或 `{ ok: false, error }`。

- `wrapResult(jsExpr)` → `{ ok: true, value: jsExpr }`
- `wrapError(jsExpr, errorVariant)` → `{ ok: false, error: { variant: errorVariant, value: jsExpr } }`
- `propagationCheck(jsExpr)` → `if (!result.ok) return result;` 模式
- `matchToSwitch(jsExpr, arms)` → `switch (result.error.variant) { case "X": ... }`

---

## 任务 3：实现 IR 节点的 JS 发射

每种 IR 节点类型的 JS 发射策略：

| IR Node Kind | JS Emit |
|---|---|
| `action` | 函数调用 + ok 包装 |
| `compute` | 表达式求值 + ok 包装 |
| `decision` | `if/else if/else` 分支 |
| `terminal` | `return { ok: true, value: result }` |
| `error-terminal` | `return { ok: false, error: { variant, value } }` |

控制流边 → 顺序调用或条件分支
条件边 → `if (guard)` 分支
错误边 → `return { ok: false, ... }`

---

## 任务 4：实现 JS 运行时 Prelude

**文件：**
- 创建：`src/codegen/prelude.ts`
- 创建：`tests/codegen/prelude.test.ts`

生成 JS 代码时需要内联或导入的辅助函数：
- `__tangle_struct(fields)` — 创建不可变结构体
- `__tangle_with(obj, updates)` — `with` 更新语法
- `__tangle_match(result, handlers)` — match 运行时
- `__tangle_isError(result)` — 错误检查

---

## 任务 5：实现完整编译流水线

**文件：**
- 创建：`src/pipeline.ts`
- 修改：`src/index.ts`
- 创建：`tests/pipeline.test.ts`

`compile(source, file): { js: string; diagnostics: TangleDiagnostic[] }`:
1. `compileModule` (A1) → `TangleModule`
2. `checkModule` (A2+A3) → `CheckedModule`
3. `compileToIR` (A4) → `RuleGraph`
4. `emitJS` (A5) → JS string

端到端测试：输入 `.md` 文件，输出可执行的 JS 代码。

---

## 任务 6：实现 CLI 入口和参数解析

**文件：**
- 创建：`src/cli/main.ts`
- 修改：`package.json`（添加 `bin` 字段和 `tangle` 命令）

```json
{
  "bin": {
    "tangle": "./dist/src/cli/main.js"
  },
  "scripts": {
    "build": "tsc",
    "tangle": "node dist/src/cli/main.js"
  }
}
```

CLI 参数结构：
```
tangle run <file.md> [--args key=value ...]
tangle test [--filter <pattern>]
```

---

## 任务 7：实现 `tangle run`

**文件：**
- 创建：`src/cli/run.ts`
- 创建：`tests/cli/run.test.ts`

`tangle run ./main.md`:
1. 读取 `.md` 文件
2. 运行完整编译流水线
3. 若编译错误，输出诊断到 stderr 并退出码 1
4. 若编译通过，eval/写入临时 JS 文件并用 Node.js 执行
5. 将 CLI 参数以结构体形式注入 `@entry` 函数

---

## 任务 8：实现 `tangle test`

**文件：**
- 创建：`src/cli/test.ts`
- 创建：`tests/cli/test.test.ts`

`tangle test`:
1. 扫描项目中所有 `.md` 文件（或指定目录）
2. 编译每个文件
3. 收集所有 `@test` 指令
4. 逐个执行测试（用 JS codegen + Node.js）
5. 输出测试报告（通过/失败计数）

---

## 任务 9：错误报告格式化

**文件：**
- 修改：`src/cli/main.ts`
- 创建：`tests/cli/diagnostics.test.ts`

将 `TangleDiagnostic[]` 格式化为类似 Rust 编译器的友好输出：
```
error[TANGLE_TYPE_MISMATCH]: Operator + requires matching types
  --> user.md:12:5
   |
12 |   return 1 + "hello"
   |            ^^^^^^^^^
```

---

## 任务 10：全量验证

运行：`npm test` — 全部 PASS（A1-A5）
运行：`npm run typecheck` — PASS
运行：`npx tangle run ./examples/test.md` — 可执行

---

## 计划自检清单

- 规格覆盖：§6.1-6.3
- 明确排除：TypeScript 声明文件生成、source map 生成、多文件项目编译、watch 模式
- 占位符扫描：无 TODO
- 向后兼容：不修改 A1-A4 的模块
