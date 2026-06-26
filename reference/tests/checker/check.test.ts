import { describe, expect, it } from "vitest";
import { checkExpression, createEnv } from "../../src/index";
import type { TypeEnv } from "../../src/index";
import { tokenize, parseExpression } from "../../src/index";

function makeEnv(overrides?: Partial<TypeEnv>): TypeEnv {
  return { variables: {}, structs: {}, interfaces: {}, ...overrides };
}

describe("checkExpression", () => {
  it("types number literals as Int", () => {
    const [type, diags] = checkExpression(parseExpression(tokenize("42", "test.md")), makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "Int" });
    expect(diags).toHaveLength(0);
  });

  it("types string literals as String", () => {
    const [type] = checkExpression(parseExpression(tokenize('"hello"', "test.md")), makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "String" });
  });

  it("types boolean literals as Bool", () => {
    const [type] = checkExpression(parseExpression(tokenize("true", "test.md")), makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "Bool" });
  });

  it("looks up variables in env", () => {
    const env = makeEnv({ variables: { count: { kind: "primitive", name: "Int" } } });
    const [type] = checkExpression(parseExpression(tokenize("count", "test.md")), env);
    expect(type).toEqual({ kind: "primitive", name: "Int" });
  });

  it("reports undefined variable", () => {
    const [, diags] = checkExpression(parseExpression(tokenize("unknownVar", "test.md")), makeEnv());
    expect(diags).toEqual([expect.objectContaining({ code: "TANGLE_TYPE_UNDEFINED_VARIABLE" })]);
  });

  it("types this with receiver context", () => {
    const env = makeEnv({
      receiver: { structName: "User", fields: { email: { kind: "primitive", name: "String" } } }
    });
    const [type] = checkExpression(parseExpression(tokenize("this", "test.md")), env);
    expect(type).toMatchObject({ kind: "struct", name: "User" });
  });

  it("types member access on this", () => {
    const env = makeEnv({
      receiver: { structName: "User", fields: { email: { kind: "primitive", name: "String" } } }
    });
    const [type] = checkExpression(parseExpression(tokenize("this.email", "test.md")), env);
    expect(type).toEqual({ kind: "primitive", name: "String" });
  });

  it("types record-update checking fields", () => {
    const userStruct = { kind: "struct" as const, name: "User", fields: { is_active: { kind: "primitive" as const, name: "Bool" as const } }, methods: {} };
    const env = makeEnv({ structs: { User: userStruct }, variables: { user: userStruct } });
    const [type] = checkExpression(parseExpression(tokenize("user { is_active: true }", "test.md")), env);
    expect(type).toMatchObject({ kind: "struct", name: "User" });
  });

  it("reports error for unknown field in record-update", () => {
    const userStruct = { kind: "struct" as const, name: "User", fields: { is_active: { kind: "primitive" as const, name: "Bool" as const } }, methods: {} };
    const env = makeEnv({ structs: { User: userStruct }, variables: { user: userStruct } });
    const [, diags] = checkExpression(parseExpression(tokenize("user { unknown_field: true }", "test.md")), env);
    expect(diags).toEqual([expect.objectContaining({ code: "TANGLE_TYPE_UNKNOWN_FIELD" })]);
  });

  it("types binary arithmetic returning Int", () => {
    const [type] = checkExpression(parseExpression(tokenize("1 + 2", "test.md")), makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "Int" });
  });

  it("reports type mismatch in binary", () => {
    const [, diags] = checkExpression(parseExpression(tokenize('1 + "hello"', "test.md")), makeEnv());
    expect(diags).toEqual([expect.objectContaining({ code: "TANGLE_TYPE_MISMATCH" })]);
  });
});
