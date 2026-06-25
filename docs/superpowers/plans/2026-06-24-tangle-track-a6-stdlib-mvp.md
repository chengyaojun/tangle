# Tangle Track A6: Standard Library & Business MVP 实现计划

> **语法精炼勘误（2026-06-25）：** (1) `with { }` → 无关键字大括号更新。(2) `Struct -> method` → 隐式方法绑定。(3) `=>` → `->`。(4) 新增 `|>` 管道。(5) 新增标题大小写对齐契约。(6) 移除 @export，改为下划线隐式私有。(7) 移除 @entry，改为 main 隐式入口契约。详见设计规格 §3.2。

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 A5（JS Codegen + CLI）的基础上，实现 Tangle 标准库子集和端到端业务 MVP，验证整个编译流水线的实用性。

**架构：** A5 提供了可运行的 `tangle run` 和 `tangle test`。A6 新增：
1. **标准库** (`stdlib/`)：用 Tangle 语言 + JS 运行时实现的基础库
2. **JS Runtime** (`stdlib/js/`)：宿主平台运行时支持
3. **业务 MVP** (`examples/mvp/`)：验证语言设计的真实业务示例

**技术栈：** Tangle（.md 文件）+ TypeScript/JS（运行时辅助）+ Node.js。

---

## 规格来源

- `docs/superpowers/specs/2026-06-24-tangle-language-design.md`
- A6 覆盖：§6.4（标准库边界）、附录 C 第 9 项（真实业务 MVP）

---

## 标准库设计原则

1. **优先跨宿主语义一致**：每个模块先在 Tangle 中定义接口，再提供 JS 实现
2. **最小可用子集**：不追求完整，只实现 MVP 需要的部分
3. **Tangle 源码即文档**：每个标准库模块是带完整注释的 .md 文件

---

## 文件结构

- 创建：`stdlib/List.tangle.md` — 泛型 List 类型
- 创建：`stdlib/Map.tangle.md` — 泛型 Map 类型
- 创建：`stdlib/Option.tangle.md` — Option<T> 类型
- 创建：`stdlib/String.tangle.md` — 字符串工具
- 创建：`stdlib/JSON.tangle.md` — JSON 序列化
- 创建：`stdlib/HTTP.tangle.md` — HTTP 客户端
- 创建：`stdlib/IO.tangle.md` — 文件 I/O
- 创建：`stdlib/DateTime.tangle.md` — 日期时间
- 创建：`stdlib/Regex.tangle.md` — 正则表达式
- 创建：`stdlib/Math.tangle.md` — 数学函数
- 创建：`stdlib/Crypto.tangle.md` — 加密哈希
- 创建：`stdlib/js/runtime.ts` — JS 运行时基础：结构体创建、with 更新、match 等
- 创建：`stdlib/js/list.ts` — JS List 实现
- 创建：`stdlib/js/map.ts` — JS Map 实现
- 创建：`stdlib/js/option.ts` — JS Option 实现
- 创建：`examples/mvp/order-service.tangle.md` — 订单服务 MVP

---

## 任务 1：实现 JS 运行时基础

**文件：**
- 创建：`stdlib/js/runtime.ts`

JS 运行时辅助函数，供 codegen 输出的代码调用：

```ts
// 不可变结构体创建
export function __tangle_struct<T extends Record<string, unknown>>(fields: T): Readonly<T> {
  return Object.freeze({ ...fields });
}

// with 更新：创建新对象而非修改
export function __tangle_with<T extends Record<string, unknown>>(
  obj: Readonly<T>,
  updates: Partial<T>
): Readonly<T> {
  return Object.freeze({ ...obj, ...updates });
}

// 错误结果类型
export type TangleResult<T, E = string> =
  | { ok: true; value: T }
  | { ok: false; error: { variant: E; value: unknown } };

export function Ok<T>(value: T): TangleResult<T, never> {
  return { ok: true, value };
}

export function Err<E extends string>(variant: E, value?: unknown): TangleResult<never, E> {
  return { ok: false, error: { variant, value } };
}

// match 辅助
export function __tangle_match<T, R>(
  result: TangleResult<T, string>,
  handlers: Record<string, (value: unknown) => R>,
  defaultHandler?: () => R
): R {
  if (result.ok) {
    if (handlers._) return handlers._(result.value);
    throw new Error(`Unexpected Ok value in match`);
  }
  const handler = handlers[result.error.variant];
  if (handler) return handler(result.error.value);
  if (defaultHandler) return defaultHandler();
  throw new Error(`Non-exhaustive match: missing variant '${result.error.variant}'`);
}

// ? 传播辅助
export function __tangle_propagate<T, E extends string>(result: TangleResult<T, E>): T {
  if (!result.ok) throw result; // will be caught by caller wrapper
  return result.value;
}
```

