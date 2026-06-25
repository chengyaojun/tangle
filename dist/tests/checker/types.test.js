import { describe, expect, it } from "vitest";
import { typesEqual, isSubtype } from "../../src/index";
describe("checker types", () => {
    it("defines primitive types", () => {
        const t = { kind: "primitive", name: "String" };
        expect(t.kind).toBe("primitive");
    });
    it("defines struct types with fields", () => {
        const t = {
            kind: "struct", name: "User",
            fields: { id: { kind: "primitive", name: "Int" } },
            methods: {}
        };
        expect(t.fields.id).toEqual({ kind: "primitive", name: "Int" });
    });
    describe("typesEqual", () => {
        it("primitives equal by name", () => {
            expect(typesEqual({ kind: "primitive", name: "String" }, { kind: "primitive", name: "String" })).toBe(true);
            expect(typesEqual({ kind: "primitive", name: "String" }, { kind: "primitive", name: "Int" })).toBe(false);
        });
        it("structs equal by name", () => {
            const a = { kind: "struct", name: "User", fields: {}, methods: {} };
            const b = { kind: "struct", name: "User", fields: {}, methods: {} };
            expect(typesEqual(a, b)).toBe(true);
        });
    });
    describe("isSubtype", () => {
        it("struct satisfies interface with matching method signatures", () => {
            const sig = {
                params: [{ name: "msg", type: { kind: "primitive", name: "String" } }],
                returns: { kind: "primitive", name: "Bool" }
            };
            const iface = { kind: "interface", name: "Notifiable", methods: { notify: sig } };
            const struct = { kind: "struct", name: "User", fields: {}, methods: { notify: sig } };
            expect(isSubtype(struct, iface)).toBe(true);
        });
    });
});
