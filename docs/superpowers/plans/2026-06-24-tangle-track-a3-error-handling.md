# Tangle Track A3: Error Handling Semantics 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 A2 的类型系统基础上，实现错误变体声明追踪、`expr?` 传播运算符、`match` 穷举检查、双值解构语法糖和 `panic` 不可恢复错误语义。

**架构：** A2 输出 `CheckedModule`（含类型环境和已解析的 Code AST）。A3 新增：
1. **错误注册层** (`src/checker/errors.ts`)：从 `@error` 指令提取错误变体类型，建立错误族
2. **传播分析层** (`src/checker/propagation.ts`)：`?` 运算符的类型级传播规则
3. **模式匹配层** (`src/checker/match.ts`)：`match` 穷举检查、模式类型收窄
4. **Panic 语义** (`src/checker/panic.ts`)：`panic` 不可恢复标记

**技术栈：** TypeScript ESM、Vitest。无新增依赖。

---

## 规格来源

- `docs/superpowers/specs/2026-06-24-tangle-language-design.md`
- A3 覆盖：§4.5（错误编译器强制）、§4.6（错误传播与处理，`?`、`match`、双值解构）、§4.7（Panic）

---

## 文件结构

- 创建：`src/checker/errors.ts` — 错误变体类型注册、错误族建模
- 创建：`src/checker/propagation.ts` — `?` 运算符传播规则
- 创建：`src/checker/match.ts` — `match` 穷举检查 + 模式类型收窄
- 创建：`src/checker/panic.ts` — `panic` 语义
- 修改：`src/checker/check.ts` — 增强类型检查器支持 `?`、`match`、`panic`
- 修改：`src/checker/checkModule.ts` — 集成错误验证到 pipeline
- 修改：`src/ast.ts` — 新增 `MatchExpr`、`PropagationExpr`、`DestructureExpr`、`PanicExpr` AST 节点
- 修改：`src/parser/parser.ts` — 解析 `?`、`match`、`panic`、双值解构语法
- 修改：`src/index.ts` — barrel 导出

---

## 任务 1：扩展 Code AST 以支持错误处理语法

**文件：**
- 修改：`src/ast.ts`
- 修改：`src/index.ts`
- 修改：`tests/parser/parser.test.ts`

- [ ] **步骤 1：编写 AST 扩展测试**

在 `tests/parser/parser.test.ts` 中追加：

```ts
import type { MatchExpr, PropagationExpr, DestructureExpr, PanicExpr } from "../../src/index";

describe("error handling AST types", () => {
  it("defines PropagationExpr for ? operator", () => {
    const pe: PropagationExpr = {
      kind: "propagation",
      expr: { kind: "identifier", name: "result", span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 7 } },
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 8 }
    };
    expect(pe.kind).toBe("propagation");
  });

  it("defines MatchExpr with arms", () => {
    const me: MatchExpr = {
      kind: "match",
      expr: { kind: "identifier", name: "result", span: { file: "t.md", startLine: 1, startColumn: 7, endLine: 1, endColumn: 13 } },
      arms: [],
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 5, endColumn: 1 }
    };
    expect(me.kind).toBe("match");
  });

  it("defines DestructureExpr for double-value destructuring", () => {
    const de: DestructureExpr = {
      kind: "destructure",
      okName: "receipt",
      errName: "err",
      expr: { kind: "identifier", name: "confirm", span: { file: "t.md", startLine: 1, startColumn: 2, endLine: 1, endColumn: 9 } },
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 30 }
    };
    expect(de.okName).toBe("receipt");
    expect(de.errName).toBe("err");
  });

  it("defines PanicExpr", () => {
    const pe: PanicExpr = {
      kind: "panic",
      message: { kind: "literal", literalKind: "string", value: "unrecoverable", span: { file: "t.md", startLine: 1, startColumn: 8, endLine: 1, endColumn: 24 } },
      span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 25 }
    };
    expect(pe.kind).toBe("panic");
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/parser/parser.test.ts`
预期：FAIL。

- [ ] **步骤 3：在 `src/ast.ts` 中追加新类型**

```ts
export type PropagationExpr = {
  kind: "propagation";
  expr: Expr;
  span: SourceSpan;
};

export type MatchArm = {
  pattern: MatchPattern;
  body: Expr;
  span: SourceSpan;
};

export type MatchPattern =
  | { kind: "variantPattern"; name: string; binding?: string; span: SourceSpan }
  | { kind: "wildcardPattern"; span: SourceSpan };

export type MatchExpr = {
  kind: "match";
  expr: Expr;
  arms: MatchArm[];
  span: SourceSpan;
};

export type DestructureExpr = {
  kind: "destructure";
  okName: string;
  errName: string;
  expr: Expr;
  span: SourceSpan;
};

export type PanicExpr = {
  kind: "panic";
  message: Expr;
  span: SourceSpan;
};
```

