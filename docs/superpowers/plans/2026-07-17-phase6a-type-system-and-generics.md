# Phase 6a: 类型系统对齐 + 局部泛型推导 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 闭合 order-service.tangle 差分测试 + 实现 Rust/TS 双端局部泛型类型推导

**架构：** 两个独立工作流。工作流 1（任务 2-5）TS 端类型检查器对齐 Rust，闭合 order-service。工作流 2（任务 6-12）Rust 先实现泛型推导（unify 算法 + stdlib 泛型签名），TS 端忠实移植。任务 13 出口闸门。

**技术栈：** Rust + serde_json（ir-diff）、TypeScript + vitest（reference）、PowerShell（diff-ir.ps1）

**规格文档：** `docs/superpowers/specs/2026-07-17-phase6a-type-system-and-generics-design.md`

---

## 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `reference/src/checker/check.ts` | TS call 泛型推导 + identifier 扩展 + 结构体构造器 | 修改 |
| `reference/src/checker/checkModule.ts` | 参数类型解析（用 typeName 而非默认 String） | 修改 |
| `reference/src/checker/resolve.ts` | 方法返回类型改 Any + 导出 typeExprToType | 修改 |
| `reference/src/checker/builtins.ts` | Err/Ok 注册 + 泛型签名 | 修改 |
| `reference/src/checker/types.ts` | 添加 typeVar/generic 构造函数 | 修改 |
| `reference/src/checker/unify.ts` | TS 类型统一算法 | 创建 |
| `reference/src/checker/env.ts` | TypeEnv 添加 functions 字段 | 修改 |
| `compiler/tangle-cli/src/checker/types.rs` | 泛型构造辅助函数 type_var/generic | 修改 |
| `compiler/tangle-cli/src/checker/unify.rs` | 类型统一算法 unify + substitute | 创建 |
| `compiler/tangle-cli/src/checker/mod.rs` | 注册 unify 模块 | 修改 |
| `compiler/tangle-cli/src/checker/check.rs` | call 加入泛型推导 | 修改 |
| `compiler/tangle-cli/src/stdlib/signatures.rs` | stdlib 泛型签名（List/Map/Option/Set） | 修改 |
| `reference/tests/checker/order-service.test.ts` | order-service 类型检查测试 | 创建 |
| `reference/tests/checker/generics.test.ts` | TS 泛型推导测试 | 创建 |
| `compiler/tangle-cli/tests/v06_phase6/generics_inference.rs` | Rust 泛型推导测试 | 创建 |
| `compiler/tangle-cli/Cargo.toml` | 新增 [[test]] 条目 | 修改 |
| `tests/v06_phase6/generics.tangle.md` | 泛型 fixture | 创建 |

**总计**：修改 11 个文件，创建 6 个文件。

---

## 任务 1：Worktree 与分支准备

**文件：** 无文件修改，仅 git 操作

- [ ] **步骤 1：基于 main 创建 worktree**

运行：
```bash
cd e:\GitProjects\tangle
git fetch origin
git worktree add .worktrees/phase6a-v0.6.0 -b phase6a/v0.6.0 main
```

预期：创建 `.worktrees/phase6a-v0.6.0` 目录，新分支 `phase6a/v0.6.0` 基于 `main` 最新 commit。

- [ ] **步骤 2：验证 worktree 可编译**

运行：
```bash
cd .worktrees/phase6a-v0.6.0
cargo build --workspace
```

预期：编译成功，无错误。

- [ ] **步骤 3：验证基线测试全绿**

运行：
```bash
cargo test --workspace
```

预期：所有现有测试通过（280 passed）。

- [ ] **步骤 4：记录差分测试基线**

运行：
```bash
pwsh tests/audit/diff-ir.ps1
```

预期：10 MATCH / 1 SKIPPED (order-service)。记录输出作为基线。

- [ ] **步骤 5：验证 reference 项目基线**

运行：
```bash
cd reference
npm install
npm run build
npm test
cd ..
```

预期：TS 编译成功，现有 vitest 测试通过（177 passed）。

---

## 任务 2：TS check.ts identifier 查找 structs + env.ts 添加 functions

**文件：**
- 修改：`reference/src/checker/check.ts:21-29`
- 修改：`reference/src/checker/env.ts`
- 创建：`reference/tests/checker/identifier-struct.test.ts`

**背景：** 当前 TS `check.ts` 的 `identifier` case 只查找 `env.variables` 和 `env.receiver.fields`，不查找 `env.structs`。导致 `Order` 报 `Undefined variable`。Rust 端 `check.rs:20` 有 `env.structs.get(name)`。

- [ ] **步骤 1：编写失败的测试**

创建 `reference/tests/checker/identifier-struct.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { checkExpression } from "../../src/checker/check.js";
import { createEnv } from "../../src/checker/env.js";
import type { StructType } from "../../src/checker/types.js";

describe("identifier resolves struct", () => {
  it("returns struct type when identifier matches env.structs", () => {
    const env = createEnv();
    const orderStruct: StructType = {
      kind: "struct",
      name: "Order",
      fields: { id: { kind: "primitive", name: "String" } },
      methods: {},
    };
    env.structs["Order"] = orderStruct;

    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Order", span: { file: "test.tangle", startLine: 1, startColumn: 1, endLine: 1, endColumn: 5 } },
      env
    );

    expect(diags).toHaveLength(0);
    expect(type.kind).toBe("struct");
    if (type.kind === "struct") {
      expect(type.name).toBe("Order");
    }
  });

  it("returns undefined variable diagnostic for unknown identifier", () => {
    const env = createEnv();
    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Unknown", span: { file: "test.tangle", startLine: 1, startColumn: 1, endLine: 1, endColumn: 7 } },
      env
    );
    expect(diags.some(d => d.code === "TANGLE_TYPE_UNDEFINED_VARIABLE")).toBe(true);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd reference
npx vitest run tests/checker/identifier-struct.test.ts
```

预期：FAIL，第一个测试报 `Undefined variable: Order`（因为 check.ts 未查找 env.structs）。

- [ ] **步骤 3：实现 identifier 查找 structs**

修改 `reference/src/checker/check.ts:21-29`，在 `identifier` case 中添加 `env.structs` 查找：

```typescript
    case "identifier": {
      if (env.variables[expr.name]) return [env.variables[expr.name]!, diags];
      if (env.receiver?.fields[expr.name]) return [env.receiver.fields[expr.name]!, diags];
      if (env.structs[expr.name]) return [env.structs[expr.name]!, diags];       // ← 新增
      if (env.functions[expr.name]) {                                            // ← 新增
        const fn = env.functions[expr.name]!;
        return [{ kind: "function", params: fn.params.map(p => p.type), returns: fn.returns }, diags];
      }
      if (["String", "Int", "Bool"].includes(expr.name)) {
        return [{ kind: "primitive", name: expr.name as "String" | "Int" | "Bool" }, diags];
      }
      diags.push({ code: "TANGLE_TYPE_UNDEFINED_VARIABLE", message: `Undefined variable: ${expr.name}`, span: expr.span });
      return [{ kind: "primitive", name: "String" }, diags];
    }
```

修改 `reference/src/checker/env.ts`，添加 `functions` 字段：

```typescript
import type { Type, StructType, InterfaceType, CallableSignature } from "./types.js";
import type { ErrorRegistry } from "./errors.js";

export type ReceiverContext = {
  structName: string;
  fields: Record<string, Type>;
};

export type TypeEnv = {
  variables: Record<string, Type>;
  structs: Record<string, StructType>;
  interfaces: Record<string, InterfaceType>;
  functions: Record<string, CallableSignature>;  // ← 新增
  receiver?: ReceiverContext;
  errorRegistry?: ErrorRegistry;
};

export function createEnv(): TypeEnv {
  return { variables: {}, structs: {}, interfaces: {}, functions: {} };
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
npx vitest run tests/checker/identifier-struct.test.ts
```

