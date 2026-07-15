use tangle_cli::ir::lower_rule_tree::lower_rule_tree;
use tangle_cli::ir::graph::FreshNodeId;

#[test]
fn span_tree_empty_branch_diagnostic_has_nonzero_line() {
    let md = "\
- no_children
- has_action
    - condition
    - Action: do_something
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_tree(md, "test.md", &mut id_gen);
    let empty_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_EMPTY_BRANCH");
    assert!(empty_diag.is_some(), "should emit TANGLE_RULE_EMPTY_BRANCH");
    let diag = empty_diag.unwrap();
    assert_eq!(diag.span.start_line, 1, "empty branch 'no_children' is on line 1");
    assert_eq!(diag.span.file, "test.md");
}

#[test]
fn span_tree_no_action_diagnostic_has_nonzero_line() {
    let md = "\
- first_branch
    - Action: ok
- has_conditions
    - cond_a
    - cond_b
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_tree(md, "test.md", &mut id_gen);
    let no_action_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_NO_ACTION");
    assert!(no_action_diag.is_some(), "should emit TANGLE_RULE_NO_ACTION");
    let diag = no_action_diag.unwrap();
    assert_eq!(diag.span.start_line, 3, "branch 'has_conditions' is on line 3");
}
