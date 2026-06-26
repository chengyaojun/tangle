use serde::{Deserialize, Serialize};
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IREdge {
    pub from: String,
    pub to: String,
    pub kind: IREdgeKind,
    pub guard: Option<String>,
    pub source_span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IRErrorEdge {
    pub from: String,
    pub error_variant: String,
    pub source_span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

pub struct FreshNodeId {
    counter: u64,
}

impl FreshNodeId {
    pub fn new() -> Self { FreshNodeId { counter: 0 } }

    pub fn next(&mut self) -> String {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        id
    }

    pub fn reset(&mut self) { self.counter = 0; }
}

impl Default for RuleGraph {
    fn default() -> Self {
        RuleGraph {
            nodes: vec![], edges: vec![], error_edges: vec![],
            entry_node_id: String::new(), imported_stdlib: vec![], stdlib_imports: vec![],
        }
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
    }
}
