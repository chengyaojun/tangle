# Phase 6a: 类型系统对齐 + 局部泛型推导 设计规格

> **本阶段属于 B5 类型系统扩展的第一阶段。** 完整 B5 通过分阶段实现：
> - **Phase 6a（本规格）：** TS 端对齐 Rust（闭合 order-service）+ Rust/TS 双端局部泛型推导
> - **Phase 6b（后续）：** 完善泛型在 codegen 和 IR 中的表示
> - **Phase 6c（后续）：** 返回类型推导 + 类型收窄（type narrowing）

**目标：** 闭合 order-service.tangle 差分测试（最后一个 SKIPPED）+ 实现 Rust/TS 双端局部泛型类型推导，让 stdlib 泛型函数（List/Map/Option）获得精确类型而非退化为 Any。

**成功指标：** `tests/audit/diff-ir.ps1` 从 `10 MATCH + 1 SKIPPED` 变为 `11 MATCH + 0 SKIPPED`，且新增泛型 fixture 通过差分测试。

---

## 1. 现状分析

### 1.1 order-service.tangle 差分测试失败根因

Rust 端 `cargo run -- build tests/mvp/order-service.tangle.md --emit-ir` 已正常工作，生成 `functions[]` 含 `create`/`confirm`/`main` 三个函数。

TS 端 `node reference/dist/src/cli/main.js run tests/mvp/order-service.tangle.md --emit-ir` 报 5 个错误：

| # | 错误 | 根因 | 对应 Rust 位置 |
|---|------|------|---------------|
| 1 | `Undefined variable: Order` | TS `check.ts` identifier 未查找 `env.structs` | [Rust check.rs:20](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L20) 有 `env.structs.get(name)` |
| 2 | `Undefined variable: Err` / `Ok` | `Err`/`Ok` 构造器未注册 | Rust `builtins.rs` 已注册 |
| 3 | `Unknown member: amount` | 方法参数类型硬编码 `String`，应为 `Order` | Rust 用 `param.type_name` 解析 |
| 4 | `record update requires a struct type` | 同 #3，`order` 类型为 `String` 而非 `Order` | 同上 |
| 5 | `Unknown member: create` | `Order.create` 方法调用，因 #1 导致 `Order` 未解析 | 同 #1 |

### 1.2 泛型类型推导现状

