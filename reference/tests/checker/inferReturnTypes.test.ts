import { describe, it, expect } from "vitest";
import { unifyAll, unifyPair } from "../../src/checker/unify.js";
import type { Type } from "../../src/checker/types.js";

function prim(name: "String" | "Int" | "Bool"): Type {
  return { kind: "primitive", name };
}

function listInt(): Type {
  return { kind: "genericInstance", base: "List", args: [prim("Int")] };
}

describe("unifyAll", () => {
  it("returns null for empty array", () => {
    expect(unifyAll([])).toBeNull();
  });

  it("returns single type", () => {
    expect(unifyAll([prim("Int")])).toEqual(prim("Int"));
  });

  it("unifies same types", () => {
    expect(unifyAll([prim("Int"), prim("Int"), prim("Int")])).toEqual(prim("Int"));
  });

  it("returns null on conflict", () => {
    expect(unifyAll([prim("Int"), prim("String")])).toBeNull();
  });

  it("unifies with Any", () => {
    expect(unifyAll([prim("Int"), { kind: "any" }])).toEqual(prim("Int"));
  });

  it("unifies generic instances", () => {
    expect(unifyAll([listInt(), listInt()])).toEqual(listInt());
  });
});

describe("unifyPair", () => {
  it("unifies same types", () => {
    expect(unifyPair(prim("Int"), prim("Int"))).toEqual(prim("Int"));
  });

  it("returns null on conflict", () => {
    expect(unifyPair(prim("Int"), prim("String"))).toBeNull();
  });

  it("unifies with Any", () => {
    expect(unifyPair(prim("Int"), { kind: "any" })).toEqual(prim("Int"));
  });
});
