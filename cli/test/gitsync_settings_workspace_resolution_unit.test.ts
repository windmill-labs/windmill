import { expect, test } from "bun:test";
import type { SyncOptions } from "../src/core/conf.ts";
import { getEffectiveSettingsForWorkspace } from "../src/commands/gitsync-settings/utils.ts";

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
