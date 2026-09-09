/**
 * The pre-check is what stops a push from silently reassigning an item's run-as
 * user. Raw apps were missing from it, so the one kind whose file records no
 * policy at all was also the one that changed owner without a word.
 */

import { expect, test } from "bun:test";
import { preCheckPermissionedAs } from "../src/core/permissioned_as.ts";

/** Non-interactive and without the override flag, the pre-check exits rather
 * than reassigning silently — so a thrown exit is the signal it fired. */
async function precheck(paths: string[]): Promise<string | undefined> {
  const exit = process.exit;
  let code: number | undefined;
  (process as any).exit = (c?: number) => {
    code = c;
    throw new Error(`exit:${c}`);
  };
  const logged: string[] = [];
  const err = console.error;
  console.error = (...a: unknown[]) => void logged.push(a.join(" "));
  try {
    await preCheckPermissionedAs(
      paths.map((path) => ({ name: "edited" as const, path, before: "summary: x\n" })),
      "pusher@corp",
      false,
      false,
      false,
    );
  } catch (e) {
    if (!String(e).startsWith("Error: exit:")) throw e;
  } finally {
    (process as any).exit = exit;
    console.error = err;
  }
  return code === undefined ? undefined : logged.join("\n");
}

test("a raw-app push warns the non-deployer it will take over the run-as user", async () => {
  const message = await precheck(["f/test/myapp.raw_app/index.tsx"]);

  expect(message).toBeDefined();
  expect(message).toContain("f/test/myapp.raw_app");
  expect(message).toContain("pusher@corp");
});

test("an app is listed once however many of its files changed", async () => {
  const message = await precheck([
    "f/test/myapp.raw_app/index.tsx",
    "f/test/myapp.raw_app/raw_app.yaml",
    "f/test/myapp.raw_app/backend/a.ts",
    "f/test/low.app/app.yaml",
    "f/test/low.app/inline.ts",
  ]);

  expect(message).toContain("2 item(s)");
});
