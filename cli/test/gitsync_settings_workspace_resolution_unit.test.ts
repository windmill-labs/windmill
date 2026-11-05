import { expect, test } from "bun:test";
import type { SyncOptions } from "../src/core/conf.ts";
import { getEffectiveSettingsForWorkspace } from "../src/commands/gitsync-settings/utils.ts";

// Entries sharing a gitBranch must resolve by workspace, not by the first one
// bound to the branch.
test("gitsync-settings: entries sharing a gitBranch resolve by workspace", async () => {
  const config: SyncOptions = {
    includes: ["f/**"],
    workspaces: {
      "flh-nonprod": {
        baseUrl: "https://windmill.example.com/",
        gitBranch: "nonprod",
        overrides: { includes: ["f/ops_flh/**"] },
      },
      provinzial: {
        baseUrl: "https://windmill.example.com/",
        workspaceId: "provinzial-nonprod",
        gitBranch: "nonprod",
        overrides: { includes: ["f/ops_provinzial/**"] },
      },
    },
  };

  const byFlag = await getEffectiveSettingsForWorkspace(
    config,
    { workspace: "flh-nonprod", jsonOutput: true },
    { remote: "http://windmill_server:8000/", workspaceId: "flh-nonprod" },
  );
  expect(byFlag.includes).toEqual(["f/ops_flh/**"]);

  const byProfile = await getEffectiveSettingsForWorkspace(
    config,
    { workspace: "provinzial-nonprod", jsonOutput: true },
    { remote: "https://windmill.example.com/", workspaceId: "provinzial-nonprod" },
  );
  expect(byProfile.includes).toEqual(["f/ops_provinzial/**"]);
});
