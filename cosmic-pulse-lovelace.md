# Tangle 编程语言 —— 整体设计规格

> 文档即程序，文件即模块。
> 本规格为 `/brainstorming` 流程产出的设计定稿，覆盖语言核心、类型系统、对象模型、错误处理、规则系统、统一 IR、执行引擎。实施分期列于末尾。

---

## 0. 设计决策总表（已锁定）

| # | 决策维度 | 选择 |
|---|---------|------|
| 1 | 执行体语言 | Tangle 自有语言（JS-like 语法，独立语义，编译到 IR → JS/Python/Go）|
| 2 | 语言定位 | 代码与规则双一等公民 |
| 3 | IR 统一原则 | 图统一：节点 = `@tangle` 代码块（动作），边 = 规则条件（守卫/跳转）|
| 4 | 对象模型 | 值结构体 + 自由函数（Rust/Go 风），默认不可变，`this` = 接收者糖，结构化接口 |
| 5 | 类型系统 | 静态类型 + 推导（Rust/Haskell 风）|
| 6 | 模块导入 | 链接即导入：`[alias](./mod.md)`，`alias.symbol()` 访问，链接双重身份（文档超链接 + 代码导入）|
| 7 | 错误处理 | `@error` 指令声明命名错误结构体 + 和类型返回 + `?` 传播 + `match` 穷举 + `(v,e)=f()` 双值解构糖 + 图错误边 |
| 8 | 可变性 | 默认不可变 + 函数式更新 `expr with { field: val }` |
| 9 | 入口/CLI | `@entry` 唯一入口；程序 = 目录 + 入口 `.md`；`tangle run ./main.md`；CLI 参数以结构体注入 |
| 10 | 标准库 | 最小集：`List/Map/Set/String/Option/JSON/HTTP/IO/数学/DateTime/Regex/Crypto`，宿主差异抽象层 |
| 11 | Source Map | IR 节点携带源码 span，错误回溯 `.md` 行号；codegen 嵌入 sourcemap |
| 12 | 实现路线 | 双轨两步走：Track A — TS 引导期（0.x，仅 JS/TS codegen，验证 + 业务 MVP）；Track B — Rust 权威期（1.0，原生重写 `tangle-cli`，补齐 Py/Go codegen）；远期 2.0 — Tangle 自举（用 Tangle 写 Tangle）|

> ★ 标记为已推荐但尚未经用户逐条确认的次级决策，用户可在审查时调整。（注：截至本轮，全部 ★ 项已确认定稿，详见 §13）

---

## 1. 语言核心

### 1.1 文件即模块（File as Module）
- 每个 `.md` 文件自动成为一个独立命名空间/模块。
- 文件名即模块默认名（`math_utils.md` → 模块 `math_utils`）。
- 跨模块导入用 Markdown 链接（见 §6）。

### 1.2 标题即作用域（Headings as Scopes）
| 标题层级 | 语义 |
|---------|------|
| `#`（一级） | 程序名 / 模块主入口；`@entry` 通常挂于此 |
| `##`（二级） | 值结构体（Struct）或主功能块（Object）；配合定义列表声明字段 |
| `###`/`####`（三/四级） | 函数（Function）或方法（Method）；标题文本即函数名，下方为函数体上下文 |

### 1.3 代码块即执行体（Code Blocks as Execution Body）
- 仅被 `@tangle` 标记的代码块执行（语法 ```` ```@tangle ````）。
- 块内是 Tangle 自有语言代码（JS-like 语法，独立语义）。
- 块内变量与逻辑流遵循 Tangle 语义，编译为 IR 节点。

### 1.4 列表与引用即参数与配置
- 无序列表（`*`/`-`）：定义函数输入参数或配置项；`* `name`: 描述 (Type)` 形式，`(Type)` 为类型标注（可省略，由推导补全）。
- 引用块（`>`）：定义断言、测试用例或返回值说明。

---

## 2. 类型系统（静态 + 推导）

- **全量编译期检查**，大部分类型可推导，标注可选。
- 参数列表的 `(String)` 是真类型标注；缺省时编译器推导。
- 支持的类型构造：
  - 值结构体（`##` 声明）
  - 和类型（Sum Type）：`T | E1 | E2`，用于错误返回与代数数据类型；`match` 穷举检查
  - 泛型（标准库 `List<T>`/`Option<T>`/`Map<K,V>` 等）
  - 函数类型 `T1 -> T2`
