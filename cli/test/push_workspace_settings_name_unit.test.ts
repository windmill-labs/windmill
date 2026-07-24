/**
 * Regression guard: a pull (pushWorkspaceSettings) must not apply the workspace
 * display name from settings.yaml. Rationale lives at the apply site in settings.ts.
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
