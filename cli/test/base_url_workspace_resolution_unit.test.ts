import { describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { resolveWorkspace } from "../src/core/context.ts";
import { getWorkspaceConfigFilePath } from "../windmill-utils-internal/src/config/config.ts";
import type { GlobalOptions } from "../src/types.ts";

const BASE_URL = "http://localhost:9999/";

// --base-url pins the target: --workspace reaches the API as a workspace id and
// wmill.yaml is not consulted. The warning that says so may only peek at the
// file — readConfigFile() exits on an unsupported syncBehavior and throws on a
// malformed one, so resolving through it lets an unrelated config fail a
// command that never needed it.
async function withWmillYaml(
  wmillYaml: string,
  fn: (opts: GlobalOptions) => Promise<void>
): Promise<void> {
  const repoDir = await mkdtemp(path.join(os.tmpdir(), "wmill_baseurl_repo_"));
  const configDir = await mkdtemp(path.join(os.tmpdir(), "wmill_baseurl_conf_"));
  const originalCwd = process.cwd();
  try {
    await writeFile(path.join(repoDir, "wmill.yaml"), wmillYaml);
    await writeFile(await getWorkspaceConfigFilePath(configDir), "");

    process.chdir(repoDir);
    await fn({
      configDir,
      baseUrl: BASE_URL,
      token: "sometoken",
      workspace: "staging",
    } as GlobalOptions);
  } finally {
    process.chdir(originalCwd);
    await rm(repoDir, { recursive: true, force: true });
    await rm(configDir, { recursive: true, force: true });
  }
}

describe("--base-url workspace resolution", () => {
  const rejectedConfigs: [string, string][] = [
    ["an unsupported syncBehavior", "syncBehavior: v2\n"],
    ["a malformed file", 'workspaces:\n  staging:\n   baseUrl: "unterminated\n'],
  ];

  for (const [label, wmillYaml] of rejectedConfigs) {
    test(`resolves despite ${label}`, async () => {
      await withWmillYaml(wmillYaml, async (opts) => {
        const workspace = await resolveWorkspace(opts);
        expect(workspace.workspaceId).toBe("staging");
        expect(workspace.remote).toBe(BASE_URL);
      });
    });
  }

  test("a workspaces mapping never overrides the explicit workspace id", async () => {
    await withWmillYaml(
      "workspaces:\n  staging:\n    baseUrl: http://elsewhere.example/\n    workspaceId: admins\n",
      async (opts) => {
        const workspace = await resolveWorkspace(opts);
        expect(workspace.workspaceId).toBe("staging");
        expect(workspace.remote).toBe(BASE_URL);
      }
    );
  });
});