- 推导算法：Hindley-Milner 风格扩展（支持结构化接口与和类型）。
- 跨宿主安全：类型信息保留至 IR，codegen 据此生成宿主类型标注（TS 类型 / Python type hint / Go 类型）。

---

## 3. 对象模型（值结构体 + 自由函数）

### 3.1 结构体
- `##` 声明值结构体，默认**不可变**。
- 字段用定义列表声明：
  ```markdown
  ## User
  * `id`: 用户ID (Int)
  * `email`: 邮箱 (String)
  * `is_active`: 是否激活 (Bool)
  ```

### 3.2 方法与 `this` 绑定
- `### 结构体名 -> 方法名 (internal_name)` 定义方法。
- 方法本质是**接收者为首参的自由函数**；`this` 是接收者的语法糖。
- 调用点 `user.send_notification(msg)` 脱糖为 `send_notification(user, msg)`。
- 示例：
  ```markdown
  ### User -> 发送通知 (send_notification)
  @error NotifyFailed
  * `message`: 通知内容 (String)

  ```@tangle
  if (this.is_active) {
      email_service.send(this.email, message)?
  }
  ```
  ```

### 3.3 接口（结构化契合）
- `## Name (接口)` 声明接口契约。
- 接口方法用 `### 声明 -> 方法名` 定义，**无执行体**，仅约束参数与命名。
- **结构化契合**：任何具备匹配签名方法的结构体自动满足接口，无需显式 `implements`。
- 分发：默认静态单态化（按接收者具体类型生成特化代码）；需动态多态时由编译器在 IR 层插入 vtable。

### 3.4 可变性
- 结构体默认不可变。
- 更新用函数式 record update 语法：`expr with { field: val }` 返回新结构体。
- "修改"方法返回新结构体，无原地修改。
- 理由：图 IR 无别名分析负担，单态化/并行友好，跨宿主语义一致。

---

## 4. 错误处理（`@error` 指令式）

### 4.1 声明错误变体
- `@error` 在标题下声明**命名错误结构体**，跨模块可导入：
  ```markdown
  ## 支付服务
  @error PayFailed("支付失败", code: Int)
  @error Refused("银行拒绝")
  @error Timeout("超时")
  ```
- 错误变体是值结构体，可携带载荷字段。

### 4.2 签名声明
- `@tangle` 块上方的 `@error` 声明该块返回类型包含的错误变体（函数签名的一部分）。
- 编译器检查：返回未声明的错误变体 = 编译错误。
  ```markdown
  ### 确认支付 (confirm)
  @error PayFailed
  @error Timeout
  * `order`: 订单 (Order)
  ```
- 该函数返回类型为和类型：`Receipt | PayFailed | Timeout`。

### 4.3 返回（不抛）
- 错误是**返回值**，`return` 显式给出，**无 `raise`/`throw`**。
-  ```@tangle
  receipt = gateway.charge(order.amount)
  if (!receipt.ok) {
      return PayFailed(code: receipt.code)
  }
  return receipt
  ```

### 4.4 传播 `?`
- `expr?` 是语法糖：若返回错误变体，传播到当前子图（函数/规则图）出口。
-  ```@tangle
  receipt = confirm(order)?   // 若 confirm 返回错误，当前函数立即返回该错误
  ```

### 4.5 处理 `match`
- `match` 对返回的和类型穷举解构，编译期保证不漏：
  ```@tangle
  match confirm(order) {
      Receipt(r) => process(r)
      PayFailed(e) => log("支付失败: " + e.code)
      Timeout => retry()
  }
  ```

### 4.6 双值解构糖（Go 风）
- `(v, e) = f()` 把和类型解构为双值：`v` 为值（错误时为单元/默认），`e` 为错误（值时为 `nil`）。
- 底层仍是同一和类型，仅解构语法不同，兼容 Go 习惯：
  ```@tangle
  (receipt, err) = confirm(order)
  if (err != nil) {
      return err?
  }
  ```

### 4.7 图语义（错误边）
- 节点返回值是和类型；边按返回变体匹配：
  - 值变体 → 正常边
  - 错误变体 → 错误边
