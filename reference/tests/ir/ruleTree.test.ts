import { describe, it, expect } from "vitest";
import { lowerRuleTree } from "../../src/ir/ruleTree.js";

describe("lowerRuleTree", () => {
  // Mirror of Rust lower_tree_dnf_basic: DNF 基础（多分支 OR + 链式条件 AND + Action）
  it("ruleTree_lower_dnf_basic: DNF 基础 lowering（OR 分支 + AND 链 + Action）", () => {
    const md = `* Branch A
    * Income: high
    * Credit: good
    * Action: approve
* Branch B
    * Income: low
    * Action: reject`;
    const result = lowerRuleTree(md, "test.tangle");

    // entry + [Income:high, Credit:good, Action:approve] + [Income:low, Action:reject]
    // = 1 + 3 + 2 = 6 nodes
    expect(result.graph.nodes.length).toBe(6);
    // edges: entry→Income:high, Income:high→Credit:good, Credit:good→Action:approve
    //        entry→Income:low, Income:low→Action:reject = 5 edges
    expect(result.graph.edges.length).toBe(5);
    // DNF basic 无 diagnostic
    expect(result.diagnostics.length).toBe(0);
    const kinds = result.graph.nodes.map(n => n.kind);
    expect(kinds).toContain("decision");
    expect(kinds).toContain("action");
  });

  // Mirror of Rust lower_tree_action_node_kind: Action: 子项作为 Action 节点
  it("ruleTree_lower_action_node_kind: Action: 子项作为 Action 节点", () => {
    const md = `* Branch A
    * Action: approve`;
    const result = lowerRuleTree(md, "test.tangle");

    const actionNode = result.graph.nodes.find(n => n.label === "approve");
    expect(actionNode).toBeDefined();
    expect(actionNode!.kind).toBe("action");
  });

  // Mirror of Rust lower_tree_no_action_warns: 分支无 Action 标记产生 diagnostic
  it("ruleTree_lower_no_action_warns: 分支无 Action 标记警告", () => {
    const md = `* Branch A
    * Income: high`;
    const result = lowerRuleTree(md, "test.tangle");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_NO_ACTION")).toBe(true);
  });

  // Mirror of Rust lower_tree_empty_branch_warns: 空分支产生 diagnostic
  it("ruleTree_lower_empty_branch_warns: 空分支警告", () => {
    const md = `* Branch A`;
    const result = lowerRuleTree(md, "test.tangle");

    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_EMPTY_BRANCH")).toBe(true);
  });

  // AND 语义：链式条件验证（entry → cond1 → cond2 → action）
  it("ruleTree_lower_chained_conditions_and: AND 链式条件", () => {
    const md = `* Branch
    * Cond A
    * Cond B
    * Action: X`;
    const result = lowerRuleTree(md, "test.tangle");

    // entry + Cond A + Cond B + Action X = 4 nodes
    expect(result.graph.nodes.length).toBe(4);
    // edges: entry→A, A→B, B→X = 3 edges（全是 condition 类型直到 action）
    expect(result.graph.edges.length).toBe(3);
    // 前 2 条 edge 是 condition（有 guard），最后 1 条是 control（无 guard）
    const condEdges = result.graph.edges.filter(e => e.kind === "condition");
    expect(condEdges.length).toBe(2);
    expect(condEdges.some(e => e.guard !== undefined)).toBe(true);
    const ctrlEdges = result.graph.edges.filter(e => e.kind === "control");
    expect(ctrlEdges.length).toBe(1);
  });

  // OR 语义：多分支从 entry 分叉
  it("ruleTree_lower_multiple_branches_or: OR 多分支分叉", () => {
    const md = `* Branch A
    * Action: X
* Branch B
    * Action: Y`;
    const result = lowerRuleTree(md, "test.tangle");

    // entry + Action X + Action Y = 3 nodes
    expect(result.graph.nodes.length).toBe(3);
    // 2 条 edge 都从 entry 出发（OR 分叉）
    expect(result.graph.edges.length).toBe(2);
    expect(result.graph.edges.every(e => e.from === result.graph.entryNodeId)).toBe(true);
  });

  // 多 Action 标记创建并行 action 节点（Rust 注释明示此语义）
  it("ruleTree_lower_multiple_actions_parallel: 多 Action 并行", () => {
    const md = `* Branch
    * Cond A
    * Action: X
    * Action: Y`;
    const result = lowerRuleTree(md, "test.tangle");

    // entry + Cond A + Action X + Action Y = 4 nodes
    expect(result.graph.nodes.length).toBe(4);
    const actions = result.graph.nodes.filter(n => n.kind === "action");
    expect(actions.length).toBe(2);
    // 两个 action 都从 Cond A 出发（共享 prevId）
    const condA = result.graph.nodes.find(n => n.label === "Cond A");
    expect(condA).toBeDefined();
    const actionEdges = result.graph.edges.filter(e => e.kind === "control");
    expect(actionEdges.length).toBe(2);
    expect(actionEdges.every(e => e.from === condA!.id)).toBe(true);
  });

  // 缩进深度计算（4 空格 = 1 depth，tab = 1 depth）
  it("ruleTree_lower_indent_depth: 4 空格缩进解析", () => {
    const md = `* A
    * B
        * Action: deep`;
    const result = lowerRuleTree(md, "test.tangle");

    // Rust 行为：只处理 root 分支（depth 0）的直接子节点（depth 1）。
    // 这里 A 是 root，B 是 A 的子节点（depth 1），deep 在 depth 2 被跳过。
    // 因此 A 无 Action（B 不是 Action），产生 NO_ACTION diagnostic。
    // 节点：entry + B = 2（A 作为 root 分支不加入节点，仅其 children 被处理）
    expect(result.graph.nodes.length).toBeGreaterThan(0);
    // A 无 Action → diagnostic
    expect(result.diagnostics.some(d => d.code === "TANGLE_RULE_NO_ACTION")).toBe(true);
  });
});
