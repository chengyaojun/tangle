import { describe, expect, it } from "vitest";
import { compileModule, checkModule, compileToIR } from "../../src/index";
import { USER_MODULE_WITH_INTERFACE } from "../fixtures";

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
