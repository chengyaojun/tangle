use serde::{Deserialize, Serialize};
use crate::checker::types::Type;
use crate::model::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IRNodeKind {
    Action,
    Compute,
    Decision,
    Terminal,
    #[serde(rename = "error-terminal")]
    ErrorTerminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IREdgeKind {
    Control,
    Condition,
    Error,
    Dashed,
    Thick,
    Crossed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IRNode {
    pub id: String,
    pub kind: IRNodeKind,
    pub label: String,
    pub source_span: Option<SourceSpan>,
    #[serde(default)]
    pub source_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IREdge {
    pub from: String,
    pub to: String,
    pub kind: IREdgeKind,
    pub guard: Option<String>,
    pub source_span: Option<SourceSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IRErrorEdge {
    pub from: String,
    pub error_variant: String,
    pub source_span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuleGraph {
    pub nodes: Vec<IRNode>,
    pub edges: Vec<IREdge>,
    pub error_edges: Vec<IRErrorEdge>,
    pub entry_node_id: String,
    #[serde(default)]
    pub imported_stdlib: Vec<String>,
    #[serde(default)]
    pub stdlib_imports: Vec<(String, String)>,  // (alias, target_module)
    /// Heading-defined functions. When non-empty, codegen emits one JS function
    /// per entry (with params as arguments) instead of a single merged function.
    #[serde(default)]
    pub functions: Vec<IRFunction>,
}

/// IR 参数：name + 可选类型（来自 Tangle 源码注解 `param: TypeName`）。
/// `type_` 序列化为 JSON `"type"`（`type` 是 Rust 关键字，故字段名加下划线）。
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

pub struct FreshNodeId {
    counter: u64,
}

impl FreshNodeId {
    pub fn new() -> Self { FreshNodeId { counter: 0 } }

    pub fn fresh(&mut self) -> String {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        id
    }

    pub fn reset(&mut self) { self.counter = 0; }
}

impl Default for FreshNodeId {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_graph(entry_node_id: String) -> RuleGraph {
    RuleGraph {
        nodes: vec![],
        edges: vec![],
        error_edges: vec![],
        entry_node_id,
        imported_stdlib: vec![],
        stdlib_imports: vec![],
        functions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_node_has_group_and_style_fields() {
        let node = IRNode {
            id: "n0".into(),
            kind: IRNodeKind::Action,
            label: "test".into(),
            source_span: None,
            source_text: None,
            group: Some("Approval".into()),
            style: Some("className".into()),
        };
        assert_eq!(node.group.as_deref(), Some("Approval"));
        assert_eq!(node.style.as_deref(), Some("className"));
    }

    #[test]
    fn ir_edge_has_priority_and_style_fields() {
        let edge = IREdge {
            from: "n0".into(),
            to: "n1".into(),
            kind: IREdgeKind::Condition,
            guard: Some("x = 1".into()),
            source_span: None,
            priority: Some(0),
            style: Some("stroke:#ff3".into()),
        };
        assert_eq!(edge.priority, Some(0));
        assert_eq!(edge.style.as_deref(), Some("stroke:#ff3"));
    }

    #[test]
    fn ir_edge_kind_has_new_variants() {
        assert_eq!(IREdgeKind::Dashed, IREdgeKind::Dashed);
        assert_eq!(IREdgeKind::Thick, IREdgeKind::Thick);
        assert_eq!(IREdgeKind::Crossed, IREdgeKind::Crossed);
    }

    #[test]
    fn ir_node_serializes_new_fields() {
        let node = IRNode {
            id: "n0".into(),
            kind: IRNodeKind::Action,
            label: "test".into(),
            source_span: None,
            source_text: None,
            group: Some("G1".into()),
            style: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"group\":\"G1\""));
        assert!(!json.contains("\"style\""));
    }
}

#[cfg(test)]
mod ir_param_tests {
    use super::*;
    use crate::checker::types::{GenericTypeInstance, PrimitiveType, Type};

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
