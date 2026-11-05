import { expect, test } from "bun:test";
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

// `pull --override` must never write into an entry it cannot prove is this
// workspace's: the branch, or a fork's parent, does not identify one.
test("gitsync-settings: override writes only into the workspace's own entry", () => {
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
  // A fork reads its parent's entry, but writes beside it
  expect(target({ staging: {} }, "wm-fork-abc", "wm-fork/staging/abc")).toEqual("wm-fork/staging/abc");
});
