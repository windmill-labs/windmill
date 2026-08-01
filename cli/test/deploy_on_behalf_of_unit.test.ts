import { expect, test } from "bun:test";
import { deployItem } from "../windmill-utils-internal/src/deploy.ts";

// `deployItem` spreads the source item into the request body, and the principal it carries
// (`on_behalf_of`, at the top level for a script or flow and inside the policy for an app)
// names a username that only exists in the source workspace. Sending it to the target pairs
// one workspace's principal with the other's email, which the backend rejects. Deleting the
// spread is an easy regression, so pin that the principal never reaches the wire while the
// caller's chosen address does.
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
        on_behalf_of_email: "alice@corp",
      },
    }),
    createApp: async (p: any) => void captured.push(["createApp", p.requestBody]),
  } as any;
}

test("deployItem: never sends the source workspace's on_behalf_of", async () => {
  const captured: [string, any][] = [];

  // The clear is written out once per branch, so exercise all three: a flow that
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
  await deployItem(
    recordingProvider(captured, false),
    "app" as any,
    "f/x/a",
    "src",
    "dst",
    "alice@corp",
  );

  expect(captured.map(([fn]) => fn)).toEqual([
    "createFlow",
    "updateFlow",
    "createScript",
    "createApp",
  ]);
  for (const [name, body] of captured) {
    expect(body.preserve_on_behalf_of).toBe(true);
    // Both surfaces spell it `on_behalf_of`; only its nesting differs — an app carries the
    // identity inside its policy, the others at the top level.
    const identity = name === "createApp" ? body.policy : body;
    // The email is still overridden with the caller's choice...
    expect(identity.on_behalf_of_email).toBe("alice@corp");
    // ...while the principal is dropped, so the backend derives the target's own.
    expect("on_behalf_of" in JSON.parse(JSON.stringify(identity))).toBe(false);
  }
});
