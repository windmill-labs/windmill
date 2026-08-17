/**
 * Unit test for the key format `wmill app generate-policy` writes.
 *
 * The server's raw-app deploy (`/apps/create_raw_source`, the MCP `createApp`
 * tool) runs this command on a worker to derive the deployed app's
 * `triggerables_v2`, and then matches every component run against those keys.
 * A key that disagrees with what the app editor writes leaves the runnable
 * "forbidden by policy" at run time — an app that deploys and then does
 * nothing. Pin the format so a change to it has to be deliberate.
 *
 * No backend required — calls the shared derivation directly.
 */

import { expect, test, describe } from "bun:test";
import { createHash } from "node:crypto";
import * as windmillUtils from "@windmill-labs/shared-utils";

const INLINE = "export async function main(a: string) { return a }\n";
const INLINE_SHA = createHash("sha256").update(INLINE).digest("hex");

describe("app generate-policy", () => {
  test("keys inline runnables by the sha256 of their code, path ones by path", async () => {
    const policy: any = await windmillUtils.updateRawAppPolicy(
      {
        inline: {
          type: "inline",
          inlineScript: { content: INLINE, language: "bun" },
          fields: {
            pinned: { type: "static", value: "by-the-publisher" },
            byviewer: { type: "user", allowUserResources: true },
          },
        },
        by_script: {
          type: "path",
          runType: "script",
          path: "u/admin/dep",
          fields: {},
        },
        by_flow: {
          type: "path",
          runType: "flow",
          path: "u/admin/dep_flow",
          fields: {},
        },
        // hub scripts are dispatched as scripts
        by_hub: {
          type: "path",
          runType: "hubscript",
          path: "hub/1/x",
          fields: {},
        },
      } as any,
      undefined,
    );

    expect(Object.keys(policy.triggerables_v2).sort()).toEqual([
      `by_flow:flow/u/admin/dep_flow`,
      `by_hub:script/hub/1/x`,
      `by_script:script/u/admin/dep`,
      `inline:rawscript/${INLINE_SHA}`,
    ]);

    // Static fields become the publisher-pinned inputs the server forces onto
    // every run; the ones the viewer supplies must not be pinned with them.
    const inline = policy.triggerables_v2[`inline:rawscript/${INLINE_SHA}`];
    expect(inline.static_inputs).toEqual({ pinned: "by-the-publisher" });
    expect(inline.allow_user_resources).toEqual(["byviewer"]);
  });
});