- `@rule.flow` 可声明**错误终态节点**（如 Mermaid `(错误: PayFailed)`），错误边路由到匹配的错误终态。
- 未声明错误终态时，错误变体冒泡到子图出口。

### 4.8 跨宿主映射
| 宿主 | 错误表示 | `?` 映射 |
|------|---------|---------|
| Go | `(value, error)` 双返回值，error 为 tagged struct | `if err != nil { return nil, err }` |
| JS | 标记对象 `{ok:true,value}` / `{ok:false,error}`（非异常） | `if(!r.ok) return r` |
| Python | 标记对象或 tuple（非异常） | `if r[0] is None: return r` |

### 4.9 与 `Option<T>` 的关系
- `Option<T>` 保留为标准库类型，**仅用于"值的有无"**（非错误场景，如查找可能无结果）。
- 错误一律走 `@error`。职责分离：`Option` = 数据形状，`@error` = 异常控制流。

### 4.10 Panic
- `panic(msg)` 用于真正不可恢复的错误（IR 内部不变式破坏、空指针解引用等）。
- 直接中止整个程序执行，**不可 catch**。
- 与 `@error` 的可恢复错误严格区分。

---

## 5. 规则系统（四种规则 → 图编译）

### 5.1 `@rule.flow`（图表即工作流）
- Mermaid 图**即** IR 子图。
- Mermaid 节点 → IR 节点（指向某 `@tangle` 动作或终态）。
- Mermaid 边 → IR 边（标签 = 条件表达式）。
- 错误终态节点（`(错误: PayFailed)`）→ 错误边路由目标。
  ```markdown
  ### 订单状态机 (order_lifecycle)
  @rule.flow

  ```mermaid
  graph TD
      A[新订单: Created] -->|用户支付成功| B(待发货: Paid)
      A -->|24小时未支付| C(已取消: Cancelled)
      B -->|物流发货| D(已发货: Shipped)
      D -->|用户签收| E(已完成: Completed)
      D -->|拒收/退款| C
      B -->|支付失败| F(错误: PayFailed)
  ```
  ```

### 5.2 `@rule.table`（决策表）
- 表格每行 = 一条贯穿决策子图的路径（条件 AND → 动作节点）。
- 编译为决策子图：行条件合取为路径守卫，行动作指向目标节点。

### 5.3 `@rule.tree`（嵌套列表）
- 多级嵌套列表：同级 = AND，子级 = OR（或显式关键字指定）。
- 编译为决策树子图。
  ```markdown
  ### 信用卡审批 (approve_credit_card)
  @rule.tree
  * `user`: 用户对象

  * 核心准入条件：
      * 收入门槛：`user.income >= 10000`
      * 信用良好：`user.credit_score > 700`
  * 风险对冲（满足其一）：
      * 资产证明：`user.has_house == true`
      * 担保人：`user.has_guarantor == true`
  * 结果：返回 `true`
  ```

### 5.4 `@rule.toggle`（复选框即开关）
- 复选框列表 → 布尔源节点，作为边条件被引用。
  ```markdown
  ### 全局功能灰度 (get_features)
  @rule.toggle

  - [x] `enable_new_ui`: 启用 2026 全新 UI
  - [ ] `enable_crypto_payment`: 开启加密货币支付
  - [x] `enable_ai_assistant`: 启用 AI 助手
  ```

### 5.5 代码与规则的互操作
- `@tangle` 块可调用规则函数（规则编译后即图函数）。
- 规则条件可引用 `@tangle` 函数（含返回 `Result`/和类型的函数，按 §4.7 匹配错误边）。
- 二者在 IR 层同为节点/边，无隔阂。

---

## 6. 模块系统（链接即导入）

### 6.1 导入语法
- Markdown 链接即导入，出现在**导入区**（文件首部或 `## 依赖` 节）：
  ```markdown
  ## 依赖

  [Math](./math_utils.md)
  [Notify](./notify_service.md)
  ```
- 链接文本 = 别名；`./mod.md` = 相对路径。
- 同一条链接：渲染文档时是超链接，编译时是导入。

### 6.2 导出与可见性
- `@export` 标记标题（函数/结构体）为公开，导入方可访问。
- 未 `@export` 的符号在 IR 生成时打 `private: true`，外部模块不可调用。
- `@entry` 隐式获得 `@export`。

