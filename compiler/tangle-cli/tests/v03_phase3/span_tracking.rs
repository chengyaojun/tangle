use tangle_cli::ir::lower_rule_tree::lower_rule_tree;
use tangle_cli::ir::lower_rule_table::lower_rule_table_with_diagnostics;
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

#[test]
fn span_table_duplicate_diagnostic_has_exact_line() {
    // 行号（1-based）：
    // Line 1: | status | action |   (header)
    // Line 2: |--------|--------|   (separator, filtered out)
    // Line 3: | -      | ok     |   (row1)
    // Line 4: | -      | ok     |   (row2, duplicate of row1)
    // TANGLE_RULE_DUPLICATE 指向 row j（即 row2，第 4 行）
    let md = "\
| status | action |
|--------|--------|
| -      | ok     |
| -      | ok     |
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
    let dup_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_DUPLICATE");
    assert!(dup_diag.is_some(), "should emit TANGLE_RULE_DUPLICATE for identical rows");
    let diag = dup_diag.unwrap();
    assert_eq!(diag.span.start_line, 4, "duplicate row2 is on line 4, got {}", diag.span.start_line);
    assert_eq!(diag.span.file, "test.md");
}

#[test]
fn span_table_unreachable_diagnostic_has_exact_line() {
    // Line 1: header
    // Line 2: separator (filtered)
    // Line 3: | - | - | approve |  (row1, covers everything)
    // Line 4: | high | good | reject |  (row2, unreachable, covered by row1)
    // TANGLE_RULE_UNREACHABLE 指向 row j（即 row2，第 4 行）
    let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| - | - | approve |
| high | good | reject |
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
    let unreach_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_UNREACHABLE");
    assert!(unreach_diag.is_some(), "should emit TANGLE_RULE_UNREACHABLE");
    let diag = unreach_diag.unwrap();
    assert_eq!(diag.span.start_line, 4, "unreachable row2 is on line 4, got {}", diag.span.start_line);
}

#[test]
fn span_table_overlap_diagnostic_has_exact_line() {
    // Line 1: header
    // Line 2: separator (filtered)
    // Line 3: | high | - | approve |   (row1)
    // Line 4: | - | good | review |    (row2, overlaps row1)
    // TANGLE_RULE_OVERLAP 指向 row j（即 row2，第 4 行）
    let md = "\
| Income | Credit | Result |
|--------|--------|--------|
| high | - | approve |
| - | good | review |
";
    let mut id_gen = FreshNodeId::new();
    let (_graph, diagnostics) = lower_rule_table_with_diagnostics(md, "test.md", &mut id_gen);
    let overlap_diag = diagnostics.iter().find(|d| d.code == "TANGLE_RULE_OVERLAP");
    assert!(overlap_diag.is_some(), "should emit TANGLE_RULE_OVERLAP");
    let diag = overlap_diag.unwrap();
    assert_eq!(diag.span.start_line, 4, "overlap row2 is on line 4, got {}", diag.span.start_line);
}