同时更新 `Expr` 联合类型增加 `PropagationExpr | MatchExpr | DestructureExpr | PanicExpr`，更新 `Stmt` 增加 `IfErrStmt`（`if (err != nil) { ... }` 形式）。

- [ ] **步骤 4：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/parser/parser.test.ts`
预期：PASS。`npm run typecheck` 预期：PASS。

---

## 任务 2：实现错误变体类型注册

**文件：**
- 创建：`src/checker/errors.ts`
- 修改：`src/index.ts`
- 创建：`tests/checker/errors.test.ts`

- [ ] **步骤 1：编写测试**

```ts
// tests/checker/errors.test.ts
import { describe, expect, it } from "vitest";
import { ErrorRegistry, registerError, findError } from "../../src/index";

describe("ErrorRegistry", () => {
  it("registers and looks up error variants", () => {
    const reg = new ErrorRegistry();
    reg.register("PayFailed", { code: "Int", reason: "String" });
    const variant = reg.lookup("PayFailed");
    expect(variant).toBeDefined();
    expect(variant!.fields.code).toEqual({ kind: "primitive", name: "Int" });
  });

  it("checks if a type is an error variant", () => {
    const reg = new ErrorRegistry();
    reg.register("Timeout", {});
    expect(reg.isError("Timeout")).toBe(true);
    expect(reg.isError("User")).toBe(false);
  });

  it("collects error variants from @error directives", () => {
    const reg = new ErrorRegistry();
    reg.collectFromDirectives([
      { kind: "error", raw: "@error PayFailed(\"支付失败\", code: Int)", name: "PayFailed", args: '"支付失败", code: Int', span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 40 } }
    ]);
    expect(reg.isError("PayFailed")).toBe(true);
  });
});
```

- [ ] **步骤 2：运行测试验证失败 + 实现**

实现 `src/checker/errors.ts`：
- `ErrorRegistry` 类：`register(name, fields)`, `lookup(name)`, `isError(name)`, `collectFromDirectives(directives)`
- 每个错误变体作为 sum type 的一个 variant
- 错误族绑定到其声明的模块/作用域

运行测试验证 PASS。

---

## 任务 3：扩展 Parser 支持 `?`、`match`、`panic`、双值解构

**文件：**
- 修改：`src/parser/parser.ts`
- 修改：`tests/parser/parser.test.ts`

- [ ] **步骤 1：编写解析测试**

在 `tests/parser/parser.test.ts` 中追加 `parseExpression` 测试：

```ts
it("parses propagation operator ?", () => {
  const expr = parseExpression(tokenize("result?", "test.md"));
  expect(expr).toMatchObject({ kind: "propagation", expr: { kind: "identifier", name: "result" } });
});

it("parses panic expressions", () => {
  const expr = parseExpression(tokenize('panic("unrecoverable")', "test.md"));
  expect(expr).toMatchObject({ kind: "panic" });
});

it("parses destructure expressions", () => {
  const expr = parseExpression(tokenize("(receipt, err) = confirm(order)", "test.md"));
  expect(expr).toMatchObject({ kind: "destructure", okName: "receipt", errName: "err" });
});
```

- [ ] **步骤 2：实现解析**

在 `src/parser/parser.ts` 中：
- Postfix `?` 操作符：在 `parseInfix` 中处理 `question` token → `PropagationExpr`
- `panic` 关键字：prefix 解析，`panic("msg")` → `PanicExpr`
- `(ident, ident) = expr`：在语句解析中识别双值解构模式 → `DestructureExpr`
- `match expr { ... }`：在 prefix 解析中处理 → `MatchExpr`

---

## 任务 4：实现 `?` 传播类型规则

**文件：**
- 创建：`src/checker/propagation.ts`
- 修改：`src/checker/check.ts`
- 创建：`tests/checker/propagation.test.ts`

- [ ] **步骤 1：编写测试**

```ts
import { describe, expect, it } from "vitest";
import { checkPropagation } from "../../src/index";

