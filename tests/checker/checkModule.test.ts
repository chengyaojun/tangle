import { describe, expect, it } from "vitest";
import { checkModule, compileModule, parseCodeBlocks } from "../../src/index";
import { USER_MODULE_WITH_INTERFACE } from "../fixtures";

describe("checkModule (full pipeline)", () => {
  it("runs A1 compile + A2 parse + A2 type check end-to-end", () => {
    const mod = compileModule({ file: "test.md", source: USER_MODULE_WITH_INTERFACE });
    const checked = checkModule(mod);

    expect(checked.parsedBlocks).toBeDefined();
    expect(checked.parsedBlocks.length).toBeGreaterThanOrEqual(1);
    expect(checked.parsedBlocks[0]!.body.kind).toBe("codeBody");
    expect(checked.typeEnv).toBeDefined();
    expect(checked.typeEnv.structs.User).toBeDefined();
    expect(checked.diagnostics.length).toBeGreaterThanOrEqual(0);
  });

  it("parseCodeBlocks can be called standalone", () => {
    const mod = compileModule({ file: "test.md", source: USER_MODULE_WITH_INTERFACE });
    const blocks = parseCodeBlocks(mod);
    expect(blocks.length).toBeGreaterThanOrEqual(1);
    blocks.forEach(b => {
      expect(b.headingId).toBeTruthy();
      expect(b.body.kind).toBe("codeBody");
    });
  });

  it("reports type errors for unknown field access", () => {
    const mod = compileModule({
      file: "bad.md",
      source: `### User

* \`id\`: user ID (Int)

#### fail (fail)

\`\`\`@tangle
return this.unknown_field
\`\`\`
`
    });
    const checked = checkModule(mod);
    expect(checked.diagnostics.some(d => d.code === "TANGLE_TYPE_UNKNOWN_FIELD")).toBe(true);
  });
});
