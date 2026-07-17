import { describe, it, expect } from "vitest";
import { checkModule, compileModule } from "../../src/index";

describe("method param type resolution", () => {
  it("resolves struct param type from typeName annotation", () => {
    const mod = compileModule({
      file: "test.md",
      source: `### Order

* \`id\`: order ID (String)

#### confirm (confirm)

* \`order\`: the order (Order)

\`\`\`@tangle
return order.id
\`\`\`
`
    });
    const checked = checkModule(mod);
    // order.id should resolve to String — no unknown member error
    const unknownFieldErrors = checked.diagnostics.filter(
      d => d.code === "TANGLE_TYPE_UNKNOWN_FIELD"
    );
    expect(unknownFieldErrors).toHaveLength(0);
  });

  it("resolves primitive param type from typeName annotation", () => {
    const mod = compileModule({
      file: "test.md",
      source: `### Calc

#### add (add)

* \`x\`: first number (Int)
* \`y\`: second number (Int)

\`\`\`@tangle
return x + y
\`\`\`
`
    });
    const checked = checkModule(mod);
    // x + y should work — both resolved to Int, no type mismatch
    const mismatchErrors = checked.diagnostics.filter(
      d => d.code === "TANGLE_TYPE_MISMATCH"
    );
    expect(mismatchErrors).toHaveLength(0);
  });

  it("falls back to any for params without typeName", () => {
    const mod = compileModule({
      file: "test.md",
      source: `### Worker

#### run (run)

* \`task\`: a task without type

\`\`\`@tangle
return task
\`\`\`
`
    });
    const checked = checkModule(mod);
    // task should be typed as any — no undefined variable error
    const undefinedErrors = checked.diagnostics.filter(
      d => d.code === "TANGLE_TYPE_UNDEFINED_VARIABLE"
    );
    expect(undefinedErrors).toHaveLength(0);
  });
});
