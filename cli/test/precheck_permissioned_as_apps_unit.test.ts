/**
 * The pre-check is what stops a push from silently reassigning an item's run-as
 * user. Raw apps were missing from it, so the one kind whose file records no
 * policy at all was also the one that changed owner without a word.
 */

import { expect, test } from "bun:test";
import { preCheckPermissionedAs } from "../src/core/permissioned_as.ts";

/** Non-interactive and without the override flag, the pre-check exits rather
 * than reassigning silently — so a thrown exit is the signal it fired. */
type Shape = "edited" | "added" | "deleted";

function change(path: string, name: Shape = "edited") {
  return { name, path, before: "summary: x\n", content: "summary: x\n" };
}

async function precheck(
  changes: ReturnType<typeof change>[],
): Promise<string | undefined> {
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
    await preCheckPermissionedAs(changes, "pusher@corp", false, false, false);
  } catch (e) {
    if (!String(e).startsWith("Error: exit:")) throw e;
  } finally {
    (process as any).exit = exit;
    console.error = err;
  }
  return code === undefined ? undefined : logged.join("\n");
}

test("a raw-app push warns the non-deployer it will take over the run-as user", async () => {
  const message = await precheck([change("f/test/myapp.raw_app/index.tsx")]);

  expect(message).toBeDefined();
  expect(message).toContain("f/test/myapp.raw_app");
  expect(message).toContain("pusher@corp");
});

// Deleting one file re-pushes the whole app rather than deleting it, so the
// takeover happens there too.
test("deleting one of an app's files warns like editing one", async () => {
  const message = await precheck([
    change("f/test/myapp.raw_app/gone.tsx", "deleted"),
  ]);

  expect(message).toContain("f/test/myapp.raw_app");
});

// The metadata file going with it means the app itself is created or removed —
// neither takes an owner over.
test("an app arriving or leaving whole is not a takeover", async () => {
  const created = await precheck([
    change("f/test/new.raw_app/raw_app.yaml", "added"),
    change("f/test/new.raw_app/index.tsx", "added"),
  ]);
  const removed = await precheck([
    change("f/test/old.raw_app/raw_app.yaml", "deleted"),
    change("f/test/old.raw_app/index.tsx", "deleted"),
  ]);

  expect(created).toBeUndefined();
  expect(removed).toBeUndefined();
});

// An app carries no owner in its files, so nothing about it depends on their
// content — an empty one redeploys it exactly like any other.
test("an empty file still counts as a change to the app", async () => {
  const added = await precheck([
    { name: "added", path: "f/test/myapp.raw_app/blank.ts", content: "" },
  ]);
  const edited = await precheck([
    { name: "edited", path: "f/test/myapp.raw_app/blank.ts", before: "" },
  ]);

  expect(added).toContain("f/test/myapp.raw_app");
  expect(edited).toContain("f/test/myapp.raw_app");
});

// `extractFolderPath` normalizes separators but the metadata predicates match a
// literal `/`, so a Windows path must not take a different branch.
test("a Windows path classifies the same as its posix twin", async () => {
  const created = await precheck([
    change("f\\test\\new.raw_app\\raw_app.yaml", "added"),
    change("f\\test\\new.raw_app\\index.tsx", "added"),
  ]);
  const edited = await precheck([
    change("f\\test\\myapp.raw_app\\index.tsx"),
  ]);

  expect(created).toBeUndefined();
  expect(edited).toContain("f/test/myapp.raw_app");
});

// `collectAppFiles` never sends these, and the sync diff never stops listing
// them (nothing uploads them, so they stay "added" forever) — so warning on one
// would gate every push of a scaffolded app on the override flag.
test("a file the push never sends is not a change to the app", async () => {
  const artifacts = await precheck([
    change("f/test/myapp.raw_app/AGENTS.md", "added"),
    change("f/test/myapp.raw_app/sql_to_apply/a.sql", "added"),
    change("f/test/myapp.raw_app/node_modules/dep/index.js", "added"),
    change("f/test/myapp.raw_app/recordings/r.json", "added"),
    change("f/test/myapp.raw_app/package-lock.json"),
    change("f/test/myapp.raw_app/wmill.d.ts"),
    // Only the backend folder's *top level* is a runnable; nothing reads deeper,
    // so the depth limit is what keeps a `backend/node_modules/` from becoming
    // the perpetual diff this predicate exists to remove.
    change("f/test/myapp.raw_app/backend/node_modules/dep/index.js", "added"),
  ]);
  // The three channels a push does send through: bundled file, metadata, runnable.
  const sent = await precheck([change("f/test/myapp.raw_app/index.tsx")]);
  const meta = await precheck([change("f/test/myapp.raw_app/raw_app.yaml")]);
  const runnable = await precheck([change("f/test/myapp.raw_app/backend/a.ts")]);
  // The runnable channel is not the bundle: the bundle's name exclusions don't
  // reach into it, so a runnable file sharing one of those names still deploys.
  const namesake = await precheck([
    change("f/test/myapp.raw_app/backend/wmill.d.ts"),
  ]);

  expect(artifacts).toBeUndefined();
  expect(sent).toContain("f/test/myapp.raw_app");
  expect(meta).toContain("f/test/myapp.raw_app");
  expect(runnable).toContain("f/test/myapp.raw_app");
  expect(namesake).toContain("f/test/myapp.raw_app");
});

test("an app is listed once however many of its files changed", async () => {
  const message = await precheck([
    change("f/test/myapp.raw_app/index.tsx"),
    change("f/test/myapp.raw_app/raw_app.yaml"),
    change("f/test/myapp.raw_app/backend/a.ts"),
    change("f/test/low.app/app.yaml"),
    change("f/test/low.app/inline.ts"),
  ]);

  expect(message).toContain("2 item(s)");
});
