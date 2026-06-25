import { describe, expect, it } from "vitest";
import { ErrorRegistry } from "../../src/index";

describe("ErrorRegistry", () => {
  it("registers and looks up error variants", () => {
    const reg = new ErrorRegistry();
    reg.register("PayFailed", { code: { kind: "primitive", name: "Int" } });
    const variant = reg.lookup("PayFailed");
    expect(variant).toBeDefined();
    expect(variant!.fields.code).toEqual({ kind: "primitive", name: "Int" });
  });

  it("checks if a type is an error variant", () => {
    const reg = new ErrorRegistry();
    reg.register("Timeout", {});
    expect(reg.isError("Timeout")).toBe(true);
    expect(reg.isError("User")).toBe(false);
  });

  it("collects errors from Error: prefix headings", () => {
    const reg = new ErrorRegistry();
    reg.collectFromHeadings([
      { title: "Error: PayFailed" },
    ]);
    expect(reg.isError("PayFailed")).toBe(true);
  });

  it("returns all registered variants", () => {
    const reg = new ErrorRegistry();
    reg.register("A", {});
    reg.register("B", {});
    expect(reg.allVariants()).toHaveLength(2);
  });
});