**类型定义已存在但未使用：**
- [types.rs:8](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/types.rs#L8) `GenericInstance(GenericTypeInstance)` — 已定义
- [types.rs:11](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/types.rs#L11) `Var(TypeVariable)` — 已定义
- [type_parser.rs:109-150](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/parser/type_parser.rs#L109-L150) 能解析 `Result<String, Error>`、`Array<Int>` 语法

**stdlib 签名退化为 Any：**
- [signatures.rs:70-76](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L70-L76) `List.map(list: Any, fn: Any) -> Any`
- [signatures.rs:78-85](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L78-L85) `Map.get(map: Any, key: Any) -> Any`
- [signatures.rs:98-106](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L98-L106) `Option.Some(value: Any) -> Any`

**call 检查无推导：**
- [check.rs:77-124](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L77-L124) 用 `types_equal` 直接比较参数类型，不绑定类型变量

**结论：** 泛型基础设施已就位（类型定义、解析器），缺的是推导算法和 stdlib 泛型签名。

---

## 2. 方案选择

### 2.1 采用方案：局部泛型推导

**核心思路：** 在 `check_expression` 处理 `Expr::Call` 时：
1. 获取函数签名（可能含 `TypeVariable`）
2. 对每个实际参数类型，调用 `unify(param_ty, arg_ty, &mut subst)` 绑定类型变量
3. 返回类型用 `substitute(returns, &subst)` 替换 `TypeVariable`

**不采用 Hindley-Milner 的理由：**
- Tangle 是脚本语言，不需要全局类型推导
- 局部推导满足 stdlib 泛型函数需求
- 实现复杂度低，易于 TS 端同步移植
- 避免引入 let 多态等复杂语义

**示例：** `List.map(myList, fn)` 其中 `myList: List<Int>`
- 签名：`map<T,U>(list: List<T>, fn: Func<T,U>) -> List<U>`
- 参数 1：`List<Int>` 统一 `List<T>` → 绑定 `T = Int`
- 参数 2：`fn: Int -> String` 统一 `Func<T,U>` → 绑定 `U = String`
- 返回：`List<U>` → 替换为 `List<String>`

### 2.2 两个工作流

```
Phase 6a
├── 工作流 1: TS 端类型检查器对齐（闭合 order-service）
│   └── 6 个改动点（见 §3）
│
└── 工作流 2: Rust+TS 双端局部泛型推导
    └── 6 个改动点（见 §4）
```

**依赖关系：** 两个工作流相互独立。工作流 2 的 TS 端同步实现依赖 Rust 端先完成（Rust 作为参考）。

---

## 3. 工作流 1：TS 端类型检查器对齐

### 3.1 改动点 1：check.ts identifier 查找 structs/functions

**当前**（[check.ts:21-29](file:///e:/GitProjects/tangle/reference/src/checker/check.ts#L21-L29)）：
```typescript
case "identifier": {
  if (env.variables[expr.name]) return [env.variables[expr.name]!, diags];
  if (env.receiver?.fields[expr.name]) return [env.receiver.fields[expr.name]!, diags];
  if (["String", "Int", "Bool"].includes(expr.name)) { ... }
  // → 报 undefined variable
}
```

**改为**（对齐 [Rust check.rs:14-43](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L14-L43)）：
```typescript
case "identifier": {
  if (env.variables[expr.name]) return [env.variables[expr.name]!, diags];
  if (env.receiver?.fields[expr.name]) return [env.receiver.fields[expr.name]!, diags];
  if (env.structs[expr.name]) return [env.structs[expr.name]!, diags];       // ← 新增
  if (env.functions[expr.name]) {                                            // ← 新增
    const fn = env.functions[expr.name]!;
    return [{ kind: "function", params: fn.params, returns: fn.returns }, diags];
  }
  if (["String", "Int", "Bool"].includes(expr.name)) { ... }
  // → 报 undefined variable
}
```

### 3.2 改动点 2：checkModule.ts 参数类型解析

**当前**（[checkModule.ts:63](file:///e:/GitProjects/tangle/reference/src/checker/checkModule.ts#L63)）：
```typescript
checkEnv.variables[param.name] = { kind: "primitive", name: "String" };
```

**改为**：使用 `param.typeName` 解析类型
```typescript
if (param.typeName) {
  try {
    const te = parseTypeExpr(param.typeName, param.span.file);
    checkEnv.variables[param.name] = typeExprToType(te);
  } catch {
    checkEnv.variables[param.name] = { kind: "any" };
  }
} else {
  checkEnv.variables[param.name] = { kind: "any" };  // 无标注用 Any 而非 String
}
```

**新增辅助函数** `typeExprToType`：将 `TypeExpr` 转为 `Type`（处理 Primitive/Named/Generic）。

### 3.3 改动点 3：builtins.ts 注册 Err/Ok

**新增**：注册 `Err` 和 `Ok` 构造器为函数类型
```typescript
const errOkConstructors: Record<string, Type> = {
  Err: {
    kind: "function",
    params: [{ kind: "primitive", name: "String" }, { kind: "primitive", name: "String" }],
    returns: { kind: "any" },
  },
  Ok: {
    kind: "function",
    params: [{ kind: "any" }],
    returns: { kind: "any" },
  },
};
```

在 `checkEnv` 初始化时合并到 `env.variables` 或 `env.functions`。

### 3.4 改动点 4：check.ts call 处理结构体构造器

**当前**（[check.ts:56-67](file:///e:/GitProjects/tangle/reference/src/checker/check.ts#L56-L67)）：非 function callee 返回 Bool

**改为**：
```typescript
case "call": {
  const [calleeType, calleeDiags] = checkExpression(expr.callee, env);
  // ... 参数检查 ...
  if (calleeType.kind === "function") return [calleeType.returns, diags];
  if (calleeType.kind === "struct") return [calleeType, diags];  // ← 新增
  return [{ kind: "any" }, diags];  // ← 改为 Any 而非 Bool
}
```

### 3.5 改动点 5：resolve.ts 方法返回类型

**当前**（[resolve.ts:114](file:///e:/GitProjects/tangle/reference/src/checker/resolve.ts#L114)）：`returns: { kind: "primitive", name: "Bool" }`

**改为**：`returns: { kind: "any" }`（对齐 Rust 的 `collect_method_sigs` 返回 Any，避免误报）

### 3.6 改动点 6：propagation（保持现状）

`?` 传播当前处理 sum 类型，返回第一个非 error 变体。由于 Err/Ok 返回 Any，propagation 对 Any 直接返回，不误报。**本阶段不改。**

---

## 4. 工作流 2：局部泛型推导

### 4.1 改动点 1：types.rs 泛型构造辅助函数

**已有**：`TypeVariable { id: usize }`、`GenericTypeInstance { base: String, args: Vec<Type> }`

**新增**（在 [types.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/types.rs) 末尾）：
```rust
pub fn type_var(id: usize) -> Type { Type::Var(TypeVariable { id }) }

pub fn generic(base: &str, args: Vec<Type>) -> Type {
    Type::GenericInstance(GenericTypeInstance { base: base.into(), args })
}
```

### 4.2 改动点 2：新建 checker/unify.rs — 类型统一算法

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
            if a.base != b.base { return Err(format!("Expected {}, got {}", a.base, b.base)); }
            if a.args.len() != b.args.len() { return Err("Generic arity mismatch".into()); }
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
            if a.params.len() != b.params.len() { return Err("Function arity mismatch".into()); }
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

### 4.3 改动点 3：check.rs call 表达式加入泛型推导

修改 [check.rs:77-124](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L77-L124) 的 `Expr::Call` 分支：

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
            // arity 检查（保持现有逻辑）
            let expected = sig.params.len();
            let actual = arg_types.len();
            if sig.is_variadic {
                if actual < expected.saturating_sub(1) { /* 报错 */ }
            } else if actual != expected { /* 报错 */ }

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
        // 非 function callee：保持现有逻辑（返回 Any）
        _ => Type::Any,
    }
}
```

### 4.4 改动点 4：signatures.rs stdlib 签名改用类型变量

**新增构造函数** `sig_generic`：
```rust
/// 构造泛型函数签名（type_var id 从 0 开始，对应 type_params 顺序）
fn sig_generic(
    type_params: &[&str],   // 仅用于文档目的，实际用 id 索引
    params: &[(&str, Type)],
    returns: Type,
) -> CallableSignature {
    CallableSignature {
        params: params.iter().map(|(n, t)| (*n, t.clone())).collect(),
        returns: Box::new(returns),
        is_variadic: false,
    }
}
```

**更新 List 模块**（[signatures.rs:70-76](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L70-L76)）：
```rust
m.insert("List", module(&[
    ("length", sig_fixed(&[("list", generic("List", [type_var(0)]))], int_t())),
    ("map", sig_generic(&["T", "U"],
        &[("list", generic("List", [type_var(0)])),
          ("fn", Type::Function(FunctionType {
              params: vec![type_var(0)],
              returns: Box::new(type_var(1)),
              is_variadic: false,
          }))],
        generic("List", [type_var(1)]))),
    ("filter", sig_generic(&["T"],
        &[("list", generic("List", [type_var(0)])),
          ("fn", Type::Function(FunctionType {
              params: vec![type_var(0)],
              returns: Box::new(bool_t()),
              is_variadic: false,
          }))],
        generic("List", [type_var(0)]))),
    ("push", sig_generic(&["T"],
        &[("list", generic("List", [type_var(0)])), ("item", type_var(0))],
        generic("List", [type_var(0)]))),
    ("get", sig_generic(&["T"],
        &[("list", generic("List", [type_var(0)])), ("index", int_t())],
        type_var(0))),
]));
```

**更新 Map 模块**（[signatures.rs:78-85](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L78-L85)）：
```rust
m.insert("Map", module(&[
    ("get", sig_generic(&["K", "V"],
        &[("map", generic("Map", [type_var(0), type_var(1)])), ("key", type_var(0))],
        type_var(1))),
    ("set", sig_generic(&["K", "V"],
        &[("map", generic("Map", [type_var(0), type_var(1)])),
          ("key", type_var(0)), ("value", type_var(1))],
        generic("Map", [type_var(0), type_var(1)]))),
    ("has", sig_generic(&["K", "V"],
        &[("map", generic("Map", [type_var(0), type_var(1)])), ("key", type_var(0))],
        bool_t())),
    ("keys", sig_generic(&["K", "V"],
        &[("map", generic("Map", [type_var(0), type_var(1)]))],
        generic("List", [type_var(0)]))),
    ("values", sig_generic(&["K", "V"],
        &[("map", generic("Map", [type_var(0), type_var(1)]))],
        generic("List", [type_var(1)]))),
    ("delete", sig_generic(&["K", "V"],
        &[("map", generic("Map", [type_var(0), type_var(1)])), ("key", type_var(0))],
        generic("Map", [type_var(0), type_var(1)]))),
]));
```

**更新 Option 模块**（[signatures.rs:98-106](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L98-L106)）：
```rust
m.insert("Option", module(&[
    ("Some", sig_generic(&["T"], &[("value", type_var(0))], generic("Option", [type_var(0)]))),
    ("None", sig_generic(&["T"], &[], generic("Option", [type_var(0)]))),
    ("unwrap", sig_generic(&["T"], &[("opt", generic("Option", [type_var(0)]))], type_var(0))),
    ("is_some", sig_generic(&["T"], &[("opt", generic("Option", [type_var(0)]))], bool_t())),
    ("is_none", sig_generic(&["T"], &[("opt", generic("Option", [type_var(0)]))], bool_t())),
    ("map", sig_generic(&["T", "U"],
        &[("opt", generic("Option", [type_var(0)])),
          ("fn", Type::Function(FunctionType {
              params: vec![type_var(0)],
              returns: Box::new(type_var(1)),
              is_variadic: false,
          }))],
        generic("Option", [type_var(1)]))),
    ("or_else", sig_generic(&["T"],
        &[("opt", generic("Option", [type_var(0)])),
          ("fn", Type::Function(FunctionType {
              params: vec![],
              returns: Box::new(generic("Option", [type_var(0)])),
              is_variadic: false,
          }))],
        generic("Option", [type_var(0)]))),
]));
```

**Set 模块**（[signatures.rs:87-96](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/stdlib/signatures.rs#L87-L96)）：同步改造为 `Set<T>`。

**保留 Any 的模块**：`Math`、`String`、`IO`、`Time` 等非容器模块保持现有签名（无泛型需求）。

### 4.5 改动点 5：TS 端同步实现

- `reference/src/checker/types.ts`：添加 `TypeVariable` 类型、`typeVar()`/`generic()` 构造函数
- 新建 `reference/src/checker/unify.ts`：忠实移植 Rust `unify.rs`（`unify` + `substitute`）
- `reference/src/checker/check.ts`：call 表达式加入泛型推导（同步 §4.3）
- `reference/src/checker/builtins.ts`：stdlib 泛型签名（同步 §4.4）

### 4.6 改动点 6：新 fixture

**`tests/v06_phase6/generics.tangle.md`**：使用 `List.map`、`Map.get`、`Option.Some` 的示例
```markdown
# Generic Type Inference Test

### main
* `numbers`: list of numbers

```@tangle
let doubled = List.map(numbers, fn(x) { x * 2 })
return doubled
```
```

---

## 5. 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `compiler/tangle-cli/src/checker/types.rs` | 泛型构造辅助函数 `type_var`/`generic` | 修改 |
| `compiler/tangle-cli/src/checker/unify.rs` | 类型统一算法 `unify` + `substitute` | 创建 |
| `compiler/tangle-cli/src/checker/mod.rs` | 注册 `unify` 模块 | 修改 |
| `compiler/tangle-cli/src/checker/check.rs` | call 加入泛型推导 | 修改 |
| `compiler/tangle-cli/src/stdlib/signatures.rs` | stdlib 泛型签名（List/Map/Option/Set） | 修改 |
| `reference/src/checker/types.ts` | TS 泛型类型 + 构造函数 | 修改 |
| `reference/src/checker/unify.ts` | TS 类型统一算法 | 创建 |
| `reference/src/checker/check.ts` | TS call 泛型推导 + identifier 扩展 + 结构体构造器 | 修改 |
| `reference/src/checker/checkModule.ts` | 参数类型解析 | 修改 |
| `reference/src/checker/resolve.ts` | 方法返回类型改 Any | 修改 |
| `reference/src/checker/builtins.ts` | Err/Ok 注册 + 泛型签名 | 修改 |
| `reference/tests/checker/order-service.test.ts` | order-service 类型检查测试 | 创建 |
| `reference/tests/checker/generics.test.ts` | TS 泛型推导测试 | 创建 |
| `compiler/tangle-cli/tests/v06_phase6/generics_inference.rs` | Rust 泛型推导测试 | 创建 |
| `tests/v06_phase6/generics.tangle.md` | 泛型 fixture | 创建 |

**总计**：修改 9 个文件，创建 6 个文件。

---

## 6. 测试策略

### 6.1 工作流 1 测试（TS 端对齐）

**`reference/tests/checker/order-service.test.ts`**（新建）：
- 验证 order-service.tangle 类型检查无 `Undefined variable` 错误
- 验证 `Order` 解析为结构体类型
- 验证 `Err`/`Ok` 解析为函数类型
- 验证方法参数 `order: Order` 解析为结构体类型

**差分测试**：order-service 从 SKIPPED → MATCH

### 6.2 工作流 2 测试（泛型推导）

**`compiler/tangle-cli/tests/v06_phase6/generics_inference.rs`**（新建）：
- `unify` 算法测试：
  - 类型变量绑定：`unify(Var(0), Int) → subst[0] = Int`
  - 已绑定变量一致性：`unify(Var(0), Int)` 后 `unify(Var(0), Int)` 成功
  - 已绑定变量冲突：`unify(Var(0), Int)` 后 `unify(Var(0), String)` 失败
  - 嵌套泛型：`unify(List<Var(0)>, List<Int>) → subst[0] = Int`
  - 函数类型统一：`unify(Func<Var(0), Var(1)>, Func<Int, String>)`
  - Any 兼容：`unify(Any, Int)` 成功
- stdlib 泛型函数调用测试：
  - `List.map(numbers, fn)` 推导返回 `List<String>`
  - `Map.get(map, key)` 推导返回 `V`
  - `Option.Some(value)` 推导返回 `Option<T>`
- `substitute` 测试：
  - 替换返回类型中的 `TypeVariable`
  - 递归替换嵌套泛型

**`reference/tests/checker/generics.test.ts`**（新建）：
- 移植 Rust 测试，验证 TS 端同步实现

**差分测试**：新 generics fixture MATCH

### 6.3 回归测试

- 所有现有 Rust 测试通过（当前 280 passed）
- 所有现有 TS 测试通过（当前 177 passed）
- Phase 4/5 回归测试通过
- 审计测试 0 failing

---

## 7. 成功标准

| # | 标准 | 验证方式 |
|---|------|---------|
| 1 | diff-ir.ps1: **11 MATCH + 0 SKIPPED** | `pwsh tests/audit/diff-ir.ps1` |
| 2 | 所有现有 Rust 测试通过 | `cargo test --workspace` |
| 3 | 新增泛型推导测试通过 | `cargo test --test phase6_generics_inference` |
| 4 | Clippy 零警告 | `cargo clippy --workspace --all-targets -- -D warnings` |
| 5 | 所有 TS 测试通过 | `cd reference && npm test` |
| 6 | TS 类型错误零 | `cd reference && npm run build` |
| 7 | 审计 0 failing | `pwsh tests/audit/run-audit.ps1` |

---

## 8. 出口闸门（8 项）

1. `cargo test --workspace` 全绿
2. `cargo clippy --workspace --all-targets -- -D warnings` 零警告
3. `pwsh tests/audit/run-audit.ps1` 0 failing
4. `pwsh tests/audit/diff-ir.ps1` **11 MATCH + 0 SKIPPED**
5. Phase 4/5 回归测试通过
6. Phase 6 新测试通过（`phase6_generics_inference` + ir-diff generics fixture）
7. `cd reference && npm test` 全绿
8. `cd reference && npm run build && npx tsc --noEmit` 零错误

---

## 9. 非目标（推迟到后续 Phase）

- **返回类型推导**（Phase 6c）：用户函数返回类型仍用 Any，不做全局推导
- **类型收窄**（Phase 6c）：if 语句内的类型细化（如 `if x is Some { ... }` 后 x 类型收窄）
- **Hindley-Milner 全局推导**：不实现，局部推导已满足需求
- **泛型在 codegen 中的表示**（Phase 6b）：JS/Py/Go emitter 对 `List<Int>` 等类型的处理
- **用户自定义泛型类型**：用户不能定义 `Foo<T>`，只有 stdlib 的 List/Map/Option/Set 是泛型
- **泛型约束**（如 `T: Comparable`）：不实现

---

## 10. 风险与缓解

| 风险 | 缓解 |
|------|------|
| stdlib 签名改泛型后现有测试失败 | 先运行基线测试，逐步改造模块，每改一个模块立即跑测试 |
| unify 算法边界情况（循环、深度嵌套） | 单元测试覆盖边界情况，递归深度依赖 Rust 栈（足够） |
| TS 端同步实现与 Rust 行为不一致 | 差分测试验证，Rust 作为参考实现 |
| Err/Ok 返回 Any 导致 propagation 误报 | propagation 对 Any 直接返回，不误报（已验证） |
| 泛型 fixture 差分测试不匹配 | 先确保 Rust 端正确，再对齐 TS 端 |
