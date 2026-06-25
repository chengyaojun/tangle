# Tangle

> 文档即程序，文件即模块。

Tangle 是一种 **Markdown 原生的编程语言**。每一个 `.md` 文件既是可读的文档，也是可编译的程序模块。标题定义作用域，列表定义参数，代码块承载执行体逻辑，语义约定取代显式指令——实现**零指令的文学编程**体验。

---

## 语言设计

### 六级标题层级体系

| 层级 | 角色 | 语义 | 大小写 |
|------|------|------|--------|
| `#` | `program` | 包 / 文档根上下文 | PascalCase |
| `##` | `section` | 命名空间 / 领域区 | PascalCase |
| `###` | `type` | 结构体、接口、错误族 | PascalCase |
| `####` | `callable` | 函数、方法 | camelCase |
| `#####` | `semantic-section` | 前置条件、步骤、分支 | camelCase |
| `######` | `semantic-atom` | 原子动作、断言、测试 | camelCase |

### 隐式方法绑定

四级标题（`####`）物理嵌套在三级标题（`###` 结构体）下方时，编译器自动将四级标题识别为该结构体的方法——无需箭头语法。

### 不可变结构体与更新语法

结构体默认不可变。大括号表达式同时用于构造与复制更新：

```@tangle
user = User { id: 1, email: "alice@tangle.io" }
updated = user { is_active: true }
```

### 错误处理

错误是返回值，绝不抛出。使用和类型与 `Error:` 前缀约定：

```@tangle
receipt = confirm(order)?
match result {
    Receipt(r) => process(r)
    PayFailed(e) => log(e.code)
}
```

### 可见性

- **默认公开** — 深度 3-4、无 `_` 前缀的符号自动导出
- **`_` 前缀私有** — 以 `_` 开头的符号为模块私有
- **无需 `@export`**

### 程序入口

深度 4、标识符为 `main` 的可调用定义隐式成为程序入口：

```markdown
#### main
```

### Rule Graph 统一 IR

Tangle 将代码与规则统一编译为 **Rule Graph**（节点 + 边 + 错误边），实现 `@tangle` 代码块与决策逻辑（表格、列表、Mermaid 图、复选框）的混合执行——全部通过 `Rule:` 标题前缀标记。

---

## 编译流水线

```
Markdown 源文件 (.md)
  → compileModule       (解析 + 标题树 + 符号表)
  → checkModule         (解析 @tangle 代码 + 类型检查)
  → compileToIR         (降至 Rule Graph IR)
  → emitJS              (生成 JavaScript)
  → Node.js 执行        (tangle run)
```

---

## 快速开始

### 安装

```bash
npm install
npm run build
```

### 运行 Tangle 程序

```bash
node dist/src/cli/main.js run ./examples/mvp/order-service.tangle.md
```

### 运行测试

```bash
npm test       # 132 个测试, 31 个测试文件
```

### 类型检查

```bash
npm run typecheck
```

---

## 示例：不可变结构体与方法

````markdown
### User
* `id`: 用户 ID (Int)
* `email`: 邮箱 (String)
* `is_active`: 是否激活 (Bool)

#### 激活 (activate)
* `reason`: 激活原因 (String)

```@tangle
return this { is_active: true }
```
````

---

## 示例：错误处理

````markdown
#### 确认支付 (confirm)
* `order`: 订单

##### Error: PayFailed
##### Error: Timeout

```@tangle
result = gateway.charge(order.amount)?
return Ok(result)
```
````

---

## 项目结构

```
tangle/
├── src/
│   ├── model.ts              # DSL 类型定义
│   ├── ast.ts                # Code AST 类型
│   ├── front-end/            # Markdown → TangleModule (A1)
│   ├── markdown/             # Markdown 解析器封装
│   ├── parser/               # 词法 + 递归下降解析器 (A2)
│   ├── checker/              # 类型检查 + 错误处理 (A2+A3)
│   ├── ir/                   # Rule Graph IR (A4)
│   ├── codegen/              # JS 代码生成 (A5)
│   ├── cli/                  # CLI 入口
│   └── pipeline.ts           # 完整编译流水线
├── tests/                    # 31 个测试文件, 132 个测试
├── stdlib/                   # 标准库 (.tangle.md 模块)
├── examples/mvp/             # 业务 MVP 示例
└── docs/
    ├── superpowers/specs/    # 语言设计规格
    └── superpowers/plans/    # 实现计划 (A1-A6)
```

---

## 标准库

| 模块 | 类型 / 函数 |
|------|------------|
| `List` | `length`, `map`, `filter` |
| `Option` | `Some`, `None`, `unwrap` |
| `Map` | `get` |
| `String` | `length`, `concat` |
| `JSON` | `parse`, `stringify` |
| `IO` | `readFile`, `writeFile` |
| `Math` | `abs`, `min` |

---

## 语言语义约定

| 特性 | 约定 | 示例 |
|------|------|------|
| 可见性 | `_` 前缀 = 私有 | `_internalInit` |
| 入口点 | 深度 4 标识符 `main` | `#### main` |
| 弃用 | `~~` 删除线 | `### ~~OldConfig~~` |
| 错误 | `Error:` 前缀标题 | `##### Error: PayFailed` |
| 规则 | `Rule:` 前缀标题 | `##### Rule: 审批流` |
| 测试 | `Test:` 前缀标题 | `##### Test: 正常流` |
| 导入 | Markdown 链接 | `[Alias](./module.md)` |

---

## 许可证

MIT