describe("propagation type checking", () => {
  it("strips Ok variant from result type on ?", () => {
    // result: Receipt | PayFailed
    // result? : Receipt (with PayFailed propagated)
    const resultType = {
      kind: "sum" as const,
      variants: [
        { kind: "struct" as const, name: "Receipt", fields: {}, methods: {} },
        { kind: "struct" as const, name: "PayFailed", fields: {}, methods: {} }
      ]
    };
    const [type, errors] = checkPropagation(resultType);
    // Should extract the non-error variant type
    expect(errors).toEqual([]);
  });
});
```

- [ ] **步骤 2：实现**

`checkPropagation(type, errorRegistry)`:
- 若 type 是 sum type，分离错误变体（在 ErrorRegistry 中注册的）和非错误变体
- 若只有 1 个非错误变体，返回该类型
- 若多个非错误变体，报错
- 记录传播的错误变体到当前函数签名的错误集合

---

## 任务 5：实现 `match` 穷举检查

**文件：**
- 创建：`src/checker/match.ts`
- 修改：`src/checker/check.ts`
- 创建：`tests/checker/match.test.ts`

- [ ] **步骤 1：编写测试**

```ts
import { describe, expect, it } from "vitest";
import { checkMatchExhaustiveness } from "../../src/index";

describe("match exhaustiveness", () => {
  it("accepts match covering all variants", () => {
    const sumType = {
      kind: "sum" as const,
      variants: [
        { kind: "struct" as const, name: "Receipt", fields: {}, methods: {} },
        { kind: "struct" as const, name: "PayFailed", fields: {}, methods: {} }
      ]
    };
    const arms = ["Receipt", "PayFailed"];
    const missing = checkMatchExhaustiveness(sumType, arms);
    expect(missing).toHaveLength(0);
  });

  it("reports missing variants in non-exhaustive match", () => {
    const sumType = {
      kind: "sum" as const,
      variants: [
        { kind: "struct" as const, name: "Receipt", fields: {}, methods: {} },
        { kind: "struct" as const, name: "PayFailed", fields: {}, methods: {} },
        { kind: "struct" as const, name: "Timeout", fields: {}, methods: {} }
      ]
    };
    const arms = ["Receipt"];
    const missing = checkMatchExhaustiveness(sumType, arms);
    expect(missing).toContain("PayFailed");
    expect(missing).toContain("Timeout");
  });

  it("accepts wildcard arm as covering remaining", () => {
    const missing = checkMatchExhaustiveness(
      { kind: "sum" as const, variants: [{ kind: "primitive" as const, name: "Int" }, { kind: "primitive" as const, name: "String" }] },
      ["Int", "_"]
    );
    expect(missing).toHaveLength(0);
  });
});
```

- [ ] **步骤 2：实现**

`checkMatchExhaustiveness(sumType, armPatterns)`:
- 收集 sum type 所有 variant 名称
- 对每个 arm pattern，匹配覆盖的 variant
- 通配符 `_` 覆盖所有剩余
- 返回未覆盖的 variant 名称列表

---

## 任务 6：实现 `panic` 语义

**文件：**
- 创建：`src/checker/panic.ts`
- 修改：`src/checker/check.ts`
- 创建：`tests/checker/panic.test.ts`

`checkPanic`：标记表达式为不可恢复。panic 调用后代码路径被标记为 terminated（dead code）。`panic` 不得被 `match` 或 `?` 捕获。

---

## 任务 7：编译器强制错误声明检查

**文件：**
- 修改：`src/checker/checkModule.ts`
- 创建：`tests/checker/errorDeclaration.test.ts`

验证函数若传播（`?`）或返回未声明的错误变体，编译失败：
- 新增 diagnostic 码 `TANGLE_UNDECLARED_ERROR_PROPAGATION`
- 在 `checkModule` 流程中追踪每个函数的声明错误集 (`@error` directives)
- 对函数体中每个 `?` 或 `match` 分支，验证错误变体在已声明集中

---

## 任务 8：双值解构语法糖类型检查

`(ok, err) = expr` 等价于 match 的语法糖。类型规则：
- `expr` 必须是 sum type
- `ok` 绑定到非错误变体的类型
- `err` 绑定到错误变体的并集类型
- `if (err != nil)` 作为模式匹配 guard

---

## 任务 9：Pipeline 集成与端到端测试

**文件：**
- 修改：`src/checker/checkModule.ts`
- 修改：`tests/fixtures.ts` — 新增错误处理夹具
- 创建：`tests/checker/errorPipeline.test.ts`

端到端测试验证：
1. `?` 在未声明错误时产生 diagnostic
2. `match` 穷举检查报告缺失 variant
3. 正确传播错误变体链

---

## 任务 10：全量验证

运行：`npm test` — 全部 PASS（A1 + A2 + A3）
运行：`npm run typecheck` — PASS

---

## 计划自检清单

- 规格覆盖：§4.5（@error 编译器强制）、§4.6（expr? / match / 双值解构）、§4.7（panic）
- 明确排除：运行时错误处理执行（codegen 阶段）、try-catch、异常互操作
- 占位符扫描：每个步骤含实际代码和测试
- 向后兼容：A1 和 A2 测试继续通过
