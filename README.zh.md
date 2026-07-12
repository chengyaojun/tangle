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
├── examples/                # 示例程序 (5 个 .tangle.md 文件)
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

# 编译并运行
cargo run -- run tests/basic/hello.tangle.md
cargo run -- run tests/basic/hello.tangle.md --target py
cargo run -- run tests/basic/hello.tangle.md --target go

# 仅编译（输出源码）
cargo run -- build tests/basic/hello.tangle.md
cargo run -- build tests/basic/hello.tangle.md --emit-ir
cargo run -- build tests/basic/hello.tangle.md --incremental
cargo run -- run tests/basic/hello.tangle.md --interp

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

## 导入标准库

通过 Markdown 链接导入 stdlib 模块。三种导入粒度：

```markdown
## 依赖

[fmt](fmt)                     ← 模块导入：fmt.println("hello")
[println](fmt)                 ← 单函数：println("hello")
[print, println](fmt)          ← 多函数：print("hi") + println("hi")
```

| 写法 | 用法 |
|------|------|
| `[fmt](fmt)` | `fmt.println("hello")` |
| `[println](fmt)` | `println("hello")` |
| `[print, println](fmt)` | `println("hi"); print("hi")` |

裸名称 = stdlib 模块，带路径（`./`） = 本地文件。

---

## CLI 命令参考

### `tangle run` — 编译并执行

```bash
tangle run <file.md>                           # 编译为 JS 并执行
tangle run <file.md> --target py               # 编译为 Python 并执行
tangle run <file.md> --target go               # 编译为 Go 并执行
tangle run <file.md> --incremental             # 增量编译
tangle run <file.md> --interp                  # 原生 IR 解释器执行（无需外部宿主）
```

| 标志 | 说明 |
|------|------|
| `--target <js\|py\|go>` | 目标语言（默认 `js`） |
| `--incremental` | 启用增量编译，缓存到 `.cache/` |
| `--interp` | 通过原生 Rust IR 解释器执行（零外部宿主依赖） |

### `tangle build` — 仅编译（输出源码）

```bash
tangle build <file.md>                         # 编译为 JS，输出源码
tangle build <file.md> --target py             # 编译为 Python 源码
tangle build <file.md> --emit-ir               # 输出 IR JSON
tangle build <file.md> --incremental           # 增量编译
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

### ✅ Track B — Rust 权威期 (v0.2.0) — 全部完成

| 阶段 | 状态 | 内容 |
|------|------|------|
| B1 — Rust 编译器骨架 | ✅ | 前端 → 解析 → 检查 → IR → JS Codegen + CLI |
| B2 — 差分测试对齐 | ✅ | IR JSON Schema、共享测试固件、TS `--emit-ir` |
| B3 — 多宿主 Codegen | ✅ | Python + Go 代码生成、`--target` 标志 |
| B4 — 标准库 | ✅ | 22 模块 (集合/文本/I/O/系统/网络/数学/并发/时间/加密) |
| B5 — 性能与工具链 | ✅ | 增量编译 + IR 缓存 + LSP + Doc HTML |

**B5 后增强 (v0.2.x)：**
- 按需 stdlib 导入：Markdown 链接 `[fmt](fmt)`
- 单/多函数导入：`[println](fmt)`、`[print, println](fmt)`
- `tangle build` — 仅编译输出源码（对标 `go build`）
- `tangle run` — 编译+执行一步到位
- 源码直译 codegen（AST → 真实代码，替代注释占位）
- 按模块 stdlib 预绑（只 emit 实际导入的模块）

### ✅ v0.2.1 — 质量审计 — 已完成

| 闸门 | 状态 |
|------|------|
| 审计矩阵 (210 单元格) 零虚假诊断 | ✅ |
| `cargo test --workspace` (108 测试) | ✅ |
| `cargo clippy --workspace -- -D warnings` | ✅ |
| 对 TS 参考实现的差分 IR 测试 | ✅ |

详见 [docs/audit/findings.md](docs/audit/findings.md)。

### ✅ v0.3.0 阶段一 — Call 表达式完整类型检查 — 已完成

| 闸门 | 状态 |
|------|------|
| 审计矩阵 (210 单元格) 零虚假诊断 | ✅ |
| `cargo test --workspace` (127 测试) | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| 兼容性验证 (6 examples + 9 fixtures 零诊断) | ✅ |

**要点：**
- `Type::Any` + `is_variadic` 支持变参 stdlib 函数
- F-024：顶层 callable 符号解析
- 19 模块 stdlib 签名注册表
- `Call` 参数 arity + 类型检查（`TANGLE_ARITY_MISMATCH`、`TANGLE_TYPE_ERROR`）

**B5 后 v0.3.0 演进路径 — 四阶段收紧与独立执行：**

| 阶段 | 聚焦 | 要点 |
|------|------|------|
| 一 — Stdlib 签名 | Checker stdlib 函数签名细化 | 每个 stdlib 函数精确 `CallableSignature { params, returns }`；消除 `TANGLE_TYPE_ERROR` 误报 |
| 二 — 规则 Lowering | 规则形式 lowering 完善 | 嵌套列表 AND/OR 结合律；表格行级优先级拓扑排序 + 重叠检测；Mermaid 图/子图解构增强 |
| 三 — 类型化 Codegen | AST 类型化 codegen 翻译 | 重构 `src/codegen/` 接收标准 Tangle AST；静态分析 Tree-shaking；按模块宿主预绑（如 `fmt.println` → `console.log`） |
| 四 — IR 解释器 | 原生 IR 树行走解释器 **（核心跃迁）** | `tangle-cli` 内部纯 Rust 执行，由 Rule Graph IR 驱动；通用图遍历求值算法；`tangle run --interp` 实验性标志；`?` 错误传播差分压测 |

### 🔮 2.0 — 自举 (v1.0.0)

用 Tangle 编写 Tangle 编译器。Rust 版降级为 bootstrap 工具。

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