预期：PASS，2 个测试全通过。

- [ ] **步骤 5：验证现有测试不回归**

运行：
```bash
npm test
```

预期：所有现有测试通过（177+2 passed）。

- [ ] **步骤 6：Commit**

```bash
git add reference/src/checker/check.ts reference/src/checker/env.ts reference/tests/checker/identifier-struct.test.ts
git commit -m "feat(checker): TS identifier 查找 structs + env.functions 字段"
```

---

## 任务 3：TS checkModule.ts 参数类型解析 + resolve.ts 导出 typeExprToType

**文件：**
- 修改：`reference/src/checker/checkModule.ts:60-64`
- 修改：`reference/src/checker/resolve.ts:114` 和 `resolve.ts:117`（导出 typeExprToType）
- 创建：`reference/tests/checker/param-type.test.ts`

**背景：** 当前 `checkModule.ts:63` 把所有方法参数硬编码为 `{ kind: "primitive", name: "String" }`。导致 `order: Order` 参数类型为 String 而非 Order 结构体。需要用 `param.typeName` 解析类型。`typeExprToType` 函数已在 `resolve.ts:117` 定义但未导出。

- [ ] **步骤 1：编写失败的测试**

创建 `reference/tests/checker/param-type.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { checkModule } from "../../src/checker/checkModule.js";
import type { TangleModule } from "../../src/model.js";

describe("method param type resolution", () => {
  it("resolves param type from typeName annotation", () => {
    const module: TangleModule = {
      headings: [{
        id: 1, depth: 3, role: "type", title: "Order",
        symbolName: "Order", params: [
          { name: "id", typeName: "String", span: { file: "t", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } },
          { name: "amount", typeName: "Int", span: { file: "t", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } },
        ],
        children: [], codeBlocks: [],
      }, {
        id: 2, depth: 4, role: "callable", title: "confirm",
        symbolName: "confirm", params: [
          { name: "order", typeName: "Order", span: { file: "t", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } },
        ],
        children: [], codeBlocks: [],
      }],
      diagnostics: [],
    } as unknown as TangleModule;

    const checked = checkModule(module);
    // 不应有类型错误（Order 参数解析为 struct 类型）
    const typeErrors = checked.allDiagnostics.filter(d => d.code.startsWith("TANGLE_TYPE"));
    expect(typeErrors).toHaveLength(0);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
npx vitest run tests/checker/param-type.test.ts
```

预期：FAIL（因为 checkModule.ts 硬编码 String，导致 `order` 参数类型为 String 而非 Order）。

注意：测试结构可能需要调整以匹配实际的 TangleModule 结构。如果测试框架需要更完整的 mock，参考现有 `reference/tests/checker/` 下的测试。

- [ ] **步骤 3：导出 typeExprToType**

修改 `reference/src/checker/resolve.ts:117`，将 `function typeExprToType` 改为 `export function typeExprToType`：

```typescript
export function typeExprToType(te: TypeExpr): Type {
  switch (te.kind) {
    // ... 保持现有实现不变
  }
}
```

- [ ] **步骤 4：实现参数类型解析**

修改 `reference/src/checker/checkModule.ts:60-64`：

```typescript
    // Add method params as variables
    for (const param of heading.params ?? []) {
      if (param.typeName) {
        try {
          const te = parseTypeExpr(param.typeName, param.span.file);
          checkEnv.variables[param.name] = typeExprToType(te);
        } catch {
          checkEnv.variables[param.name] = { kind: "any" };
        }
      } else {
        checkEnv.variables[param.name] = { kind: "any" };  // 无标注用 Any
      }
    }
```

在 `checkModule.ts` 顶部添加导入：

```typescript
import { parseTypeExpr } from "../parser/typeParser.js";
import { typeExprToType } from "./resolve.js";
```

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
npx vitest run tests/checker/param-type.test.ts
```

预期：PASS。

- [ ] **步骤 6：验证现有测试不回归**

运行：
```bash
npm test
```

预期：所有测试通过。

- [ ] **步骤 7：Commit**

```bash
git add reference/src/checker/checkModule.ts reference/src/checker/resolve.ts reference/tests/checker/param-type.test.ts
git commit -m "feat(checker): TS 参数类型解析用 typeName 而非默认 String"
```

---

## 任务 4：TS builtins.ts 注册 Err/Ok + check.ts call 结构体构造器

**文件：**
- 修改：`reference/src/checker/builtins.ts`
- 修改：`reference/src/checker/check.ts:56-67`
- 修改：`reference/src/checker/resolve.ts:114`
- 创建：`reference/tests/checker/err-ok.test.ts`

**背景：** `Err`/`Ok` 构造器未注册，导致 `Undefined variable: Err`。同时 `check.ts:66` 非 function callee 返回 Bool，应改为返回 Any，且结构体构造器调用应返回结构体类型。`resolve.ts:114` 方法返回类型硬编码 Bool，应改为 Any。

- [ ] **步骤 1：编写失败的测试**

创建 `reference/tests/checker/err-ok.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { checkExpression } from "../../src/checker/check.js";
import { createEnv } from "../../src/checker/env.js";
import { registerBuiltins } from "../../src/checker/builtins.js";

