import { describe, expect, it } from "vitest";
import { compileModule, checkModule, compileToIR } from "../../src/index";
import { USER_MODULE_WITH_INTERFACE } from "../fixtures";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function loadRuleFixture(name: string) {
  const fixturePath = path.join(__dirname, "../../../tests/rules", name);
  const source = fs.readFileSync(fixturePath, "utf-8");
  const mod = compileModule({ file: fixturePath, source });
  return checkModule(mod);
}

describe("compileToIR", () => {
  it("compiles a checked module to IR end-to-end", () => {
    const mod = compileModule({ file: "test.md", source: USER_MODULE_WITH_INTERFACE });
    const checked = checkModule(mod);
    const { graph, diagnostics } = compileToIR(checked);
    expect(graph).toBeDefined();
    expect(graph.nodes.length).toBeGreaterThanOrEqual(1);
    expect(typeof graph.entryNodeId).toBe("string");
  });

  it("produces valid IR for simple module", () => {
    const mod = compileModule({
      file: "simple.md",
      source: `### Calc

#### add (add)

* \`a\`: first (Int)
* \`b\`: second (Int)

\`\`\`@tangle
return a + b
\`\`\`
`
    });
    const checked = checkModule(mod);
    const { graph, diagnostics } = compileToIR(checked);
    expect(graph.nodes.length).toBeGreaterThan(0);
    // No critical IR diagnostics
    const criticalDiags = diagnostics.filter(d => d.code.startsWith("TANGLE_IR_"));
    expect(criticalDiags).toHaveLength(0);
  });
});

describe("compileToIR rule lowering 集成", () => {
  it("decision-tree fixture 生成非空 IR（Tree rule）", () => {
    const checked = loadRuleFixture("decision-tree.tangle.md");
    const { graph } = compileToIR(checked);
    expect(graph.nodes.length).toBeGreaterThan(0);
    expect(graph.edges.length).toBeGreaterThan(0);
  });

  it("feature-toggles fixture 生成非空 IR（Toggle rule）", () => {
    const checked = loadRuleFixture("feature-toggles.tangle.md");
    const { graph } = compileToIR(checked);
    expect(graph.nodes.length).toBeGreaterThan(0);
    expect(graph.edges.length).toBeGreaterThan(0);
  });

  it("decision-table fixture 生成非空 IR（Table rule）", () => {
    const checked = loadRuleFixture("decision-table.tangle.md");
    const { graph } = compileToIR(checked);
    expect(graph.nodes.length).toBeGreaterThan(0);
    expect(graph.edges.length).toBeGreaterThan(0);
  });

  it("decision-table-overlap fixture 生成非空 IR（Table rule + diagnostics）", () => {
    const checked = loadRuleFixture("decision-table-overlap.tangle.md");
    const { graph, diagnostics } = compileToIR(checked);
    expect(graph.nodes.length).toBeGreaterThan(0);
    expect(graph.edges.length).toBeGreaterThan(0);
    // Overlap fixture should produce rule diagnostics
    const ruleDiags = diagnostics.filter(d => d.code.startsWith("TANGLE_RULE_"));
    expect(ruleDiags.length).toBeGreaterThan(0);
  });

  it("approval-flow fixture 生成非空 IR（Flow rule）", () => {
    const checked = loadRuleFixture("approval-flow.tangle.md");
    const { graph } = compileToIR(checked);
    expect(graph.nodes.length).toBeGreaterThan(0);
    expect(graph.edges.length).toBeGreaterThan(0);
  });

  it("approval-flow-subgraph fixture 生成非空 IR（Flow rule + subgraph）", () => {
    const checked = loadRuleFixture("approval-flow-subgraph.tangle.md");
    const { graph } = compileToIR(checked);
    expect(graph.nodes.length).toBeGreaterThan(0);
    expect(graph.edges.length).toBeGreaterThan(0);
  });
});
