import { expect, test } from "bun:test";
import { deployItem } from "../windmill-utils-internal/src/deploy.ts";

// `deployItem` spreads the source item into the request body, and the principal it carries
// (`on_behalf_of`, at the top level for a script or flow and inside the policy for an app)
// names a username that only exists in the source workspace. Sending it to the target would
// run the item as whoever holds that name there, or as nobody. Deleting the overwrite is an
// easy regression, so pin that the source principal never reaches the wire — and that each
// kind sends the identity in the form it takes: an address for a flow or script, the target's
// own principal for an app.
function recordingProvider(captured: [string, any][], flowExists: boolean) {
  const source = {
    on_behalf_of_email: "alice@corp",
    on_behalf_of: "u/alice",
  };
  return {
    existsFlowByPath: async () => flowExists,
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
    existsApp: async () => false,
    getAppByPath: async () => ({
      path: "f/x/a",
      summary: "",
      value: {},
      raw_app: false,
      policy: {
        execution_mode: "publisher",
        on_behalf_of: "u/alice",
      },
    }),
    createApp: async (p: any) => void captured.push(["createApp", p.requestBody]),
  } as any;
}

test("deployItem: never sends the source workspace's on_behalf_of", async () => {
  const captured: [string, any][] = [];

  // The overwrite is written out once per branch, so exercise all three: a flow that
  // does not exist in the target (create), one that does (update — the branch
  // `wmill workspace merge` takes for anything already deployed), and a script.
  await deployItem(
    recordingProvider(captured, false),
    "flow" as any,
    "f/x/f",
    "src",
    "dst",
    "alice@corp",
  );
  await deployItem(
    recordingProvider(captured, true),
    "flow" as any,
    "f/x/f",
    "src",
    "dst",
    "alice@corp",
  );
  await deployItem(
    recordingProvider(captured, false),
    "script" as any,
    "f/x/s",
    "src",
    "dst",
    "alice@corp",
  );
  // An app is handed a principal, not an address: `getOnBehalfOf` read it out of the
  // target workspace, where `u/` and `g/` names mean what they say.
  await deployItem(
    recordingProvider(captured, false),
    "app" as any,
    "f/x/a",
    "src",
    "dst",
    "g/ops",
  );

  expect(captured.map(([fn]) => fn)).toEqual([
    "createFlow",
    "updateFlow",
    "createScript",
    "createApp",
  ]);
  for (const [name, body] of captured) {
    expect(body.preserve_on_behalf_of).toBe(true);
    if (name === "createApp") {
      expect(body.policy.on_behalf_of).toBe("g/ops");
      expect(body.policy.on_behalf_of_email).toBeUndefined();
    } else {
      expect(body.on_behalf_of_email).toBe("alice@corp");
      // The principal is dropped, so the backend derives the target's own from the address.
      expect("on_behalf_of" in JSON.parse(JSON.stringify(body))).toBe(false);
    }
  }
});