describe("Err/Ok constructors", () => {
  it("resolves Err as function type", () => {
    const env = createEnv();
    registerBuiltins(env);
    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Err", span: { file: "t", startLine: 1, startColumn: 1, endLine: 1, endColumn: 3 } },
      env
    );
    expect(diags).toHaveLength(0);
    expect(type.kind).toBe("function");
  });

  it("resolves Ok as function type", () => {
    const env = createEnv();
    registerBuiltins(env);
    const [type, diags] = checkExpression(
      { kind: "identifier", name: "Ok", span: { file: "t", startLine: 1, startColumn: 1, endLine: 1, endColumn: 2 } },
      env
    );
    expect(diags).toHaveLength(0);
    expect(type.kind).toBe("function");
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
npx vitest run tests/checker/err-ok.test.ts
```

预期：FAIL，`registerBuiltins` 未导出，`Err`/`Ok` 未注册。

- [ ] **步骤 3：实现 registerBuiltins**

修改 `reference/src/checker/builtins.ts`，添加 `registerBuiltins` 函数：

```typescript
import type { TypeEnv } from "./env.js";
import type { Type } from "./types.js";

export const builtinTypes: Record<string, Type> = {
  String: { kind: "primitive", name: "String" },
  Int: { kind: "primitive", name: "Int" },
  Bool: { kind: "primitive", name: "Bool" },
};

/// 注册 Err/Ok 构造器到 env.functions
export function registerBuiltins(env: TypeEnv): void {
  env.functions["Err"] = {
    params: [
      { name: "kind", type: { kind: "primitive", name: "String" } },
      { name: "msg", type: { kind: "primitive", name: "String" } },
    ],
    returns: { kind: "any" },
    is_variadic: false,
  };
  env.functions["Ok"] = {
    params: [{ name: "value", type: { kind: "any" } }],
    returns: { kind: "any" },
    is_variadic: false,
  };
}
```

注意：如果 `builtins.ts` 已有 `builtinTypes` 定义，保留它并添加 `registerBuiltins`。

- [ ] **步骤 4：在 checkModule.ts 中调用 registerBuiltins**

修改 `reference/src/checker/checkModule.ts`，在 `checkEnv` 创建后调用 `registerBuiltins`：

```typescript
import { registerBuiltins } from "./builtins.js";
// ...
    const checkEnv = createEnv();
    checkEnv.structs = env.structs;
    checkEnv.interfaces = env.interfaces;
    registerBuiltins(checkEnv);  // ← 新增
    checkEnv.errorRegistry = errorRegistry;
```

- [ ] **步骤 5：修改 check.ts call 处理结构体构造器**

修改 `reference/src/checker/check.ts:56-67`：

```typescript
    case "call": {
      const [calleeType, calleeDiags] = checkExpression(expr.callee, env);
      diags.push(...calleeDiags);
      for (const arg of expr.args) {
        const [, argDiags] = checkExpression(arg, env);
        diags.push(...argDiags);
      }
      if (calleeType.kind === "function") {
        return [calleeType.returns, diags];
      }
      if (calleeType.kind === "struct") {
        return [calleeType, diags];  // ← 新增：结构体构造器返回结构体类型
      }
      return [{ kind: "any" }, diags];  // ← 改为 Any 而非 Bool
    }
```

- [ ] **步骤 6：修改 resolve.ts 方法返回类型**

修改 `reference/src/checker/resolve.ts:114`：

```typescript
  return { params, returns: { kind: "any" } };  // ← 改为 Any 而非 Bool
```

- [ ] **步骤 7：运行测试验证通过**

运行：
```bash
npx vitest run tests/checker/err-ok.test.ts
```

预期：PASS。

- [ ] **步骤 8：验证现有测试不回归**

运行：
```bash
npm test
npm run build
```

预期：所有测试通过，TS 编译零错误。

- [ ] **步骤 9：Commit**

```bash
git add reference/src/checker/builtins.ts reference/src/checker/check.ts reference/src/checker/checkModule.ts reference/src/checker/resolve.ts reference/tests/checker/err-ok.test.ts
git commit -m "feat(checker): TS 注册 Err/Ok + call 结构体构造器 + 返回类型改 Any"
```

---

## 任务 5：order-service 差分测试验证

**文件：**
- 创建：`reference/tests/checker/order-service.test.ts`
- 修改：`tests/audit/diff-ir.ps1`（如有需要移除 SKIPPED）

**背景：** 任务 2-4 完成后，TS 端应能正确处理 order-service.tangle。需验证差分测试从 SKIPPED → MATCH。

- [ ] **步骤 1：编写 order-service 类型检查测试**

创建 `reference/tests/checker/order-service.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { tokenize } from "../../src/parser/lexer.js";
import { parseModule } from "../../src/parser/parser.js";
import { checkModule } from "../../src/checker/checkModule.js";

describe("order-service type checking", () => {
  it("has no type errors", () => {
    const source = readFileSync(join(__dirname, "../../../tests/mvp/order-service.tangle.md"), "utf-8");
    const tokens = tokenize(source, "order-service.tangle.md");
    const module = parseModule(tokens);
    const checked = checkModule(module);
    const typeErrors = checked.allDiagnostics.filter(d => d.code.startsWith("TANGLE_TYPE"));
    expect(typeErrors).toHaveLength(0);
  });
});
```

- [ ] **步骤 2：运行测试验证通过**

运行：
```bash
cd reference
npx vitest run tests/checker/order-service.test.ts
```

预期：PASS，order-service 类型检查无错误。

- [ ] **步骤 3：构建 reference 并运行差分测试**

运行：
```bash
npm run build
cd ..
pwsh tests/audit/diff-ir.ps1
```

预期：order-service 从 SKIPPED → MATCH。总计 11 MATCH + 0 SKIPPED。

如果仍有差异，检查 IR 输出并调整 TS 端实现。

- [ ] **步骤 4：Commit**

```bash
git add reference/tests/checker/order-service.test.ts
git commit -m "test(checker): order-service 类型检查无错误 + 差分测试 MATCH"
```

---

## 任务 6：Rust types.rs 泛型构造辅助函数

**文件：**
- 修改：`compiler/tangle-cli/src/checker/types.rs`

**背景：** Rust 已有 `Type::Var(TypeVariable)` 和 `Type::GenericInstance(GenericTypeInstance)`，但缺少构造辅助函数。添加 `type_var` 和 `generic` 函数简化泛型签名构造。

- [ ] **步骤 1：编写失败的测试**

在 `compiler/tangle-cli/src/checker/types.rs` 的 `#[cfg(test)] mod tests` 块末尾添加测试：

```rust
    // --- 10. type_var and generic constructors ---

    #[test]
    fn type_var_constructor() {
        let v = type_var(0);
        assert!(matches!(v, Type::Var(TypeVariable { id: 0 })));
    }

    #[test]
    fn generic_constructor() {
        let list_int = generic("List", vec![prim("Int")]);
        match list_int {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "List");
                assert_eq!(g.args.len(), 1);
            }
            _ => panic!("Expected GenericInstance"),
        }
    }
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd .worktrees/phase6a-v0.6.0
cargo test --lib -p tangle-cli types::tests::type_var_constructor
```

预期：FAIL，`type_var` 函数未定义。

- [ ] **步骤 3：实现构造函数**

在 `compiler/tangle-cli/src/checker/types.rs` 的 `types_equal` 函数前添加：

```rust
/// 构造类型变量
pub fn type_var(id: usize) -> Type {
    Type::Var(TypeVariable { id })
}

/// 构造泛型实例
pub fn generic(base: &str, args: Vec<Type>) -> Type {
    Type::GenericInstance(GenericTypeInstance {
        base: base.to_string(),
        args,
    })
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test --lib -p tangle-cli types::tests
```

预期：PASS，所有 types 测试通过（含新增 2 个）。

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/types.rs
git commit -m "feat(types): 添加 type_var/generic 泛型构造辅助函数"
```

---

## 任务 7：Rust 新建 unify.rs 类型统一算法

**文件：**
- 创建：`compiler/tangle-cli/src/checker/unify.rs`
- 修改：`compiler/tangle-cli/src/checker/mod.rs`

**背景：** 实现 `unify` 函数（类型统一）和 `substitute` 函数（类型变量替换）。这是局部泛型推导的核心算法。

- [ ] **步骤 1：编写失败的测试**

创建 `compiler/tangle-cli/tests/v06_phase6/unify_test.rs`（临时测试文件，正式测试在任务 10）：

```rust
use tangle_cli::checker::types::*;
use tangle_cli::checker::unify::*;

#[test]
fn unify_binds_type_variable() {
    let mut subst = Substitution::new();
    let result = unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst);
    assert!(result.is_ok());
    assert_eq!(subst.get(&0), Some(&Type::Primitive(PrimitiveType { name: "Int".into() })));
}
```

注意：需要确保 `tangle_cli` 库导出 `checker` 模块。如果 `checker` 模块未导出，需要在 `lib.rs` 中添加 `pub mod checker;` 或使用 `pub use`。

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --test unify_test
```

预期：FAIL，`unify` 模块未定义。

- [ ] **步骤 3：实现 unify.rs**

创建 `compiler/tangle-cli/src/checker/unify.rs`：

```rust
use std::collections::HashMap;
use crate::checker::types::*;

/// 类型变量替换表：TypeVarId → 实际类型
pub type Substitution = HashMap<usize, Type>;

/// 统一 expected 类型与 actual 类型，更新 subst。
/// 成功：类型匹配（或类型变量被绑定）；失败：返回冲突描述。
pub fn unify(expected: &Type, actual: &Type, subst: &mut Substitution) -> Result<(), String> {
    match (expected, actual) {
        // Any 总是成功（双向）
        (Type::Any, _) | (_, Type::Any) => Ok(()),

        // 类型变量（expected 侧）：绑定或递归检查
        (Type::Var(v), actual) => {
            if let Some(existing) = subst.get(&v.id) {
                unify(existing, actual, subst)
            } else {
                subst.insert(v.id, actual.clone());
                Ok(())
            }
        }
        // 类型变量（actual 侧）：对称处理
        (expected, Type::Var(v)) => {
            if let Some(existing) = subst.get(&v.id) {
                unify(expected, existing, subst)
            } else {
                subst.insert(v.id, expected.clone());
                Ok(())
            }
        }

        // 泛型实例：base 必须相同，递归统一参数
        (Type::GenericInstance(a), Type::GenericInstance(b)) => {
            if a.base != b.base {
                return Err(format!("Expected {}, got {}", a.base, b.base));
            }
            if a.args.len() != b.args.len() {
                return Err("Generic arity mismatch".into());
            }
            for (e, a) in a.args.iter().zip(&b.args) {
                unify(e, a, subst)?;
            }
            Ok(())
        }

        // 基本类型：名称匹配
        (Type::Primitive(a), Type::Primitive(b)) => {
            if a.name == b.name { Ok(()) } else { Err(format!("Expected {}, got {}", a.name, b.name)) }
        }

        // 结构体：名称匹配
        (Type::Struct(a), Type::Struct(b)) => {
            if a.name == b.name { Ok(()) } else { Err(format!("Expected {}, got {}", a.name, b.name)) }
        }

        // 函数类型：参数和返回类型递归统一
        (Type::Function(a), Type::Function(b)) => {
            if a.params.len() != b.params.len() {
                return Err("Function arity mismatch".into());
            }
            for (e, a) in a.params.iter().zip(&b.params) {
                unify(e, a, subst)?;
            }
            unify(&a.returns, &b.returns, subst)
        }

        _ => Err(format!("Type mismatch: {:?} vs {:?}", expected, actual)),
    }
}

/// 用 subst 替换类型中的 TypeVariable（递归）
pub fn substitute(ty: &Type, subst: &Substitution) -> Type {
    match ty {
        Type::Var(v) => subst.get(&v.id).cloned().unwrap_or_else(|| ty.clone()),
        Type::GenericInstance(g) => Type::GenericInstance(GenericTypeInstance {
            base: g.base.clone(),
            args: g.args.iter().map(|a| substitute(a, subst)).collect(),
        }),
        Type::Function(f) => Type::Function(FunctionType {
            params: f.params.iter().map(|p| substitute(p, subst)).collect(),
            returns: Box::new(substitute(&f.returns, subst)),
            is_variadic: f.is_variadic,
        }),
        _ => ty.clone(),
    }
}
```

- [ ] **步骤 4：注册 unify 模块**

修改 `compiler/tangle-cli/src/checker/mod.rs`，添加：

```rust
pub mod unify;
```

确保 `checker` 模块在 `lib.rs` 中是 `pub` 的。如果 `lib.rs` 中是 `mod checker;`，改为 `pub mod checker;`。

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
cargo test --test unify_test
```

预期：PASS。

- [ ] **步骤 6：验证现有测试不回归**

运行：
```bash
cargo test --workspace
```

预期：所有测试通过。

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/checker/unify.rs compiler/tangle-cli/src/checker/mod.rs compiler/tangle-cli/src/lib.rs compiler/tangle-cli/tests/v06_phase6/unify_test.rs
git commit -m "feat(checker): 新建 unify.rs 类型统一算法"
```

---

## 任务 8：Rust check.rs call 加入泛型推导

**文件：**
- 修改：`compiler/tangle-cli/src/checker/check.rs:77-124`

**背景：** 修改 `Expr::Call` 分支，用 `unify` 替代 `types_equal`，用 `substitute` 替换返回类型。

- [ ] **步骤 1：编写失败的测试**

在 `compiler/tangle-cli/tests/v06_phase6/unify_test.rs` 中添加：

```rust
use tangle_cli::run_collecting_ir;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests").join(name)
}

#[test]
fn list_map_infers_return_type() {
    // 这个测试验证 List.map 调用后返回类型被正确推导
    // 需要 stdlib 签名已改为泛型（任务 9 完成后）
    // 这里先写测试，任务 9 完成后运行
    let path = fixture_path("v06_phase6/generics.tangle.md");
    if !path.exists() {
        return; // fixture 尚未创建，跳过
    }
    let (graph, _diags) = run_collecting_ir(&path);
    // 验证 IR 生成无错误
    assert!(graph.functions.len() >= 1 || graph.nodes.len() >= 1);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --test unify_test list_map_infers_return_type
```

预期：测试跳过（fixture 未创建）或 FAIL（如果 check.rs 未改）。

- [ ] **步骤 3：实现 call 泛型推导**

修改 `compiler/tangle-cli/src/checker/check.rs:77-124` 的 `Expr::Call` 分支：

在文件顶部添加导入：
```rust
use crate::checker::unify::{unify, substitute, Substitution};
```

修改 `Expr::Call` 分支（替换第 109-118 行的参数检查和返回）：

```rust
        Expr::Call(e) => {
            let (callee_ty, mut callee_diags) = check_expression(&e.callee, env);
            diags.append(&mut callee_diags);
            let arg_types: Vec<Type> = e.args.iter().map(|arg| {
                let (ty, mut d) = check_expression(arg, env);
                diags.append(&mut d);
                ty
            }).collect();

            match &callee_ty {
                Type::Function(sig) => {
                    let expected = sig.params.len();
                    let actual = arg_types.len();
                    if sig.is_variadic {
                        if actual < expected.saturating_sub(1) {
                            diags.push(TangleDiagnostic {
                                code: "TANGLE_ARITY_MISMATCH".into(),
                                message: format!(
                                    "Expected at least {} args, got {}",
                                    expected.saturating_sub(1),
                                    actual
                                ),
                                span: e.span.clone(),
                            });
                        }
                    } else if actual != expected {
                        diags.push(TangleDiagnostic {
                            code: "TANGLE_ARITY_MISMATCH".into(),
                            message: format!("Expected {} args, got {}", expected, actual),
                            span: e.span.clone(),
                        });
                    }

                    // 泛型推导：统一参数类型
                    let mut subst: Substitution = HashMap::new();
                    for (i, (arg_ty, param_ty)) in arg_types.iter().zip(&sig.params).enumerate() {
                        if let Err(msg) = unify(param_ty, arg_ty, &mut subst) {
                            diags.push(TangleDiagnostic {
                                code: "TANGLE_TYPE_ERROR".into(),
                                message: format!("Arg {} type mismatch: {}", i + 1, msg),
                                span: e.span.clone(),
                            });
                        }
                    }
                    // 用 subst 替换返回类型
                    substitute(&sig.returns, &subst)
                }
                _ => Type::Any,
            }
        }
```

注意：需要 `use std::collections::HashMap;` 导入。

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test --workspace
```

预期：所有测试通过（现有测试不应回归，因为非泛型签名的 unify 行为与 types_equal 一致）。

- [ ] **步骤 5：Commit**

```bash
git add compiler/tangle-cli/src/checker/check.rs compiler/tangle-cli/tests/v06_phase6/unify_test.rs
git commit -m "feat(checker): call 表达式加入泛型推导 (unify + substitute)"
```

---

## 任务 9：Rust signatures.rs stdlib 泛型签名

**文件：**
- 修改：`compiler/tangle-cli/src/stdlib/signatures.rs`

**背景：** 将 List/Map/Option/Set 模块的签名从 `any_t()` 改为带类型变量的泛型签名。保留 Math/String/IO/Time 等非容器模块不变。

- [ ] **步骤 1：编写失败的测试**

在 `compiler/tangle-cli/tests/v06_phase6/unify_test.rs` 中添加：

```rust
use tangle_cli::stdlib::signatures::stdlib_module_signatures;

#[test]
fn list_map_has_generic_signature() {
    let list_mod = stdlib_module_signatures("List").expect("List module exists");
    let map_sig = list_mod.get("map").expect("map function exists");
    // 返回类型应为泛型 List<U>（含类型变量）
    match &map_sig.returns {
        tangle_cli::checker::types::Type::GenericInstance(g) => {
            assert_eq!(g.base, "List");
            assert_eq!(g.args.len(), 1);
        }
        _ => panic!("Expected GenericInstance for List.map return type, got {:?}", map_sig.returns),
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test --test unify_test list_map_has_generic_signature
```

预期：FAIL，当前 `List.map` 返回 `Any`。

- [ ] **步骤 3：实现泛型签名**

修改 `compiler/tangle-cli/src/stdlib/signatures.rs`：

在文件顶部添加导入：
```rust
use crate::checker::types::{type_var, generic, Type, FunctionType, PrimitiveType};
```

替换 List 模块（第 70-76 行）：

```rust
        m.insert("List", module(&[
            ("length", sig_fixed(&[("list", generic("List", vec![type_var(0)]))], int_t())),
            ("map", sig_generic(&["T", "U"],
                &[("list", generic("List", vec![type_var(0)])),
                  ("fn", Type::Function(FunctionType {
                      params: vec![type_var(0)],
                      returns: Box::new(type_var(1)),
                      is_variadic: false,
                  }))],
                generic("List", vec![type_var(1)]))),
            ("filter", sig_generic(&["T"],
                &[("list", generic("List", vec![type_var(0)])),
                  ("fn", Type::Function(FunctionType {
                      params: vec![type_var(0)],
                      returns: Box::new(bool_t()),
                      is_variadic: false,
                  }))],
                generic("List", vec![type_var(0)]))),
            ("push", sig_generic(&["T"],
                &[("list", generic("List", vec![type_var(0)])), ("item", type_var(0))],
                generic("List", vec![type_var(0)]))),
            ("get", sig_generic(&["T"],
                &[("list", generic("List", vec![type_var(0)])), ("index", int_t())],
                type_var(0))),
        ]));
```

替换 Map 模块（第 78-85 行）：

```rust
        m.insert("Map", module(&[
            ("get", sig_generic(&["K", "V"],
                &[("map", generic("Map", vec![type_var(0), type_var(1)])), ("key", type_var(0))],
                type_var(1))),
            ("set", sig_generic(&["K", "V"],
                &[("map", generic("Map", vec![type_var(0), type_var(1)])),
                  ("key", type_var(0)), ("value", type_var(1))],
                generic("Map", vec![type_var(0), type_var(1)]))),
            ("has", sig_generic(&["K", "V"],
                &[("map", generic("Map", vec![type_var(0), type_var(1)])), ("key", type_var(0))],
                bool_t())),
            ("keys", sig_generic(&["K", "V"],
                &[("map", generic("Map", vec![type_var(0), type_var(1)]))],
                generic("List", vec![type_var(0)]))),
            ("values", sig_generic(&["K", "V"],
                &[("map", generic("Map", vec![type_var(0), type_var(1)]))],
                generic("List", vec![type_var(1)]))),
            ("delete", sig_generic(&["K", "V"],
                &[("map", generic("Map", vec![type_var(0), type_var(1)])), ("key", type_var(0))],
                generic("Map", vec![type_var(0), type_var(1)]))),
        ]));
```

替换 Option 模块（第 98-106 行）：

```rust
        m.insert("Option", module(&[
            ("Some", sig_generic(&["T"], &[("value", type_var(0))], generic("Option", vec![type_var(0)]))),
            ("None", sig_generic(&["T"], &[], generic("Option", vec![type_var(0)]))),
            ("unwrap", sig_generic(&["T"], &[("opt", generic("Option", vec![type_var(0)]))], type_var(0))),
            ("is_some", sig_generic(&["T"], &[("opt", generic("Option", vec![type_var(0)]))], bool_t())),
            ("is_none", sig_generic(&["T"], &[("opt", generic("Option", vec![type_var(0)]))], bool_t())),
            ("map", sig_generic(&["T", "U"],
                &[("opt", generic("Option", vec![type_var(0)])),
                  ("fn", Type::Function(FunctionType {
                      params: vec![type_var(0)],
                      returns: Box::new(type_var(1)),
                      is_variadic: false,
                  }))],
                generic("Option", vec![type_var(1)]))),
            ("or_else", sig_generic(&["T"],
                &[("opt", generic("Option", vec![type_var(0)])),
                  ("fn", Type::Function(FunctionType {
                      params: vec![],
                      returns: Box::new(generic("Option", vec![type_var(0)])),
                      is_variadic: false,
                  }))],
                generic("Option", vec![type_var(0)]))),
        ]));
```

替换 Set 模块（第 87-96 行）：

```rust
        m.insert("Set", module(&[
            ("add", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)])), ("value", type_var(0))],
                generic("Set", vec![type_var(0)]))),
            ("remove", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)])), ("value", type_var(0))],
                generic("Set", vec![type_var(0)]))),
            ("contains", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)])), ("value", type_var(0))],
                bool_t())),
            ("size", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)]))],
                int_t())),
            ("union", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)])), ("other", generic("Set", vec![type_var(0)]))],
                generic("Set", vec![type_var(0)]))),
            ("intersection", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)])), ("other", generic("Set", vec![type_var(0)]))],
                generic("Set", vec![type_var(0)]))),
            ("difference", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)])), ("other", generic("Set", vec![type_var(0)]))],
                generic("Set", vec![type_var(0)]))),
            ("to_list", sig_generic(&["T"],
                &[("set", generic("Set", vec![type_var(0)]))],
                generic("List", vec![type_var(0)]))),
        ]));
