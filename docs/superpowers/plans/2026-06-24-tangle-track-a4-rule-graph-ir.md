# Tangle Track A4: Rule Graph IR & Rule Directives 实现计划

> **语法精炼勘误（2026-06-25）：** (1) `with { }` → 无关键字大括号更新。(2) `Struct -> method` → 隐式方法绑定。(3) `=>` → `->`。(4) 新增 `|>` 管道。(5) 新增标题大小写对齐契约。(6) 移除 @export，改为下划线隐式私有。详见设计规格 §3.2。

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 A2（类型检查器）和 A3（错误处理）的基础上，设计并实现 Tangle 的统一中间表示（Rule Graph IR），并将类型检查后的代码和 4 种规则指令（@rule.flow/table/tree/toggle）lowering 到 IR。

**架构：** A3 输出类型检查和错误验证后的 `CheckedModule`。A4 新增：
1. **IR 数据结构** (`src/ir/graph.ts`)：Node、Edge、ErrorEdge、RuleGraph，全部携带 source span
2. **代码 Lowering** (`src/ir/lower.ts`)：Typed AST → IR nodes/edges
3. **规则指令 Lowering** (`src/ir/ruleFlow.ts`, `ruleTable.ts`, `ruleTree.ts`, `ruleToggle.ts`)
4. **IR 验证** (`src/ir/visibility.ts`, `src/ir/validate.ts`)：可见性检查、结构完整性验证

**技术栈：** TypeScript ESM、Vitest。可能需要 `mermaid` 解析库（或手写子集解析器）。

---

## 规格来源

- `docs/superpowers/specs/2026-06-24-tangle-language-design.md`
- A4 覆盖：§5.1（Rule Graph 统一原则）、§5.2（@rule.flow）、§5.3（@rule.table）、§5.4（@rule.tree）、§5.5（@rule.toggle）、§5.6（IR 可见性）

---

## IR 设计概览

```
RuleGraph
  ├── nodes: IRNode[]
  │     ├── kind: "action" | "compute" | "decision" | "terminal" | "error-terminal"
  │     ├── id: string
  │     ├── label: string
  │     ├── sourceSpan: SourceSpan
  │     └── code?: TypedExpr (for compute/action nodes)
  ├── edges: IREdge[]
  │     ├── from: nodeId
  │     ├── to: nodeId
  │     ├── kind: "control" | "condition" | "error"
  │     ├── guard?: TypedExpr
  │     └── sourceSpan: SourceSpan
  └── errorEdges: IRErrorEdge[]
        ├── from: nodeId
        ├── errorVariant: string
        └── sourceSpan: SourceSpan
```

---

## 文件结构

- 创建：`src/ir/graph.ts` — IR 数据结构定义
- 创建：`src/ir/lower.ts` — Typed AST → IR lowering
- 创建：`src/ir/ruleFlow.ts` — Mermaid 解析 + @rule.flow lowering
- 创建：`src/ir/ruleTable.ts` — @rule.table 决策表 lowering
- 创建：`src/ir/ruleTree.ts` — @rule.tree 嵌套列表 lowering
- 创建：`src/ir/ruleToggle.ts` — @rule.toggle 复选框 lowering
- 创建：`src/ir/visibility.ts` — IR 级可见性检查
- 创建：`src/ir/validate.ts` — IR 结构验证
- 修改：`src/index.ts` — barrel 导出

---

## 任务 1：定义 IR 数据结构

**文件：**
- 创建：`src/ir/graph.ts`
- 修改：`src/index.ts`
- 创建：`tests/ir/graph.test.ts`

- [ ] **步骤 1：编写 IR 数据结构测试**

创建 `tests/ir/graph.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import type { IRNode, IREdge, IRErrorEdge, RuleGraph } from "../../src/index";

describe("IR data structures", () => {
  it("defines IRNode discriminated union", () => {
    const node: IRNode = {
      kind: "action",
      id: "n1",
      label: "send notification",
      sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 20 }
    };
    expect(node.kind).toBe("action");
    expect(node.id).toBe("n1");
  });

  it("defines IRNode variants", () => {
    const kinds = ["action", "compute", "decision", "terminal", "error-terminal"] as const;
    kinds.forEach(k => {
      const node: IRNode = { kind: k, id: "n", label: k, sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } };
      expect(node.kind).toBe(k);
    });
  });

  it("defines IREdge with guard", () => {
    const edge: IREdge = {
      from: "n1",
      to: "n2",
      kind: "condition",
      guard: "x > 0",
      sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 10 }
    };
    expect(edge.kind).toBe("condition");
    expect(edge.guard).toBe("x > 0");
  });

  it("defines IRErrorEdge", () => {
    const errEdge: IRErrorEdge = {
      from: "n1",
      errorVariant: "PayFailed",
      sourceSpan: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 10 }
    };
    expect(errEdge.errorVariant).toBe("PayFailed");
  });

  it("assembles a minimal RuleGraph", () => {
    const graph: RuleGraph = {
      nodes: [],
      edges: [],
      errorEdges: [],
      entryNodeId: "n1"
    };
    expect(graph.entryNodeId).toBe("n1");
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/ir/graph.test.ts`
预期：FAIL。

- [ ] **步骤 3：实现 `src/ir/graph.ts`**

