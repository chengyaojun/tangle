import { describe, expect, it } from "vitest";
import { headingRoleForDepth, parseHeadingText } from "../../src/index";
describe("headingRoleForDepth", () => {
    it("maps six markdown heading levels to Tangle roles", () => {
        expect(headingRoleForDepth(1)).toBe("program");
        expect(headingRoleForDepth(2)).toBe("section");
        expect(headingRoleForDepth(3)).toBe("type");
        expect(headingRoleForDepth(4)).toBe("callable");
        expect(headingRoleForDepth(5)).toBe("semantic-section");
        expect(headingRoleForDepth(6)).toBe("semantic-atom");
    });
});
describe("parseHeadingText", () => {
    it("extracts a stable internal symbol from a trailing parenthesized identifier", () => {
        expect(parseHeadingText("\u53D1\u9001\u901A\u77E5 (send_notification)")).toEqual({
            title: "\u53D1\u9001\u901A\u77E5",
            symbolName: "send_notification"
        });
    });
    it("keeps the full text as title when no internal symbol exists", () => {
        expect(parseHeadingText("\u7528\u6237\u4E2D\u5FC3")).toEqual({
            title: "\u7528\u6237\u4E2D\u5FC3",
            symbolName: undefined
        });
    });
});
