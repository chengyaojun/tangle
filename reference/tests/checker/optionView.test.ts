import { describe, it, expect } from "vitest";
import { asSumView, resolveStructInEnv } from "../../src/checker/optionView.js";
import { createEnv } from "../../src/checker/env.js";
import type { Type } from "../../src/checker/types.js";

describe("asSumView", () => {
  it("recognizes Option<Int> as Sum with Some/None variants", () => {
    const opt: Type = {
      kind: "genericInstance",
      base: "Option",
      args: [{ kind: "primitive", name: "Int" }],
    };
    const sum = asSumView(opt);
    expect(sum).not.toBeNull();
    expect(sum?.kind).toBe("sum");
    if (sum?.kind === "sum") {
      expect(sum.variants.length).toBe(2);
      expect(sum.variants[0]).toEqual({
        kind: "genericInstance",
        base: "Some",
        args: [{ kind: "primitive", name: "Int" }],
      });
      expect(sum.variants[1]).toEqual({
        kind: "struct",
        name: "None",
        fields: {},
        methods: {},
      });
    }
  });

  it("passes through existing Sum types unchanged", () => {
    const sum: Type = {
      kind: "sum",
      variants: [{ kind: "primitive", name: "Int" }],
    };
    const view = asSumView(sum);
    expect(view).not.toBeNull();
    expect(view?.kind).toBe("sum");
    if (view?.kind === "sum") {
      expect(view.variants.length).toBe(1);
    }
  });

  it("returns null for non-Option genericInstance", () => {
    const list: Type = {
      kind: "genericInstance",
      base: "List",
      args: [{ kind: "primitive", name: "Int" }],
    };
    expect(asSumView(list)).toBeNull();
  });

  it("returns null for primitive types", () => {
    expect(asSumView({ kind: "primitive", name: "Int" })).toBeNull();
  });

  it("returns null for Any type", () => {
    expect(asSumView({ kind: "any" })).toBeNull();
  });

  it("returns null for struct types", () => {
    expect(
      asSumView({ kind: "struct", name: "Foo", fields: {}, methods: {} })
    ).toBeNull();
  });

  it("handles Option with missing args as Option<Any>", () => {
    const opt: Type = { kind: "genericInstance", base: "Option", args: [] };
    const sum = asSumView(opt);
    expect(sum).not.toBeNull();
    if (sum?.kind === "sum") {
      const some = sum.variants[0]!;
      if (some.kind === "genericInstance") {
        expect(some.base).toBe("Some");
        expect(some.args[0]).toEqual({ kind: "any" });
      } else {
        throw new Error("Expected genericInstance for Some variant");
      }
    }
  });
});

describe("resolveStructInEnv", () => {
  it("fills empty fields from env.structs", () => {
    const env = createEnv();
    env.structs["Item"] = {
      kind: "struct",
      name: "Item",
      fields: {
        name: { kind: "primitive", name: "String" },
        price: { kind: "primitive", name: "Int" },
      },
      methods: {},
    };
    const emptyItem: Type = {
      kind: "struct",
      name: "Item",
      fields: {},
      methods: {},
    };
    const resolved = resolveStructInEnv(emptyItem, env);
    expect(resolved.kind).toBe("struct");
    if (resolved.kind === "struct") {
      expect(Object.keys(resolved.fields).length).toBe(2);
      expect(resolved.fields["name"]).toEqual({ kind: "primitive", name: "String" });
      expect(resolved.fields["price"]).toEqual({ kind: "primitive", name: "Int" });
    }
  });

  it("passes through non-struct types unchanged", () => {
    const env = createEnv();
    const resolved = resolveStructInEnv({ kind: "primitive", name: "Int" }, env);
    expect(resolved).toEqual({ kind: "primitive", name: "Int" });
  });

  it("passes through struct when env.structs lacks entry", () => {
    const env = createEnv();
    const shell: Type = {
      kind: "struct",
      name: "Unknown",
      fields: {},
      methods: {},
    };
    const resolved = resolveStructInEnv(shell, env);
    expect(resolved).toEqual(shell);
  });
});
