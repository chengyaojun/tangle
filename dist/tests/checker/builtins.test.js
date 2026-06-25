import { describe, expect, it } from "vitest";
import { builtinTypes, isBuiltinType } from "../../src/index";
describe("builtins", () => {
    it("registers String, Int, Bool", () => {
        expect(builtinTypes.String).toEqual({ kind: "primitive", name: "String" });
        expect(builtinTypes.Int).toEqual({ kind: "primitive", name: "Int" });
        expect(builtinTypes.Bool).toEqual({ kind: "primitive", name: "Bool" });
    });
    it("recognizes builtin type names", () => {
        expect(isBuiltinType("String")).toBe(true);
        expect(isBuiltinType("User")).toBe(false);
    });
});
