import { describe, expect, it } from "vitest";
import { collectLinks, compileModule, headingRoleForDepth, isTangleCodeBlock, parseDirectiveLine, parseHeadingText, parseMarkdown, parseParamItem, plainText, spanFromNode } from "../../src/index";
describe("public API", () => {
    it("exports the Track A1 frontend API", () => {
        expect(typeof collectLinks).toBe("function");
        expect(typeof compileModule).toBe("function");
        expect(typeof headingRoleForDepth).toBe("function");
        expect(typeof isTangleCodeBlock).toBe("function");
        expect(typeof parseDirectiveLine).toBe("function");
        expect(typeof parseHeadingText).toBe("function");
        expect(typeof parseMarkdown).toBe("function");
        expect(typeof parseParamItem).toBe("function");
        expect(typeof plainText).toBe("function");
        expect(typeof spanFromNode).toBe("function");
    });
});
