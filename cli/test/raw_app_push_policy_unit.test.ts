/**
 * `raw_app.yaml` records none of the policy but the access-mode markers, so a
 * push that regenerated the whole policy reset the deploy drawer's settings —
 * run-as identity, sandbox isolation — to the pushing user's. Pin that the
 * deployed policy is carried over, and that the markers still close a deployed
 * open app back down.
 */

import { beforeEach, expect, mock, test } from "bun:test";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

let updateCalls: any[] = [];
let deployedPolicy: any;

mock.module("../gen/services.gen.ts", () => ({
  getAppByPath: async () => ({
    path: "f/test/raw",
    summary: "raw",
    value: { files: {}, runnables: {} },
    policy: deployedPolicy,
  }),
  updateAppRaw: async (a: unknown) => {
    updateCalls.push(a);
  },
}));

const realBundle = await import("../src/commands/app/bundle.ts");
mock.module("../src/commands/app/bundle.ts", () => ({
  ...realBundle,
  createBundle: async () => ({ js: "", css: "" }),
}));

const { pushRawApp } = await import("../src/commands/app/raw_apps.ts");

const ADMIN = {
  userCache: new Map(),
  userIsAdminOrDeployer: true,
  userEmail: "deployer@windmill.dev",
};

async function push(yamlTail: string, admin = true): Promise<any> {
  updateCalls = [];
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
  expect(updateCalls).toHaveLength(1);
  return updateCalls[0].formData.app;
}

beforeEach(() => {
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
  // Only the caller who may claim it gets the flag; everyone else deploys as
  // themselves, which is what the backend enforces anyway.
  expect((await push("", false)).preserve_on_behalf_of).toBeUndefined();
});