```ts
import type { SourceSpan } from "../model.js";

export type IRNodeKind = "action" | "compute" | "decision" | "terminal" | "error-terminal";

export type IRNode = {
  kind: IRNodeKind;
  id: string;
  label: string;
  sourceSpan: SourceSpan;
  // For compute/action nodes, optional link to typed expression
  typedExpr?: object;
};

export type IREdgeKind = "control" | "condition" | "error";

export type IREdge = {
  from: string;
  to: string;
  kind: IREdgeKind;
  guard?: string;
  sourceSpan: SourceSpan;
};

export type IRErrorEdge = {
  from: string;
  errorVariant: string;
  sourceSpan: SourceSpan;
};

export type RuleGraph = {
  nodes: IRNode[];
  edges: IREdge[];
  errorEdges: IRErrorEdge[];
  entryNodeId: string;
};
```

- [ ] **步骤 4：修改 `src/index.ts` + 验证**

运行：`npm test -- tests/ir/graph.test.ts` 预期：PASS。
运行：`npm run typecheck` 预期：PASS。

---

## 任务 2：实现代码到 IR 的 Lowering

**文件：**
- 创建：`src/ir/lower.ts`
- 创建：`tests/ir/lower.test.ts`

将 Typed AST (Express/Stmt) 转换为 IR Node/Edge：

- `return expr` → terminal/compute node
- `expr?` → compute node + error edge
- `if (cond) a else b` → decision node + 条件边到 a/b 子图
- `match expr { arms }` → decision node + 每个 arm 一条边
- `let x = expr` → compute node + variable binding
- 连续语句 → sequential edges

每个 IR 元素携带 source span 用于错误回溯。

---

## 任务 3：实现 @rule.flow — Mermaid 图解析

**文件：**
- 创建：`src/ir/ruleFlow.ts`
- 创建：`tests/ir/ruleFlow.test.ts`

解析简单 Mermaid `graph TD` 图：
- `A[新订单] -->|支付成功| B(已支付)` → IR node A 到 node B 的 condition edge
- `F(错误: PayFailed)` → error-terminal node
- 节点标签中的 `label: TypeName` 映射到 IR 类型

简单手写 Mermaid 解析器（覆盖 `graph TD` 子集：节点声明 `X[label]` 或 `X(label)`，边 `-->|label| Y`）。

---

## 任务 4：实现 @rule.table — 决策表

**文件：**
- 创建：`src/ir/ruleTable.ts`
- 创建：`tests/ir/ruleTable.test.ts`

将 Markdown 表格转换为 IR：
- 表格第一行是列名（条件列 + 动作列）
- 每一数据行 → 条件合取作为守卫的 decision node
- 动作列 → 目标 action/terminal node

```markdown
@rule.table
| 金额 > 1000 | 用户等级 | 结果 |
|-------------|----------|------|
| true        | VIP      | 通过 |
| false       | -        | 拒绝 |
```

---

## 任务 5：实现 @rule.tree — 决策树

**文件：**
- 创建：`src/ir/ruleTree.ts`
- 创建：`tests/ir/ruleTree.test.ts`

将嵌套 Markdown 列表转换为 IR：
- 同级列表项 → AND（所有条件必须满足）
- 子级列表项 → OR（任一子树满足即可）
- 叶子项 → 条件表达式

---

## 任务 6：实现 @rule.toggle — 布尔配置

**文件：**
- 创建：`src/ir/ruleToggle.ts`
- 创建：`tests/ir/ruleToggle.test.ts`

将复选框列表转换为 IR：
- `- [x] flag: description` → `flag = true` 配置 node
- `- [ ] flag: description` → `flag = false` 配置 node

---

## 任务 7：实现 IR 可见性检查

**文件：**
- 创建：`src/ir/visibility.ts`
- 创建：`tests/ir/visibility.test.ts`

`checkIRVisibility(graph, visibleSymbols)`:
- 遍历 IR nodes，检查引用的符号是否在 `visibleSymbols` 中
- 非 `@export` 符号只能在模块内部 IR 中引用
- 跨模块边需要验证导入/导出配对

---

## 任务 8：实现 IR 验证

**文件：**
- 创建：`src/ir/validate.ts`
- 创建：`tests/ir/validate.test.ts`

IR 结构完整性检查：
- 所有 edges 的 from/to 指向存在的 nodes
- entryNodeId 指向存在的 node
- error edges 引用的 error variant 在 ErrorRegistry 中已注册
- 无孤立节点（除 terminal 外每个 node 至少有一条出边）
- 无循环（可配置，某些场景允许循环）

---

## 任务 9：Pipeline 集成 — compileToIR

**文件：**
- 创建：`src/ir/compileToIR.ts`
- 创建：`tests/ir/compileToIR.test.ts`

`compileToIR(checkedModule): RuleGraph`:
1. 从 `CheckedModule` 的 parsedBlocks lower 代码 → IR
2. 扫描 headings，处理 `@rule.*` 指令
3. 合并所有子图
4. 运行 IR 验证
5. 返回完整 `RuleGraph`

---

## 任务 10：全量验证

运行：`npm test` — 全部 PASS（A1 + A2 + A3 + A4）
运行：`npm run typecheck` — PASS

---

## 计划自检清单

- 规格覆盖：§5.1-5.6 全覆盖
- 明确排除：IR 优化 pass、IR 可视化输出、IR 持久化格式
- 占位符扫描：所有任务含具体类型签名和测试
- 类型一致性：`IRNode`、`IREdge`、`IRErrorEdge`、`RuleGraph` 跨任务一致
- 向后兼容：不修改 A1-A3 的文件和测试