```

在文件末尾添加 `sig_generic` 函数（`sig_fixed` 旁边）：

```rust
/// 构造泛型函数签名（type_var id 从 0 开始，对应 type_params 顺序）
fn sig_generic(
    _type_params: &[&str],   // 仅用于文档目的，实际用 id 索引
    params: &[(&str, Type)],
    returns: Type,
) -> CallableSignature {
    CallableSignature {
        params: params.iter().map(|(n, t)| (n.to_string(), t.clone())).collect(),
        returns: Box::new(returns),
        is_variadic: false,
    }
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test --test unify_test list_map_has_generic_signature
```

预期：PASS。

- [ ] **步骤 5：验证现有测试不回归**

运行：
```bash
cargo test --workspace
```

预期：所有测试通过。如果有测试因泛型签名改变而失败，检查是否是合理的类型检查行为变化。

- [ ] **步骤 6：Commit**

```bash
git add compiler/tangle-cli/src/stdlib/signatures.rs
git commit -m "feat(stdlib): List/Map/Option/Set 签名改用类型变量"
```

---

## 任务 10：Rust 泛型推导测试 + fixture

**文件：**
- 创建：`compiler/tangle-cli/tests/v06_phase6/generics_inference.rs`
- 创建：`tests/v06_phase6/generics.tangle.md`
- 修改：`compiler/tangle-cli/Cargo.toml`
- 删除：`compiler/tangle-cli/tests/v06_phase6/unify_test.rs`（临时文件）

**背景：** 创建正式的泛型推导测试和 fixture，注册到 Cargo.toml。

- [ ] **步骤 1：在 Cargo.toml 注册测试**

修改 `compiler/tangle-cli/Cargo.toml`，在 `[[test]]` 列表末尾追加：

```toml
[[test]]
name = "phase6_generics_inference"
path = "tests/v06_phase6/generics_inference.rs"
```

- [ ] **步骤 2：创建泛型 fixture**

创建 `tests/v06_phase6/generics.tangle.md`：

```markdown
# Generic Type Inference Test

### main

```@tangle
let numbers = [1, 2, 3]
let doubled = List.map(numbers, fn(x) { x * 2 })
return doubled
```
```

- [ ] **步骤 3：编写正式测试**

创建 `compiler/tangle-cli/tests/v06_phase6/generics_inference.rs`：

```rust
use tangle_cli::checker::types::*;
use tangle_cli::checker::unify::*;
use tangle_cli::stdlib::signatures::stdlib_module_signatures;

// === unify 算法测试 ===

#[test]
fn unify_binds_type_variable() {
    let mut subst = Substitution::new();
    let result = unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst);
    assert!(result.is_ok());
    assert_eq!(subst.get(&0), Some(&Type::Primitive(PrimitiveType { name: "Int".into() })));
}

