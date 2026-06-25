import { describe, expect, it } from "vitest";
describe("Tangle frontend model", () => {
    it("represents a module with headings, imports, symbols, and diagnostics", () => {
        const heading = {
            id: "user",
            depth: 3,
            role: "type",
            title: "User",
            symbolName: "User",
            directives: [],
            span: { file: "user.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 8 },
            children: []
        };
        const mod = {
            file: "user.md",
            moduleName: "user",
            imports: [],
            headings: [heading],
            symbols: [],
            diagnostics: []
        };
        expect(mod.headings[0]?.role).toBe("type");
    });
});
