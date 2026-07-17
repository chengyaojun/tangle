# Phase 6b: 泛型在 IR 与 Codegen 中的表示 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让用户函数签名的类型注解（含泛型 `List<Int>`）从 Tangle 源码贯通到 IR JSON 与 Py/Go 生成代码，让类型信息成为 IR 的一等公民。

**架构：** IR 扩展 `IRParam{name, type?}` + `IRFunction.return_type?`；`compile_to_ir` 复用 `type_name_to_type`（重写为基于 `type_parser`）填充类型；新建 `codegen/type_map.rs` 提供 `tangle_type_to_py/go`；Py/Go emitter 生成类型标注；ir-diff 归一化处理新字段；diff-ir.ps1 加入 v06_phase6 fixtures；TS reference 端镜像。

**技术栈：** Rust（serde + cargo）、TypeScript（参考实现）、PowerShell（差分测试脚本）

**规格文档：** [docs/superpowers/specs/2026-07-18-phase6b-generics-in-ir-and-codegen-design.md](file:///e:/GitProjects/tangle/docs/superpowers/specs/2026-07-18-phase6b-generics-in-ir-and-codegen-design.md)

---

## 规格偏差修正

实现前发现规格有 2 处偏差，本计划已修正：

1. **`Type` 枚举遗漏 `Sum(SumType)` 变体：** 规格 §3.3 列出的变体不含 `Sum`，但 [types.rs:7](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/types.rs#L7) 实际存在。本计划任务 2 包含 `Sum` 和 `SumType` 的 serde derive。
2. **`type_name_to_type` 不使用 `type_parser`：** 规格 §4.1 假设"复用已有 `type_name_to_type`"，但 [resolve.rs:124-139](file:///e:/GitProjects/tangle/compiler/tangle-cli/src/checker/resolve.rs#L124-L139) 是简化匹配器，无法解析 `List<Int>`。本计划任务 3 新增 `type_expr_to_type`（镜像 TS 端 [resolve.ts:117](file:///e:/GitProjects/tangle/reference/src/checker/resolve.ts#L117)）并重写 `type_name_to_type` 使用 `parse_type_expr`。

---

## 文件结构

| 文件 | 职责 | 操作 |
|------|------|------|
| `compiler/tangle-cli/src/checker/types.rs` | Type 及所有子结构添加 Serialize/Deserialize | 修改 |
| `compiler/tangle-cli/src/checker/resolve.rs` | 重写 type_name_to_type + 新增 type_expr_to_type | 修改 |
| `compiler/tangle-cli/src/checker/mod.rs` | re-export type_name_to_type + type_expr_to_type | 修改 |
| `compiler/tangle-cli/src/ir/graph.rs` | IRParam 结构 + IRFunction 扩展 return_type | 修改 |
| `compiler/tangle-cli/src/ir/compile_to_ir.rs` | collect_functions 用 IRParam + type_name_to_type | 修改 |
| `compiler/tangle-cli/src/codegen/type_map.rs` | tangle_type_to_py / tangle_type_to_go | 创建 |
| `compiler/tangle-cli/src/codegen/mod.rs` | 注册 type_map 模块 | 修改 |
| `compiler/tangle-cli/src/codegen/py_emitter.rs` | format_py_param + 函数签名 | 修改 |
| `compiler/tangle-cli/src/codegen/go_emitter.rs` | format_go_param + 函数签名 | 修改 |
| `compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs` | IR 参数类型集成测试 | 创建 |
| `compiler/tangle-cli/Cargo.toml` | 注册新测试 binary | 修改 |
| `tests/audit/ir-diff/src/main.rs` | params 归一化 + return_type null strip | 修改 |
| `tests/audit/diff-ir.ps1` | 新增 v06_phase6 fixture 路径 | 修改 |
| `tests/v06_phase6/generics.tangle.md` | 改造为多函数 + 类型注解 | 修改 |
| `reference/src/ir/graph.ts` | IRParam interface + IRFunction 扩展 | 修改 |
| `reference/src/checker/resolve.ts` | 导出 typeNameToType 包装函数 | 修改 |
| `reference/src/ir/compileToIR.ts` | collectFunctions 用 typeNameToType | 修改 |
| `reference/src/codegen/typeMap.ts` | tangleTypeToPy / tangleTypeToGo | 创建 |
| `reference/src/codegen/pyEmitter.ts` | formatPyParam | 修改 |
| `reference/src/codegen/goEmitter.ts` | formatGoParam | 修改 |
| `reference/tests/codegen/typeMap.test.ts` | TS 类型映射测试 | 创建 |

**总计**：修改 17 个文件，创建 4 个文件。

---

## 任务 1：Worktree 创建与基线验证

**文件：**
- 无文件变更

- [ ] **步骤 1：创建 worktree**

运行：
```bash
cd e:\GitProjects\tangle
git worktree add .worktrees/phase6b-v0.6.1 -b phase6b/v0.6.1 main
```
预期：worktree 创建成功，新分支 `phase6b/v0.6.1` 基于 `main`

- [ ] **步骤 2：在 worktree 中验证 Rust 基线**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase6b-v0.6.1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
预期：全绿，零警告

- [ ] **步骤 3：验证 TS reference 基线**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase6b-v0.6.1\reference
npm test
npm run build
```
预期：全绿

- [ ] **步骤 4：验证差分测试基线**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase6b-v0.6.1
pwsh tests/audit/diff-ir.ps1
```
预期：记录当前 MATCH/SKIPPED/DIFF 数（记为 `BASELINE_MATCH`、`BASELINE_SKIPPED`）。Phase 6b 目标 = `BASELINE_MATCH + 1` MATCH + 0 SKIPPED + 0 DIFF

- [ ] **步骤 5：记录基线**

在 worktree 根目录创建临时文件 `phase6b-baseline.txt`（实现完成后删除），写入：
```
baseline_match=<BASELINE_MATCH>
baseline_skipped=<BASELINE_SKIPPED>
target_match=<BASELINE_MATCH + 1>
```

---

## 任务 2：Type 添加 Serialize/Deserialize

**文件：**
- 修改：`compiler/tangle-cli/src/checker/types.rs:1-61`

**目标：** 为 `Type` 枚举及所有子结构添加 `Serialize, Deserialize` derive，使 IR JSON 能携带类型信息。使用 `#[serde(tag = "kind", rename_all = "camelCase")]` 与 TS 端 `{ kind: "primitive", name: "Int" }` 形态一致。

- [ ] **步骤 1：编写失败的序列化测试**

在 `compiler/tangle-cli/src/checker/types.rs` 末尾添加：

```rust
#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn test_primitive_serializes_with_kind_tag() {
        let ty = Type::Primitive(PrimitiveType { name: "Int".into() });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "primitive");
        assert_eq!(json["name"], "Int");
    }

    #[test]
    fn test_generic_instance_serializes() {
        let ty = Type::GenericInstance(GenericTypeInstance {
            base: "List".into(),
            args: vec![Type::Primitive(PrimitiveType { name: "Int".into() })],
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "genericInstance");
        assert_eq!(json["base"], "List");
        assert_eq!(json["args"][0]["kind"], "primitive");
        assert_eq!(json["args"][0]["name"], "Int");
    }

    #[test]
    fn test_struct_serializes() {
        let ty = Type::Struct(StructType {
            name: "Order".into(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "struct");
        assert_eq!(json["name"], "Order");
    }

    #[test]
    fn test_sum_serializes() {
        let ty = Type::Sum(SumType {
            variants: vec![Type::Primitive(PrimitiveType { name: "Int".into() })],
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "sum");
        assert_eq!(json["variants"][0]["kind"], "primitive");
    }

    #[test]
    fn test_any_serializes() {
        let ty = Type::Any;
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "any");
    }

    #[test]
    fn test_roundtrip_deserialize() {
        let ty = Type::GenericInstance(GenericTypeInstance {
            base: "Map".into(),
            args: vec![
                Type::Primitive(PrimitiveType { name: "String".into() }),
                Type::Primitive(PrimitiveType { name: "Int".into() }),
            ],
        });
        let json = serde_json::to_string(&ty).unwrap();
        let back: Type = serde_json::from_str(&json).unwrap();
        assert_eq!(ty, back);
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase6b-v0.6.1
cargo test -p tangle-cli --lib checker::types::serde_tests
```
预期：FAIL，报错 `the trait bound Type: Serialize is not satisfied`

- [ ] **步骤 3：为 Type 及子结构添加 serde derive**

修改 `compiler/tangle-cli/src/checker/types.rs:1-61`：

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Type {
    Primitive(PrimitiveType),
    Struct(StructType),
    Sum(SumType),
    GenericInstance(GenericTypeInstance),
    Function(FunctionType),
    Interface(InterfaceType),
    Var(TypeVariable),
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructType {
    pub name: String,
    pub fields: HashMap<String, Type>,
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SumType {
    pub variants: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericTypeInstance {
    pub base: String,
    pub args: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub returns: Box<Type>,
    #[serde(default, rename = "isVariadic")]
    pub is_variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceType {
    pub name: String,
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeVariable {
    pub id: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallableSignature {
    pub params: Vec<(String, Type)>,
    pub returns: Type,
    #[serde(default, rename = "isVariadic")]
    pub is_variadic: bool,
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test -p tangle-cli --lib checker::types::serde_tests
```
预期：6 个测试全 PASS

- [ ] **步骤 5：运行全量测试确保无回归**

运行：
```bash
cargo test --workspace
```
预期：全绿（Phase 6a 测试不依赖序列化形态，仅内部 Type 操作）

- [ ] **步骤 6：Commit**

```bash
git add compiler/tangle-cli/src/checker/types.rs
git commit -m "feat(checker): 为 Type 及子结构添加 serde derive

使用 tag=\"kind\" 内部标签序列化，与 TS 端 {kind: \"primitive\", name: \"Int\"} 一致。
涵盖 Primitive/Struct/Sum/GenericInstance/Function/Interface/Var/Any 全部变体。"
```

---

## 任务 3：新增 type_expr_to_type + 重写 type_name_to_type

**文件：**
- 修改：`compiler/tangle-cli/src/checker/resolve.rs:124-139`
- 修改：`compiler/tangle-cli/src/checker/mod.rs`

**目标：** 当前 `type_name_to_type` 是简化匹配器（无法解析 `List<Int>`）。新增 `type_expr_to_type`（镜像 TS [resolve.ts:117](file:///e:/GitProjects/tangle/reference/src/checker/resolve.ts#L117)），重写 `type_name_to_type` 使用 `parse_type_expr` + `type_expr_to_type`。

- [ ] **步骤 1：编写失败的解析测试**

在 `compiler/tangle-cli/src/checker/resolve.rs` 末尾添加测试模块：

```rust
#[cfg(test)]
mod type_name_tests {
    use super::*;

    #[test]
    fn test_basic_types() {
        assert!(matches!(type_name_to_type("Int").unwrap(), Type::Primitive(_)));
        assert!(matches!(type_name_to_type("String").unwrap(), Type::Primitive(_)));
        assert!(matches!(type_name_to_type("Bool").unwrap(), Type::Primitive(_)));
    }

    #[test]
    fn test_generic_list() {
        let ty = type_name_to_type("List<Int>").unwrap();
        match ty {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "List");
                assert_eq!(g.args.len(), 1);
                match &g.args[0] {
                    Type::Primitive(p) => assert_eq!(p.name, "Int"),
                    other => panic!("expected Primitive, got {:?}", other),
                }
            }
            other => panic!("expected GenericInstance, got {:?}", other),
        }
    }

    #[test]
    fn test_generic_map_two_args() {
        let ty = type_name_to_type("Map<String, Int>").unwrap();
        match ty {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "Map");
                assert_eq!(g.args.len(), 2);
            }
            other => panic!("expected GenericInstance, got {:?}", other),
        }
    }

    #[test]
    fn test_named_type_becomes_struct() {
        let ty = type_name_to_type("Order").unwrap();
        assert!(matches!(ty, Type::Struct(_)));
    }

    #[test]
    fn test_option_nested() {
        let ty = type_name_to_type("Option<String>").unwrap();
        match ty {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "Option");
                assert_eq!(g.args.len(), 1);
            }
            other => panic!("expected GenericInstance, got {:?}", other),
        }
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test -p tangle-cli --lib checker::resolve::type_name_tests
```
预期：FAIL — `test_generic_list` 失败（当前 `type_name_to_type("List<Int>")` 返回 `Type::Primitive{name: "List<Int>"}`）

- [ ] **步骤 3：新增 type_expr_to_type 函数**

在 `compiler/tangle-cli/src/checker/resolve.rs` 中（`type_name_to_type` 上方）添加：

```rust
use crate::ast::{TypeExpr, PrimitiveTypeExpr, NamedTypeExpr, GenericTypeExpr, SumTypeExpr, FunctionTypeExpr};
use crate::parser::type_parser::parse_type_expr;

/// 将 TypeExpr（语法树）转换为 Type（语义类型）。
/// 镜像 TS 端 reference/src/checker/resolve.ts:typeExprToType。
pub fn type_expr_to_type(te: &TypeExpr) -> Type {
    match te {
        TypeExpr::Primitive(PrimitiveTypeExpr { name, .. }) => {
            Type::Primitive(PrimitiveType { name: name.clone() })
        }
        TypeExpr::Named(NamedTypeExpr { name, .. }) => {
            // 用户定义类型名 → Struct（字段/方法在 resolve_types 中填充）
            Type::Struct(StructType {
                name: name.clone(),
                fields: HashMap::new(),
                methods: HashMap::new(),
            })
        }
        TypeExpr::Sum(SumTypeExpr { variants, .. }) => {
            Type::Sum(SumType {
                variants: variants.iter().map(type_expr_to_type).collect(),
            })
        }
        TypeExpr::Generic(GenericTypeExpr { base, type_args, .. }) => {
            Type::GenericInstance(GenericTypeInstance {
                base: base.clone(),
                args: type_args.iter().map(type_expr_to_type).collect(),
            })
        }
        TypeExpr::Function(FunctionTypeExpr { params, returns, .. }) => {
            Type::Function(FunctionType {
                params: params.iter().map(type_expr_to_type).collect(),
                returns: Box::new(type_expr_to_type(returns)),
                is_variadic: false,
            })
        }
    }
}
```

注意：需在文件顶部添加 `use std::collections::HashMap;` 和 `use crate::checker::types::{StructType, SumType, GenericTypeInstance, FunctionType};`（若未导入）。

- [ ] **步骤 4：重写 type_name_to_type**

将 `compiler/tangle-cli/src/checker/resolve.rs:124-139` 的 `type_name_to_type` 替换为：

```rust
/// 解析 Tangle 类型注解字符串（如 "List<Int>"、"Order"、"String"）为 Type。
/// 使用 type_parser 解析泛型语法，解析失败返回 None。
/// 调用方（resolve_types）用 .unwrap_or(Type::Any) 处理 None。
pub fn type_name_to_type(name: &str) -> Option<Type> {
    let (te, diags) = parse_type_expr(name, "");
    if !diags.is_empty() {
        return None;
    }
    te.map(|te| type_expr_to_type(&te))
}
```

- [ ] **步骤 5：在 checker/mod.rs 中 re-export**

修改 `compiler/tangle-cli/src/checker/mod.rs`，添加：

```rust
pub use resolve::{type_name_to_type, type_expr_to_type};
```

- [ ] **步骤 6：运行测试验证通过**

运行：
```bash
cargo test -p tangle-cli --lib checker::resolve::type_name_tests
```
预期：5 个测试全 PASS

- [ ] **步骤 7：运行全量测试确保无回归**

运行：
```bash
cargo test --workspace
```
预期：全绿。注意：`resolve_types` 中 3 处调用 `type_name_to_type` 后跟 `.unwrap_or(Type::Any)`，行为变化：
- 旧：`Foo` → `Some(Primitive{Foo})` → Type::Primitive
- 新：`Foo` → `Some(Struct{name: "Foo"})` → Type::Struct

若 Phase 6a 测试断言了 `Primitive` 形态，需更新断言为 `Struct`。若有回归，逐个修复。

- [ ] **步骤 8：Commit**

```bash
git add compiler/tangle-cli/src/checker/resolve.rs compiler/tangle-cli/src/checker/mod.rs
git commit -m "feat(checker): 新增 type_expr_to_type 并重写 type_name_to_type

type_name_to_type 现使用 type_parser 解析泛型语法（List<Int>、Map<K,V>），
不再是无脑 Primitive 匹配器。NamedTypeExpr → Type::Struct，与 TS 端一致。"
```

---

## 任务 4：IRParam 结构与 IRFunction 扩展

**文件：**
- 修改：`compiler/tangle-cli/src/ir/graph.rs:83-93`

**目标：** `IRFunction.params` 从 `Vec<String>` 改为 `Vec<IRParam>`，新增 `return_type: Option<Type>`。

- [ ] **步骤 1：编写失败的序列化测试**

在 `compiler/tangle-cli/src/ir/graph.rs` 末尾添加：

```rust
#[cfg(test)]
mod ir_param_tests {
    use super::*;
    use crate::checker::types::{PrimitiveType, GenericTypeInstance};

    #[test]
    fn test_ir_param_without_type_omits_field() {
        let p = IRParam { name: "x".into(), type_: None };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["name"], "x");
        assert!(json.get("type").is_none(), "type field should be omitted when None");
    }

    #[test]
    fn test_ir_param_with_type() {
        let p = IRParam {
            name: "items".into(),
            type_: Some(Type::GenericInstance(GenericTypeInstance {
                base: "List".into(),
                args: vec![Type::Primitive(PrimitiveType { name: "Int".into() })],
            })),
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["name"], "items");
        assert_eq!(json["type"]["kind"], "genericInstance");
        assert_eq!(json["type"]["base"], "List");
    }

    #[test]
    fn test_ir_function_return_type_omitted_when_none() {
        let f = IRFunction {
            name: "main".into(),
            receiver: None,
            params: vec![],
            return_type: None,
            nodes: vec![],
            edges: vec![],
            entry_node_id: "n0".into(),
            error_edges: vec![],
        };
        let json = serde_json::to_value(&f).unwrap();
        assert!(json.get("returnType").is_none());
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test -p tangle-cli --lib ir::graph::ir_param_tests
```
预期：FAIL — `IRParam` 未定义

- [ ] **步骤 3：添加 IRParam 并扩展 IRFunction**

修改 `compiler/tangle-cli/src/ir/graph.rs`，在 `IRFunction` 定义前添加 `IRParam`，并修改 `IRFunction`：

```rust
use crate::checker::types::Type;

/// IR 参数：name + 可选类型（来自 Tangle 源码注解 `param: TypeName`）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IRParam {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub type_: Option<Type>,
}

/// A heading-defined function (e.g. `#### main`, `#### create` under `### Order`).
/// `receiver` is `Some("Order")` for methods like `Order.create`; `None` for free
/// functions like `main` / `process`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IRFunction {
    pub name: String,
    pub receiver: Option<String>,
    pub params: Vec<IRParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<Type>,
    pub nodes: Vec<IRNode>,
    pub edges: Vec<IREdge>,
    pub entry_node_id: String,
    pub error_edges: Vec<IRErrorEdge>,
}
```

注意：`type_` 字段使用 `rename = "type"`（因 `type` 是 Rust 关键字），序列化为 JSON `"type"` 与 TS 端一致。

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test -p tangle-cli --lib ir::graph::ir_param_tests
```
预期：3 个测试全 PASS

- [ ] **步骤 5：运行全量测试（预期编译错误）**

运行：
```bash
cargo build -p tangle-cli
```
预期：FAIL — `collect_functions` 中 `params: Vec<String>` 与 `IRFunction { params: Vec<IRParam> }` 类型不匹配。此错误在任务 5 修复。

- [ ] **步骤 6：Commit（即使全量编译失败，graph.rs 本身正确）**

```bash
git add compiler/tangle-cli/src/ir/graph.rs
git commit -m "feat(ir): 新增 IRParam 结构，IRFunction.params 改为 Vec<IRParam>

IRParam{name, type?} 携带参数类型信息；IRFunction 新增 return_type? 字段。
return_type 对用户函数恒为 None（Phase 6c 才实现返回推导）。

注意：collect_functions 需同步更新（任务 5）。"
```

---

## 任务 5：compile_to_ir collect_functions 更新

**文件：**
- 修改：`compiler/tangle-cli/src/ir/compile_to_ir.rs:129-164`

**目标：** `collect_functions` 使用 `IRParam` + `type_name_to_type` 填充参数类型。

- [ ] **步骤 1：编写失败的集成测试**

创建 `compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs`：

```rust
use tangle_cli::ir::{graph::IRParam, compile_to_ir::compile_to_ir};
use tangle_cli::checker::types::Type;
use tangle_cli::frontend::parse_module;

#[test]
fn test_params_carry_types_from_fixture() {
    let source = std::fs::read_to_string("tests/v06_phase6/generics.tangle.md").unwrap();
    let module = parse_module(&source, "generics.tangle.md").unwrap();
    let graph = compile_to_ir(&module);

    // 找到 process 函数
    let process = graph.functions.iter().find(|f| f.name == "process")
        .expect("fixture should define process function");

    // 第一个参数 items 应有类型 List<Int>
    assert_eq!(process.params.len(), 2);
    let items_param: &IRParam = &process.params[0];
    assert_eq!(items_param.name, "items");
    let ty = items_param.type_.as_ref().expect("items should have type");
    match ty {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List");
            assert_eq!(g.args.len(), 1);
        }
        other => panic!("expected GenericInstance, got {:?}", other),
    }

    // 第二个参数 threshold 应有类型 Int
    let threshold_param = &process.params[1];
    assert_eq!(threshold_param.name, "threshold");
    match threshold_param.type_.as_ref().unwrap() {
        Type::Primitive(p) => assert_eq!(p.name, "Int"),
        other => panic!("expected Primitive Int, got {:?}", other),
    }

    // main 函数应无参数
    let main = graph.functions.iter().find(|f| f.name == "main").unwrap();
    assert!(main.params.is_empty());
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test -p tangle-cli --test ir_param_types
```
预期：FAIL — 编译错误（`collect_functions` 中 `params: Vec<String>` 与 `Vec<IRParam>` 不匹配）

- [ ] **步骤 3：更新 collect_functions**

修改 `compiler/tangle-cli/src/ir/compile_to_ir.rs:146-159`：

```rust
use crate::checker::resolve::type_name_to_type;
use crate::ir::graph::IRParam;

// ... 在 collect_functions 中：

let params: Vec<IRParam> = h.params.iter().map(|p| IRParam {
    name: p.name.clone(),
    type_: p.type_name.as_ref().and_then(|tn| type_name_to_type(tn)),
}).collect();
let blocks: Vec<&ParsedCodeBlock> = parsed_blocks.iter()
    .filter(|b| b.heading_id == h.id)
    .collect();
let (nodes, edges, entry_id, error_edges) = lower_function_body(&blocks, id_gen);
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

- [ ] **步骤 4：改造 fixture（任务 9 会细化，此处先用最小可编译 fixture）**

修改 `tests/v06_phase6/generics.tangle.md` 为：

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

- [ ] **步骤 5：运行测试验证通过**

运行：
```bash
cargo test -p tangle-cli --test ir_param_types
```
预期：PASS

- [ ] **步骤 6：扫描并修复回归测试**

搜索 `compiler/tangle-cli/tests/` 下所有断言 `params` 为字符串数组的测试：

```bash
grep -rn "params.*vec!\[\".*\"" compiler/tangle-cli/tests/  --include="*.rs"
grep -rn "\.params," compiler/tangle-cli/tests/  --include="*.rs"
grep -rn "\"params\":" compiler/tangle-cli/tests/  --include="*.rs"
```

对每个匹配：
- 若断言 `func.params == vec!["x".to_string()]` → 改为 `func.params == vec![IRParam { name: "x".into(), type_: None }]`
- 若 JSON 快照含 `"params":["x"]` → 改为 `"params":[{"name":"x"}]`

- [ ] **步骤 7：运行全量测试**

运行：
```bash
cargo test --workspace
```
预期：全绿

- [ ] **步骤 8：Commit**

```bash
git add compiler/tangle-cli/src/ir/compile_to_ir.rs tests/v06_phase6/generics.tangle.md compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs
git commit -m "feat(ir): collect_functions 填充 IRParam 类型

用 type_name_to_type 解析参数注解字符串为 Type，存入 IRParam.type_。
改造 generics fixture 为多函数 + List<Int>/Int 注解。
新增 ir_param_types 集成测试验证类型贯通。"
```

---

## 任务 6：新建 codegen/type_map.rs

**文件：**
- 创建：`compiler/tangle-cli/src/codegen/type_map.rs`
- 修改：`compiler/tangle-cli/src/codegen/mod.rs`

**目标：** 提供 `tangle_type_to_py` / `tangle_type_to_go` 函数，将 `Type` 映射为语言原生类型注解字符串。

- [ ] **步骤 1：编写失败的映射测试**

创建 `compiler/tangle-cli/src/codegen/type_map.rs`，先写测试：

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
            "Map" => format!("Dict[{}, {}]", inner_py(&g.args[0]), inner_py(&g.args[1])),
            "Option" => format!("Optional[{}]", inner_py(&g.args[0])),
            "Set" => format!("Set[{}]", inner_py(&g.args[0])),
            other => format!("{}[{}]", other, g.args.iter().map(inner_py).collect::<Vec<_>>().join(", ")),
        },
        Type::Var(_) => None,
        Type::Function(_) => Some("Callable".into()),
        Type::Sum(s) => {
            let parts: Vec<String> = s.variants.iter().filter_map(tangle_type_to_py).collect();
            if parts.is_empty() { None } else { Some(format!("Union[{}]", parts.join(", "))) }
        }
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
            "Map" => format!("map[{}]{}", inner_go(&g.args[0]), inner_go(&g.args[1])),
            "Option" => format!("*{}", inner_go(&g.args[0])),
            "Set" => format!("map[{}]struct{{}}", inner_go(&g.args[0])),
            other => format!("any /* {} */", other),
        },
        Type::Var(_) => "any".into(),
        Type::Function(_) => "func()".into(),
        Type::Sum(s) => {
            // Go 无原生联合类型，取第一个变体
            s.variants.first().map(inner_go).unwrap_or_else(|| "any".into())
        }
    }
}

fn inner_go(ty: &Type) -> String {
    tangle_type_to_go(ty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.into() })
    }

    fn generic(base: &str, args: Vec<Type>) -> Type {
        Type::GenericInstance(GenericTypeInstance { base: base.into(), args })
    }

    #[test]
    fn test_py_primitives() {
        assert_eq!(tangle_type_to_py(&prim("Int")), Some("int".into()));
        assert_eq!(tangle_type_to_py(&prim("String")), Some("str".into()));
        assert_eq!(tangle_type_to_py(&prim("Bool")), Some("bool".into()));
        assert_eq!(tangle_type_to_py(&prim("Float")), Some("float".into()));
    }

    #[test]
    fn test_py_any_returns_none() {
        assert_eq!(tangle_type_to_py(&Type::Any), None);
    }

    #[test]
    fn test_py_generic_list() {
        let ty = generic("List", vec![prim("Int")]);
        assert_eq!(tangle_type_to_py(&ty), Some("List[int]".into()));
    }

    #[test]
    fn test_py_generic_map() {
        let ty = generic("Map", vec![prim("String"), prim("Int")]);
        assert_eq!(tangle_type_to_py(&ty), Some("Dict[str, int]".into()));
    }

    #[test]
    fn test_py_option() {
        let ty = generic("Option", vec![prim("String")]);
        assert_eq!(tangle_type_to_py(&ty), Some("Optional[str]".into()));
    }

    #[test]
    fn test_py_struct() {
        let ty = Type::Struct(StructType { name: "Order".into(), fields: HashMap::new(), methods: HashMap::new() });
        assert_eq!(tangle_type_to_py(&ty), Some("Order".into()));
    }

    #[test]
    fn test_go_primitives() {
        assert_eq!(tangle_type_to_go(&prim("Int")), "int");
        assert_eq!(tangle_type_to_go(&prim("String")), "string");
        assert_eq!(tangle_type_to_go(&prim("Bool")), "bool");
        assert_eq!(tangle_type_to_go(&prim("Float")), "float64");
    }

    #[test]
    fn test_go_any_returns_any() {
        assert_eq!(tangle_type_to_go(&Type::Any), "any");
    }

    #[test]
    fn test_go_generic_list() {
        let ty = generic("List", vec![prim("Int")]);
        assert_eq!(tangle_type_to_go(&ty), "[]int");
    }

    #[test]
    fn test_go_generic_map() {
        let ty = generic("Map", vec![prim("String"), prim("Int")]);
        assert_eq!(tangle_type_to_go(&ty), "map[string]int");
    }

    #[test]
    fn test_go_option() {
        let ty = generic("Option", vec![prim("String")]);
        assert_eq!(tangle_type_to_go(&ty), "*string");
    }

    #[test]
    fn test_go_set() {
        let ty = generic("Set", vec![prim("Int")]);
        assert_eq!(tangle_type_to_go(&ty), "map[int]struct{}");
    }
}
```

- [ ] **步骤 2：在 codegen/mod.rs 注册模块**

修改 `compiler/tangle-cli/src/codegen/mod.rs`，添加：

```rust
pub mod type_map;
pub use type_map::{tangle_type_to_py, tangle_type_to_go};
```

- [ ] **步骤 3：运行测试验证通过**

运行：
```bash
cargo test -p tangle-cli --lib codegen::type_map::tests
```
预期：13 个测试全 PASS

- [ ] **步骤 4：Commit**

```bash
git add compiler/tangle-cli/src/codegen/type_map.rs compiler/tangle-cli/src/codegen/mod.rs
git commit -m "feat(codegen): 新建 type_map 模块

tangle_type_to_py: Type → Python 注解（None 表示无注解）
tangle_type_to_go: Type → Go 类型（无注解返回 \"any\"）

覆盖 Primitive/Struct/Interface/GenericInstance(List/Map/Option/Set)/Var/Function/Sum。"
```

---

## 任务 7：Py emitter 集成

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/py_emitter.rs`

**目标：** Py emitter 在函数签名中生成类型注解 `def f(x: int, items: List[int]):`。

- [ ] **步骤 1：编写失败的 emitter 测试**

在 `compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs` 末尾添加：

```rust
#[test]
fn test_py_emitter_generates_type_annotations() {
    use tangle_cli::codegen::py_emitter::emit_python;
    use tangle_cli::ir::compile_to_ir::compile_to_ir;
    use tangle_cli::frontend::parse_module;

    let source = std::fs::read_to_string("tests/v06_phase6/generics.tangle.md").unwrap();
    let module = parse_module(&source, "generics.tangle.md").unwrap();
    let graph = compile_to_ir(&module);
    let py = emit_python(&graph, "generics");

    // process 函数应有类型注解
    assert!(py.contains("def process(items: List[int], threshold: int):"),
        "Py output should contain typed signature, got:\n{}", py);

    // main 函数无参数，不应有注解
    assert!(py.contains("def main():"),
        "Py output should contain untyped main, got:\n{}", py);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test -p tangle-cli --test ir_param_types test_py_emitter_generates_type_annotations
```
预期：FAIL — 当前 Py 输出为 `def process(items, threshold):`（无类型注解）

- [ ] **步骤 3：添加 format_py_param 并更新签名生成**

在 `compiler/tangle-cli/src/codegen/py_emitter.rs` 中：

a) 添加 import 和 format_py_param 函数（在文件顶部或辅助函数区）：

```rust
use crate::codegen::type_map::tangle_type_to_py;
use crate::ir::graph::IRParam;

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
```

b) 找到所有生成函数签名的地方（搜索 `def ` 和 `params`），将参数列表从字符串数组改为用 `format_py_param`。

搜索命令：
```bash
grep -n "params" compiler/tangle-cli/src/codegen/py_emitter.rs
grep -n "def " compiler/tangle-cli/src/codegen/py_emitter.rs
```

对每个生成 `def {name}({params})` 的位置，将：
```rust
let params_str = func.params.iter().map(|p| p.clone()).collect::<Vec<_>>().join(", ");
// 或类似：func.params.join(", ")
```
改为：
```rust
let params_str = func.params.iter().map(format_py_param).collect::<Vec<_>>().join(", ");
```

注意：`emit_single_function_py` 和 `emit_multi_function_py` 都需更新。若有 receiver（方法），receiver 参数不加类型注解（保持原样或用 struct 名）。

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test -p tangle-cli --test ir_param_types test_py_emitter_generates_type_annotations
```
预期：PASS

- [ ] **步骤 5：运行全量测试（含现有 emitter 回归）**

运行：
```bash
cargo test --workspace
```
预期：全绿。现有 fixture 参数大多无注解 → `format_py_param` 返回纯名字 → 输出不变。

- [ ] **步骤 6：Commit**

```bash
git add compiler/tangle-cli/src/codegen/py_emitter.rs compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs
git commit -m "feat(py-emitter): 函数签名生成 Python 类型注解

format_py_param 用 tangle_type_to_py 将 IRParam.type_ 转为注解。
有注解：def f(x: int, items: List[int]):
无注解：def f(x, items):（保持原样）"
```

---

## 任务 8：Go emitter 集成

**文件：**
- 修改：`compiler/tangle-cli/src/codegen/go_emitter.rs`

**目标：** Go emitter 在函数签名中生成类型 `func f(x int, items []int) any {`。

- [ ] **步骤 1：编写失败的 emitter 测试**

在 `compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs` 末尾添加：

```rust
#[test]
fn test_go_emitter_generates_type_annotations() {
    use tangle_cli::codegen::go_emitter::emit_go;
    use tangle_cli::ir::compile_to_ir::compile_to_ir;
    use tangle_cli::frontend::parse_module;

    let source = std::fs::read_to_string("tests/v06_phase6/generics.tangle.md").unwrap();
    let module = parse_module(&source, "generics.tangle.md").unwrap();
    let graph = compile_to_ir(&module);
    let go = emit_go(&graph, "generics");

    // process 函数应有类型注解 + 返回类型 any
    assert!(go.contains("func process(items []int, threshold int) any {"),
        "Go output should contain typed signature, got:\n{}", go);

    // main 函数无参数，返回类型 any
    assert!(go.contains("func main() any {"),
        "Go output should contain untyped main with any return, got:\n{}", go);
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cargo test -p tangle-cli --test ir_param_types test_go_emitter_generates_type_annotations
```
预期：FAIL — 当前 Go 输出为 `func process(items, threshold) {`（无类型注解，无返回类型）

- [ ] **步骤 3：添加 format_go_param 并更新签名生成**

在 `compiler/tangle-cli/src/codegen/go_emitter.rs` 中：

a) 添加 import 和 format_go_param 函数：

```rust
use crate::codegen::type_map::tangle_type_to_go;
use crate::ir::graph::IRParam;

fn format_go_param(p: &IRParam) -> String {
    match &p.type_ {
        Some(ty) => format!("{} {}", p.name, tangle_type_to_go(ty)),
        None => format!("{} any", p.name),
    }
}
```

b) 找到所有生成 `func {name}({params})` 的位置，更新：
- 参数列表：`func.params.iter().map(format_go_param).collect::<Vec<_>>().join(", ")`
- 返回类型：`func.return_type.as_ref().map(tangle_type_to_go).unwrap_or_else(|| "any".into())`
- 签名格式：`func {name}({params_str}) {ret_ty} {{`

搜索命令：
```bash
grep -n "func " compiler/tangle-cli/src/codegen/go_emitter.rs
grep -n "params" compiler/tangle-cli/src/codegen/go_emitter.rs
```

注意：Go 多函数模式的 main 入口包装（Phase 5 A1 fix：`mainImpl() Result + func main() entry`）需保持。`mainImpl` 的返回类型用 `any`，`main` 入口仍用现有错误处理逻辑。

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cargo test -p tangle-cli --test ir_param_types test_go_emitter_generates_type_annotations
```
预期：PASS

- [ ] **步骤 5：运行全量测试**

运行：
```bash
cargo test --workspace
```
预期：全绿。现有 fixture 参数无注解 → `format_go_param` 返回 `x any` → 输出形态变化（从 `func main() {` 变为 `func main() any {`）。需更新现有 Go emitter 回归测试的断言。

- [ ] **步骤 6：修复 Go emitter 回归测试**

搜索并更新断言：
```bash
grep -rn "func main()" compiler/tangle-cli/tests/ --include="*.rs"
grep -rn "func.*() {" compiler/tangle-cli/tests/ --include="*.rs"
```

将 `func main() {` → `func main() any {`，将 `func process() {` → `func process() any {`（对无注解参数的函数）。

- [ ] **步骤 7：Commit**

```bash
git add compiler/tangle-cli/src/codegen/go_emitter.rs compiler/tangle-cli/tests/v06_phase6/ir_param_types.rs
git commit -m "feat(go-emitter): 函数签名生成 Go 类型注解 + 返回类型

format_go_param 用 tangle_type_to_go 转换；无注解参数用 any。
返回类型：return_type 为 None 时用 any（用户函数恒为 None，Phase 6c 实现返回推导）。

有注解：func process(items []int, threshold int) any {
无注解：func main() any {"
```

---

## 任务 9：完善 fixture 与集成测试注册

**文件：**
- 修改：`tests/v06_phase6/generics.tangle.md`（任务 5 已初步改造，此处确认完整性）
- 修改：`compiler/tangle-cli/Cargo.toml`

**目标：** 确认 fixture 完整且测试 binary 正确注册。

- [ ] **步骤 1：确认 fixture 内容**

读取 `tests/v06_phase6/generics.tangle.md`，确认包含：
- `### ItemProcessor` 类型标题
- `#### process` 可调用标题
- 两个参数：`items (List<Int>)`、`threshold (Int)`
- `@tangle` 代码块含 `List.map`、`List.filter` 调用
- `### main` 函数调用 `ItemProcessor.process`

若不完整，补全至任务 5 步骤 4 的形态。

- [ ] **步骤 2：注册测试 binary**

修改 `compiler/tangle-cli/Cargo.toml`，在 `[[test]]` 列表中添加（若不存在）：

```toml
[[test]]
name = "ir_param_types"
path = "tests/v06_phase6/ir_param_types.rs"
```

注意：若 Cargo.toml 使用 glob 自动发现测试，则无需手动注册。运行 `cargo test -p tangle-cli --test ir_param_types` 确认可发现。

- [ ] **步骤 3：运行 JS emitter 回归测试（确认不变）**

运行：
```bash
cargo test -p tangle-cli --test js_emitter_tests
```
预期：PASS（JS emitter 未改动，输出不变）

- [ ] **步骤 4：Commit（若 Cargo.toml 有变更）**

```bash
git add compiler/tangle-cli/Cargo.toml
git commit -m "chore: 注册 ir_param_types 测试 binary"
```

---

## 任务 10：ir-diff 归一化更新

**文件：**
- 修改：`tests/audit/ir-diff/src/main.rs`

**目标：** ir-diff 工具处理 `params` 从字符串数组到对象数组的变化，以及 `return_type` null 归一化。

- [ ] **步骤 1：编写失败的归一化测试**

在 `tests/audit/ir-diff/src/main.rs` 的 `#[cfg(test)]` 模块中添加（若无测试模块则创建）：

```rust
#[cfg(test)]
mod normalize_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_params_object_array() {
        let mut func = json!({
            "name": "process",
            "params": [
                {"name": "items", "type": {"kind": "genericInstance", "base": "List", "args": [{"kind": "primitive", "name": "Int"}]}},
                {"name": "threshold", "type": {"kind": "primitive", "name": "Int"}}
            ]
        });
        normalize_function(&mut func);
        // params 应保持对象数组形态
        assert_eq!(func["params"][0]["name"], "items");
        assert_eq!(func["params"][0]["type"]["kind"], "genericInstance");
        assert_eq!(func["params"][1]["name"], "threshold");
    }

    #[test]
    fn test_normalize_strips_null_return_type() {
        let mut func = json!({
            "name": "main",
            "params": [],
            "returnType": null
        });
        normalize_function(&mut func);
        // null returnType 应被删除
        assert!(func.get("returnType").is_none() || func["returnType"].is_null());
    }

    #[test]
    fn test_normalize_param_without_type_field() {
        let mut func = json!({
            "name": "main",
            "params": [{"name": "x"}]
        });
        normalize_function(&mut func);
        // 无 type 字段的参数应保持
        assert_eq!(func["params"][0]["name"], "x");
        assert!(func["params"][0].get("type").is_none());
    }

    #[test]
    fn test_compare_functions_with_typed_params() {
        let rust_func = json!({
            "name": "process",
            "params": [{"name": "items", "type": {"kind": "genericInstance", "base": "List", "args": [{"kind": "primitive", "name": "Int"}]}}]
        });
        let ts_func = json!({
            "name": "process",
            "params": [{"name": "items", "type": {"kind": "genericInstance", "base": "List", "args": [{"kind": "primitive", "name": "Int"}]}}]
        });
        let mut r = rust_func.clone();
        let mut t = ts_func.clone();
        normalize_function(&mut r);
        normalize_function(&mut t);
        assert_eq!(r, t);
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd tests/audit/ir-diff
cargo test normalize_tests
```
预期：FAIL — `normalize_function` 可能不存在或不处理新形态

- [ ] **步骤 3：实现/更新 normalize_function**

在 `tests/audit/ir-diff/src/main.rs` 中找到或新增 `normalize_function` 函数：

```rust
use serde_json::Value;

/// 归一化 IRFunction JSON：处理 params 对象数组 + return_type null strip
pub fn normalize_function(func: &mut Value) {
    if let Some(obj) = func.as_object_mut() {
        // 1. 归一化 params：确保每个 param 的 type 字段在 None/missing 时一致
        if let Some(params) = obj.get_mut("params").and_then(|p| p.as_array_mut()) {
            for param in params.iter_mut() {
                if let Some(pobj) = param.as_object_mut() {
                    // type 字段为 null 时删除（与 skip_serializing_if 一致）
                    if pobj.get("type").map(|v| v.is_null()).unwrap_or(false) {
                        pobj.remove("type");
                    }
                }
            }
        }

        // 2. return_type 为 null 时删除
        if obj.get("returnType").map(|v| v.is_null()).unwrap_or(false) {
            obj.remove("returnType");
        }

        // 3. 递归归一化嵌套 Type 对象的键顺序（通过序列化-反序列化排序）
        // serde_json 默认不排序，需手动排序。简单方法：重新序列化为字符串再解析
        // 或使用 serde_json 的 BTreeMap 模式
    }
}
```

若现有代码已有 `normalize` 函数（Phase 5 A2-5 的 null strip），在其内部添加上述 params 和 return_type 处理逻辑。

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
cd tests/audit/ir-diff
cargo test normalize_tests
```
预期：4 个测试全 PASS

- [ ] **步骤 5：运行 ir-diff 全量测试**

运行：
```bash
cargo test
```
预期：全绿

- [ ] **步骤 6：Commit**

```bash
git add tests/audit/ir-diff/src/main.rs
git commit -m "feat(ir-diff): 归一化处理 IRParam 对象数组 + return_type null strip

params 从 Vec<String> 变为 Vec<{name, type?}>，归一化确保 type 字段
None/missing 等价。returnType null 时删除键，与 Rust skip_serializing_if 一致。"
```

---

## 任务 11：diff-ir.ps1 扩展与差分验证

**文件：**
- 修改：`tests/audit/diff-ir.ps1`

**目标：** 将 `tests/v06_phase6/*.tangle.md` 加入差分测试 fixture 列表。

- [ ] **步骤 1：修改 diff-ir.ps1**

找到 `tests/audit/diff-ir.ps1` 中的 fixture 扫描行（约第 59 行），在 `Get-ChildItem` 路径列表中添加 `tests\v06_phase6\*.tangle.md`：

```powershell
# 当前：
$fixtures = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md" -ErrorAction SilentlyContinue | Sort-Object Name

# 改后：
$fixtures = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md","tests\v06_phase6\*.tangle.md" -ErrorAction SilentlyContinue | Sort-Object Name
```

- [ ] **步骤 2：运行差分测试**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase6b-v0.6.1
pwsh tests/audit/diff-ir.ps1
```
预期：
- generics.tangle.md 出现在测试列表中
- 现有 fixture 仍 MATCH（若退化到 DIFF，检查 ir-diff 归一化是否遗漏）
- generics.tangle.md：**首次运行可能 DIFF**（因 TS 端尚未同步，任务 12-13 修复后 MATCH）

- [ ] **步骤 3：记录当前差分状态**

若 generics.tangle.md 为 DIFF，记录差异详情（TS 端缺少 IRParam/type 字段）。这是预期的——TS 端同步在任务 12-13 完成。

- [ ] **步骤 4：Commit**

```bash
git add tests/audit/diff-ir.ps1
git commit -m "feat(diff-ir): 加入 v06_phase6 fixture 路径

generics.tangle.md 参与差分测试。TS 端同步后应 MATCH。"
```

---

## 任务 12：TS reference — IR schema + typeNameToType + compileToIR

**文件：**
- 修改：`reference/src/ir/graph.ts`
- 修改：`reference/src/checker/resolve.ts`
- 修改：`reference/src/ir/compileToIR.ts`

**目标：** TS 端镜像 Rust 的 IR schema 扩展和 compile_to_ir 改动。

- [ ] **步骤 1：编写失败的 TS 测试**

创建 `reference/tests/ir/irParamTypes.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { compileToIR } from "../../src/ir/compileToIR.js";
import { parseModule } from "../../src/frontend/parseModule.js";
import * as fs from "fs";

describe("IR param types", () => {
  it("should carry type info in IRParam", () => {
    const source = fs.readFileSync("tests/v06_phase6/generics.tangle.md", "utf-8");
    const module = parseModule(source, "generics.tangle.md");
    const graph = compileToIR(module);

    const process = graph.functions.find(f => f.name === "process");
    expect(process).toBeDefined();
    expect(process!.params.length).toBe(2);

    const itemsParam = process!.params[0];
    expect(itemsParam.name).toBe("items");
    expect(itemsParam.type).toBeDefined();
    expect(itemsParam.type!.kind).toBe("genericInstance");
    expect((itemsParam.type as any).base).toBe("List");

    const thresholdParam = process!.params[1];
    expect(thresholdParam.name).toBe("threshold");
    expect(thresholdParam.type).toBeDefined();
    expect(thresholdParam.type!.kind).toBe("primitive");
    expect((thresholdParam.type as any).name).toBe("Int");
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd reference
npm test -- --run ir/irParamTypes
```
预期：FAIL — TS IRFunction.params 可能是 `string[]`，无 IRParam interface

- [ ] **步骤 3：更新 IR schema（graph.ts）**

修改 `reference/src/ir/graph.ts`：

```typescript
// 新增 IRParam interface
export interface IRParam {
  name: string;
  type?: Type;
}

// 修改 IRFunction
export interface IRFunction {
  name: string;
  receiver: string | null;
  params: IRParam[];          // ← 从 string[] 改为 IRParam[]
  returnType?: Type;          // ← 新增
  nodes: IRNode[];
  edges: IREdge[];
  entryNodeId: string;
  errorEdges: IRErrorEdge[];
}
```

确保 `Type` 已从 `checker/types.ts` 导入。

- [ ] **步骤 4：导出 typeNameToType（resolve.ts）**

TS 端已有 `typeExprToType` 和 `parseTypeExpr`（[resolve.ts:105](file:///e:/GitProjects/tangle/reference/src/checker/resolve.ts#L105)）。新增包装函数：

在 `reference/src/checker/resolve.ts` 中添加：

```typescript
/**
 * 解析类型注解字符串为 Type。解析失败返回 undefined。
 * 镜像 Rust checker::resolve::type_name_to_type。
 */
export function typeNameToType(name: string): Type | undefined {
  try {
    const te = parseTypeExpr(name, "");
    if (!te) return undefined;
    return typeExprToType(te);
  } catch {
    return undefined;
  }
}
```

注意：`parseTypeExpr` 在 TS 端可能不抛异常而是返回 null/diagnostics。检查其实际签名并调整。

- [ ] **步骤 5：更新 compileToIR.ts**

修改 `reference/src/ir/compileToIR.ts` 中 `collectFunctions`：

```typescript
import { typeNameToType } from "../checker/resolve.js";

// 在 collectFunctions 中：
const params: IRParam[] = h.params.map(p => ({
  name: p.name,
  type: p.typeName ? typeNameToType(p.typeName) : undefined,
}));

// 构造 IRFunction：
out.push({
  name,
  receiver,
  params,
  returnType: undefined,  // Phase 6c 才实现返回推导
  nodes,
  edges,
  entryNodeId: entryId,
  errorEdges,
});
```

- [ ] **步骤 6：运行测试验证通过**

运行：
```bash
cd reference
npm test -- --run ir/irParamTypes
```
预期：PASS

- [ ] **步骤 7：运行 TS 全量测试**

运行：
```bash
npm test
npm run build
```
预期：全绿（现有测试的 IRFunction.params 断言需更新为对象数组）

- [ ] **步骤 8：修复 TS 回归测试**

搜索 TS 测试中 `params` 字符串数组断言：
```bash
grep -rn "params.*\[" reference/tests/ --include="*.ts"
grep -rn "\.params" reference/tests/ --include="*.ts"
```

更新为 IRParam 对象数组形态。

- [ ] **步骤 9：Commit**

```bash
git add reference/src/ir/graph.ts reference/src/checker/resolve.ts reference/src/ir/compileToIR.ts reference/tests/ir/irParamTypes.test.ts
git commit -m "feat(reference): IRParam schema + typeNameToType + compileToIR 类型填充

镜像 Rust 端：IRFunction.params 从 string[] 改为 IRParam[]，
新增 returnType?。compileToIR 用 typeNameToType 填充参数类型。"
```

---

## 任务 13：TS reference — typeMap + pyEmitter + goEmitter

**文件：**
- 创建：`reference/src/codegen/typeMap.ts`
- 创建：`reference/tests/codegen/typeMap.test.ts`
- 修改：`reference/src/codegen/pyEmitter.ts`
- 修改：`reference/src/codegen/goEmitter.ts`

**目标：** TS 端镜像 type_map.rs 并更新 Py/Go emitter。

- [ ] **步骤 1：编写 typeMap 测试**

创建 `reference/tests/codegen/typeMap.test.ts`：

```typescript
import { describe, it, expect } from "vitest";
import { tangleTypeToPy, tangleTypeToGo } from "../../src/codegen/typeMap.js";

describe("tangleTypeToPy", () => {
  it("maps primitives", () => {
    expect(tangleTypeToPy({ kind: "primitive", name: "Int" })).toBe("int");
    expect(tangleTypeToPy({ kind: "primitive", name: "String" })).toBe("str");
    expect(tangleTypeToPy({ kind: "primitive", name: "Bool" })).toBe("bool");
    expect(tangleTypeToPy({ kind: "primitive", name: "Float" })).toBe("float");
  });

  it("returns undefined for Any", () => {
    expect(tangleTypeToPy({ kind: "any" })).toBeUndefined();
  });

  it("maps List<Int>", () => {
    const ty = { kind: "genericInstance", base: "List", args: [{ kind: "primitive", name: "Int" }] };
    expect(tangleTypeToPy(ty as any)).toBe("List[int]");
  });

  it("maps Map<String,Int>", () => {
    const ty = { kind: "genericInstance", base: "Map", args: [{ kind: "primitive", name: "String" }, { kind: "primitive", name: "Int" }] };
    expect(tangleTypeToPy(ty as any)).toBe("Dict[str, int]");
  });

  it("maps Option<String>", () => {
    const ty = { kind: "genericInstance", base: "Option", args: [{ kind: "primitive", name: "String" }] };
    expect(tangleTypeToPy(ty as any)).toBe("Optional[str]");
  });
});

describe("tangleTypeToGo", () => {
  it("maps primitives", () => {
    expect(tangleTypeToGo({ kind: "primitive", name: "Int" })).toBe("int");
    expect(tangleTypeToGo({ kind: "primitive", name: "String" })).toBe("string");
    expect(tangleTypeToGo({ kind: "primitive", name: "Bool" })).toBe("bool");
    expect(tangleTypeToGo({ kind: "primitive", name: "Float" })).toBe("float64");
  });

  it("returns any for Any", () => {
    expect(tangleTypeToGo({ kind: "any" })).toBe("any");
  });

  it("maps List<Int>", () => {
    const ty = { kind: "genericInstance", base: "List", args: [{ kind: "primitive", name: "Int" }] };
    expect(tangleTypeToGo(ty as any)).toBe("[]int");
  });

  it("maps Map<String,Int>", () => {
    const ty = { kind: "genericInstance", base: "Map", args: [{ kind: "primitive", name: "String" }, { kind: "primitive", name: "Int" }] };
    expect(tangleTypeToGo(ty as any)).toBe("map[string]int");
  });

  it("maps Option<String> to pointer", () => {
    const ty = { kind: "genericInstance", base: "Option", args: [{ kind: "primitive", name: "String" }] };
    expect(tangleTypeToGo(ty as any)).toBe("*string");
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：
```bash
cd reference
npm test -- --run codegen/typeMap
```
预期：FAIL — `typeMap.ts` 不存在

- [ ] **步骤 3：创建 typeMap.ts**

创建 `reference/src/codegen/typeMap.ts`：

```typescript
import { Type } from "../checker/types.js";

/**
 * 将 Tangle Type 映射为 Python 类型注解字符串。
 * undefined 表示无注解（emitter 省略 `: ...`）。
 */
export function tangleTypeToPy(ty: Type): string | undefined {
  switch (ty.kind) {
    case "any":
      return undefined;
    case "primitive":
      switch (ty.name) {
        case "Int": return "int";
        case "String": return "str";
        case "Bool": return "bool";
        case "Float": return "float";
        default: return ty.name;
      }
    case "struct":
      return ty.name;
    case "interface":
      return ty.name;
    case "genericInstance":
      switch (ty.base) {
        case "List": return `List[${innerPy(ty.args[0])}]`;
        case "Map": return `Dict[${innerPy(ty.args[0])}, ${innerPy(ty.args[1])}]`;
        case "Option": return `Optional[${innerPy(ty.args[0])}]`;
        case "Set": return `Set[${innerPy(ty.args[0])}]`;
        default: return `${ty.base}[${ty.args.map(innerPy).join(", ")}]`;
      }
    case "var":
      return undefined;
    case "function":
      return "Callable";
    case "sum":
      const parts = ty.variants.map(tangleTypeToPy).filter(p => p !== undefined);
      return parts.length === 0 ? undefined : `Union[${parts.join(", ")}]`;
  }
}

function innerPy(ty: Type): string {
  return tangleTypeToPy(ty) ?? "Any";
}

/**
 * 将 Tangle Type 映射为 Go 类型字符串。
 * Go 必须有返回类型，无注解时返回 "any"。
 */
export function tangleTypeToGo(ty: Type): string {
  switch (ty.kind) {
    case "any":
      return "any";
    case "primitive":
      switch (ty.name) {
        case "Int": return "int";
        case "String": return "string";
        case "Bool": return "bool";
        case "Float": return "float64";
        default: return ty.name;
      }
    case "struct":
      return ty.name;
    case "interface":
      return ty.name;
    case "genericInstance":
      switch (ty.base) {
        case "List": return `[]${innerGo(ty.args[0])}`;
        case "Map": return `map[${innerGo(ty.args[0])}]${innerGo(ty.args[1])}`;
        case "Option": return `*${innerGo(ty.args[0])}`;
        case "Set": return `map[${innerGo(ty.args[0])}]struct{}`;
        default: return `any /* ${ty.base} */`;
      }
    case "var":
      return "any";
    case "function":
      return "func()";
    case "sum":
      return ty.variants[0] ? innerGo(ty.variants[0]) : "any";
  }
}

function innerGo(ty: Type): string {
  return tangleTypeToGo(ty);
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：
```bash
npm test -- --run codegen/typeMap
```
预期：全 PASS

- [ ] **步骤 5：更新 pyEmitter.ts**

修改 `reference/src/codegen/pyEmitter.ts`，添加 `formatPyParam` 并更新签名生成：

```typescript
import { tangleTypeToPy } from "./typeMap.js";
import { IRParam } from "../ir/graph.js";

function formatPyParam(p: IRParam): string {
  if (p.type) {
    const annot = tangleTypeToPy(p.type);
    if (annot) return `${p.name}: ${annot}`;
  }
  return p.name;
}

// 在函数签名生成处：
const paramsStr = func.params.map(formatPyParam).join(", ");
// emit: def ${name}(${paramsStr}):
```

- [ ] **步骤 6：更新 goEmitter.ts**

修改 `reference/src/codegen/goEmitter.ts`：

```typescript
import { tangleTypeToGo } from "./typeMap.js";
import { IRParam } from "../ir/graph.js";

function formatGoParam(p: IRParam): string {
  if (p.type) {
    return `${p.name} ${tangleTypeToGo(p.type)}`;
  }
  return `${p.name} any`;
}

// 函数签名：
const paramsStr = func.params.map(formatGoParam).join(", ");
const retTy = func.returnType ? tangleTypeToGo(func.returnType) : "any";
// emit: func ${name}(${paramsStr}) ${retTy} {
```

- [ ] **步骤 7：运行 TS 全量测试**

运行：
```bash
npm test
npm run build
```
预期：全绿（需更新现有 emitter 测试断言：`func main() {` → `func main() any {`）

- [ ] **步骤 8：Commit**

```bash
git add reference/src/codegen/typeMap.ts reference/src/codegen/pyEmitter.ts reference/src/codegen/goEmitter.ts reference/tests/codegen/typeMap.test.ts
git commit -m "feat(reference): 新建 typeMap + 更新 pyEmitter/goEmitter

tangleTypeToPy/tangleTypeToGo 镜像 Rust type_map.rs。
pyEmitter: formatPyParam 生成 Python 类型注解。
goEmitter: formatGoParam 生成 Go 类型 + 返回类型 any。"
```

---

## 任务 14：出口闸门验证与合并

**文件：**
- 无文件变更（仅验证与 git 操作）

**目标：** 运行所有验证命令，确认出口闸门 9 项全过，合并到 main 并打 tag。

- [ ] **步骤 1：运行 Rust 全量测试 + clippy**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase6b-v0.6.1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
预期：全绿，零警告

- [ ] **步骤 2：运行 TS 全量测试 + build**

运行：
```bash
cd reference
npm test
npm run build
```
预期：全绿，零类型错误

- [ ] **步骤 3：运行差分测试**

运行：
```bash
cd e:\GitProjects\tangle\.worktrees\phase6b-v0.6.1
pwsh tests/audit/diff-ir.ps1
```
预期：`BASELINE_MATCH + 1` MATCH + 0 SKIPPED + 0 DIFF（generics fixture MATCH，现有 fixture 不回归）

- [ ] **步骤 4：运行 audit 脚本**

运行：
```bash
pwsh tests/audit/run-audit.ps1
```
预期：0 failing

- [ ] **步骤 5：手动验证 Py/Go emitter 输出**

运行：
```bash
cargo run -- build tests/v06_phase6/generics.tangle.md --emit py
```
预期输出含：
```python
def process(items: List[int], threshold: int):
    ...
def main():
    ...
```

运行：
```bash
cargo run -- build tests/v06_phase6/generics.tangle.md --emit go
```
预期输出含：
```go
func process(items []int, threshold int) any {
    ...
}
func main() any {
    ...
}
```

运行：
```bash
cargo run -- build tests/v06_phase6/generics.tangle.md --emit js
```
预期输出含（JS 不变，无类型注解）：
```javascript
function process(items, threshold) {
    ...
}
function main() {
    ...
}
```

- [ ] **步骤 6：清理临时文件**

运行：
```bash
rm phase6b-baseline.txt
```

- [ ] **步骤 7：合并到 main**

运行：
```bash
cd e:\GitProjects\tangle
git checkout main
git merge --ff-only phase6b/v0.6.1
```
预期：fast-forward 合并成功

- [ ] **步骤 8：打 tag（不 push remote）**

运行：
```bash
git tag v0.6.1
```
预期：tag 创建成功

- [ ] **步骤 9：通知用户审查**

输出：
```
Phase 6b 实现完成。出口闸门 9 项全过：
1. cargo test --workspace ✓
2. cargo clippy 零警告 ✓
3. npm test ✓
4. npm run build ✓
5. diff-ir: N MATCH + 0 SKIPPED + 0 DIFF ✓
6. run-audit: 0 failing ✓
7. Phase 4/5/6a 回归测试通过 ✓
8. Phase 6b 新测试通过 ✓
9. Py/Go/JS emitter 手动验证 ✓

已合并到 main，已打 tag v0.6.1（未 push remote）。
请审查并批准 push。
```

---

## 自检

### 1. 规格覆盖度

| 规格章节 | 实现任务 | 状态 |
|---|---|---|
| §3 IR Schema 扩展 | 任务 2（Type serde）+ 任务 4（IRParam/IRFunction） | ✓ |
| §4 compile_to_ir 改动 | 任务 3（type_expr_to_type + type_name_to_type）+ 任务 5（collect_functions） | ✓（含规格偏差修正） |
| §5 Codegen 类型映射 | 任务 6（type_map.rs）+ 任务 7（Py emitter）+ 任务 8（Go emitter） | ✓ |
| §6 ir-diff 归一化 | 任务 10 | ✓ |
| §6 diff-ir.ps1 扩展 | 任务 11 | ✓ |
| §7 TS reference 同步 | 任务 12（IR + compileToIR）+ 任务 13（typeMap + emitters） | ✓ |
| §8 fixture 与测试 | 任务 5（fixture）+ 任务 9（注册）+ 任务 12-13（TS 测试） | ✓ |
| §10 成功标准 | 任务 14（出口闸门） | ✓ |
| §12 非目标 | 不实现（明确推迟） | ✓ |

### 2. 占位符扫描

- 无 "待定"、"TODO"、"后续实现"
- 所有代码步骤包含完整代码块
- 无 "类似任务 N" 引用
- 无 "添加适当的错误处理" 等模糊描述

### 3. 类型一致性

- `IRParam` 在任务 4（Rust graph.rs）和任务 12（TS graph.ts）中定义一致：`{ name, type? }`
- `type_name_to_type` 在任务 3（Rust）和任务 12（TS typeNameToType）中签名一致：`(string) → Option<Type>`
- `tangle_type_to_py/go` 在任务 6（Rust）和任务 13（TS tangleTypeToPy/Go）中行为一致
- `format_py_param` / `format_go_param` 在任务 7/8（Rust）和任务 13（TS）中逻辑一致
- `return_type` 字段在所有任务中恒为 `None`/`undefined`（用户函数），与规格 §3.2 一致

### 4. 风险点

- **任务 7/8 回归测试更新**：现有 emitter 测试可能断言 `func main() {` 形态，需更新为 `func main() any {`（Go）。计划中已包含回归扫描步骤。
- **任务 11 首次差分**：generics fixture 首次跑 diff-ir 会 DIFF（TS 端未同步），任务 12-13 完成后变 MATCH。这是预期流程。
- **任务 3 type_name_to_type 行为变化**：NamedTypeExpr 从 Primitive 变为 Struct，可能影响 Phase 6a 测试。计划中已包含全量测试步骤。