#[test]
fn unify_type_variable_consistent() {
    let mut subst = Substitution::new();
    unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst).unwrap();
    // 再次统一相同类型应成功
    let result = unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst);
    assert!(result.is_ok());
}

#[test]
fn unify_type_variable_conflict() {
    let mut subst = Substitution::new();
    unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst).unwrap();
    // 统一不同类型应失败
    let result = unify(&type_var(0), &Type::Primitive(PrimitiveType { name: "String".into() }), &mut subst);
    assert!(result.is_err());
}

#[test]
fn unify_nested_generic() {
    let mut subst = Substitution::new();
    let expected = generic("List", vec![type_var(0)]);
    let actual = generic("List", vec![Type::Primitive(PrimitiveType { name: "Int".into() })]);
    let result = unify(&expected, &actual, &mut subst);
    assert!(result.is_ok());
    assert_eq!(subst.get(&0), Some(&Type::Primitive(PrimitiveType { name: "Int".into() })));
}

#[test]
fn unify_function_type() {
    let mut subst = Substitution::new();
    let expected = Type::Function(FunctionType {
        params: vec![type_var(0)],
        returns: Box::new(type_var(1)),
        is_variadic: false,
    });
    let actual = Type::Function(FunctionType {
        params: vec![Type::Primitive(PrimitiveType { name: "Int".into() })],
        returns: Box::new(Type::Primitive(PrimitiveType { name: "String".into() })),
        is_variadic: false,
    });
    let result = unify(&expected, &actual, &mut subst);
    assert!(result.is_ok());
    assert_eq!(subst.get(&0), Some(&Type::Primitive(PrimitiveType { name: "Int".into() })));
    assert_eq!(subst.get(&1), Some(&Type::Primitive(PrimitiveType { name: "String".into() })));
}