### 6.3 跨模块符号引用
- `alias.symbol()` 访问导入模块的导出符号。
- 错误变体（`@error` 声明的命名结构体）作为类型可跨模块导入与引用。

---

## 7. 统一 IR（Rule Graph）

### 7.1 编译流水线
```
Markdown 文本
    ↓  (Remark/Markdown-it)
Markdown AST（Heading/Table/List/Link/Fence）
    ↓  (+ 指令 @)
DSL 模型（结构体/函数/规则/错误的领域模型）
    ↓
Unified IR（Rule Graph：节点 + 边 + 错误边）
    ↓
Execution Engine（转译 JS/Py/Go 或自研 VM）
```

### 7.2 IR 结构
- **节点（Node）**：`@tangle` 代码块编译而来的原子动作；携带签名（参数类型 → 返回和类型）、源码 span。
- **边（Edge）**：条件表达式（守卫/跳转），源自规则（表格/列表/Mermaid 边）或 `@tangle` 内的 `if`/`match` 分支。
- **错误边（Error Edge）**：匹配特定错误变体的特殊边，路由到 `@rule.flow` 错误终态或冒泡。

### 7.3 IR 层职责
- 代码优化（节点合并、死边消除、单态化）
- 静态类型检查（节点签名一致性、边条件类型匹配、错误变体穷举）
- 循环依赖检测（模块图层面）
- Source Map（源码 span → `.md` 行号追踪）

---

## 8. 执行引擎

### 8.1 转译模式（JIT/AOT）
- Rule Graph 生成标准 JS/Python/Go 代码，调用宿主引擎运行。
- **TS 引导期（Track A，0.x）**：仅实现 JS/TS codegen，验证语言核心与业务 MVP，Python/Go 后端推迟。
- **Rust 权威期（Track B，1.0）**：补齐 Python/Go codegen，三宿主一致。
- 默认采用转译模式（复用宿主生态、性能、工具链）。

### 8.2 自研 VM（解释执行）
- 轻量图执行引擎，内存中按条件触发节点。
- 适用场景：嵌入式规则引擎、无需宿主运行时。

### 8.3 程序入口与 CLI
- `@entry` 标记唯一入口函数；一个项目/模块目录只能有一个 `@entry`。
- 程序 = 一个目录 + 入口 `.md` 文件。
- `tangle run ./main.md` 执行程序。
- CLI 参数以结构体注入 `@entry` 函数。
- `@entry` 隐式 `@export`，由运行时自动调用。

### 8.4 测试
- `@test(input=..., expect=...)` 定义单条测试用例。
- `tangle test` 提取所有 `@test` 指令并执行断言。

---

## 9. 指令集（形式化）

| 指令 | 位置 | 语义 |
|------|------|------|
| `@export` | 标题正下方 | 标记公开，导入方可访问 |
| `@entry` | 标题正下方 | 程序唯一入口，隐式 `@export`，运行时自动调用 |
| `@deprecated("原因")` | 标题正下方 | 标记弃用；调用时编译/运行警告；doc HTML 加删除线 |
| `@test(input=..., expect=...)` | 标题正下方或代码块上方 | 单条测试用例，`tangle test` 执行 |
| `@hideCode` | 代码块上方 | doc HTML 隐藏下方 `@tangle` 块，仅保留标题与参数 |
| `@version` | Front Matter | 文件级元数据 |
| `@error VariantName(...)` | 标题正下方 | 声明命名错误结构体（可导入）；块上方时声明该块返回的错误变体 |
| `@rule.table` | 代码块/表格上方 | 决策表 → 决策子图 |
| `@rule.tree` | 列表上方 | 嵌套列表 → 决策树子图 |
| `@rule.toggle` | 复选框列表上方 | 复选框 → 布尔源节点 |
| `@rule.flow` | Mermaid 块上方 | Mermaid 图 → IR 子图 |

**位置纪律**：指令只能出现在标题行正下方或代码块正上方，禁止夹入普通段落。

---

## 10. 标准库

最小集，作为宿主差异抽象层，codegen 映射到宿主等价物：
- 集合：`List<T>`、`Map<K,V>`、`Set<T>`
- 字符串：`String`
- 可空值：`Option<T>`
- 序列化：`JSON`
- 网络：`HTTP`
- IO：`IO`
- 数学：基础数学函数
- 日期与时间：`DateTime`（时间戳、格式化、时区、Duration 运算）
- 正则表达式：`Regex`（模式匹配、捕获组、替换）
- 加密与哈希：`Crypto`（摘要哈希 MD5/SHA、对称加密 AES、非对称 RSA、签名验签、HMAC）——贴合支付场景的签名验签需求

