import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { checkModule, compileModule } from "../../src/index";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

describe("order-service type checking", () => {
  it("has no type errors", () => {
    const source = readFileSync(
      join(__dirname, "../../../tests/mvp/order-service.tangle.md"),
      "utf-8"
    );
    const mod = compileModule({ file: "order-service.tangle.md", source });
    const checked = checkModule(mod);
    const typeErrors = checked.diagnostics.filter(d => d.code.startsWith("TANGLE_TYPE"));
    expect(typeErrors).toHaveLength(0);
  });
});
