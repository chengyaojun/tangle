use tangle_cli::ir::graph::*;
use tangle_cli::ir::lower_rule_toggle::lower_rule_toggle;
use tangle_cli::model::TangleDiagnostic;

fn fresh_id_gen() -> FreshNodeId {
    FreshNodeId::new()
}

#[test]
fn toggle_span_tracking_populates_line_numbers() {
    let md = "- [x] enable_new_ui: true\n- [ ] enable_crypto: false";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    // nodes[0] is entry (span None), nodes[1] and nodes[2] are checkboxes
    assert!(graph.nodes[1].source_span.is_some(), "checkbox node should have span");
    let span = graph.nodes[1].source_span.as_ref().unwrap();
    assert_eq!(span.start_line, 1, "first checkbox on line 1");
    assert_eq!(span.file, "test.tangle");
    let span2 = graph.nodes[2].source_span.as_ref().unwrap();
    assert_eq!(span2.start_line, 2, "second checkbox on line 2");
}

#[test]
fn toggle_name_extraction_from_colon() {
    let md = "- [x] enable_new_ui: true";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert_eq!(graph.nodes[1].label, "enable_new_ui = true");
}

#[test]
fn toggle_name_extraction_from_backtick() {
    let md = "- [x] `enable_new_ui`: some description";
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert_eq!(graph.nodes[1].label, "enable_new_ui = true");
}

#[test]
fn toggle_missing_name_emits_diagnostic() {
    let md = "- [x] no name here";
    let (_graph, diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_TOGGLE_MISSING_NAME"),
        "should emit MISSING_NAME diagnostic, got: {:?}", diags);
}

#[test]
fn toggle_malformed_checkbox_emits_diagnostic() {
    let md = "- [?] invalid checkbox";
    let (_graph, diags) = lower_rule_toggle(md, "test.tangle", &mut fresh_id_gen());
    assert!(diags.iter().any(|d| d.code == "TANGLE_RULE_TOGGLE_MALFORMED"),
        "should emit MALFORMED diagnostic, got: {:?}", diags);
}
