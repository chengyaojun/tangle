# Phase 6b: 泛型在 IR 与 Codegen 中的表示 设计规格

> **本阶段属于 B5 类型系统扩展的第二阶段。** 完整 B5 通过分阶段实现：
> - **Phase 6a（已完成）：** TS 端对齐 Rust（闭合 order-service）+ Rust/TS 双端局部泛型推导
> - **Phase 6b（本规格）：** 泛型类型信息从 checker 贯通到 IR 与 codegen，Py/Go emitter 生成类型标注
> - **Phase 6c（后续）：** 返回类型推导 + 类型收窄（type narrowing）

**目标：** 让用户函数签名的类型注解（包括泛型 `List<Int>`、`Map<String, Order>` 等）从 Tangle 源码贯通到 IR JSON 与 Py/Go 生成代码，让类型信息成为 IR 的一等公民，并通过差分测试验证双端对齐。

**成功指标：** `tests/audit/diff-ir.ps1` 达成 **11 MATCH + 0 SKIPPED + 0 DIFF**（含新 generics fixture，且现有 10 个 fixture 不回归）；Py/Go emitter 输出含正确的类型标注。

---

## 1. 现状分析

### 1.1 IR 不携带类型信息

[graph.rs:83-93](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/graph.rs#L83-L93) 中 `IRFunction` 定义：

```rust
pub struct IRFunction {
    pub name: String,
    pub receiver: Option<String>,
    pub params: Vec<String>,          // ← 仅参数名，类型丢弃
    pub nodes: Vec<IRNode>,
    pub edges: Vec<IREdge>,
    pub entry_node_id: String,
    pub error_edges: Vec<IRErrorEdge>,
}
```

- `params: Vec<String>` 仅存参数名
- 无 `return_type` 字段
- `IRNode` / `IREdge` 也不携带类型信息

### 1.2 compile_to_ir 丢弃类型

[compile_to_ir.rs:146](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/compile_to_ir.rs#L146) 中 `collect_functions`：

```rust
let params: Vec<String> = h.params.iter().map(|p| p.name.clone()).collect();
```

`h.params` 的 `type_name: Option<String>` 字段被忽略。

### 1.3 三个 emitter 均为字符串翻译器

`js_emitter.rs` / `py_emitter.rs` / `go_emitter.rs` 均不引用 `crate::checker`，通过模式匹配翻译 Tangle 源码字符串，不使用任何 Type 信息。

### 1.4 管线中类型信息被丢弃

[run.rs:90-96](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/cli/run.rs#L90-L96) 中：

```
check_module(module) → CheckedModule{headings[].params[].type_name: Option<String>}
    ↓
compile_to_ir(&checked) → RuleGraph{functions[].params: Vec<String>}  ← 类型被丢弃
    ↓
emit_js/py/go(graph, module_name)  ← 字符串翻译，无类型
```

### 1.5 Tangle 无返回类型注解语法

[resolve.rs:31](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/resolve.rs#L31) 和 [resolve.rs:114](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/resolve.rs#L114) 中用户函数返回类型硬编码为 `Type::Any`。Tangle heading 语法不支持 `#### create(...): Order` 这类返回类型注解。

### 1.6 diff-ir.ps1 不含 v06_phase6 fixtures

[diff-ir.ps1:59](file:///e:/GitProjects/tangle/tests/audit/diff-ir.ps1#L59) 仅扫描 `tests/basic|errors|mvp|rules|structs`，`tests/v06_phase6/generics.tangle.md` 未参与差分测试。

### 1.7 type_parser 已支持泛型语法

[type_parser.rs:109-150](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/parser/type_parser.rs#L109-L150) 能解析 `Result<String, Error>`、`Array<Int>` 等泛型语法。Phase 6a 已为 `Type::GenericInstance` / `Type::Var` 添加构造函数与 unify 算法。

---

## 2. 方案选择

### 2.1 采用方案：IR 扩展为 IRParam 结构 + 语言相关类型映射

**核心思路：**
- `IRFunction.params: Vec<String>` → `Vec<IRParam>`，新增 `IRParam { name, type_: Option<Type> }`
- `IRFunction` 新增 `return_type: Option<Type>`（用户函数恒为 `None`，因 Tangle 无返回类型注解语法；返回推导推迟到 Phase 6c）
- `compile_to_ir.collect_functions` 解析 `param.type_name` 字符串 → `Type`，复用 `resolve.rs` 中已有的 `type_name_to_type` 函数（导出为 pub）
- 三个 emitter 接收带类型的 IRFunction，生成对应语言的类型标注
- `Type` 添加 `Serialize`/`Deserialize` derive，序列化形态 `{ kind: "primitive", name: "Int" }` 与 TS 端一致
- ir-diff 工具更新归一化逻辑处理新字段
- diff-ir.ps1 在 fixture 列表加入 `tests/v06_phase6/*.tangle.md`

### 2.2 不采用的方案

**方案 B：IR 存 type_name 原始字符串，emitter 自行解析**
- 劣势：类型解析逻辑在 4 处重复（Rust checker、Rust py_emitter、Rust go_emitter、TS reference 各 emitter）
- 劣势：与 Phase 6a 的 `Type::GenericInstance` 体系脱节，推导出的类型无法存入 IR
- 劣势：Phase 6c 实现返回推导时仍需重做

**方案 C：IR 不变，向 emitter 额外传 TypeEnv**
- 劣势：无法通过 diff-ir 验证类型对齐（TypeEnv 不进 IR JSON）
- 劣势：emitter 签名变重，与"IR 携带类型"目标矛盾

### 2.3 类型映射表

| Tangle Type | Python | Go | JS |
|---|---|---|---|
| `Int` | `int` | `int` | — |
| `String` | `str` | `string` | — |
| `Bool` | `bool` | `bool` | — |
| `Float` | `float` | `float64` | — |
| `List<T>` | `List[T]` | `[]T` | — |
| `Map<K,V>` | `Dict[K, V]` | `map[K]V` | — |
| `Option<T>` | `Optional[T]` | `*T` | — |
| `Set<T>` | `Set[T]` | `map[T]struct{}` | — |
| `Order`（struct） | `Order` | `Order` | — |
| `Any`/未标注 | 省略注解 | `any` | — |
| `Var`（未解析） | 省略注解 | `any` | — |
| `Function<...>` | `Callable` | `func()` | — |

---

## 3. IR Schema 扩展

### 3.1 新增 IRParam 结构

修改 [graph.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/graph.rs)：

```rust
/// IR 参数：name + 可选类型（来自 Tangle 源码注解 `param: TypeName`）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IRParam {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<Type>,
}
```

### 3.2 IRFunction 扩展

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IRFunction {
    pub name: String,
    pub receiver: Option<String>,
    pub params: Vec<IRParam>,          // ← Vec<String> → Vec<IRParam>
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<Type>,     // ← 新增；用户函数恒为 None
    pub nodes: Vec<IRNode>,
    pub edges: Vec<IREdge>,
    pub entry_node_id: String,
    pub error_edges: Vec<IRErrorEdge>,
}
```

### 3.3 Type 添加 serde derive

修改 [types.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/types.rs)：

```rust
// 现有：#[derive(Debug, Clone, PartialEq)]
// 改为：
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Type {
    Any,
    Primitive(PrimitiveType),
    Struct(StructType),
    Interface(InterfaceType),
    Function(FunctionType),
    GenericInstance(GenericTypeInstance),
    Var(TypeVariable),
}
```

- `tag = "kind"` 使序列化形态为 `{ "kind": "primitive", "name": "Int" }`，与 TS 端一致
- 所有变体的子结构（`PrimitiveType` / `StructType` / `InterfaceType` / `FunctionType` / `GenericTypeInstance` / `TypeVariable`）同步添加 `Serialize, Deserialize`
- `FunctionType.is_variadic` 字段保持现有 `camelCase` 序列化

### 3.4 IR JSON 输出示例

```json
{
  "functions": [{
    "name": "processItems",
    "receiver": null,
    "params": [
      {"name": "items", "type": {"kind": "genericInstance", "base": "List", "args": [{"kind": "primitive", "name": "Int"}]}}
    ],
    "nodes": [...],
    "entryNodeId": "n0",
    "errorEdges": []
  }]
}
```

- 未标注类型的参数 → `type` 字段省略（`skip_serializing_if = Option::is_none`）
- `returnType` 字段对用户函数省略

---

## 4. compile_to_ir 与 resolve.rs 改动

### 4.1 导出 type_name_to_type

修改 [resolve.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/resolve.rs)，将 `type_name_to_type` 改为 `pub`：

```rust
/// 解析 Tangle 类型注解字符串（如 "List<Int>"、"Order"、"String"）为 Type。
/// 解析失败返回 None（emitter 视为无注解）。
pub fn type_name_to_type(type_name: &str) -> Option<Type> {
    // ... 现有实现不变
}
```

在 `checker/mod.rs` 中 re-export：
```rust
pub use resolve::type_name_to_type;
```

### 4.2 修改 collect_functions

修改 [compile_to_ir.rs:129-164](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/compile_to_ir.rs#L129-L164) 的 `collect_functions`：

```rust
use crate::checker::resolve::type_name_to_type;
// ...

let params: Vec<IRParam> = h.params.iter().map(|p| IRParam {
    name: p.name.clone(),
    type_: p.type_name.as_ref().and_then(|tn| type_name_to_type(tn)),
}).collect();
// ...

out.push(IRFunction {
    name: name.clone(),
    receiver,
    params,
    return_type: None,  // Phase 6c 才实现返回推导
    nodes,
    edges,
    entry_node_id: entry_id,
    error_edges,
});
```

### 4.3 影响范围

- `IRParam` / `IRFunction` 字段变更触发所有构造点修改——目前仅 `collect_functions` 一处构造 `IRFunction`
- `create_graph` ([graph.rs:117](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/graph.rs#L117)) 不涉及 `IRFunction`，无需改
- 现有 fixtures（10 个）：参数大多无 `type_name` → `type_` 为 `None` → JSON 中 `type` 字段省略 → IR 输出仅多了 `params: [{name: "x"}]` 形态变化（从 `params: ["x"]`）

---

## 5. Codegen 类型映射

### 5.1 新建 codegen/type_map.rs

```rust
use crate::checker::types::*;

/// 将 Tangle Type 映射为 Python 类型注解字符串。
/// None 表示无注解（emitter 省略 `: ...`）。
pub fn tangle_type_to_py(ty: &Type) -> Option<String> {
    match ty {
        Type::Any => None,
        Type::Primitive(p) => Some(match p.name.as_str() {
            "Int" => "int".into(),
            "String" => "str".into(),
            "Bool" => "bool".into(),
            "Float" => "float".into(),
            other => other.into(),
        }),
        Type::Struct(s) => Some(s.name.clone()),
        Type::Interface(i) => Some(i.name.clone()),
        Type::GenericInstance(g) => match g.base.as_str() {
            "List" => format!("List[{}]", inner_py(&g.args[0])),
            "Map"  => format!("Dict[{}, {}]", inner_py(&g.args[0]), inner_py(&g.args[1])),
            "Option" => format!("Optional[{}]", inner_py(&g.args[0])),
            "Set"  => format!("Set[{}]", inner_py(&g.args[0])),
            other => format!("{}[{}]", other, g.args.iter().map(inner_py).collect::<Vec<_>>().join(", ")),
        },
        Type::Var(_) => None,
        Type::Function(_) => Some("Callable".into()),
    }
}

fn inner_py(ty: &Type) -> String {
    tangle_type_to_py(ty).unwrap_or_else(|| "Any".into())
}

/// 将 Tangle Type 映射为 Go 类型字符串。
/// Go 必须有返回类型，无注解时返回 "any"。
pub fn tangle_type_to_go(ty: &Type) -> String {
    match ty {
        Type::Any => "any".into(),
        Type::Primitive(p) => match p.name.as_str() {
            "Int" => "int".into(),
            "String" => "string".into(),
            "Bool" => "bool".into(),
            "Float" => "float64".into(),
            other => other.into(),
        },
        Type::Struct(s) => s.name.clone(),
        Type::Interface(i) => i.name.clone(),
        Type::GenericInstance(g) => match g.base.as_str() {
            "List" => format!("[]{}", inner_go(&g.args[0])),
            "Map"  => format!("map[{}]{}", inner_go(&g.args[0]), inner_go(&g.args[1])),
            "Option" => format!("*{}", inner_go(&g.args[0])),
            "Set"  => format!("map[{}]struct{{}}", inner_go(&g.args[0])),
            other => format!("any /* {} */", other),
        },
        Type::Var(_) => "any".into(),
        Type::Function(_) => "func()".into(),
    }
}

fn inner_go(ty: &Type) -> String {
    tangle_type_to_go(ty)
}
```

在 `codegen/mod.rs` 注册：`pub mod type_map; pub use type_map::*;`

### 5.2 Py emitter 改动

修改 [py_emitter.rs:244](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/py_emitter.rs#L244) `emit_single_function_py` 与 `emit_multi_function_py`：

```rust
fn format_py_param(p: &IRParam) -> String {
    match &p.type_ {
        Some(ty) => {
            if let Some(annot) = tangle_type_to_py(ty) {
                format!("{}: {}", p.name, annot)
            } else {
                p.name.clone()
            }
        }
        None => p.name.clone(),
    }
}

// 函数签名：
let params_str = func.params.iter().map(format_py_param).collect::<Vec<_>>().join(", ");
// emit: def {name}({params_str}):
```

**输出示例：**
```python
def process_items(items: List[int], threshold: int):    # 参数有注解
def main():                                              # 参数无注解
```

### 5.3 Go emitter 改动

修改 [go_emitter.rs:228](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/go_emitter.rs#L228) `emit_single_function_go` 与 `emit_multi_function_go`：

```rust
fn format_go_param(p: &IRParam) -> String {
    match &p.type_ {
        Some(ty) => format!("{} {}", p.name, tangle_type_to_go(ty)),
        None => format!("{} any", p.name),  // Go 必须有类型，无注解用 any
    }
}

// 函数签名（用户函数 return_type 恒为 None → 用 "any"）：
let ret_ty = func.return_type.as_ref().map(tangle_type_to_go).unwrap_or_else(|| "any".into());
// emit: func {name}({params_str}) {ret_ty} {
```

**输出示例：**
```go
func processItems(items []int, threshold int) any {    // 参数有注解
func main() any {                                       // 参数无注解
```

### 5.4 JS emitter 不改

JS 无类型注解语法，[js_emitter.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/js_emitter.rs) 保持字符串翻译，输出形态不变。

---

## 6. ir-diff 归一化与差分测试扩展

### 6.1 ir-diff 工具更新

`tests/audit/ir-diff/` 中的归一化逻辑需扩展：

1. **`params` 字段形态变化**：从 `Vec<String>` → `Vec<{name, type?}>`。比较逻辑需递归比较 `name` 和 `type_`。
2. **`return_type` 新字段**：归一化时 `None` 与缺失视为等价（null strip），与 Phase 5 A2-5 的 null 归一化策略一致。
3. **Type 嵌套归一化**：`{kind: "genericInstance", base: "List", args: [{kind: "primitive", name: "Int"}]}` 递归比较；键排序后字符串化。

**归一化规则：**

```rust
fn normalize_param(p: &Value) -> Value {
    // 确保 type_ 字段：None/missing 视为等价
    // 类型对象按 key 排序后比较
}

fn normalize_function(f: &mut Value) {
    // params: 逐项 normalize_param
    // return_type: 若为 null 删除该键
}
```

TS 端必须镜像相同归一化（Phase 5 A2-5 的 `null` strip 已存在于 TS 端，扩展即可）。

### 6.2 diff-ir.ps1 扩展

修改 [diff-ir.ps1:59](file:///e:/GitProjects/tangle/tests/audit/diff-ir.ps1#L59)：

```powershell
# 当前：
$fixtures = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md" -ErrorAction SilentlyContinue | ...

# 改后（新增 v06_phase6）：
$fixtures = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md","tests\v06_phase6\*.tangle.md" -ErrorAction SilentlyContinue | ...
```

### 6.3 预期差分测试结果

- 现有 10 个 fixture：IR 输出 `params` 形态变化，但 Rust/TS 双端一致 → 仍 MATCH（前提是 ir-diff 归一化正确处理新形态）
- 新增 generics fixture：MATCH
- 总计：11 MATCH + 0 SKIPPED + 0 DIFF

---

## 7. TS reference 端同步

| Rust 文件 | TS 对应文件 | 改动 |
|---|---|---|
| `ir/graph.rs` | `reference/src/ir/graph.ts` | `IRFunction.params: IRParam[]`、新增 `returnType?`、定义 `IRParam` interface |
| `checker/types.rs` | `reference/src/checker/types.ts` | Type 已有 `kind` 标签（Phase 6a 已同步），无需改 |
| `checker/resolve.rs` | `reference/src/checker/resolve.ts` | 导出 `typeNameToType`（对应 `type_name_to_type`） |
| `ir/compile_to_ir.rs` | `reference/src/ir/compileToIR.ts` | `collectFunctions` 用 `typeNameToType` 填充 `IRParam.type` |
| `codegen/type_map.rs` | `reference/src/codegen/typeMap.ts` | 新建：`tangleTypeToPy` / `tangleTypeToGo` |
| `codegen/py_emitter.rs` | `reference/src/codegen/pyEmitter.ts` | `formatPyParam` 用 typeMap |
| `codegen/go_emitter.rs` | `reference/src/codegen/goEmitter.ts` | `formatGoParam` 用 typeMap |

TS reference 端 IR JSON 须与 Rust 端逐字节一致（经 ir-diff 归一化后）。

---

## 8. fixture 与测试策略

### 8.1 fixture 改造

当前 `tests/v06_phase6/generics.tangle.md` 仅 `### main` 无参数，无法验证签名级类型。**改造为多函数 + 带类型注解**：

```markdown
# Generic Type Inference Test

### ItemProcessor

#### process

* `items`: List of integers to double (List<Int>)
* `threshold`: Cutoff value (Int)

```@tangle
let doubled = List.map(items, fn(x) { x * 2 })
let filtered = List.filter(doubled, fn(x) { x > threshold })
return filtered
```

### main

```@tangle
let numbers = [1, 2, 3]
let result = ItemProcessor.process(numbers, 2)
return result
```
```

**已验证：** [blocks.rs:34-37](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/frontend/blocks.rs#L34-L37) 中 `parse_param_item` 从列表项最后的 `( ... )` 对中提取 `type_name`。因此 `* \`items\`: ... (List<Int>)` 会提取 `type_name = Some("List<Int>")`，再经 `type_name_to_type`（[resolve.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/resolve.rs)）调用 type_parser（Phase 6a 已确认支持 `List<Int>` 解析）转为 `Type::GenericInstance`。无需降级。

**降级备选（仅当实现时发现 type_parser 边界问题）：** 若 `List<Int>` 解析失败，fixture 改用 `items: List`（解析为 Struct），并在 type_parser 单元测试中单独覆盖泛型解析；IR 集成测试改用直接构造 `IRParam { type_: Some(generic("List", vec![prim("Int")])) }` 验证序列化。

### 8.2 测试矩阵

| 测试类型 | 位置 | 内容 |
|---|---|---|
| Rust 单元测试 | `compiler/tangle-cli/src/ir/graph.rs` `#[cfg(test)]` | IRParam 序列化、Type 序列化含 `kind` 标签 |
| Rust 单元测试 | `compiler/tangle-cli/src/codegen/type_map.rs` `#[cfg(test)]` | `tangle_type_to_py/go` 各类型映射 |
| Rust 集成测试 | `compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs`（新建） | 加载 generics fixture，验证 IRFunction.params 含正确类型 |
| Rust 回归测试 | `compiler/tangle-cli/tests/v03_phase1/*.rs` 等 | 现有测试不应回归；若断言 `params: ["x"]` 需更新为 `params: [{name: "x"}]` |
| TS 单元测试 | `reference/tests/checker/generics.test.ts` 扩展 | typeNameToType 解析泛型 |
| TS 单元测试 | `reference/tests/codegen/typeMap.test.ts`（新建） | tangleTypeToPy/Go 映射 |
| 差分测试 | `tests/audit/diff-ir.ps1` | 11 MATCH + 0 SKIPPED + 0 DIFF（含新 generics fixture） |

### 8.3 回归测试扫描

实现前需扫描 `compiler/tangle-cli/tests/` 下所有 `.rs` 文件，查找：
- `params:` 字符串数组断言（如 `assert_eq!(func.params, vec!["x".to_string()])`）
- IR JSON 快照中的 `"params":["..."]` 形态

更新为对象数组形态：`assert_eq!(func.params, vec![IRParam { name: "x".into(), type_: None }])`。

### 8.4 Py/Go emitter 输出验证

集成测试断言 emitter 输出字符串含期望的类型标注：
- Py：`def process_items(items: List[int], threshold: int):`
- Go：`func processItems(items []int, threshold int) any {`
- JS：`function processItems(items, threshold) {`（不变）

---

## 9. 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `compiler/tangle-cli/src/ir/graph.rs` | IRParam 结构、IRFunction 扩展 | 修改 |
| `compiler/tangle-cli/src/checker/types.rs` | Type 及子结构添加 Serialize/Deserialize | 修改 |
| `compiler/tangle-cli/src/checker/resolve.rs` | type_name_to_type 导出为 pub | 修改 |
| `compiler/tangle-cli/src/checker/mod.rs` | re-export type_name_to_type | 修改 |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | collect_functions 用 IRParam + type_name_to_type | 修改 |
| `compiler/tangle-cli/src/codegen/type_map.rs` | tangle_type_to_py / tangle_type_to_go | 创建 |
| `compiler/tangle-cli/src/codegen/mod.rs` | 注册 type_map 模块 | 修改 |
| `compiler/tangle-cli/src/codegen/py_emitter.rs` | format_py_param + 函数签名生成 | 修改 |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | format_go_param + 函数签名生成 | 修改 |
| `compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs` | IR 参数类型集成测试 | 创建 |
| `compiler/tangle-cli/Cargo.toml` | 注册新测试 | 修改 |
| `tests/audit/ir-diff/src/main.rs` | params 归一化 + return_type null strip | 修改 |
| `tests/audit/diff-ir.ps1` | 新增 v06_phase6 fixture 路径 | 修改 |
| `tests/v06_phase6/generics.tangle.md` | 改造为多函数 + 类型注解 | 修改 |
| `reference/src/ir/graph.ts` | IRParam interface、IRFunction 扩展 | 修改 |
| `reference/src/checker/resolve.ts` | 导出 typeNameToType | 修改 |
| `reference/src/ir/compileToIR.ts` | collectFunctions 用 typeNameToType | 修改 |
| `reference/src/codegen/typeMap.ts` | tangleTypeToPy / tangleTypeToGo | 创建 |
| `reference/src/codegen/pyEmitter.ts` | formatPyParam | 修改 |
| `reference/src/codegen/goEmitter.ts` | formatGoParam | 修改 |
| `reference/tests/codegen/typeMap.test.ts` | TS 类型映射测试 | 创建 |

**总计**：修改 17 个文件，创建 4 个文件。

---

## 10. 成功标准

| # | 标准 | 验证方式 |
|---|------|---------|
| 1 | `cargo test --workspace` 全绿 | 含新增 type_map 测试、IRParam 序列化测试、回归测试更新 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` 零警告 | |
| 3 | `cd reference && npm test` 全绿 | 含新增 typeMap 测试 |
| 4 | `cd reference && npm run build` 零类型错误 | TS 编译通过 |
| 5 | `pwsh tests/audit/diff-ir.ps1` **11 MATCH + 0 SKIPPED + 0 DIFF** | 含新 generics fixture，且现有 10 个不回归 |
| 6 | `pwsh tests/audit/run-audit.ps1` 0 failing | |
| 7 | Py emitter 输出含 `def f(x: int, items: List[int]):` 形态 | 集成测试断言 |
| 8 | Go emitter 输出含 `func f(x int, items []int) any {` 形态 | 集成测试断言 |
| 9 | JS emitter 输出**不变**（无类型注解） | 回归测试断言 |

---

## 11. 出口闸门（9 项）

1. `cargo test --workspace` 全绿
2. `cargo clippy --workspace --all-targets -- -D warnings` 零警告
3. `cd reference && npm test` 全绿
4. `cd reference && npm run build` 零类型错误
5. `pwsh tests/audit/diff-ir.ps1` **11 MATCH + 0 SKIPPED + 0 DIFF**
6. `pwsh tests/audit/run-audit.ps1` 0 failing
7. Phase 4/5/6a 回归测试通过
8. Phase 6b 新测试通过（type_map + IRParam 序列化 + fixture 集成测试）
9. 手动验证 Py/Go emitter 输出：对 generics fixture 运行 `cargo run -- build ... --emit py` / `--emit go`，肉眼确认类型标注正确

---

## 12. 非目标（推迟到 Phase 6c 或更晚）

- **返回类型推导**（Phase 6c）：用户函数 `return_type` 恒为 `None`，不实现从 `return` 语句推导
- **类型收窄**（Phase 6c）：if 语句内的类型细化（如 `if x is Some { ... }` 后 x 类型收窄）
- **局部变量类型注解**：仅函数签名级，let 绑定不生成 Py `x: int = ...` / Go 显式类型声明
- **IRNode 类型字段**：节点级不携带类型信息
- **JS 类型注解**：JS 无类型系统，不做 JSDoc 生成
- **类型检查器增强**：泛型推导仍仅作用于 stdlib 调用，不扩展到用户函数体
- **类型映射的完备性**：`Function<T,U>` 等复杂类型 → Py `Callable` / Go `func()`，不做精确签名映射
- **用户自定义泛型类型**：用户不能定义 `Foo<T>`，只有 stdlib 的 List/Map/Option/Set 是泛型
- **泛型约束**（如 `T: Comparable`）：不实现

---

## 13. 风险与缓解

| 风险 | 缓解 |
|---|---|
| ir-diff 归一化遗漏导致现有 MATCH 退化为 DIFF | 先单独跑一次差分测试，逐个 fixture 检查输出；归一化单元测试覆盖 params 形态变化 |
| heading parser 不支持 `List<Int>` 语法 | 实现前先用 `cargo test` 验证；若不支持，fixture 降级为 `items: List`（解析为 Struct）+ 单独写 type_parser 单元测试覆盖泛型解析 |
| TS 端 Type 序列化与 Rust 不一致（如键顺序、null 处理） | 复用 Phase 5 A2-5 的 null strip 归一化；TS 端 `JSON.stringify` 后由 ir-diff 统一排序 |
| Py/Go emitter 类型映射错误（如 `Map<K,V>` → Go 语法错） | type_map 单元测试覆盖所有 Type 变体；emitter 集成测试断言输出字符串 |
| Type 添加 serde derive 后破坏现有 Phase 6a 测试 | Phase 6a 测试主要在 `checker/types.rs` 内部 `#[cfg(test)]`，不依赖序列化形态；运行全量测试验证 |

---

## 14. 版本与分支

- 新分支：`phase6b/v0.6.1`，基于 `main`
- worktree 路径：`.worktrees/phase6b-v0.6.1`
- 完成后合并到 `main`，打 tag `v0.6.1`
- **tag v0.6.1 不 push remote，直到用户批准**
