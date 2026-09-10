/**
 * `raw_app.yaml` records none of the policy but the access-mode markers, so a
 * push that regenerated the whole policy reset the deploy drawer's settings —
 * run-as identity, sandbox isolation — to the pushing user's. Pin that the
 * deployed policy is carried over, that a first push still starts from what the
 * file states, and that the markers still close a deployed open app back down.
 */

import { afterAll, beforeEach, expect, mock, test } from "bun:test";
import { mkdtemp, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

let calls: any[] = [];
let deployedPolicy: any;
/** No app deployed at the path: `getAppByPath` 404s and the push creates one. */
let deployed = true;

// Stub only what no other in-process suite imports, and treat a stub as
// permanent for the run (see "Module mocks" in cli/TESTING.md). These three API
// functions qualify — nothing else in `test/` imports them. `bundle.ts` did not:
// stubbing it left `raw_app_svelte_plugin_unit.test.ts` asserting against an
// empty bundle, which an `afterAll` hand-back did not prevent. So the real
// bundler runs instead, on the app each push writes below.
const realServices = await import("../gen/services.gen.ts");

mock.module("../gen/services.gen.ts", () => ({
  ...realServices,
  getAppByPath: async () => {
    if (!deployed) throw new Error("not found");
    return {
      path: "f/test/raw",
      summary: "raw",
      value: { files: {}, runnables: {} },
      policy: deployedPolicy,
    };
  },
  updateAppRaw: async (a: unknown) => {
    calls.push(a);
  },
  createAppRaw: async (a: unknown) => {
    calls.push(a);
  },
}));

// Belt and braces: nothing else in-process calls these, and a hand-back is not
// what makes that safe.
afterAll(() => {
  mock.module("../gen/services.gen.ts", () => realServices);
});

const { pushRawApp } = await import("../src/commands/app/raw_apps.ts");

const ADMIN = {
  userCache: new Map(),
  userIsAdminOrDeployer: true,
  userEmail: "deployer@windmill.dev",
};

async function push(yamlTail: string, admin = true): Promise<any> {
  calls = [];
  const dir = await mkdtemp(join(tmpdir(), "windmill_raw_push_"));
  await writeFile(
    join(dir, "raw_app.yaml"),
    `summary: raw\nrunnables: {}\n${yamlTail}`,
    "utf-8",
  );
  // Any file the remote doesn't have, so the push isn't short-circuited as
  // up to date. It is also the bundler's entry point, so it has to compile.
  await writeFile(join(dir, "index.tsx"), "export default 1\n", "utf-8");
  await writeFile(
    join(dir, "package.json"),
    JSON.stringify({ name: "app", private: true }),
    "utf-8",
  );
  // `ensureNodeModules` only checks the directory is there; borrowing the CLI's
  // own skips an npm install per push.
  await symlink(join(process.cwd(), "node_modules"), join(dir, "node_modules"));
  await pushRawApp("w", "f/test/raw", dir, undefined, "bun", admin ? ADMIN : undefined);
  expect(calls).toHaveLength(1);
  return calls[0].formData.app;
}

beforeEach(() => {
  deployed = true;
  deployedPolicy = {
    on_behalf_of: "u/svc",
    on_behalf_of_email: "svc@corp",
    sandbox: true,
    frontend_sdk_scopes: ["jobs:run"],
    execution_mode: "anonymous",
    // Legacy v1 grants: the backend folds them into v2 at run time, so keeping
    // them would keep granting runnables a push has removed.
    triggerables: { "script/f/test/gone": {} },
    triggerables_v2: { "a:script/f/test/gone": {} },
  };
});

test("a raw-app push keeps the deployed run-as and sandbox settings", async () => {
  const body = await push("public: true\n");

  expect(body.policy.on_behalf_of).toBe("u/svc");
  expect(body.policy.on_behalf_of_email).toBe("svc@corp");
  expect(body.preserve_on_behalf_of).toBe(true);
  expect(body.policy.sandbox).toBe(true);
  expect(body.policy.frontend_sdk_scopes).toEqual(["jobs:run"]);
  expect(body.policy.execution_mode).toBe("anonymous");
  expect(body.policy.triggerables).toBeUndefined();
  expect(body.policy.triggerables_v2).toEqual({});
});

test("a raw-app push without the marker closes an anonymous app back down", async () => {
  const body = await push("");

  expect(body.policy.execution_mode).toBe("publisher");
});

test("a push that may not claim the deployed identity doesn't send it", async () => {
  const body = await push("", false);

  expect(body.preserve_on_behalf_of).toBeUndefined();
  // Not just the flag: the identity itself stays off the wire, so no server can
  // deploy this push under it.
  expect(body.policy.on_behalf_of).toBeUndefined();
  expect(body.policy.on_behalf_of_email).toBeUndefined();
  // Everything the pusher is entitled to carry over still comes along.
  expect(body.policy.sandbox).toBe(true);
});

test("a first raw-app push deploys the policy its file states", async () => {
  deployed = false;
  const body = await push(
    "policy:\n  sandbox: true\n  on_behalf_of: u/impostor\n  on_behalf_of_email: impostor@corp\n",
  );

  expect(body.policy.sandbox).toBe(true);
  // A repo doesn't get to pick who an app runs as: the identity never reaches
  // the wire, so no server can be talked into deploying under it.
  expect(body.policy.on_behalf_of).toBeUndefined();
  expect(body.policy.on_behalf_of_email).toBeUndefined();
  expect(body.preserve_on_behalf_of).toBeUndefined();
});
