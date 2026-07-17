import { describe, it, expect } from "vitest";
import { compileModule, checkModule, compileToIR } from "../../src/index";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

describe("IR param types", () => {
  it("should carry type info in IRParam", () => {
    const fixturePath = path.join(__dirname, "../../../tests/v06_phase6/generics.tangle.md");
    const source = fs.readFileSync(fixturePath, "utf-8");
    const mod = compileModule({ file: fixturePath, source });
    const checked = checkModule(mod);
    const { graph } = compileToIR(checked);

    const functions = (graph as any).functions;
    expect(functions).toBeDefined();
    expect(functions.length).toBeGreaterThan(0);

    const process = functions.find((f: any) => f.name === "process");
    expect(process).toBeDefined();
    expect(process.params.length).toBe(2);

    // items: List<Int>
    const itemsParam = process.params[0];
    expect(itemsParam.name).toBe("items");
    expect(itemsParam.type).toBeDefined();
    expect(itemsParam.type.kind).toBe("genericInstance");
    expect(itemsParam.type.base).toBe("List");

    // threshold: Int
    const thresholdParam = process.params[1];
    expect(thresholdParam.name).toBe("threshold");
    expect(thresholdParam.type).toBeDefined();
    expect(thresholdParam.type.kind).toBe("primitive");
    expect(thresholdParam.type.name).toBe("Int");
  });

  it("should have empty params for main with no parameters", () => {
    const fixturePath = path.join(__dirname, "../../../tests/v06_phase6/generics.tangle.md");
    const source = fs.readFileSync(fixturePath, "utf-8");
    const mod = compileModule({ file: fixturePath, source });
    const checked = checkModule(mod);
    const { graph } = compileToIR(checked);

    const functions = (graph as any).functions;
    const main = functions.find((f: any) => f.name === "main");
    expect(main).toBeDefined();
    expect(main.params).toEqual([]);
  });
});
