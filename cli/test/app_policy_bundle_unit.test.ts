/**
 * The raw-app bundle job carries the frontend's policy derivation, vendored by
 * cli/generate-app-policy.ts into backend/windmill-api/src/apps_raw_policy.gen.js
 * and prepended to the job script.
 *
 * If that copy drifts from the frontend source, deployed apps get policy keys
 * the app editor would not have written, and every runnable is refused at run
 * time with "forbidden by policy" — an app that deploys and then does nothing.
 * So rebuild the bundle here and fail when the committed one no longer matches.
 * Fix by running `bun run gen:app-policy` from cli/.
 *
 * No backend required.
 */

import { expect, test, describe } from "bun:test";
import { readFileSync } from "node:fs";
import { buildAppPolicyBundle, OUT_FILE } from "../generate-app-policy.ts";

describe("raw app policy bundle", () => {
  test("the committed bundle matches the frontend source", async () => {
    // Line endings normalized: a CRLF checkout is the same bundle, and must not
    // read as drift (the committed file's header arrives as CRLF on Windows).
    const lf = (s: string) => s.replace(/\r\n/g, "\n");
    expect(lf(readFileSync(OUT_FILE, "utf-8"))).toBe(
      lf(await buildAppPolicyBundle()),
    );
  });

  test("derives the keys the app editor writes", async () => {
    // Exercise the committed artifact itself, not the frontend module: it is
    // what actually runs on the worker.
    // A module's top-level `var` is not a global, and the job prepends this
    // bundle into its own module, so reach the binding the same way it does.
    const { updateRawAppPolicy } = new Function(
      `${readFileSync(OUT_FILE, "utf-8")}\nreturn __wmillAppPolicy`,
    )();

    const content = "export async function main(a: string) { return a }\n";
    const sha = new Bun.CryptoHasher("sha256").update(content).digest("hex");

    const policy = await updateRawAppPolicy(
      {
        inline: {
          type: "inline",
          inlineScript: { content, language: "bun" },
          fields: {
            pinned: { type: "static", value: "by-the-publisher" },
            secret: { type: "static", value: "shh", sensitive: true },
          },
        },
        by_flow: { type: "path", runType: "flow", path: "u/admin/f", fields: {} },
      },
      undefined,
    );

    expect(Object.keys(policy.triggerables_v2).sort()).toEqual([
      "by_flow:flow/u/admin/f",
      `inline:rawscript/${sha}`,
    ]);
    // `sensitive_inputs` is what makes the server encrypt the arg before it
    // reaches the job, so losing it would silently store the value in plaintext.
    const inline = policy.triggerables_v2[`inline:rawscript/${sha}`];
    expect(inline.static_inputs).toEqual({
      pinned: "by-the-publisher",
      secret: "shh",
    });
    expect(inline.sensitive_inputs).toEqual(["secret"]);
  });
});
