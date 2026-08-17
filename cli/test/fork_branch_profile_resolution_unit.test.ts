import { describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { execSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { stringify as yamlStringify } from "yaml";

import { tryResolveBranchWorkspace } from "../src/core/context.ts";
import { setLastUsedProfile } from "../src/core/branch-profiles.ts";
import { getWorkspaceConfigFilePath } from "../windmill-utils-internal/src/config/config.ts";
import type { GlobalOptions } from "../src/types.ts";
import type { Workspace } from "../src/commands/workspace/workspace.ts";

const BASE_URL = "http://localhost:9999/";
const PARENT_WORKSPACE_ID = "parent";

// Fork-branch workspace resolution: every profile-selection path in
// tryResolveBranchWorkspace must rewrite the returned profile to the fork
// workspace id derived from the wm-fork/<base>/<id> branch — a path that
// returns the parent profile untouched silently targets the parent workspace.
async function withForkBranchSetup(
  profiles: Workspace[],
  fn: (opts: GlobalOptions) => Promise<void>,
): Promise<void> {
  const repoDir = await mkdtemp(path.join(os.tmpdir(), "wmill_fork_repo_"));
  const configDir = await mkdtemp(path.join(os.tmpdir(), "wmill_fork_conf_"));
  const originalCwd = process.cwd();
  try {
    execSync(`git init -q -b wm-fork/main/myfork`, { cwd: repoDir });
    execSync(`git config user.email test@example.com`, { cwd: repoDir });
    execSync(`git config user.name test`, { cwd: repoDir });

    await writeFile(
      path.join(repoDir, "wmill.yaml"),
      yamlStringify({
        workspaces: {
          main: { baseUrl: BASE_URL, workspaceId: PARENT_WORKSPACE_ID },
        },
      }),
    );
    execSync(`git add wmill.yaml && git commit -q -m init`, { cwd: repoDir });

    const remotesPath = await getWorkspaceConfigFilePath(configDir);
    await writeFile(
      remotesPath,
      profiles.map((p) => JSON.stringify(p)).join("\n") + "\n",
    );

    process.chdir(repoDir);
    await fn({ configDir } as GlobalOptions);
  } finally {
    process.chdir(originalCwd);
    await rm(repoDir, { recursive: true, force: true });
    await rm(configDir, { recursive: true, force: true });
  }
}

function profile(name: string): Workspace {
  return {
    name,
    remote: BASE_URL,
    workspaceId: PARENT_WORKSPACE_ID,
    token: `token-${name}`,
  };
}

describe("tryResolveBranchWorkspace on a fork branch", () => {
  test("single matching profile targets the fork workspace", async () => {
    await withForkBranchSetup([profile("prod")], async (opts) => {
      const ws = await tryResolveBranchWorkspace(opts);
      expect(ws?.workspaceId).toEqual("wm-fork-myfork");
      expect(ws?.token).toEqual("token-prod");
    });
  });

  test("last-used profile among multiple still targets the fork workspace", async () => {
    await withForkBranchSetup(
      [profile("prod"), profile("prod-alt")],
      async (opts) => {
        await setLastUsedProfile(
          "main",
          BASE_URL,
          PARENT_WORKSPACE_ID,
          "prod-alt",
          opts.configDir,
        );
        const ws = await tryResolveBranchWorkspace(opts);
        expect(ws?.workspaceId).toEqual("wm-fork-myfork");
        expect(ws?.token).toEqual("token-prod-alt");
      },
    );
  });
});
