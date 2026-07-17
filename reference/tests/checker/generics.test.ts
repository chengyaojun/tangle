import { describe, it, expect } from "vitest";
import { unify, substitute, type Substitution } from "../../src/checker/unify.js";
import { typeVar, generic } from "../../src/checker/types.js";
import type { Type } from "../../src/checker/types.js";

const intType: Type = { kind: "primitive", name: "Int" };
const strType: Type = { kind: "primitive", name: "String" };

describe("unify algorithm", () => {
  it("binds type variable", () => {
    const subst: Substitution = new Map();
    const err = unify(typeVar(0), intType, subst);
    expect(err).toBeNull();
    expect(subst.get(0)).toEqual(intType);
  });

  it("consistent type variable", () => {
    const subst: Substitution = new Map();
    unify(typeVar(0), intType, subst);
    const err = unify(typeVar(0), intType, subst);
    expect(err).toBeNull();
  });

  it("conflicting type variable", () => {
    const subst: Substitution = new Map();
    unify(typeVar(0), intType, subst);
    const err = unify(typeVar(0), strType, subst);
    expect(err).not.toBeNull();
  });

  it("nested generic", () => {
    const subst: Substitution = new Map();
    const err = unify(generic("List", [typeVar(0)]), generic("List", [intType]), subst);
    expect(err).toBeNull();
    expect(subst.get(0)).toEqual(intType);
  });

  it("function type", () => {
    const subst: Substitution = new Map();
    const expected: Type = { kind: "function", params: [typeVar(0)], returns: typeVar(1) };
    const actual: Type = { kind: "function", params: [intType], returns: strType };
    const err = unify(expected, actual, subst);
    expect(err).toBeNull();
    expect(subst.get(0)).toEqual(intType);
    expect(subst.get(1)).toEqual(strType);
  });

  it("any always succeeds", () => {
    const subst: Substitution = new Map();
    const err = unify({ kind: "any" }, intType, subst);
    expect(err).toBeNull();
    expect(subst.size).toBe(0);
  });
});

describe("substitute", () => {
  it("replaces type variable", () => {
    const subst: Substitution = new Map([[0, intType]]);
    expect(substitute(typeVar(0), subst)).toEqual(intType);
  });

  it("recursive generic", () => {
    const subst: Substitution = new Map([[0, intType]]);
    const result = substitute(generic("List", [typeVar(0)]), subst);
    expect(result).toEqual(generic("List", [intType]));
  });
});
