/**
 * Regression guard: the standalone `wmill schedule push` must resolve ownership
 * the same way `wmill sync push` does. It only preserves the remote's
 * `permissioned_as` when the command hands `pushSchedule` a context, so a push
 * that builds none silently reassigns the schedule to whoever ran it.
 */

import { expect, test, describe, afterAll, beforeEach, mock } from "bun:test";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

let updateScheduleCalls: any[] = [];
let remotePermissionedAs: string | undefined = "u/svc";

const REMOTE_SCHEDULE = () => ({
  path: "u/admin/sched",
  schedule: "0 0 */6 * * *",
  timezone: "Etc/UTC",
  script_path: "u/admin/script",
  is_flow: false,
  args: {},
  enabled: false,
  summary: "before",
  permissioned_as: remotePermissionedAs,
});

// A module mock is process-global and outlives the file that installs it, and
// `mock.restore()` does not undo one: every mocked module has to be handed back
// its real exports here, or whichever file `bun test` happens to run next gets
// this file's stubs.
const realModules: [string, Record<string, unknown>][] = [];
async function mockModule(
  specifier: string,
  factory: (real: Record<string, unknown>) => Record<string, unknown>
): Promise<void> {
  const real = { ...((await import(specifier)) as Record<string, unknown>) };
  realModules.push([specifier, real]);
  mock.module(specifier, () => factory(real));
}

afterAll(() => {
  for (const [specifier, real] of realModules) {
    mock.module(specifier, () => real);
  }
});

await mockModule("../gen/services.gen.ts", () => ({
  getSchedule: async () => REMOTE_SCHEDULE(),
  updateSchedule: async (a: unknown) => {
    updateScheduleCalls.push(a);
  },
  whoami: async () => ({
    email: "deployer@windmill.dev",
    username: "deployer",
    is_admin: true,
    groups: [],
  }),
}));

await mockModule("../src/core/context.ts", (real) => ({
  ...real,
  resolveWorkspace: async () => ({
    workspaceId: "w",
    name: "w",
    remote: "http://localhost/",
    token: "t",
  }),
}));

await mockModule("../src/core/auth.ts", (real) => ({
  ...real,
  requireLogin: async () => ({}),
}));

const scheduleCommand = (await import("../src/commands/schedule/schedule.ts"))
  .default;

async function pushIn(wmillYamlTail: string): Promise<void> {
  const dir = await mkdtemp(join(tmpdir(), "windmill_sched_push_"));
  await writeFile(
    join(dir, "wmill.yaml"),
    `defaultTs: bun\nincludeSchedules: true\n${wmillYamlTail}`,
    "utf-8"
  );
  await writeFile(
    join(dir, "sched.schedule.yaml"),
    `schedule: "0 0 */6 * * *"\ntimezone: Etc/UTC\nscript_path: u/admin/script\nis_flow: false\nargs: {}\nenabled: false\nsummary: after\n`,
    "utf-8"
  );

  const cwd = process.cwd();
  process.chdir(dir);
  try {
    await scheduleCommand.parse([
      "push",
      "sched.schedule.yaml",
      "u/admin/sched",
    ]);
  } finally {
    process.chdir(cwd);
  }
}

describe("wmill schedule push ownership", () => {
  beforeEach(() => {
    updateScheduleCalls = [];
    remotePermissionedAs = "u/svc";
  });

  test("keeps the remote's permissioned_as under syncBehavior v1", async () => {
    await pushIn("syncBehavior: v1\n");

    expect(updateScheduleCalls).toHaveLength(1);
    const body = updateScheduleCalls[0].requestBody;
    expect(body.summary).toBe("after");
    expect(body.permissioned_as).toBe("u/svc");
    expect(body.preserve_permissioned_as).toBe(true);
  });

  // The entry to read is the one matching the workspace being pushed to, not
  // the top level: a repo that varies settings per workspace puts syncBehavior
  // under `overrides` and nowhere else.
  test("reads syncBehavior from the target workspace's overrides", async () => {
    await pushIn(
      `workspaces:\n  other:\n    baseUrl: http://localhost/\n    workspaceId: w\n    overrides:\n      syncBehavior: v1\n`
    );

    expect(updateScheduleCalls).toHaveLength(1);
    const body = updateScheduleCalls[0].requestBody;
    expect(body.permissioned_as).toBe("u/svc");
    expect(body.preserve_permissioned_as).toBe(true);
  });

  test("leaves ownership to the backend below syncBehavior v1", async () => {
    await pushIn("");

    expect(updateScheduleCalls).toHaveLength(1);
    const body = updateScheduleCalls[0].requestBody;
    expect(body.permissioned_as).toBeUndefined();
    expect(body.preserve_permissioned_as).toBeUndefined();
  });
});
