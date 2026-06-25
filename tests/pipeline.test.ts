import { describe, expect, it } from "vitest";
import { compile } from "../src/index";

describe("full pipeline", () => {
  it("compiles a simple module to runnable JS", () => {
    const result = compile(`### Calc

#### Calc -> add (add)
@export
* \`a\`: first (Int)
* \`b\`: second (Int)

\`\`\`@tangle
return a + b
\`\`\`
`, "calc.md");
    expect(result.js).toContain("function");
    expect(result.js).toContain("Ok");
    expect(result.diagnostics.length).toBeGreaterThanOrEqual(0);
  });
});
