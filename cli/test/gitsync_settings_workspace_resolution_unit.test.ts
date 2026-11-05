import { expect, test } from "bun:test";
import { execSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { SyncOptions } from "../src/core/conf.ts";
import {
  getEffectiveSettingsForWorkspace,
  resolveOverrideTarget,
} from "../src/commands/gitsync-settings/utils.ts";

// Entries sharing a gitBranch must resolve by workspace, not by the first one
// bound to the branch.
test("gitsync-settings: entries sharing a gitBranch resolve by workspace", async () => {
  const config: SyncOptions = {
    includes: ["f/**"],
    workspaces: {
      "team-a-staging": {
        baseUrl: "https://windmill.example.com/",
        gitBranch: "staging",
        overrides: { includes: ["f/team_a/**"] },
      },
      "team-b": {
        baseUrl: "https://windmill.example.com/",
        workspaceId: "team-b-staging",
        gitBranch: "staging",
        overrides: { includes: ["f/team_b/**"] },
      },
    },
  };

  const byFlag = await getEffectiveSettingsForWorkspace(
    config,
    { workspace: "team-a-staging", jsonOutput: true },
    { remote: "http://windmill_server:8000/", workspaceId: "team-a-staging" },
  );
  expect(byFlag.includes).toEqual(["f/team_a/**"]);

  const byProfile = await getEffectiveSettingsForWorkspace(
    config,
    { workspace: "team-b-staging", jsonOutput: true },
    { remote: "https://windmill.example.com/", workspaceId: "team-b-staging" },
  );
  expect(byProfile.includes).toEqual(["f/team_b/**"]);
});

// `pull --override` writes into the entry its diff reads, and never into an
// entry the branch alone attributes to this workspace.
test("gitsync-settings: override target", () => {
  const remote = "http://windmill_server:8000/";
  const target = (workspaces: SyncOptions["workspaces"], workspaceId: string, branch: string) =>
    resolveOverrideTarget({ workspaces }, { workspace: workspaceId }, { remote, workspaceId }, branch);

  const shared = {
    "team-a": { gitBranch: "staging" },
    "team-b": { gitBranch: "staging" },
  };
  expect(target(shared, "team-b", "staging")).toEqual("team-b");
  expect(() => target(shared, "team-c", "staging")).toThrow("already used by team-a, team-b");
  expect(() => target({ staging: {} }, "team-c", "staging")).toThrow("already used by staging");
  expect(() => target({ team: { workspaceId: "other", gitBranch: "staging" } }, "team-c", "staging")).toThrow();
  expect(target({ prod: { gitBranch: "main" } }, "team-c", "staging")).toEqual("staging");
});

// A fork has no entry of its own: an entry beside its parent's would never be
// read back, so its override goes into the parent's entry on the fork branch.
test("gitsync-settings: a fork's override targets its parent's entry", () => {
  const dir = mkdtempSync(join(tmpdir(), "wmill-fork-"));
  const cwd = process.cwd();
  try {
    execSync(
      "git init -q -b staging && git -c user.email=t@t -c user.name=t commit -q --allow-empty -m i && git switch -q -c wm-fork/staging/abc",
      { cwd: dir, stdio: "pipe" },
    );
    process.chdir(dir);
    const target = resolveOverrideTarget(
      { workspaces: { staging: {}, other: { gitBranch: "main" } } },
      { workspace: "wm-fork-abc" },
      { remote: "http://windmill_server:8000/", workspaceId: "wm-fork-abc" },
      "wm-fork/staging/abc",
    );
    expect(target).toEqual("staging");
  } finally {
    process.chdir(cwd);
    rmSync(dir, { recursive: true, force: true });
  }
});
