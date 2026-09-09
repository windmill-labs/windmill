/**
 * `raw_app.yaml` records none of the policy but the access-mode markers, so a
 * push that regenerated the whole policy reset the deploy drawer's settings —
 * run-as identity, sandbox isolation — to the pushing user's. Pin that the
 * deployed policy is carried over, that a first push still starts from what the
 * file states, and that the markers still close a deployed open app back down.
 */

import { afterAll, beforeEach, expect, mock, test } from "bun:test";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

let calls: any[] = [];
let deployedPolicy: any;
/** No app deployed at the path: `getAppByPath` 404s and the push creates one. */
let deployed = true;

// `mock.module` replaces the module for the whole process, so both are handed
// back at the end — `raw_app_svelte_plugin_unit.test.ts` drives the real
// `createBundle`, and would silently bundle nothing under the stub.
const realServices = await import("../gen/services.gen.ts");
const realBundle = await import("../src/commands/app/bundle.ts");

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

mock.module("../src/commands/app/bundle.ts", () => ({
  ...realBundle,
  createBundle: async () => ({ js: "", css: "" }),
}));

afterAll(() => {
  mock.module("../gen/services.gen.ts", () => realServices);
  mock.module("../src/commands/app/bundle.ts", () => realBundle);
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
  // up to date.
  await writeFile(join(dir, "index.tsx"), "export default 1\n", "utf-8");
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
