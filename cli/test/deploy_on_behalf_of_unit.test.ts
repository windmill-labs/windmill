import { expect, test } from "bun:test";
import { deployItem } from "../windmill-utils-internal/src/deploy.ts";

// `deployItem` spreads the source item into the request body, and a script's/flow's
// on_behalf_of_permissioned_as names a username that only exists in the source
// workspace. Sending it to the target pairs one workspace's principal with the other's
// email, which the backend rejects. Deleting the spread is an easy regression, so pin
// that the key never reaches the wire.
function recordingProvider(captured: [string, any][]) {
  const source = {
    on_behalf_of_email: "alice@corp",
    on_behalf_of_permissioned_as: "u/alice",
  };
  return {
    existsFlowByPath: async () => false,
    existsScriptByPath: async () => true,
    getFlowByPath: async () => ({
      path: "f/x/f",
      summary: "",
      value: { modules: [] },
      ...source,
    }),
    createFlow: async (p: any) => void captured.push(["createFlow", p.requestBody]),
    updateFlow: async (p: any) => void captured.push(["updateFlow", p.requestBody]),
    getScriptByPath: async () => ({
      path: "f/x/s",
      summary: "",
      content: "x",
      language: "bun",
      hash: "abc",
      ...source,
    }),
    createScript: async (p: any) =>
      void captured.push(["createScript", p.requestBody]),
  } as any;
}

test("deployItem: never sends the source workspace's on_behalf_of_permissioned_as", async () => {
  const captured: [string, any][] = [];
  const provider = recordingProvider(captured);

  await deployItem(provider, "flow" as any, "f/x/f", "src", "dst", "alice@corp");
  await deployItem(provider, "script" as any, "f/x/s", "src", "dst", "alice@corp");

  expect(captured.map(([fn]) => fn)).toEqual(["createFlow", "createScript"]);
  for (const [fn, body] of captured) {
    // The email is still overridden with the caller's choice...
    expect(body.on_behalf_of_email).toBe("alice@corp");
    expect(body.preserve_on_behalf_of).toBe(true);
    // ...while the principal is dropped, so the backend derives the target's own.
    expect(
      "on_behalf_of_permissioned_as" in JSON.parse(JSON.stringify(body)),
    ).toBe(false);
  }
});
