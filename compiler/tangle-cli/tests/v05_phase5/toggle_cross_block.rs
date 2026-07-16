//! B4: `@rule.toggle` 跨块不继承 group/style 语义验证。
//!
//! `lower_rule_toggle` 每次调用独立处理单个 toggle 块。前一块的
//! `pending_group`/`pending_style` 不会流入下一块。本测试覆盖：
//! - 跨块不继承 group
//! - 跨块不继承 style
//! - 每块显式声明 group 各自生效
//! - 单块内非注释非 checkbox 行清空 pending 缓存
//!
//! Fixture: `tests/v05_phase5/multi-toggle-blocks.tangle.md` — 两个 toggle 块，
//! 第一块声明 group: UI / style: highlight，第二块不声明，验证第二块为 None。

use tangle_cli::ir::graph::FreshNodeId;
use tangle_cli::ir::lower_rule_toggle;

#[test]
fn toggle_block_isolation_no_group_inheritance() {
    // 第一块：含 group
    let md1 = "<!-- group: UI -->\n- [x] `enable_ui`: Enable UI";
    let mut id_gen = FreshNodeId::new();
    let (graph1, _diags) = lower_rule_toggle(md1, "test.tangle", &mut id_gen);

    // 第二块：不含 group
    let md2 = "- [ ] `enable_crypto`: Enable crypto";
    let (graph2, _diags) = lower_rule_toggle(md2, "test.tangle", &mut id_gen);

    // 第一块的 toggle 节点 group = "UI"
    let toggle1 = graph1
        .nodes
        .iter()
        .find(|n| n.label.contains("enable_ui"))
        .unwrap();
    assert_eq!(toggle1.group.as_deref(), Some("UI"));

    // 第二块的 toggle 节点 group = None（不继承）
    let toggle2 = graph2
        .nodes
        .iter()
        .find(|n| n.label.contains("enable_crypto"))
        .unwrap();
    assert!(
        toggle2.group.is_none(),
        "second block should not inherit group, got: {:?}",
        toggle2.group
    );
}

#[test]
fn toggle_block_isolation_no_style_inheritance() {
    let md1 = "<!-- style: highlight -->\n- [x] `flag1`: desc";
    let mut id_gen = FreshNodeId::new();
    let (graph1, _diags) = lower_rule_toggle(md1, "test.tangle", &mut id_gen);

    let md2 = "- [ ] `flag2`: desc";
    let (graph2, _diags) = lower_rule_toggle(md2, "test.tangle", &mut id_gen);

    let toggle1 = graph1
        .nodes
        .iter()
        .find(|n| n.label.contains("flag1"))
        .unwrap();
    assert_eq!(toggle1.style.as_deref(), Some("highlight"));

    let toggle2 = graph2
        .nodes
        .iter()
        .find(|n| n.label.contains("flag2"))
        .unwrap();
    assert!(
        toggle2.style.is_none(),
        "second block should not inherit style, got: {:?}",
        toggle2.style
    );
}

#[test]
fn toggle_explicit_group_per_block_works() {
    // 每块显式声明 group，都生效
    let md1 = "<!-- group: A -->\n- [x] `flag1`: desc";
    let md2 = "<!-- group: B -->\n- [x] `flag2`: desc";
    let mut id_gen = FreshNodeId::new();

    let (graph1, _diags) = lower_rule_toggle(md1, "test.tangle", &mut id_gen);
    let (graph2, _diags) = lower_rule_toggle(md2, "test.tangle", &mut id_gen);

    let toggle1 = graph1
        .nodes
        .iter()
        .find(|n| n.label.contains("flag1"))
        .unwrap();
    assert_eq!(toggle1.group.as_deref(), Some("A"));

    let toggle2 = graph2
        .nodes
        .iter()
        .find(|n| n.label.contains("flag2"))
        .unwrap();
    assert_eq!(toggle2.group.as_deref(), Some("B"));
}

#[test]
fn toggle_pending_cleared_on_non_checkbox_line() {
    // 单块内：group 缓存遇非注释非 checkbox 行清空
    let md = "<!-- group: UI -->\nsome random text\n- [x] `flag`: desc";
    let mut id_gen = FreshNodeId::new();
    let (graph, _diags) = lower_rule_toggle(md, "test.tangle", &mut id_gen);

    let toggle = graph
        .nodes
        .iter()
        .find(|n| n.label.contains("flag"))
        .unwrap();
    assert!(
        toggle.group.is_none(),
        "group should be cleared by non-checkbox line, got: {:?}",
        toggle.group
    );
}