#[test]
fn unify_any_always_succeeds() {
    let mut subst = Substitution::new();
    let result = unify(&Type::Any, &Type::Primitive(PrimitiveType { name: "Int".into() }), &mut subst);
    assert!(result.is_ok());
    assert!(subst.is_empty()); // Any 不绑定变量
}

// === substitute 测试 ===

#[test]
fn substitute_replaces_type_variable() {
    let mut subst = Substitution::new();
    subst.insert(0, Type::Primitive(PrimitiveType { name: "Int".into() }));
    let ty = type_var(0);
    let result = substitute(&ty, &subst);
    assert_eq!(result, Type::Primitive(PrimitiveType { name: "Int".into() }));
}

#[test]
fn substitute_recursive_generic() {
    let mut subst = Substitution::new();
    subst.insert(0, Type::Primitive(PrimitiveType { name: "Int".into() }));
    let ty = generic("List", vec![type_var(0)]);
    let result = substitute(&ty, &subst);
    match result {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List");
            assert_eq!(g.args[0], Type::Primitive(PrimitiveType { name: "Int".into() }));
        }
        _ => panic!("Expected GenericInstance"),
    }
}

// === stdlib 泛型签名测试 ===

#[test]
fn list_map_returns_generic_list() {
    let list_mod = stdlib_module_signatures("List").expect("List module exists");
    let map_sig = list_mod.get("map").expect("map function exists");
    match &map_sig.returns {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List");
            assert_eq!(g.args.len(), 1);
        }
        _ => panic!("Expected GenericInstance for List.map return type"),
    }
}

#[test]
fn map_get_returns_type_variable() {
    let map_mod = stdlib_module_signatures("Map").expect("Map module exists");
    let get_sig = map_mod.get("get").expect("get function exists");
    // 返回类型应为 type_var(1)（V）
    assert!(matches!(get_sig.returns, Type::Var(_)));
}

#[test]
fn option_some_returns_generic_option() {
    let opt_mod = stdlib_module_signatures("Option").expect("Option module exists");
    let some_sig = opt_mod.get("Some").expect("Some function exists");
    match &some_sig.returns {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "Option");
            assert_eq!(g.args.len(), 1);
        }
        _ => panic!("Expected GenericInstance for Option.Some return type"),
    }
}
```

- [ ] **步骤 4：删除临时测试文件**

```bash
git rm compiler/tangle-cli/tests/v06_phase6/unify_test.rs
```

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
cargo test --test phase6_generics_inference
```

预期：PASS，所有测试通过。

- [ ] **步骤 6：验证现有测试不回归**

运行：
```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

预期：所有测试通过，Clippy 零警告。

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/tests/v06_phase6/generics_inference.rs compiler/tangle-cli/Cargo.toml tests/v06_phase6/generics.tangle.md
git commit -m "test(phase6): Rust 泛型推导正式测试 + generics fixture"
```

---

## 任务 11：TS 端同步实现泛型推导

**文件：**
- 修改：`reference/src/checker/types.ts`（添加构造函数）
- 创建：`reference/src/checker/unify.ts`
- 修改：`reference/src/checker/check.ts`（call 加入泛型推导）
- 修改：`reference/src/checker/builtins.ts`（泛型签名）

**背景：** 将 Rust 端的泛型推导忠实移植到 TS 端。

- [ ] **步骤 1：TS types.ts 添加构造函数**

修改 `reference/src/checker/types.ts`，在文件末尾添加：

```typescript
/// 构造类型变量
export function typeVar(id: number): Type {
  return { kind: "var", id };
}

/// 构造泛型实例
export function generic(base: string, args: Type[]): Type {
  return { kind: "genericInstance", base, args };
}
```

- [ ] **步骤 2：创建 TS unify.ts**

创建 `reference/src/checker/unify.ts`：

```typescript
import type { Type } from "./types.js";

/// 类型变量替换表：TypeVarId → 实际类型
export type Substitution = Map<number, Type>;

/// 统一 expected 类型与 actual 类型，更新 subst。
export function unify(expected: Type, actual: Type, subst: Substitution): string | null {
  // Any 总是成功（双向）
  if (expected.kind === "any" || actual.kind === "any") return null;

  // 类型变量（expected 侧）
  if (expected.kind === "var") {
    const existing = subst.get(expected.id);
    if (existing) {
      return unify(existing, actual, subst);
    }
    subst.set(expected.id, actual);
    return null;
  }
  // 类型变量（actual 侧）
  if (actual.kind === "var") {
    const existing = subst.get(actual.id);
    if (existing) {
      return unify(expected, existing, subst);
    }
    subst.set(actual.id, expected);
    return null;
  }

  // 泛型实例
  if (expected.kind === "genericInstance" && actual.kind === "genericInstance") {
    if (expected.base !== actual.base) {
      return `Expected ${expected.base}, got ${actual.base}`;
    }
    if (expected.args.length !== actual.args.length) {
      return "Generic arity mismatch";
    }
    for (let i = 0; i < expected.args.length; i++) {
      const err = unify(expected.args[i]!, actual.args[i]!, subst);
      if (err) return err;
    }
    return null;
  }

  // 基本类型
  if (expected.kind === "primitive" && actual.kind === "primitive") {
    return expected.name === actual.name ? null : `Expected ${expected.name}, got ${actual.name}`;
  }

  // 结构体
  if (expected.kind === "struct" && actual.kind === "struct") {
    return expected.name === actual.name ? null : `Expected ${expected.name}, got ${actual.name}`;
  }

  // 函数类型
  if (expected.kind === "function" && actual.kind === "function") {
    if (expected.params.length !== actual.params.length) {
      return "Function arity mismatch";
    }
    for (let i = 0; i < expected.params.length; i++) {
      const err = unify(expected.params[i]!, actual.params[i]!, subst);
      if (err) return err;
    }
    return unify(expected.returns, actual.returns, subst);
  }

  return `Type mismatch: ${expected.kind} vs ${actual.kind}`;
}

/// 用 subst 替换类型中的 TypeVariable（递归）
export function substitute(ty: Type, subst: Substitution): Type {
  switch (ty.kind) {
    case "var":
      return subst.get(ty.id) ?? ty;
    case "genericInstance":
      return {
        kind: "genericInstance",
        base: ty.base,
        args: ty.args.map(a => substitute(a, subst)),
      };
    case "function":
      return {
        kind: "function",
        params: ty.params.map(p => substitute(p, subst)),
        returns: substitute(ty.returns, subst),
      };
    default:
      return ty;
  }
}
```

