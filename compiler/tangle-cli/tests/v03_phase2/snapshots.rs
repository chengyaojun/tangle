use insta::assert_json_snapshot;
use tangle_cli::ir::graph::{RuleGraph, FreshNodeId};
use tangle_cli::ir::lower_rule_tree::lower_rule_tree;
use tangle_cli::ir::lower_rule_table::lower_rule_table;
use tangle_cli::ir::lower_rule_flow::lower_rule_flow;

fn graph_to_json(graph: &RuleGraph) -> serde_json::Value {
    serde_json::to_value(graph).unwrap()
}

#[test]
fn snapshot_tree_basic() {
    let md = "\
* Approve path
    * Income: high
    * Action: approve
";
    let mut id_gen = FreshNodeId::new();
    let (graph, _diags) = lower_rule_tree(md, "test.md", &mut id_gen);
    assert_json_snapshot!(graph_to_json(&graph));
}

#[test]
fn snapshot_tree_dnf() {
    let md = "\
* Branch A
    * Income: high
    * Credit: good
    * Action: approve
* Branch B
    * Income: low
    * Action: reject
";
    let mut id_gen = FreshNodeId::new();
    let (graph, _diags) = lower_rule_tree(md, "test.md", &mut id_gen);
    assert_json_snapshot!(graph_to_json(&graph));
}

#[test]
fn snapshot_table_basic() {
    let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| low | poor | reject |
";
    let mut id_gen = FreshNodeId::new();
    let graph = lower_rule_table(md, "test.md", &mut id_gen);
    assert_json_snapshot!(graph_to_json(&graph));
}

#[test]
fn snapshot_table_overlap() {
    let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| high | - | approve |
| - | good | review |
";
    let mut id_gen = FreshNodeId::new();
    let graph = lower_rule_table(md, "test.md", &mut id_gen);
    assert_json_snapshot!(graph_to_json(&graph));
}

#[test]
fn snapshot_flow_basic() {
    let md = "\
graph TD
    A[Start] -->|approved| B(Done: Approved)
    A -->|rejected| C(Done: Rejected)
";
    let mut id_gen = FreshNodeId::new();
    let graph = lower_rule_flow(md, "test.md", &mut id_gen);
    assert_json_snapshot!(graph_to_json(&graph));
}

#[test]
fn snapshot_flow_subgraph() {
    let md = "\
graph TD
    A[Start] --> B{Decision}
    subgraph Approval
        B -->|yes| C[Approve]
    end
    subgraph Rejection
        B -->|no| E[Reject]
    end
";
    let mut id_gen = FreshNodeId::new();
    let graph = lower_rule_flow(md, "test.md", &mut id_gen);
    assert_json_snapshot!(graph_to_json(&graph));
}

#[test]
fn snapshot_flow_multi_edge() {
    let md = "\
graph TD
    A[Start] -.-> B[Async]
    A ==> C[Critical]
    A --x D[Failed]
";
    let mut id_gen = FreshNodeId::new();
    let graph = lower_rule_flow(md, "test.md", &mut id_gen);
    assert_json_snapshot!(graph_to_json(&graph));
}
