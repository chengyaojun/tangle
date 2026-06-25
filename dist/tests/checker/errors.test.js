import { describe, expect, it } from "vitest";
import { ErrorRegistry } from "../../src/index";
describe("ErrorRegistry", () => {
    it("registers and looks up error variants", () => {
        const reg = new ErrorRegistry();
        reg.register("PayFailed", { code: { kind: "primitive", name: "Int" } });
        const variant = reg.lookup("PayFailed");
        expect(variant).toBeDefined();
        expect(variant.fields.code).toEqual({ kind: "primitive", name: "Int" });
    });
    it("checks if a type is an error variant", () => {
        const reg = new ErrorRegistry();
        reg.register("Timeout", {});
        expect(reg.isError("Timeout")).toBe(true);
        expect(reg.isError("User")).toBe(false);
    });
    it("collects error variants from @error directives", () => {
        const reg = new ErrorRegistry();
        reg.collectFromDirectives([
            {
                kind: "error",
                raw: '@error PayFailed("支付失败", code: Int)',
                name: "PayFailed",
                args: '"支付失败", code: Int',
                span: { file: "t.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 40 },
            },
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
