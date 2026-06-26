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
| `#####` | `semantic-section` | 前置条件、步骤、分支、规则 | camelCase |
| `######` | `semantic-atom` | 原子动作、断言、测试 | camelCase |

### 隐式方法绑定

四级标题（`####`）物理嵌套在三级标题（`###` 结构体）下方时，编译器自动绑定为方法。

### 不可变结构体

```@tangle
user = User { id: 1, email: "alice@tangle.io" }
updated = user { is_active: true }
```

### 错误处理

错误是返回值，绝不抛出：

```@tangle
receipt = confirm(order)?
match result {
    Receipt(r) => process(r)
    PayFailed(e) => log(e.code)
}
```

### 可见性

- **默认公开** — `_` 前缀为模块私有
- **入口点** — 深度 4 标识符 `main` 隐式入口
- **弃用** — `~~删除线~~` 标记废弃符号
- **错误** — `Error:` 前缀声明错误变体
- **规则** — `Rule:` 前缀标记决策逻辑（flow/table/tree/toggle）
- **导入** — Markdown 链接 `[Alias](./module.md)`

---

## 编译流水线

```
Markdown 源文件 (.md)
  → compileModule       (前端: 解析 + 标题树 + 符号表 + 规则检测)
  → checkModule         (检查: 解析 @tangle 代码 + 类型检查)
  → compileToIR         (IR: 降至 Rule Graph IR + 规则 lowering)
  → emitJS / emitPython / emitGo   (多宿主 codegen)
  → 宿主执行            (Node.js / Python / Go)
```

---

## 项目结构

```
tangle/
├── compiler/tangle-cli/     # Track B: Rust 权威编译器 (57 源文件)
│   ├── src/
│   │   ├── model.rs, ast.rs, diagnostic.rs
│   │   ├── frontend/        # Markdown → TangleModule
│   │   ├── markdown/        # pulldown-cmark 封装
│   │   ├── parser/          # 词法 + Pratt 语法分析
│   │   ├── checker/         # 类型检查 + 错误处理 (10 文件)
│   │   ├── ir/              # Rule Graph IR + 规则 lowering (9 文件)
│   │   ├── codegen/         # JS / Python / Go 代码生成
│   │   ├── stdlib/          # 多宿主标准库绑定
│   │   ├── incremental/     # 增量编译
│   │   ├── lsp/             # LSP 语言服务器
│   │   ├── docgen/          # HTML 文档生成
│   │   └── cli/             # tangle run / test / lsp / doc
│   └── Cargo.toml
├── library/std/             # 标准库 (22 模块)
│   └── src/
│       ├── list, map, set, option
│       ├── string, regex, encoding
│       ├── fmt, io, env, path, process
│       ├── http, json
│       ├── math, random, sort
│       ├── task, channel, sync
│       ├── datetime
│       └── crypto
├── reference/               # Track A: TypeScript 参考实现 (已冻结)
│   ├── src/                 # 36 源文件
│   ├── tests/               # 31 测试文件, 132 测试
│   ├── stdlib/               # 标准库 .md 模块
│   └── examples/mvp/        # 订单服务 MVP
├── tests/                   # 共享测试固件 (9 个 .md 文件)
├── schemas/ir.json          # IR JSON Schema (差分测试契约)
├── docs/                    # 设计文档 + 计划
└── Cargo.toml               # Rust workspace 根
```

---

## 快速开始

### Rust 编译器 (Track B)

```bash
# 构建
cargo build

# 运行
cargo run -- run tests/basic/hello.tangle.md
cargo run -- run tests/basic/hello.tangle.md --target py
cargo run -- run tests/basic/hello.tangle.md --target go
cargo run -- run tests/basic/hello.tangle.md --emit-ir
cargo run -- run tests/basic/hello.tangle.md --incremental

# 测试
cargo test -p tangle-cli        # 81 个测试

# LSP & 文档
cargo run -- lsp
cargo run -- doc tests/basic/hello.tangle.md
```

### TypeScript 参考实现 (Track A)

```bash
cd reference
npm install
npm run build

node dist/src/cli/main.js run ../tests/basic/hello.tangle.md
node dist/src/cli/main.js run ../tests/basic/hello.tangle.md --emit-ir
npm test                         # 132 个测试
```

---

## CLI 命令参考

### `tangle run` — 编译并运行程序

```bash
tangle run <file.md>                           # 编译为 JavaScript（默认）
tangle run <file.md> --target js               # 显式 JS 目标
tangle run <file.md> --target py               # 编译为 Python
tangle run <file.md> --target go               # 编译为 Go
tangle run <file.md> --emit-ir                 # 输出 IR JSON（不生成代码）
tangle run <file.md> --incremental             # 增量编译（跳过未修改文件）
```

| 标志 | 说明 |
|------|------|
| `--target <js\|py\|go>` | 目标语言（默认 `js`） |
| `--emit-ir` | 输出 Rule Graph IR JSON，跳过代码生成 |
| `--incremental` | 启用增量编译，缓存到 `.cache/` |

### `tangle test` — 运行测试

```bash
tangle test                                    # 运行所有测试
tangle test --filter <pattern>                 # 按名称过滤
```

### `tangle doc` — 生成文档

```bash
tangle doc <file.md>                           # 输出 HTML 到 stdout
tangle doc <file.md> --output docs/index.html  # 输出到文件
```

### `tangle lsp` — 启动语言服务器

```bash
tangle lsp                                     # stdio LSP 服务器
```

---

## 路线图

### ✅ Track A — TypeScript 引导期 (0.x) — 已完成

仅 JS/TS codegen。语义验证 + 业务 MVP。

| 阶段 | 状态 | 产出 |
|------|------|------|
| A1 — 编译前端 | ✅ | Markdown → `TangleModule` DSL |
| A2 — 解析器与类型检查 | ✅ | `@tangle` 代码解析、静态类型系统 |
| A3 — 错误语义 | ✅ | `?` 传播、`match` 穷举、`panic` |
| A4 — Rule Graph IR | ✅ | 统一 IR、规则 lowering |
| A5 — JS Codegen & CLI | ✅ | IR → JS、`tangle run`、`tangle test` |
| A6 — 标准库与 MVP | ✅ | 7 个标准库模块、订单服务示例 |

### ✅ Track B — Rust 权威期 (1.0) — Phase B1 完成

| 阶段 | 状态 | 内容 |
|------|------|------|
| B1 — Rust 编译器骨架 | ✅ | 前端 → 解析 → 检查 → IR → JS Codegen + CLI |
| B2 — 差分测试对齐 | ✅ | IR JSON Schema、共享测试固件、TS `--emit-ir` |
| B3 — 多宿主 Codegen | ✅ | Python + Go 代码生成、`--target` 标志 |
| B4 — 标准库 | ✅ | 22 模块 (集合/文本/I/O/系统/网络/数学/并发/时间/加密) |
| B5 — 性能与工具链 | ✅ | 增量编译 + IR 缓存 + LSP + Doc HTML |

### 🔮 2.0 — 自举

远期目标：用 Tangle 编写 Tangle 编译器。

---

## 标准库 (22 模块)

| 分类 | 模块 |
|------|------|
| 集合 | List, Map, Set, Option |
| 文本 | String, Regex, Encoding |
| I/O 与系统 | IO, fmt, Env, Path, Process |
| 网络 | HTTP, JSON |
| 数学与数据 | Math, Random, Sort |
| 并发 | Task, Channel, Sync |
| 时间 | DateTime |
| 加密 | Crypto |

---

## 许可证

MIT
