# Phase 6c: 返回类型推断 + 类型收窄 设计规格

> **本阶段属于 B5 类型系统扩展的第三阶段。** 完整 B5 通过分阶段实现：
> - **Phase 6a（已完成）：** TS 端对齐 Rust（闭合 order-service）+ Rust/TS 双端局部泛型推导
> - **Phase 6b（已完成）：** 泛型类型信息从 checker 贯通到 IR 与 codegen，Py/Go emitter 生成类型标注
> - **Phase 6c（本规格）：** 返回类型推断 + Match arm 类型收窄

**目标：** 让用户函数的返回类型从函数体中的 `return` 语句自动推断，并通过对 `match` 表达式 arm 的类型收窄提升推断准确性，使 `IRFunction.return_type` 字段被填充，IR JSON 成为类型信息的完整载体。

**成功指标：** `tests/audit/diff-ir.ps1` 达成 **15 MATCH + 0 SKIPPED + 0 DIFF**（12 现有 + 3 新增 fixture，Rust/TS 双端推断结果一致）；IR JSON 中函数携带 `returnType` 字段；emitter 外部签名保持向后兼容。

---

## 1. 现状分析

### 1.1 用户函数返回类型硬编码为 Any

[resolve.rs:35](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/resolve.rs#L35) 中 `resolve_types` 将所有用户函数的 `returns` 硬编码为 `Type::Any`：

```rust
env.functions.insert(
    name.clone(),
    FunctionType {
        params,
        returns: Box::new(Type::Any),  // ← 硬编码
        is_variadic: false,
    },
);
```

Tangle heading 语法不支持 `#### create(...): Order` 返回类型注解。

### 1.2 IRFunction.return_type 恒为 None

[compile_to_ir.rs:159](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/compile_to_ir.rs#L159) 中 `collect_functions` 硬编码：

```rust
out.push(IRFunction {
    // ...
    return_type: None,  // ← Phase 6b 留下的占位
    // ...
});
```

### 1.3 Match 表达式不收窄、不推断返回类型

[check.rs:228-246](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L228-L246) 中 `Expr::Match` 分支：

```rust
Expr::Match(e) => {
    let (matched_ty, ...) = check_expression(&e.expr, env);
    for arm in &e.arms {
        let (_, ...) = check_expression(&arm.body, env);  // ← 无收窄，binding 类型未注入
    }
    Type::Primitive(PrimitiveType { name: "Bool".into() })  // ← 恒返回 Bool
}
```

- arm body 检查时未注入 pattern binding 的收窄类型
- 表达式结果类型恒为 `Bool`，无法用于 `let x = match ...` 推断

### 1.4 If 表达式仅返回 then 分支类型

[check.rs:199-209](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L199-L209) 中 `Expr::If` 分支返回 `then_ty`，忽略 `else_branch` 类型，无统一逻辑。

### 1.5 穷尽性检查仅支持 Primitive variant

[match_check.rs:10-18](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/match_check.rs#L10-L18) 中 `check_match_exhaustiveness` 仅处理 `Type::Primitive` variant，无法识别 `Type::Struct` 或 `Type::GenericInstance` variant。

### 1.6 Go emitter 隐患

[go_emitter.rs:270-271](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/go_emitter.rs#L270-L271)：

```rust
let ret_ty = func.return_type.as_ref().map(tangle_type_to_go).unwrap_or_else(|| "Result".into());
```

Phase 6b 中 `return_type` 恒为 None，此代码路径从未触发。Phase 6c 填充 return_type 后，若不改此行，Go 会输出 `func f() int {` 而 body 仍 `return Ok(value)`，编译失败。

### 1.7 Phase 6b 已具备的基础设施

- `Type` 枚举 8 个变体（含 `Sum`、`GenericInstance`）全部支持 serde（`kind` tag）
- `IRParam { name, type_: Option<Type> }` 已携带参数类型
- `IRFunction.return_type: Option<Type>` 字段已存在
- `type_name_to_type`（[resolve.rs:165](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/resolve.rs#L165)）能解析 `List<Int>`、`Some<Int>`、`(A | B)` 等泛型与 Sum 语法
- `unify` / `substitute`（[unify.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/unify.rs)）已支持 type_var 绑定与递归替换
- stdlib 签名注册表已支持泛型 `type_var(0)`、`type_var(1)`

---

## 2. 方案选择

### 2.1 采用方案：独立 `infer_return_types` pass + Match arm 收窄

**核心思路：**
- 在 `check_module` 之后新增独立 pass `infer_return_types`，遍历每个 Callable heading 的 parsed_blocks，收集 return 语句的类型并统一
- 改进 `check_expression` 的 `Expr::Match` 分支：注入 arm binding 的收窄类型到局部 env，返回所有 arm body 类型的统一结果
- 改进 `check_expression` 的 `Expr::If` 分支：统一 then/else 分支类型
- 扩展 `check_match_exhaustiveness` 支持 Struct/GenericInstance variant
- `compile_to_ir.collect_functions` 读取推断结果填充 `IRFunction.return_type`
- Emitter 外部签名保持 `Result` 外包（设计 A），return_type 作为 IR 元数据

### 2.2 不采用的方案

**方案 B：扩展 check_module.rs 内联推断**
- 劣势：`check_module.rs` 已 185 行，再扩展会职责纠缠
- 劣势：难以单独测试推断逻辑

**方案 C：在 compile_to_ir 内联推断**
- 劣势：类型检查逻辑泄漏到 IR lowering 层
- 劣势：`compile_to_ir` 无法访问完整 TypeEnv（局部变量类型在 check_module 的 block_env 中，未持久化）
- 劣势：Match arm 收窄需要局部环境，难以在 IR 层实现

### 2.3 关键设计决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| 范围拆分 | 合并为一个 Phase 6c | 收窄与推断紧耦合：收窄提升推断准确性，单独交付价值有限 |
| 收窄范围 | 仅 Match arm 收窄 | 覆盖最常见场景；If 条件收窄需新语法（`is` 关键字），推迟 |
| return_type 用途 | 设计 A：逻辑类型 + Result 外包 | 向后兼容，emitter 不改外部签名，return_type 作 IR 元数据 |
| 冲突处理 | 最佳努力，统一失败回退 Any | 不产生新 diagnostic，保持 TS 兼容（TS fatal on TANGLE_TYPE_*） |
| TS reference | 同步实现推断 + 收窄 | 差分测试要求双端 IR JSON 一致 |

---

## 3. 架构概览

### 3.1 数据流

```
TangleModule
    ↓
check_module(module)                          ← 现有，扩展
    ↓
CheckedModule { type_env, parsed_blocks, headings, return_types }
    ↓                                                      ↑
infer_return_types(&checked) → HashMap<heading_id, Type>   ← 新增 pass
    │  遍历每个 Callable heading 的 parsed_blocks
    │  对每个 Return stmt 调用 check_expression（含改进的 Match/If）
    │  统一所有 return 路径类型
    ↓
compile_to_ir(&checked)                       ← 读取 return_types
    ↓
RuleGraph { functions[].return_type: Option<Type> }   ← 字段被填充
    ↓
emit_js / emit_py / emit_go                   ← 设计 A：emitter 外部签名不变
```

### 3.2 新增/修改文件清单

| 文件 | 操作 | 职责 |
|------|------|------|
| `compiler/tangle-cli/src/checker/infer_return_types.rs` | **新建** | 推断 pass：遍历函数体，收集+统一 return 类型 |
| `compiler/tangle-cli/src/checker/mod.rs` | 修改 | 注册新模块 + re-export |
| `compiler/tangle-cli/src/checker/check.rs` | 修改 | `Expr::Match` arm 收窄 + 返回统一类型；`Expr::If` 统一 then/else |
| `compiler/tangle-cli/src/checker/unify.rs` | 修改 | 新增 `unify_all` / `unify_pair` 共享辅助函数 |
| `compiler/tangle-cli/src/checker/match_check.rs` | 修改 | 扩展穷尽性检查支持 Struct/GenericInstance variant |
| `compiler/tangle-cli/src/checker/check_module.rs` | 修改 | 调用 `infer_return_types` 填充 `CheckedModule.return_types` |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | 修改 | `collect_functions` 读取 return_types 填充 `IRFunction.return_type` |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | 修改 | 外部签名恒为 `Result`（移除 return_type 映射） |
| `reference/src/checker/inferReturnTypes.ts` | **新建** | TS 端镜像推断算法 |
| `reference/src/checker/check.ts` | 修改 | TS 端 Match/If 表达式类型改进 + arm 收窄 |
| `reference/src/checker/matchCheck.ts` | 修改 | 扩展穷尽性检查 |
| `reference/src/checker/checkModule.ts` | 修改 | 调用 `inferReturnTypes` |
| `reference/src/ir/compileToIR.ts` | 修改 | `collectFunctions` 读取 returnTypes |

### 3.3 CheckedModule 扩展

```rust
#[derive(Debug, Clone)]
pub struct CheckedModule {
    // ... 现有字段 ...
    pub type_env: TypeEnv,
    /// 函数 heading_id → 推断出的返回类型（Phase 6c 新增）
    pub return_types: HashMap<String, Type>,
}
```

`check_module` 末尾调用 `infer_return_types` 填充此字段。key 使用 **heading_id**（而非函数名），避免不同 Type 下同名方法（如 `Order.create` 与 `User.create`）冲突。

---

## 4. Match arm 类型收窄算法

### 4.1 Variant 命名与 binding 类型规则

| Variant 类型 | Variant 名 | Binding 类型 |
|---|---|---|
| `Type::Primitive(p)` | `p.name` | `Type::Primitive(p)` 本身 |
| `Type::Struct(s)` | `s.name` | `Type::Struct(s)` 本身 |
| `Type::GenericInstance(g)` | `g.base` | `g.args[0]`（payload 类型，若 args 为空则 `Type::Any`） |
| 其他（Sum/Function/Var/Any/Interface） | 不支持作为命名 variant | — |

**设计理由：**
- Primitive/Struct variant：binding 获得 variant 类型本身（语义为"匹配到此类型，绑定整个值"）
- GenericInstance variant：binding 获得 payload（如 `Some(y)` 中 `y` 是 inner type，而非整个 `Some<Int>`），更实用

### 4.2 收窄流程

修改 [check.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs) 的 `Expr::Match` 分支：

```rust
Expr::Match(e) => {
    let (matched_ty, mut diags) = check_expression(&e.expr, env);
    let mut arm_types = vec![];

    for arm in &e.arms {
        // 构造收窄后的 arm 局部环境
        let mut arm_env = env.clone();
        if let Type::Sum(ref sum) = matched_ty {
            if let MatchPattern::Variant { ref name, ref binding } = arm.pattern {
                if let Some(variant_ty) = find_variant_by_name(sum, name) {
                    if let Some(ref bind_name) = binding {
                        let bind_ty = binding_type_of(variant_ty);
                        arm_env.variables.insert(bind_name.clone(), bind_ty);
                    }
                }
            }
        }
        let (arm_ty, mut arm_diags) = check_expression(&arm.body, &arm_env);
        diags.append(&mut arm_diags);
        arm_types.push(arm_ty);
    }

    // 穷尽性检查（扩展支持 Struct/GenericInstance variant）
    if let Type::Sum(ref sum) = matched_ty {
        let missing = check_match_exhaustiveness(sum, &e.arms);
        for m in missing {
            diags.push(TangleDiagnostic {
                code: "TANGLE_MATCH_NOT_EXHAUSTIVE".into(),
                message: format!("Match not exhaustive: missing variant '{}'", m),
                span: e.span.clone(),
            });
        }
    }

    // 返回类型：统一所有 arm body 类型（最佳努力，失败回退 Any）
    unify_all(&arm_types).unwrap_or(Type::Any)
}
```

### 4.3 辅助函数

```rust
/// 在 Sum 的 variants 中按名查找
fn find_variant_by_name<'a>(sum: &'a SumType, name: &str) -> Option<&'a Type> {
    sum.variants.iter().find(|v| variant_name(v).as_deref() == Some(name))
}

/// 提取 variant 名（Primitive/Struct/GenericInstance）
fn variant_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Primitive(p) => Some(p.name.clone()),
        Type::Struct(s) => Some(s.name.clone()),
        Type::GenericInstance(g) => Some(g.base.clone()),
        _ => None,
    }
}

/// 提取 binding 类型
fn binding_type_of(variant_ty: &Type) -> Type {
    match variant_ty {
        Type::GenericInstance(g) => g.args.first().cloned().unwrap_or(Type::Any),
        other => other.clone(),
    }
}
```

### 4.4 穷尽性检查扩展

修改 [match_check.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/match_check.rs)：

```rust
pub fn check_match_exhaustiveness(sum: &SumType, arms: &[MatchArm]) -> Vec<String> {
    let has_wildcard = arms.iter().any(|a| matches!(a.pattern, MatchPattern::Wildcard));
    if has_wildcard { return vec![]; }

    let mut missing = vec![];
    for variant in &sum.variants {
        let var_name = variant_name(variant);  // 复用 check.rs 中的辅助函数
        if let Some(name) = var_name {
            let covered = arms.iter().any(|a| match &a.pattern {
                MatchPattern::Variant { name: pn, .. } => pn == &name,
                _ => false,
            });
            if !covered { missing.push(name); }
        }
    }
    missing
}
```

### 4.5 If 表达式类型改进

修改 [check.rs:199-209](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check.rs#L199-L209)：

```rust
Expr::If(e) => {
    let (_, mut diags) = check_expression(&e.condition, env);
    let (then_ty, mut then_diags) = check_expression(&e.then_branch, env);
    diags.append(&mut then_diags);
    let ty = if let Some(ref else_b) = e.else_branch {
        let (else_ty, mut else_diags) = check_expression(else_b, env);
        diags.append(&mut else_diags);
        unify_pair(&then_ty, &else_ty).unwrap_or_else(|| then_ty.clone())
    } else {
        then_ty
    };
    ty
}
```

**注意：** Phase 6c 不实现 If 分支内的类型收窄（仅 Match arm 收窄在范围内）。If 表达式仅改进返回类型统一。

---

## 5. 返回类型推断算法

### 5.1 新建模块 `checker/infer_return_types.rs`

**核心函数：**

```rust
/// 为模块中所有 Callable heading 推断返回类型。
/// 返回 heading_id → Type 映射。
pub fn infer_return_types(checked: &CheckedModule) -> HashMap<String, Type> {
    let mut result = HashMap::new();
    collect(&checked.headings, checked, &mut result);
    result
}

fn collect(headings: &[TangleHeading], checked: &CheckedModule, out: &mut HashMap<String, Type>) {
    for h in headings {
        if h.role == HeadingRole::Callable && !h.code_blocks.is_empty() {
            if let Some(ty) = infer_function_return_type(h, checked) {
                out.insert(h.id.clone(), ty);
            }
        }
        collect(&h.children, checked, out);
    }
}
```

### 5.2 单函数推断流程

```rust
fn infer_function_return_type(
    heading: &TangleHeading,
    checked: &CheckedModule,
) -> Option<Type> {
    // 1. 构造与 check_module 一致的 block_env
    let mut env = checked.type_env.clone();
    setup_receiver_and_params(heading, checked, &mut env);

    // 2. 遍历该 heading 的所有 @tangle blocks，收集 return 类型
    let mut return_types: Vec<Type> = vec![];
    for block in &checked.parsed_blocks {
        if block.heading_id != heading.id { continue; }
        let mut block_env = env.clone();
        for stmt in &block.body.statements {
            match stmt {
                Stmt::Let(s) => {
                    let (ty, _) = check_expression(&s.value, &block_env);
                    block_env.variables.insert(s.name.clone(), ty);
                }
                Stmt::Const(s) => {
                    let (ty, _) = check_expression(&s.value, &block_env);
                    block_env.variables.insert(s.name.clone(), ty);
                }
                Stmt::Return(s) => {
                    if let Some(ref value) = s.value {
                        let (ty, _) = check_expression(value, &block_env);
                        return_types.push(ty);
                    }
                    // `return;`（无值）不贡献类型
                }
                Stmt::Expression(_) => {}
            }
        }
    }

    // 3. 统一所有 return 类型
    if return_types.is_empty() {
        None  // 无 return 语句 → None（向后兼容，emitter 用 Result 回退）
    } else {
        Some(unify_all(&return_types).unwrap_or(Type::Any))
    }
}
```

### 5.3 统一算法（共享辅助函数）

在 [unify.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/unify.rs) 中新增两个共享辅助函数，供 `check.rs`（Match arm 统一、If then/else 统一）与 `infer_return_types.rs`（return 路径统一）共同使用：

```rust
/// 统一类型列表：以第一个为锚点，逐个 unify。
/// 成功返回统一后的类型（含 type_var 替换）；失败返回 None。
pub fn unify_all(types: &[Type]) -> Option<Type> {
    if types.is_empty() { return None; }
    let mut subst: Substitution = HashMap::new();
    let anchor = &types[0];
    for other in &types[1..] {
        if unify(anchor, other, &mut subst).is_err() {
            return None;
        }
    }
    Some(substitute(anchor, &subst))
}

/// 统一两个类型（用于 If then/else）
pub fn unify_pair(a: &Type, b: &Type) -> Option<Type> {
    let mut subst: Substitution = HashMap::new();
    match unify(a, b, &mut subst) {
        Ok(()) => Some(substitute(a, &subst)),
        Err(_) => None,
    }
}
```

**位置决策：** 放在 `unify.rs` 而非 `infer_return_types.rs`，因为 `check.rs` 的 Match/If 分支也需要这些函数。`unify.rs` 已是类型统一逻辑的归属模块。

### 5.4 统一行为示例

| return 路径 | 统一结果 |
|---|---|
| `[Int, Int]` | `Int` |
| `[List<Int>, List<Int>]` | `List<Int>` |
| `[Var(0), Int]` | `Int`（type_var 被绑定） |
| `[Int, String]` | 冲突 → `Any` |
| `[Int, Any]` | `Any`（Any 双向通配） |
| `[]`（无 return） | `None`（向后兼容） |
| `[Int]`（仅一个） | `Int` |

### 5.5 环境构造辅助函数

```rust
/// 构造函数体的类型环境：设置 receiver、注入 heading params。
fn setup_receiver_and_params(
    heading: &TangleHeading,
    checked: &CheckedModule,
    env: &mut TypeEnv,
) {
    if let Some(parent) = find_receiver_heading(&heading.id, &checked.headings) {
        let struct_name = parent.symbol_name.clone()
            .unwrap_or_else(|| parent.title.clone());
        let fields = parent.params.iter()
            .map(|p| (p.name.clone(), param_type_of(p)))
            .collect();
        env.receiver = Some(ReceiverContext { struct_name, fields });
    }
    for p in &heading.params {
        env.variables.insert(p.name.clone(), param_type_of(p));
    }
}

/// 用 type_name_to_type 解析参数类型
fn param_type_of(p: &TangleParam) -> Type {
    p.type_name.as_ref()
        .and_then(type_name_to_type)
        .unwrap_or(Type::Any)
}
```

**附带改进：** `param_type_of` 使用 Phase 6b 的 `type_name_to_type`（支持 `List<Int>` 等泛型），比 [check_module.rs:117-123](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check_module.rs#L117-L123) 现有硬编码 match 更准确。

### 5.6 CheckedModule 集成

修改 [check_module.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/check_module.rs) 的 `check_module`：

```rust
pub fn check_module(module: TangleModule) -> CheckedModule {
    // ... 现有逻辑 ...
    let mut checked = CheckedModule {
        // ... 现有字段 ...
        return_types: HashMap::new(),
    };
    checked.return_types = infer_return_types(&checked);
    checked
}
```

---

## 6. IR 与 compile_to_ir 改动

### 6.1 IR Schema（无改动）

`IRFunction.return_type: Option<Type>` 字段在 Phase 6b 已存在（[graph.rs:100-101](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/graph.rs#L100-L101)），无需 schema 变更。

### 6.2 compile_to_ir 改动

修改 [compile_to_ir.rs:130-169](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/ir/compile_to_ir.rs#L130-L169) 的 `collect_functions`：

```rust
fn collect_functions(
    headings: &[TangleHeading],
    parent: Option<&TangleHeading>,
    parsed_blocks: &[ParsedCodeBlock],
    id_gen: &mut FreshNodeId,
    return_types: &HashMap<String, Type>,  // ← 新增参数，key = heading_id
    out: &mut Vec<IRFunction>,
) {
    for h in headings {
        if h.role == HeadingRole::Callable && !h.code_blocks.is_empty() {
            if let Some(ref name) = h.symbol_name {
                // ... 现有 receiver/params/nodes 逻辑不变 ...
                let return_type = return_types.get(&h.id).cloned();
                out.push(IRFunction {
                    name: name.clone(),
                    receiver,
                    params,
                    return_type,  // ← 从 return_types 查找
                    nodes,
                    edges,
                    entry_node_id: entry_id,
                    error_edges,
                });
            }
        }
        collect_functions(&h.children, Some(h), parsed_blocks, id_gen, return_types, out);
    }
}
```

`compile_to_ir` 主函数传入 `&checked.return_types`：

```rust
if has_main {
    let mut functions: Vec<IRFunction> = vec![];
    collect_functions(&checked.headings, None, &checked.parsed_blocks,
                      &mut id_gen, &checked.return_types, &mut functions);
    graph.functions = functions;
}
```

### 6.3 IR JSON 输出示例

**改前（Phase 6b，return_type 恒为 None，字段省略）：**
```json
{
  "name": "process",
  "params": [{"name": "items", "type": {"kind": "genericInstance", "base": "List", "args": [{"kind": "primitive", "name": "Int"}]}}],
  "nodes": [...],
  "entryNodeId": "n0"
}
```

**改后（Phase 6c，return_type 被填充）：**
```json
{
  "name": "process",
  "params": [{"name": "items", "type": {"kind": "genericInstance", "base": "List", "args": [{"kind": "primitive", "name": "Int"}]}}],
  "returnType": {"kind": "genericInstance", "base": "List", "args": [{"kind": "primitive", "name": "Int"}]},
  "nodes": [...],
  "entryNodeId": "n0"
}
```

**冲突场景（return_type = Any）：**
```json
{
  "name": "ambiguous",
  "returnType": {"kind": "any"}
}
```

**无 return 语句（return_type = None，字段省略，向后兼容）：**
```json
{
  "name": "noReturn"
}
```

---

## 7. Emitter 改动（设计 A — 最小化）

### 7.1 Py emitter：无改动

外部签名保持 `def f(...) -> Result:`（[py_emitter.rs:287](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/py_emitter.rs#L287)）。return_type 作为 IR 元数据，不体现在外部签名。

### 7.2 Go emitter：修复隐患

修改 [go_emitter.rs:270-271](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/go_emitter.rs#L270-L271)：

```rust
// 改前（Phase 6b 留下的隐患代码）：
let ret_ty = func.return_type.as_ref().map(tangle_type_to_go).unwrap_or_else(|| "Result".into());

// 改后（Phase 6c 设计 A：外部签名恒为 Result）：
let ret_ty = "Result";
```

**理由：** Phase 6b 的 `return_type` 恒为 None，该代码路径从未触发。Phase 6c 填充 return_type 后，若不改此行，Go 会输出 `func f() int {` 而 body 仍 `return Ok(value)`，编译失败。设计 A 要求外部签名不变。

### 7.3 JS emitter：无改动

JS 无类型注解，[js_emitter.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/js_emitter.rs) 保持字符串翻译。

### 7.4 return_type 的可见出口

IR JSON 输出（[ir_json.rs](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/codegen/ir_json.rs)）会序列化 `returnType` 字段，供 LSP/文档生成/差分测试消费。

---

## 8. TS reference 同步

### 8.1 文件对应

| Rust 文件 | TS 对应文件 | 改动 |
|---|---|---|
| `checker/infer_return_types.rs`（新建） | `reference/src/checker/inferReturnTypes.ts`（新建） | 镜像推断算法 |
| `checker/check.rs` Match 分支 | `reference/src/checker/check.ts` Match 分支 | arm 收窄 + 返回统一类型 |
| `checker/check.rs` If 分支 | `reference/src/checker/check.ts` If 分支 | 统一 then/else 类型 |
| `checker/match_check.rs` | `reference/src/checker/matchCheck.ts` | 扩展支持 Struct/GenericInstance variant |
| `checker/check_module.rs` | `reference/src/checker/checkModule.ts` | 调用 `inferReturnTypes` 填充 `CheckedModule.returnTypes` |
| `ir/compile_to_ir.rs` | `reference/src/ir/compileToIR.ts` | `collectFunctions` 读取 returnTypes 填充 `IRFunction.returnType` |

### 8.2 TS 端注意事项

- TS CLI 把 `TANGLE_TYPE_*` 当 fatal（exit 1，不输出 IR）。fixture 必须不触发 TS 类型错误
- `typeMap.ts` 中 `PrimitiveType.name` 联合类型为 `"String"|"Int"|"Bool"`（无 Float），新增代码需注意 `switch (ty.name as string)` 模式
- `args[N]` 需用 `!` 断言（`noUncheckedIndexedAccess` 严格模式）
- `inferReturnTypes.ts` 中的 unify/substitute 需镜像 Rust `unify.rs` 逻辑（Phase 6a 已有 TS 端 unify）

### 8.3 IR JSON 一致性

TS 端 `compileToIR.ts` 输出的 `returnType` 字段必须与 Rust 端逐字节一致（经 ir-diff 归一化后）。归一化规则复用 Phase 6b 已有的 null strip 与 Type 递归比较逻辑。

---

## 9. Fixture 与测试策略

### 9.1 新增 Fixture（3 个）

**Fixture 1: `tests/v06_phase6/return_inference.tangle.md`** — 基础推断

```markdown
# ReturnInferenceTest

### ItemProcessor

#### process
* `items`: List of items (List<Int>)

```@tangle
return items
```

#### main

```@tangle
return 0
```
```

**预期 IR：**
- `ItemProcessor.process`: `returnType = {"kind":"genericInstance","base":"List","args":[{"kind":"primitive","name":"Int"}]}`
- `main`: `returnType = {"kind":"primitive","name":"Int"}`

**Fixture 2: `tests/v06_phase6/match_narrowing.tangle.md`** — Match arm 收窄（同类型 arm）

```markdown
# MatchNarrowingTest

### process
* `input`: Optional value (Some<Int> | None)

```@tangle
match input {
  Some(y) => return y
  None => return 0
}
```

### main

```@tangle
return 0
```
```

**预期 IR：**
- `process`: `returnType = {"kind":"primitive","name":"Int"}`（Some arm 收窄 y:Int，None arm 字面量 Int，统一成功）
- `main`: `returnType = {"kind":"primitive","name":"Int"}`

**Fixture 3: `tests/v06_phase6/return_conflict.tangle.md`** — 冲突回退 Any

```markdown
# ReturnConflictTest

### process
* `input`: Input value (Int | String)

```@tangle
match input {
  Int(x) => return x
  String(s) => return s
}
```

### main

```@tangle
return 0
```
```

**预期 IR：**
- `process`: `returnType = {"kind":"any"}`（Int vs String 冲突 → Any）
- `main`: `returnType = {"kind":"primitive","name":"Int"}`

### 9.2 现有 Fixture 行为变化

**`tests/v06_phase6/generics.tangle.md`**（Phase 6b 已有）：

- **改前（Phase 6b）：** 两个函数 `returnType` 字段省略（None）
- **改后（Phase 6c）：** `process.returnType = List<Int>`，`main.returnType = Int`

**其他 11 个现有 fixture：** 若函数体含 `return expr`，`returnType` 将被填充。Rust/TS 双端同步实现，差分测试应保持 MATCH。

### 9.3 测试矩阵

| 测试类型 | 位置 | 内容 |
|---|---|---|
| Rust 单元测试 | `checker/infer_return_types.rs` `#[cfg(test)]` | `unify_all` 各组合、`infer_function_return_type` 单/多/冲突 return |
| Rust 单元测试 | `checker/check.rs` `#[cfg(test)]` | Match arm 收窄后 binding 类型、If 表达式统一 then/else |
| Rust 单元测试 | `checker/match_check.rs` `#[cfg(test)]` | 扩展后的穷尽性检查（Struct/GenericInstance variant） |
| Rust 集成测试 | `compiler/tangle-cli/tests/v06_phase6/return_inference.rs`（新建） | 加载 3 个新 fixture，断言 `IRFunction.return_type` |
| Rust 回归测试 | 现有 `tests/v03_phase1/`、`tests/v03_phase2/` 等 | 不回归；若有 `returnType` 字段断言需更新 |
| TS 单元测试 | `reference/tests/checker/inferReturnTypes.test.ts`（新建） | 镜像 Rust 单元测试 |
| TS 单元测试 | `reference/tests/checker/matchNarrowing.test.ts`（新建） | Match 收窄 |
| 差分测试 | `tests/audit/diff-ir.ps1` | **15 MATCH + 0 SKIPPED + 0 DIFF**（12 现有 + 3 新增） |

### 9.4 回归扫描

实现前扫描 `compiler/tangle-cli/tests/` 下所有 `.rs` 文件，查找：
- `return_type: None` 断言（需更新为推断值或移除）
- `returnType` JSON 字段缺失断言
- IR JSON 快照中无 `returnType` 字段的断言

---

## 10. 出口闸门（9 项）

| # | 标准 | 验证方式 |
|---|------|---------|
| 1 | `cargo test --workspace` 全绿 | 含新 infer_return_types 测试、Match 收窄测试、回归测试更新 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` 零警告 | |
| 3 | `cd reference && npm test` 全绿 | 含新 inferReturnTypes/matchNarrowing 测试 |
| 4 | `cd reference && npm run build` 零类型错误 | TS 编译通过 |
| 5 | `pwsh tests/audit/diff-ir.ps1` **15 MATCH + 0 SKIPPED + 0 DIFF** | 12 现有 + 3 新增 fixture |
| 6 | `pwsh tests/audit/run-audit.ps1` 0 failing | |
| 7 | Phase 4/5/6a/6b 回归测试通过 | |
| 8 | Phase 6c 新测试通过（推断 + 收窄 + 集成） | |
| 9 | 手动验证 IR JSON：`cargo run -- build tests/v06_phase6/match_narrowing.tangle.md --emit-ir` 输出含正确 `returnType` | 肉眼确认 |

---

## 11. 非目标（推迟到 Phase 6d 或更晚）

- **If 条件收窄**：`if x is Some { ... }` 内的类型细化（仅 Match arm 收窄在范围内）
- **Destructure 表达式收窄**：`let { ok, err } = ...` 模式收窄
- **Option/Result 自动拆解**：将 `Option<T>` 视为 `Some<T> | None` 的语法糖（用户需显式声明 Sum 类型）
- **返回类型 diagnostic**：冲突时不报错，仅回退 Any（保持 TS 兼容）
- **局部变量类型注解**：let 绑定不生成 Py `x: int = ...` / Go 显式类型声明
- **IRNode 类型字段**：节点级不携带类型信息
- **JS 类型注解**：JS 无类型系统，不做 JSDoc 生成
- **类型映射完备性**：`Function<T,U>` 等复杂类型 → Py `Callable` / Go `func()`，不做精确签名映射
- **用户自定义泛型类型**：用户不能定义 `Foo<T>`
- **泛型约束**（如 `T: Comparable`）：不实现

---

## 12. 风险与缓解

| 风险 | 缓解 |
|---|---|
| TS 端推断与 Rust 不一致导致 DIFF | 双端镜像相同算法；TS 单元测试覆盖相同用例；先单独跑差分测试逐 fixture 检查 |
| 现有 fixture 的 return_type 推断意外（如复杂表达式） | 现有 12 fixture 函数体简单（多为 `return items`/`return 0`），推断应稳定 |
| TS fatal on Sum type 语法 | Phase 6b 已验证 type_parser 支持 `List<Int>`；Sum 语法 `(A | B)` 应同样支持；实现前先 `cargo test` 验证 |
| Go emitter 外部签名变更破坏现有代码 | 设计 A 明确保持 `Result` 外包；go_emitter.rs 改为硬编码 "Result" |
| Match arm 收窄对未知 variant 名静默忽略 | 设计如此（best-effort）；若需严格可后续加 diagnostic |
| `heading_id` 作为 return_types key 与现有 collect_functions 不一致 | collect_functions 已有 `h.id`，直接传入即可 |
| Sum 类型 fixture 语法不被 type_parser 支持 | 实现前先用 `cargo test` 验证 `parse_type_expr("Some<Int> | None")`；若不支持，fixture 降级为 Primitive variant 的 Sum |

---

## 13. 版本与分支

- 新分支：`phase6c/v0.7.0`，基于 `main`
- worktree 路径：`.worktrees/phase6c-v0.7.0`
- 完成后合并到 `main`，打 tag `v0.7.0`（返回类型推断 + 类型收窄是显著新特性，minor 版本升级）
- **tag v0.7.0 不 push remote，直到用户批准**
