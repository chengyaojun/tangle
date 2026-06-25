import { describe, expect, it } from "vitest";
import { createEnv } from "../../src/index";
describe("TypeEnv", () => {
    it("starts empty", () => {
        const env = createEnv();
        expect(env.variables).toEqual({});
        expect(env.structs).toEqual({});
        expect(env.interfaces).toEqual({});
    });
    it("adds and looks up variables", () => {
        const env = createEnv();
        env.variables.x = { kind: "primitive", name: "Int" };
        expect(env.variables.x).toEqual({ kind: "primitive", name: "Int" });
    });
    it("sets receiver context", () => {
        const env = createEnv();
        env.receiver = { structName: "User", fields: { is_active: { kind: "primitive", name: "Bool" } } };
        expect(env.receiver.fields.is_active).toEqual({ kind: "primitive", name: "Bool" });
    });
});
