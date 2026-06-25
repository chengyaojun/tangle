import { describe, expect, it } from "vitest";
import { wrapOk, wrapErr, unwrapOrPropagate } from "../../src/index";

describe("errorMapping", () => {
  it("wraps OK values", () => {
    expect(wrapOk("42")).toBe("Ok(42)");
  });

  it("wraps error variants", () => {
    expect(wrapErr("PayFailed")).toBe('Err("PayFailed")');
  });

  it("generates propagation code", () => {
    expect(unwrapOrPropagate("result")).toBe("__tangle_propagate(result)");
  });
});