- [ ] **步骤 3：TS check.ts call 加入泛型推导**

修改 `reference/src/checker/check.ts:56-67`，替换 `call` case：

在文件顶部添加导入：
```typescript
import { unify, substitute, type Substitution } from "./unify.js";
```

替换 `call` case：

```typescript
    case "call": {
      const [calleeType, calleeDiags] = checkExpression(expr.callee, env);
      diags.push(...calleeDiags);
      const argTypes: Type[] = [];
      for (const arg of expr.args) {
        const [argType, argDiags] = checkExpression(arg, env);
        diags.push(...argDiags);
        argTypes.push(argType);
      }
      if (calleeType.kind === "function") {
        // 泛型推导
        const subst: Substitution = new Map();
        for (let i = 0; i < calleeType.params.length && i < argTypes.length; i++) {
          const err = unify(calleeType.params[i]!, argTypes[i]!, subst);
          if (err) {
            diags.push({ code: "TANGLE_TYPE_ERROR", message: `Arg ${i + 1} type mismatch: ${err}`, span: expr.span });
          }
        }
        return [substitute(calleeType.returns, subst), diags];
      }
      if (calleeType.kind === "struct") {
        return [calleeType, diags];
      }
      return [{ kind: "any" }, diags];
    }
```

- [ ] **步骤 4：TS builtins.ts 添加泛型签名**

修改 `reference/src/checker/builtins.ts`，在 `registerBuiltins` 中添加 stdlib 泛型签名：

```typescript
import { typeVar, generic } from "./types.js";
import type { Type, CallableSignature } from "./types.js";

// stdlib 泛型签名
export const stdlibGenericSignatures: Record<string, Record<string, CallableSignature>> = {
  List: {
    length: { params: [{ name: "list", type: generic("List", [typeVar(0)]) }], returns: { kind: "primitive", name: "Int" }, is_variadic: false },
    map: {
      params: [
        { name: "list", type: generic("List", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [typeVar(0)], returns: typeVar(1) } },
      ],
      returns: generic("List", [typeVar(1)]),
      is_variadic: false,
    },
    filter: {
      params: [
        { name: "list", type: generic("List", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [typeVar(0)], returns: { kind: "primitive", name: "Bool" } } },
      ],
      returns: generic("List", [typeVar(0)]),
      is_variadic: false,
    },
    push: {
      params: [{ name: "list", type: generic("List", [typeVar(0)]) }, { name: "item", type: typeVar(0) }],
      returns: generic("List", [typeVar(0)]),
      is_variadic: false,
    },
    get: {
      params: [{ name: "list", type: generic("List", [typeVar(0)]) }, { name: "index", type: { kind: "primitive", name: "Int" } }],
      returns: typeVar(0),
      is_variadic: false,
    },
  },
  Map: {
    get: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }, { name: "key", type: typeVar(0) }],
      returns: typeVar(1),
      is_variadic: false,
    },
    set: {
      params: [
        { name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) },
        { name: "key", type: typeVar(0) },
        { name: "value", type: typeVar(1) },
      ],
      returns: generic("Map", [typeVar(0), typeVar(1)]),
      is_variadic: false,
    },
    has: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }, { name: "key", type: typeVar(0) }],
      returns: { kind: "primitive", name: "Bool" },
      is_variadic: false,
    },
    keys: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }],
      returns: generic("List", [typeVar(0)]),
      is_variadic: false,
    },
    values: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }],
      returns: generic("List", [typeVar(1)]),
      is_variadic: false,
    },
    delete: {
      params: [{ name: "map", type: generic("Map", [typeVar(0), typeVar(1)]) }, { name: "key", type: typeVar(0) }],
      returns: generic("Map", [typeVar(0), typeVar(1)]),
      is_variadic: false,
    },
  },
  Option: {
    Some: { params: [{ name: "value", type: typeVar(0) }], returns: generic("Option", [typeVar(0)]), is_variadic: false },
    None: { params: [], returns: generic("Option", [typeVar(0)]), is_variadic: false },
    unwrap: { params: [{ name: "opt", type: generic("Option", [typeVar(0)]) }], returns: typeVar(0), is_variadic: false },
    is_some: { params: [{ name: "opt", type: generic("Option", [typeVar(0)]) }], returns: { kind: "primitive", name: "Bool" }, is_variadic: false },
    is_none: { params: [{ name: "opt", type: generic("Option", [typeVar(0)]) }], returns: { kind: "primitive", name: "Bool" }, is_variadic: false },
    map: {
      params: [
        { name: "opt", type: generic("Option", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [typeVar(0)], returns: typeVar(1) } },
      ],
      returns: generic("Option", [typeVar(1)]),
      is_variadic: false,
    },
    or_else: {
      params: [
        { name: "opt", type: generic("Option", [typeVar(0)]) },
        { name: "fn", type: { kind: "function", params: [], returns: generic("Option", [typeVar(0)]) } },
      ],
      returns: generic("Option", [typeVar(0)]),
      is_variadic: false,
    },
  },
};
```

注意：TS 端 `FunctionType_` 没有 `is_variadic` 字段，但 `CallableSignature` 需要。确保 `CallableSignature` 类型定义包含 `is_variadic`。如果不包含，添加它。

- [ ] **步骤 5：运行 TS 编译验证**

运行：
```bash
cd reference
npm run build
```

预期：编译成功，零类型错误。

- [ ] **步骤 6：验证现有测试不回归**

运行：
```bash
npm test
```

预期：所有测试通过。

- [ ] **步骤 7：Commit**

```bash
git add reference/src/checker/types.ts reference/src/checker/unify.ts reference/src/checker/check.ts reference/src/checker/builtins.ts
git commit -m "feat(checker): TS 端同步实现泛型推导 (unify + substitute + stdlib 签名)"
```

---

## 任务 12：TS 泛型测试 + 差分测试验证

**文件：**
- 创建：`reference/tests/checker/generics.test.ts`

**背景：** 移植 Rust 泛型测试到 TS 端，验证差分测试 generics fixture MATCH。

- [ ] **步骤 1：编写 TS 泛型测试**