---

## 任务 2-10：标准库模块

每个标准库模块按以下模式实现：

### 模式示例：`stdlib/List.tangle.md`

```markdown
# List

### List (List)
* `items`: internal items (Array<T>)

#### List -> 长度 (length)
@export

```@tangle
return items.length
```

#### List -> 映射 (map)
@export
* `fn`: transform function (T => U)

```@tangle
return List { items: items.map(fn) }
```

#### List -> 过滤 (filter)
@export
* `predicate`: filter function (T => Bool)

```@tangle
return List { items: items.filter(predicate) }
```
```

### 需要实现的标准库模块：

| 任务 | 模块 | 核心类型/函数 |
|------|------|--------------|
| 2 | `List` | `List<T>`, `length`, `map`, `filter`, `fold`, `push`, `get` |
| 3 | `Map` | `Map<K,V>`, `get`, `set`, `has`, `keys`, `values`, `delete` |
| 4 | `Option` | `Option<T>`, `Some<T>`, `None`, `isSome`, `isNone`, `unwrap`, `map` |
| 5 | `String` | `length`, `concat`, `slice`, `contains`, `startsWith`, `endsWith`, `split`, `trim` |
| 6 | `JSON` | `parse`, `stringify` |
| 7 | `IO` | `readFile`, `writeFile`, `exists` |
| 8 | `DateTime` | `now`, `parse`, `format`, `addDays` |
| 9 | `Math` | `abs`, `min`, `max`, `round`, `ceil`, `floor`, `random` |
| 10 | `HTTP` | `get`, `post`, `Response` 类型 |

---

## 任务 11：业务 MVP — 订单服务

**文件：**
- 创建：`examples/mvp/order-service.tangle.md`
- 创建：`examples/mvp/payment-service.tangle.md`
- 创建：`examples/mvp/notification-service.tangle.md`

端到端业务场景：用户下单 → 支付 → 通知。

```markdown
# Order Service
@entry

## 依赖
[Payment](./payment-service.md)
[Notify](./notification-service.md)

### Order
@export
* `id`: order ID (String)
* `amount`: amount (Int)
* `status`: status (String)

### PaymentResult
* `receipt`: Receipt
* `payFailed`: PayFailed

@error PayFailed("支付失败", code: String)
@error Timeout("超时")

#### Order -> 确认支付 (confirm)
@export
@error PayFailed
@error Timeout
* `order`: Order

```@tangle
result = Payment.charge(order.amount)?
match result {
    Receipt(r) => Order with { status: "paid" }
    PayFailed(e) => Notify.alert(order.id, "payment failed: " + e.code)?
}
```
```

---

## 任务 12：全量端到端验证

1. 用 `tangle run ./examples/mvp/order-service.tangle.md` 执行 MVP
2. 用 `tangle test` 运行所有标准库和 MVP 测试
3. 验证完整编译流水线：`.md → DSL → Typed AST → IR → JS → Node.js`
4. 验证跨模块导入、错误传播、类型检查在真实场景中正常工作

运行：`npm test` — 全部 PASS
运行：`npm run typecheck` — PASS

---

## 标准库覆盖率

| 规格要求 (§6.4) | 实现状态 |
|---|---|
| `List<T>` | ✅ |
| `Map<K,V>` | ✅ |
| `Set<T>` | 可延期（用 List.distinct 替代） |
| `String` | ✅ |
| `Option<T>` | ✅ |
| `JSON` | ✅ |
| `HTTP` | ✅ |
| `IO` | ✅ |
| 数学函数 | ✅ |
| `DateTime` | ✅ |
| `Regex` | 可延期 |
| `Crypto` | 可延期（hash 函数子集） |

---

## 计划自检清单

- 规格覆盖：§6.4 核心要求
- 明确排除：Set<T>、完整 Crypto、Regex 全部函数、流式 I/O
- 占位符扫描：每个标准库模块含具体 API 签名
- 向后兼容：不修改编译器代码，仅新增库文件和示例
