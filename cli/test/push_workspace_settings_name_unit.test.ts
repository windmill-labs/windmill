/**
 * Unit tests for workspace-name handling in pushWorkspaceSettings (settings.ts).
 *
 * A pull applies the tracked settings.yaml onto the workspace. The workspace
 * display name lives in settings.yaml but must NOT be applied on pull: it is a
 * per-instance property, and settings.yaml is shared across the branches of a
 * repo, so applying it lets one workspace's name overwrite another's when two
 * workspaces sync the same repo. These tests pin that name is never pushed.
 */

import { expect, test, describe, beforeEach, mock } from "bun:test";

let changeWorkspaceNameCalls: unknown[] = [];
let editWebhookCalls: unknown[] = [];
let remoteName = "";
let remoteWebhook: string | undefined = undefined;

// Every wmill.* call reachable from pushWorkspaceSettings is stubbed so the
// function runs without a backend; only the two we assert on record calls.
mock.module("../gen/services.gen.ts", () => ({
  getSettings: async (_a: { workspace: string }) => ({ webhook: remoteWebhook }),
  getWorkspaceName: async (_a: { workspace: string }) => remoteName,
  changeWorkspaceName: async (a: unknown) => {
    changeWorkspaceNameCalls.push(a);
  },
  editWebhook: async (a: unknown) => {
    editWebhookCalls.push(a);
  },
  editAutoInvite: async () => {},
  editErrorHandler: async () => {},
  editSuccessHandler: async () => {},
  editDeployTo: async () => {},
  editCopilotConfig: async () => {},
  editLargeFileStorageConfig: async () => {},
  editWorkspaceGitSyncConfig: async () => {},
  editWorkspaceDefaultApp: async () => {},
  editDefaultScripts: async () => {},
  workspaceMuteCriticalAlertsUi: async () => {},
  changeWorkspaceColor: async () => {},
  updateOperatorSettings: async () => {},
  editDataTableConfig: async () => {},
  editSlackCommand: async () => {},
  setWorkspaceSlackOauthConfig: async () => {},
  deleteWorkspaceSlackOauthConfig: async () => {},
}));

const { pushWorkspaceSettings } = await import("../src/core/settings.ts");

describe("pushWorkspaceSettings workspace name", () => {
  const ws = "phoenix";

  beforeEach(() => {
    changeWorkspaceNameCalls = [];
    editWebhookCalls = [];
    remoteName = "phoenix";
    remoteWebhook = undefined;
  });

  test("a differing name in settings.yaml is not applied to the workspace", async () => {
    // Another setting also differs so the function proceeds past its no-op early
    // return; only that setting must be applied, never the name.
    remoteWebhook = "https://old";
    await pushWorkspaceSettings(ws, "settings", undefined, {
      name: "phoenix-staging",
      webhook: "https://new",
    });
    expect(editWebhookCalls.length).toBe(1);
    expect(changeWorkspaceNameCalls.length).toBe(0);
  });

  test("a name-only difference is a complete no-op", async () => {
    await pushWorkspaceSettings(ws, "settings", undefined, {
      name: "phoenix-staging",
    });
    expect(editWebhookCalls.length).toBe(0);
    expect(changeWorkspaceNameCalls.length).toBe(0);
  });
});
