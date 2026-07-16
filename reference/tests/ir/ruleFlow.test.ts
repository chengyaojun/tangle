import { describe, it, expect } from "vitest";
import {
  lowerRuleFlow,
  type FlowIRNode,
  type FlowIREdge,
} from "../../src/ir/ruleFlow.js";

describe("lowerRuleFlow", () => {
  // Mirror of Rust flow_subgraph_assigns_group
  it("ruleFlow_subgraph_assigns_group: subgraph → group 字段", () => {
    const md = `graph TD
    A[Start] --> B{Decision}
    subgraph Approval
        B -->|yes| C[Approve]
    end
    subgraph Rejection
        B -->|no| E[Reject]
    end`;
    const result = lowerRuleFlow(md, "test.tangle");

    const nodeC = result.graph.nodes.find(n => n.label === "Approve");
    expect(nodeC).toBeDefined();
    expect((nodeC as FlowIRNode).group).toBe("Approval");
    const nodeE = result.graph.nodes.find(n => n.label === "Reject");
    expect(nodeE).toBeDefined();
    expect((nodeE as FlowIRNode).group).toBe("Rejection");
    const nodeA = result.graph.nodes.find(n => n.label === "Start");
    expect(nodeA).toBeDefined();
    expect((nodeA as FlowIRNode).group).toBeNull();
  });

  // Mirror of Rust flow_no_subgraph_group_none
  it("ruleFlow_no_subgraph_group_none: 无 subgraph → group null", () => {
    const md = `graph TD
    A[Start] --> B[End]`;
    const result = lowerRuleFlow(md, "test.tangle");

    for (const node of result.graph.nodes) {
      expect((node as FlowIRNode).group).toBeNull();
    }
  });

  // Mirror of Rust flow_class_assigns_style
  it("ruleFlow_class_assigns_style: classDef + class → style", () => {
    const md = `graph TD
    A[Start] --> B[End]
    classDef highlight fill:#ff0,stroke:#f00
    class B highlight`;
    const result = lowerRuleFlow(md, "test.tangle");

    const nodeB = result.graph.nodes.find(n => n.label === "End");
    expect(nodeB).toBeDefined();
    expect((nodeB as FlowIRNode).style).toBe("fill:#ff0,stroke:#f00");
  });

  // Mirror of Rust flow_style_assigns_to_node
  it("ruleFlow_style_assigns_to_node: style 指令 → node.style", () => {
    const md = `graph TD
    A[Start] --> B[End]
    style B fill:#cfc`;
    const result = lowerRuleFlow(md, "test.tangle");

    const nodeB = result.graph.nodes.find(n => n.label === "End");
    expect(nodeB).toBeDefined();
    expect((nodeB as FlowIRNode).style).toBe("fill:#cfc");
  });

  // Mirror of Rust flow_dashed_edge_maps_to_dashed
  it("ruleFlow_dashed_edge: -.-> → dashed", () => {
    const md = `graph TD
    A[Start] -.-> B[Async]`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.edges.length).toBe(1);
    expect((result.graph.edges[0] as FlowIREdge).kind).toBe("dashed");
  });

  // Mirror of Rust flow_thick_edge_maps_to_thick
  it("ruleFlow_thick_edge: ==> → thick", () => {
    const md = `graph TD
    A[Start] ==> B[Critical]`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.edges.length).toBe(1);
    expect((result.graph.edges[0] as FlowIREdge).kind).toBe("thick");
  });

  // Mirror of Rust flow_crossed_edge_maps_to_crossed
  it("ruleFlow_crossed_edge: --x → crossed", () => {
    const md = `graph TD
    A[Start] --x B[Failed]`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.edges.length).toBe(1);
    expect((result.graph.edges[0] as FlowIREdge).kind).toBe("crossed");
  });

  // Mirror of Rust flow_dashed_edge_with_guard
  it("ruleFlow_dashed_edge_with_guard: guard 存在 → Condition 覆盖 kind", () => {
    const md = `graph TD
    A[Start] -.->|async| B[Done]`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.edges.length).toBe(1);
    const edge = result.graph.edges[0] as FlowIREdge;
    expect(edge.kind).toBe("condition");
    expect(edge.guard).toBe("async");
  });

  // Mirror of Rust flow_diamond_shape_maps_to_decision
  it("ruleFlow_diamond_shape: {Label} → decision", () => {
    const md = `graph TD
    A{Is valid?}`;
    const result = lowerRuleFlow(md, "test.tangle");

    const node = result.graph.nodes.find(n => n.label === "Is valid?");
    expect(node).toBeDefined();
    expect(node!.kind).toBe("decision");
  });

  // Mirror of Rust flow_circle_shape_maps_to_terminal
  it("ruleFlow_circle_shape: ((Label)) → terminal", () => {
    const md = `graph TD
    A((Start))`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.nodes.length).toBe(1);
    expect(result.graph.nodes[0]!.kind).toBe("terminal");
  });

  // Mirror of Rust flow_rect_shape_maps_to_action
  it("ruleFlow_rect_shape: [Label] → action", () => {
    const md = `graph TD
    A[Do something]`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.nodes.length).toBe(1);
    expect(result.graph.nodes[0]!.kind).toBe("action");
  });

  // Mirror of Rust flow_inline_rect_with_brace_in_label_maps_to_action
  it("ruleFlow_inline_rect_with_brace_in_label: [Output: {json}] 仍是 action", () => {
    const md = `graph TD
    A[Start] --> B[Output: {json}]`;
    const result = lowerRuleFlow(md, "test.tangle");

    const b = result.graph.nodes.find(n => n.label === "Output: {json}");
    expect(b).toBeDefined();
    expect(b!.kind).toBe("action");
  });

  // Mirror of Rust flow_rounded_shape_maps_to_action
  it("ruleFlow_rounded_shape: (Label) → action", () => {
    const md = `graph TD
    A(Do something)`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.nodes.length).toBe(1);
    expect(result.graph.nodes[0]!.kind).toBe("action");
  });

  // Mirror of Rust flow_inline_diamond_shape_maps_to_decision
  it("ruleFlow_inline_diamond_shape: --> B{Label} → decision", () => {
    const md = `graph TD
    A[Start] --> B{Is valid?}`;
    const result = lowerRuleFlow(md, "test.tangle");

    const b = result.graph.nodes.find(n => n.label === "Is valid?");
    expect(b).toBeDefined();
    expect(b!.kind).toBe("decision");
  });

  // Mirror of Rust flow_inline_circle_shape_maps_to_terminal
  it("ruleFlow_inline_circle_shape: --> B((Label)) → terminal", () => {
    const md = `graph TD
    A[Start] --> B((End))`;
    const result = lowerRuleFlow(md, "test.tangle");

    const b = result.graph.nodes.find(n => n.label === "End");
    expect(b).toBeDefined();
    expect(b!.kind).toBe("terminal");
  });

  // Mirror of Rust flow_entry_is_first_node_with_no_incoming
  it("ruleFlow_entry_is_first_no_incoming: 首个无入边节点为 entry", () => {
    const md = `graph TD
    A[Start] --> B[Middle]
    B --> C[End]`;
    const result = lowerRuleFlow(md, "test.tangle");

    const entryNode = result.graph.nodes.find(n => n.id === result.graph.entryNodeId);
    expect(entryNode).toBeDefined();
    expect(entryNode!.label).toBe("Start");
  });

  // Mirror of Rust flow_multi_entry_picks_first
  it("ruleFlow_multi_entry_picks_first: 多个无入边节点取首个", () => {
    const md = `graph TD
    A[Start1] --> B[Mid]
    D[Start2] --> B`;
    const result = lowerRuleFlow(md, "test.tangle");

    const entryNode = result.graph.nodes.find(n => n.id === result.graph.entryNodeId);
    expect(entryNode).toBeDefined();
    expect(entryNode!.label).toBe("Start1");
  });

  // Mirror of Rust flow_entry_skips_node_with_incoming
  it("ruleFlow_entry_skips_node_with_incoming: 有入边的节点被跳过", () => {
    const md = `graph TD
    A[Start]
    B[Mid] --> A`;
    const result = lowerRuleFlow(md, "test.tangle");

    const entryNode = result.graph.nodes.find(n => n.id === result.graph.entryNodeId);
    expect(entryNode).toBeDefined();
    expect(entryNode!.label).toBe("Mid");
  });

  // Mirror of Rust flow_cyclic_falls_back_to_first_declared
  it("ruleFlow_cyclic_falls_back_to_first_declared: 循环图回退到首声明", () => {
    const md = `graph TD
    A[Start] --> B[Mid]
    B --> A`;
    const result = lowerRuleFlow(md, "test.tangle");

    const entryNode = result.graph.nodes.find(n => n.id === result.graph.entryNodeId);
    expect(entryNode).toBeDefined();
    expect(entryNode!.label).toBe("Start");
  });

  // Mirror of Rust flow_empty_graph_creates_terminal
  it("ruleFlow_empty_graph_creates_terminal: 空图创建 empty Terminal", () => {
    const md = `graph TD`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.nodes.length).toBe(1);
    expect(result.graph.nodes[0]!.kind).toBe("terminal");
    expect(result.graph.nodes[0]!.label).toBe("empty");
    expect(result.graph.entryNodeId).toBe(result.graph.nodes[0]!.id);
  });

  // 错误终端检测：error: 前缀 → error-terminal
  it("ruleFlow_error_terminal: error: 前缀 → error-terminal", () => {
    const md = `graph TD
    A[Start] --> B[Error: PayFailed]`;
    const result = lowerRuleFlow(md, "test.tangle");

    const errNode = result.graph.nodes.find(n => n.kind === "error-terminal");
    expect(errNode).toBeDefined();
  });

  // 中文错误终端检测：错误: 前缀 → error-terminal
  it("ruleFlow_error_terminal_chinese: 错误: 前缀 → error-terminal", () => {
    const md = `graph TD
    A[Start] --> B(错误: PayFailed)`;
    const result = lowerRuleFlow(md, "test.tangle");

    const errNode = result.graph.nodes.find(n => n.kind === "error-terminal");
    expect(errNode).toBeDefined();
  });

  // linkStyle 按索引应用样式到 edge
  it("ruleFlow_linkStyle_by_index: linkStyle 按索引应用 edge.style", () => {
    const md = `graph TD
    A[Start] --> B[Mid]
    B --> C[End]
    linkStyle 1 stroke:#f00`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.edges.length).toBe(2);
    const edge0 = result.graph.edges[0] as FlowIREdge;
    const edge1 = result.graph.edges[1] as FlowIREdge;
    expect(edge0.style).toBeNull();
    expect(edge1.style).toBe("stroke:#f00");
  });

  // 基础结构验证：4 节点 3 边
  it("ruleFlow_basic_graph: 基础 Mermaid graph", () => {
    const md = `graph TD
    A[Start] --> B{Decision}
    B -->|yes| C[Action]
    B -->|no| D[End]`;
    const result = lowerRuleFlow(md, "test.tangle");

    expect(result.graph.nodes.length).toBe(4);
    expect(result.graph.edges.length).toBe(3);
    // diagnostics should always be empty (Rust emits none)
    expect(result.diagnostics).toHaveLength(0);
  });
});