---

## 11. Source Map 与错误追踪

- IR 节点携带源码 span（file/line/col）。
- 编译错误、类型错误、运行时错误回溯到原始 `.md` 行号。
- codegen 嵌入 sourcemap，供宿主调试器定位到 Tangle 源。

---

## 12. 实施分期（双轨两步走）

整体设计已定稿。实现采用双轨两步走：先用 TypeScript 引导验证，再用 Rust 重写出权威实现，远期以 Tangle 自举为终极目标。详见 §14。

### Track A — TypeScript 引导期（0.x，验证 + 业务 MVP）

> 目标：用最快速度把语言核心跑起来，在真实业务（宝付支付/商户管理）中验证设计可行性。代码生成仅 JS/TS。

1. **A1 — 编译前端与语言核心**
   - Markdown 解析（Remark/Markdown-it）+ 指令提取 → DSL 模型
   - `@tangle` 块语法解析器（JS-like 子集）
   - 静态类型检查 + 推导（HM 扩展）
   - 值结构体、方法、`this` 绑定、结构化接口
   - 模块系统（链接即导入、`@export` 可见性）

2. **A2 — 统一 IR 与图编译**
   - Rule Graph 节点/边/错误边形式化
   - `@tangle` 块 → 节点；`if`/`match` → 边
   - 四种规则 → 子图编译器（flow/table/tree/toggle）
   - IR 层优化与循环依赖检测

3. **A3 — 错误处理**
   - `@error` 指令解析、错误变体作为和类型
   - `?` 传播、`match` 穷举、`(v,e)` 双值解构糖
   - 图错误边与 `@rule.flow` 错误终态路由
   - JS 宿主错误映射（标记对象，非异常）

4. **A4 — 执行引擎与 CLI（JS/TS codegen）**
   - Rule Graph → JS/TS 代码生成
   - `@entry`、`tangle run ./main.md`、CLI 参数以结构体注入
   - `tangle test` 与 `@test`
   - Source Map 生成（IR span → `.md` 行号）

5. **A5 — 标准库（JS 宿主子集）**
   - `List/Map/Set/String/Option/JSON/IO/数学/DateTime/Regex`
   - `HTTP`（基于 fetch/axios）
   - `Crypto`（贴合支付签名验签：HMAC-SHA256、RSA 签名、AES）——直接服务宝付业务

6. **A6 — 业务 MVP 验证**
   - 用 Tangle 重写一个真实宝付业务场景（如商户入网审批流、交易风控规则、对账逻辑）
   - 验证 `@rule.tree`/`@rule.flow`/`@error` 在支付场景的表达力
   - 收集痛点反馈，固化 1.0 语义基线

### Track B — Rust 权威期（1.0，工程自举与极致性能）

> 目标：核心语法、类型推导、IR 规范在 Track A 完全定型后，用 Rust 原生重写出官方 `tangle-cli`，作为权威实现。TS 版退役为参考实现。

7. **B1 — Rust 编译器骨架**
   - 用 Rust 重写 Track A1–A4 的全部能力
   - 设计与 TS 版对齐的 IR 序列化格式（便于差分测试）
   - clap/structopt 实现 `tangle-cli` 命令行

8. **B2 — 差分测试对齐**
   - 用同一批 `.md` 测试用例分别喂给 TS 版与 Rust 版
   - 比对 IR、codegen 产物、运行结果，逐项对齐
   - 任何语义分歧以 Track A 固化的基线为准（除非基线有 bug，登记 errata）

9. **B3 — 多宿主 codegen 补齐**
   - 在 Rust 版中实现 Python、Go codegen
   - 跨宿主一致性测试套件

10. **B4 — 标准库 Rust 实现与多宿主绑定**
    - 标准库在 Rust 中实现核心逻辑，各宿主 codegen 时映射到宿主等价物
    - `Crypto` 在 Go/Python 宿主的签名验签一致性

11. **B5 — 性能与工具链**
    - 增量编译、IR 缓存
    - LSP / 语言服务（基于 Rust 版）
    - doc HTML 生成（`@hideCode`、`@deprecated` 删除线）

