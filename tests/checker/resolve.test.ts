import { describe, expect, it } from "vitest";
import { compileModule, resolveTypes } from "../../src/index";

describe("resolveTypes", () => {
  it("resolves struct fields from type heading params", () => {
    const mod = compileModule({
      file: "user.md",
      source: `### User
* \`id\`: user ID (Int)
* \`email\`: email (String)
* \`is_active\`: active flag (Bool)
`
    });
    const env = resolveTypes(mod);
    const userStruct = env.structs.User;
    expect(userStruct).toBeDefined();
    expect(userStruct!.kind).toBe("struct");
    expect(userStruct!.fields.id).toEqual({ kind: "primitive", name: "Int" });
    expect(userStruct!.fields.email).toEqual({ kind: "primitive", name: "String" });
    expect(userStruct!.fields.is_active).toEqual({ kind: "primitive", name: "Bool" });
  });

  it("resolves methods from callable headings", () => {
    const mod = compileModule({
      file: "user.md",
      source: `### User
* \`id\`: user ID (Int)

#### User -> activate (activate)
* \`message\`: notification (String)
`
    });
    const env = resolveTypes(mod);
    expect(env.structs.User?.methods.activate).toBeDefined();
    expect(env.structs.User!.methods.activate!.params).toEqual([
      { name: "message", type: { kind: "primitive", name: "String" } }
    ]);
  });

  it("resolves interface types from headings marked (接口)", () => {
    const mod = compileModule({
      file: "notify.md",
      source: `### Notifyable (接口)

#### Notifyable -> send (send)
* \`msg\`: message (String)
`
    });
    const env = resolveTypes(mod);
    expect(env.interfaces.Notifyable).toBeDefined();
    expect(env.interfaces.Notifyable!.methods.send).toBeDefined();
  });
});