创建 `reference/tests/checker/generics.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { unify, substitute, type Substitution } from "../../src/checker/unify.js";
import { typeVar, generic } from "../../src/checker/types.js";
import type { Type } from "../../src/checker/types.js";

const intType: Type = { kind: "primitive", name: "Int" };
const strType: Type = { kind: "primitive", name: "String" };

describe("unify algorithm", () => {
  it("binds type variable", () => {
    const subst: Substitution = new Map();
    const err = unify(typeVar(0), intType, subst);
    expect(err).toBeNull();
    expect(subst.get(0)).toEqual(intType);
  });

  it("consistent type variable", () => {
    const subst: Substitution = new Map();
    unify(typeVar(0), intType, subst);
    const err = unify(typeVar(0), intType, subst);
    expect(err).toBeNull();
  });

  it("conflicting type variable", () => {
    const subst: Substitution = new Map();
    unify(typeVar(0), intType, subst);
    const err = unify(typeVar(0), strType, subst);
    expect(err).not.toBeNull();
  });

  it("nested generic", () => {
    const subst: Substitution = new Map();
    const err = unify(generic("List", [typeVar(0)]), generic("List", [intType]), subst);
    expect(err).toBeNull();
    expect(subst.get(0)).toEqual(intType);
  });

  it("function type", () => {
    const subst: Substitution = new Map();
    const expected: Type = { kind: "function", params: [typeVar(0)], returns: typeVar(1) };
    const actual: Type = { kind: "function", params: [intType], returns: strType };
    const err = unify(expected, actual, subst);
    expect(err).toBeNull();
    expect(subst.get(0)).toEqual(intType);
    expect(subst.get(1)).toEqual(strType);
  });

  it("any always succeeds", () => {
    const subst: Substitution = new Map();
    const err = unify({ kind: "any" }, intType, subst);
    expect(err).toBeNull();
    expect(subst.size).toBe(0);
  });
});

describe("substitute", () => {
  it("replaces type variable", () => {
    const subst: Substitution = new Map([[0, intType]]);
    expect(substitute(typeVar(0), subst)).toEqual(intType);
  });

  it("recursive generic", () => {
    const subst: Substitution = new Map([[0, intType]]);
    const result = substitute(generic("List", [typeVar(0)]), subst);
    expect(result).toEqual(generic("List", [intType]));
  });
});
```

- [ ] **步骤 2：运行测试验证通过**

运行：
```bash
cd reference
npx vitest run tests/checker/generics.test.ts
```

预期：PASS。

- [ ] **步骤 3：运行差分测试**

运行：
```bash
npm run build
cd ..
pwsh tests/audit/diff-ir.ps1
```

预期：11 MATCH + 0 SKIPPED（含 order-service 和 generics fixture）。

如果 generics fixture 不在差分测试列表中，需要将其添加到 `diff-ir.ps1` 的 fixture 列表。

- [ ] **步骤 4：验证全部测试**

运行：
```bash
cargo test --workspace
cd reference && npm test && cd ..
```

预期：Rust + TS 所有测试通过。

- [ ] **步骤 5：Commit**

```bash
git add reference/tests/checker/generics.test.ts
git commit -m "test(checker): TS 泛型推导测试 + 差分测试验证"
```

---

## 任务 13：出口闸门验证 + merge + tag v0.6.0

**文件：** 无文件修改，仅验证和 git 操作

- [ ] **步骤 1：运行完整测试套件**

运行：
```bash
cd .worktrees/phase6a-v0.6.0
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd reference && npm test && npm run build && cd ..
pwsh tests/audit/run-audit.ps1
pwsh tests/audit/diff-ir.ps1
```

预期：
- cargo test：所有测试通过
- cargo clippy：零警告
- npm test：所有测试通过
- npm run build：零类型错误
- run-audit：0 failing
- diff-ir：11 MATCH + 0 SKIPPED

- [ ] **步骤 2：验证 Phase 4/5 回归测试**

运行：
```bash
cargo test --test phase4_*
cargo test --test phase5_*
```

预期：所有 Phase 4/5 测试通过。

- [ ] **步骤 3：验证 Phase 6 新测试**

运行：
```bash
cargo test --test phase6_generics_inference
```

预期：所有 Phase 6 测试通过。

- [ ] **步骤 4：合并到 main 分支**

运行：
```bash
cd e:\GitProjects\tangle
git checkout main
git merge --no-ff phase6a/v0.6.0 -m "Merge branch 'phase6a/v0.6.0': 类型系统对齐 + 局部泛型推导

Phase 6a 完成：
- 工作流 1: TS 端类型检查器对齐 Rust（闭合 order-service）
- 工作流 2: Rust/TS 双端局部泛型推导（List/Map/Option/Set）

成果：diff-ir.ps1 从 10 MATCH + 1 SKIPPED 变为 11 MATCH + 0 SKIPPED"
```

- [ ] **步骤 5：创建 tag v0.6.0**

运行：
```bash
git tag -a v0.6.0 -m "v0.6.0: 类型系统对齐 + 局部泛型推导

Phase 6a:
- TS 端类型检查器对齐 Rust（identifier 查找 structs + 参数类型解析 + Err/Ok 注册）
- Rust/TS 双端局部泛型推导（unify 算法 + stdlib 泛型签名）
- 差分测试：11 MATCH + 0 SKIPPED"
```

- [ ] **步骤 6：清理 worktree**

运行：
```bash
git worktree remove .worktrees/phase6a-v0.6.0 --force
git worktree prune
git branch -d phase6a/v0.6.0
```

- [ ] **步骤 7：最终验证**

运行：
```bash
git log --oneline -5
git tag -l "v0.6*"
```

预期：main 分支含 Phase 6a 所有 commit，tag v0.6.0 已创建。

**注意：** tag v0.6.0 不 push remote，直到用户批准。

---

## 自检

### 1. 规格覆盖度

| 规格章节 | 对应任务 |
|---------|---------|
| §3.1 identifier 查找 structs | 任务 2 |
| §3.2 参数类型解析 | 任务 3 |
| §3.3 注册 Err/Ok | 任务 4 |
| §3.4 call 结构体构造器 | 任务 4 |
| §3.5 resolve.ts 返回类型改 Any | 任务 4 |
| §3.6 propagation（不改） | 无需任务 |
| §4.1 types.rs 构造函数 | 任务 6 |
| §4.2 unify.rs 算法 | 任务 7 |
| §4.3 check.rs call 泛型推导 | 任务 8 |
| §4.4 signatures.rs 泛型签名 | 任务 9 |
| §4.5 TS 端同步实现 | 任务 11 |
| §4.6 新 fixture | 任务 10 |
| §6 测试策略 | 任务 5, 10, 12 |
| §7-8 成功标准/出口闸门 | 任务 13 |

**遗漏：** 无。所有规格需求都有对应任务。

### 2. 占位符扫描

- 无"待定"、"TODO"、"后续实现"
- 所有步骤都有完整代码
- 无"类似任务 N"引用

### 3. 类型一致性

- Rust: `type_var(id: usize)` → `Type::Var(TypeVariable { id })`
- TS: `typeVar(id: number)` → `{ kind: "var", id }`
- Rust: `generic(base: &str, args: Vec<Type>)` → `Type::GenericInstance(GenericTypeInstance { base, args })`
- TS: `generic(base: string, args: Type[])` → `{ kind: "genericInstance", base, args }`
- Rust: `Substitution = HashMap<usize, Type>`
- TS: `Substitution = Map<number, Type>`
- Rust: `unify(expected: &Type, actual: &Type, subst: &mut Substitution) -> Result<(), String>`
- TS: `unify(expected: Type, actual: Type, subst: Substitution) -> string | null`

类型命名一致，签名匹配。

### 4. 风险注意事项

- 任务 7 的 `unify_test.rs` 是临时文件，任务 10 会删除它。如果任务 7 和 10 由不同子智能体执行，需确保临时文件被正确清理。
- 任务 11 的 `CallableSignature` 类型可能需要添加 `is_variadic` 字段（TS 端）。如果已有，跳过。
- 差分测试可能因为 IR 序列化差异（如类型变量序列化）而不匹配。任务 12 步骤 3 需要检查并修复。