### 远期 2.0 — Tangle 自举（用 Tangle 写 Tangle）

12. **C1 — Tangle 编译器自举**
    - 用 Tangle 语言本身重写 Tangle 编译器
    - 用 1.0 Rust 版 `tangle-cli` 把 Tangle 版编译器编译成可执行码
    - 自举成功后，Tangle 版编译器成为官方实现，Rust 版降级为 bootstrap 工具

---

## 13. 设计定稿确认

所有决策项均已确认定稿：

| 维度 | 状态 |
|------|------|
| 标准库范围（§10） | ✓ 已确认：含 `DateTime`/`Regex`/`Crypto` |
| 实现路线（§12/§14） | ✓ 已确认：双轨两步走，TS 引导 → Rust 权威 → Tangle 自举远期 |
| 可变性（§3.4） | ✓ 已采纳：默认不可变 + `with` 函数式更新 |
| 入口/CLI（§8.3） | ✓ 已采纳：`@entry` + `tangle run ./main.md` + 结构体参数注入 |
| Source Map（§11） | ✓ 已采纳：IR span → `.md` 行号追踪 + codegen sourcemap |
| 执行引擎（§8.1） | ✓ 已采纳：默认转译模式；TS 期仅 JS/TS，Rust 期补齐 Py/Go |

下一步：进入 **Track A1 — 编译前端与语言核心** 实现。

---

## 14. 工程实施路线（双轨两步走详解）

### 14.1 总体策略

```
Track A: TypeScript + Node.js 引导（0.x）
   └─ 快速验证语义、跑通真实业务 MVP
   └─ 仅 JS/TS codegen，范围可控
   └─ 固化 1.0 语义基线
            ↓
Track B: Rust 原生重写（1.0）
   └─ 官方权威 tangle-cli
   └─ 差分测试对齐 TS 版
   └─ 补齐 Py/Go codegen、性能、工具链
            ↓
2.0 远期: Tangle 自举
   └─ 用 Tangle 写 Tangle 编译器
   └─ Rust 版降级为 bootstrap 工具
```

### 14.2 为什么先 TS、再 Rust

- **TS 引导的优势**：生态成熟（Markdown 解析、Mermaid 解析、AST 工具链齐全）、迭代快、与首发宿主 JS 同构、便于在宝付 Node.js 技术栈中直接嵌入验证。
- **Rust 重写的收益**：极致性能（增量编译、IR 缓存）、零成本抽象（IR 数据结构）、单二进制分发（`tangle-cli` 一个文件部署）、跨平台一致。
- **避免的陷阱**：不在语义未定型时上 Rust（返工成本高）；不在 TS 版追求极致性能（验证优先）。

### 14.3 两轨之间的契约

- **语义基线**：Track A 的 A6 完成后，冻结 1.0 语义基线。Track B 必须严格对齐，分歧走 errata 流程。
- **IR 序列化格式**：两版共用同一 IR JSON Schema，使差分测试可机械化。
- **测试用例库**：随 Track A 同步积累，作为 Track B 的验收集。

### 14.4 版本号约定

| 版本 | 阶段 | 说明 |
|------|------|------|
| 0.1–0.9 | Track A | TS 引导期，语义可变，每版可 breaking |
| 1.0 | Track B 完成 | 语义冻结，Rust 权威实现，三宿主 codegen 齐备 |
| 1.x | Track B 演进 | 增量编译、LSP、性能优化，向后兼容 |
| 2.0 | 远期自举 | Tangle 自举成功，官方实现切换为 Tangle 版 |

### 14.5 关键风险与对策

| 风险 | 对策 |
|------|------|
| TS 版语义在 A6 后才发现设计缺陷 | A6 业务 MVP 必须覆盖足够多真实场景；冻结前预留 errata 机制 |
| Rust 重写周期长，期间 TS 版继续演进导致双轨漂移 | 冻结基线后 TS 版只接 bugfix，不接新特性；新特性进 1.x |
| 自举时 Tangle 表达力不足以写复杂编译器 | 2.0 为远期目标，不阻塞 1.0；表达力不足则推迟 |
| 跨宿主 codegen 语义不一致 | 标准库设计为宿主无关抽象层，跨宿主一致性测试套件前置到 Track B 早期 |
