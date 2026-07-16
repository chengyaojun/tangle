import { describe, it, expect } from "vitest";
import { lowerRuleTable, rowsOverlap, type TableIREdge } from "../../src/ir/ruleTable.js";

describe("lowerRuleTable", () => {
  // Mirror of Rust table_assigns_priority_by_row_order: 行序即 priority
  it("ruleTable_lower_priority_by_row_order: 行序分配 priority", () => {
    const md = `| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| low | - | review |
| - | poor | reject |`;
    const result = lowerRuleTable(md, "test.md");

    // 3 data rows → entry + 3 action nodes, 3 edges from entry
    const entryEdges = result.graph.edges.filter(
      e => e.from === result.graph.entryNodeId,
    );
    expect(entryEdges.length).toBe(3);
    expect((entryEdges[0] as TableIREdge).priority).toBe(0);
    expect((entryEdges[1] as TableIREdge).priority).toBe(1);
    expect((entryEdges[2] as TableIREdge).priority).toBe(2);
  });

  // Mirror of Rust table_wildcard_omits_condition: '-' 通配 → 无 guard
  it("ruleTable_lower_wildcard_omits_condition: '-' 通配省略 guard", () => {
    const md = `| Income | Result |
|--------|--------|
| - | approve |`;
    const result = lowerRuleTable(md, "test.md");

    const edge = result.graph.edges[0]!;
    expect(edge.guard).toBeUndefined();
  });

  // Mirror of Rust rows_overlap_detects_wildcard_intersection: rowsOverlap 辅助函数
  it("ruleTable_rows_overlap_detects_wildcard: rowsOverlap 通配交集检测", () => {
    expect(rowsOverlap(["high", "-"], ["high", "good"])).toBe(true);
    expect(rowsOverlap(["-", "-"], ["low", "bad"])).toBe(true);
    expect(rowsOverlap(["high", "good"], ["low", "good"])).toBe(false);
  });

  // Mirror of Rust table_overlap_emits_warning: 部分重叠 → TANGLE_RULE_OVERLAP
  it("ruleTable_lower_overlap_emits_warning: 部分重叠发 TANGLE_RULE_OVERLAP", () => {
    const md = `| Income | Credit | Result |
|--------|--------|--------|
| high | - | approve |
| - | good | review |`;
    const result = lowerRuleTable(md, "test.md");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_OVERLAP")).toBe(true);
  });

  // Mirror of Rust table_no_overlap_no_warning: 不重叠 → 无 TANGLE_RULE_OVERLAP
  it("ruleTable_lower_no_overlap_no_warning: 不重叠无警告", () => {
    const md = `| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| low | poor | reject |`;
    const result = lowerRuleTable(md, "test.md");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_OVERLAP")).toBe(false);
  });

  // Mirror of Rust table_unreachable_emits_info: 完全覆盖 → TANGLE_RULE_UNREACHABLE
  it("ruleTable_lower_unreachable_emits_info: 完全覆盖发 TANGLE_RULE_UNREACHABLE", () => {
    const md = `| Income | Credit | Result |
|--------|--------|--------|
| - | - | approve |
| high | good | reject |`;
    const result = lowerRuleTable(md, "test.md");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_UNREACHABLE")).toBe(true);
  });

  // Mirror of Rust table_duplicate_emits_warning: 完全相同 → TANGLE_RULE_DUPLICATE
  it("ruleTable_lower_duplicate_emits_warning: 完全相同发 TANGLE_RULE_DUPLICATE", () => {
    const md = `| Income | Credit | Result |
|--------|--------|--------|
| high | good | approve |
| high | good | reject |`;
    const result = lowerRuleTable(md, "test.md");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_DUPLICATE")).toBe(true);
  });

  // 基础结构验证：entry 为 decision，action 节点为 action kind
  it("ruleTable_lower_basic_structure: 基础节点/边结构", () => {
    const md = `| Income | Result |
|--------|--------|
| high | approve |
| low | reject |`;
    const result = lowerRuleTable(md, "test.tangle");

    // entry + 2 action = 3 nodes
    expect(result.graph.nodes.length).toBe(3);
    const entry = result.graph.nodes[0]!;
    expect(entry.kind).toBe("decision");
    expect(entry.label).toBe("table.entry");
    // 2 edges from entry
    expect(result.graph.edges.length).toBe(2);
    // all edges are condition kind
    expect(result.graph.edges.every(e => e.kind === "condition")).toBe(true);
    // action nodes have action kind
    const actions = result.graph.nodes.slice(1);
    expect(actions.every(n => n.kind === "action")).toBe(true);
  });
});
