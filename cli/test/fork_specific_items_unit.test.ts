import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { execSync } from "node:child_process";
import os from "node:os";
import path from "node:path";

import { inferWsNameFromForkBranch, type SyncOptions } from "../src/core/conf.ts";

// A fork id never has a wmill.yaml entry of its own. Unless it resolves to its
// parent's entry, a sync on the fork branch pushes the base files over the
// workspace-specific values the fork cloned from its parent.
test("a fork branch resolves to its parent's workspace entry", async () => {
  const repoDir = await mkdtemp(path.join(os.tmpdir(), "wmill_fork_specific_"));
  const originalCwd = process.cwd();
  try {
    execSync(
      `git init -q -b wm-fork/dev-branch/t1 && git -c user.email=t@t -c user.name=t commit -q --allow-empty -m init`,
      { cwd: repoDir },
    );
    process.chdir(repoDir);

    const config: SyncOptions = {
      workspaces: {
        dev: {
          baseUrl: "http://localhost:8000/",
          workspaceId: "dev-ws",
          gitBranch: "dev-branch",
          specificItems: { folders: ["f/team"] },
        },
      },
    };

    expect(inferWsNameFromForkBranch(config, "wm-fork-t1")).toEqual("dev");
    // Only the fork the branch names inherits the parent's entry.
    expect(inferWsNameFromForkBranch(config, "other-ws")).toBeUndefined();
  } finally {
    process.chdir(originalCwd);
    await rm(repoDir, { recursive: true, force: true });
  }
});
