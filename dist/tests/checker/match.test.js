import { describe, expect, it } from "vitest";
import { checkMatchExhaustiveness } from "../../src/index";
describe("checkMatchExhaustiveness", () => {
    it("accepts match covering all variants", () => {
        const sumType = {
            kind: "sum",
            variants: [
                { kind: "struct", name: "Receipt", fields: {}, methods: {} },
                { kind: "struct", name: "PayFailed", fields: {}, methods: {} },
            ],
        };
        expect(checkMatchExhaustiveness(sumType, ["Receipt", "PayFailed"])).toHaveLength(0);
    });
    it("reports missing variants", () => {
        const sumType = {
            kind: "sum",
            variants: [
                { kind: "struct", name: "Receipt", fields: {}, methods: {} },
                { kind: "struct", name: "PayFailed", fields: {}, methods: {} },
                { kind: "struct", name: "Timeout", fields: {}, methods: {} },
            ],
        };
        const missing = checkMatchExhaustiveness(sumType, ["Receipt"]);
        expect(missing).toContain("PayFailed");
        expect(missing).toContain("Timeout");
    });
    it("wildcard covers remaining variants", () => {
        const sumType = {
            kind: "sum",
            variants: [
                { kind: "struct", name: "A", fields: {}, methods: {} },
                { kind: "struct", name: "B", fields: {}, methods: {} },
            ],
        };
        expect(checkMatchExhaustiveness(sumType, ["A", "_"])).toHaveLength(0);
    });
});
