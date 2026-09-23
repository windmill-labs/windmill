/**
 * Regression guard: `sync push` (pushWorkspaceSettings) must never apply the
 * workspace display name from settings.yaml, and must apply the color only when
 * the file carries one. Rationale lives at the apply sites in settings.ts.
 */

import { expect, test, describe, beforeEach, mock } from "bun:test";

let changeWorkspaceNameCalls: unknown[] = [];
let changeWorkspaceColorCalls: unknown[] = [];
let editWebhookCalls: unknown[] = [];
let remoteName = "";
let remoteColor: string | undefined = undefined;
let remoteWebhook: string | undefined = undefined;

// Every wmill.* call reachable from pushWorkspaceSettings is stubbed so the
// function runs without a backend; only the three we assert on record calls.
mock.module("../gen/services.gen.ts", () => ({
  getSettings: async (_a: { workspace: string }) => ({
    webhook: remoteWebhook,
    color: remoteColor,
  }),
  getWorkspaceName: async (_a: { workspace: string }) => remoteName,
  changeWorkspaceName: async (a: unknown) => {
    changeWorkspaceNameCalls.push(a);
  },
  changeWorkspaceColor: async (a: unknown) => {
    changeWorkspaceColorCalls.push(a);
  },
  editWebhook: async (a: unknown) => {
    editWebhookCalls.push(a);
  },
  editAutoInvite: async () => {},
  editInstanceGroups: async () => {},
  editErrorHandler: async () => {},
  editSuccessHandler: async () => {},
  editCopilotConfig: async () => {},
  editLargeFileStorageConfig: async () => {},
  editWorkspaceGitSyncConfig: async () => {},
  editWorkspaceDefaultApp: async () => {},
  editDefaultScripts: async () => {},
  workspaceMuteCriticalAlertsUi: async () => {},
  updateOperatorSettings: async () => {},
  editDataTableConfig: async () => {},
  editSlackCommand: async () => {},
  setWorkspaceSlackOauthConfig: async () => {},
  deleteWorkspaceSlackOauthConfig: async () => {},
}));

const { pushWorkspaceSettings } = await import("../src/core/settings.ts");

describe("pushWorkspaceSettings workspace identity", () => {
  const ws = "phoenix";

  beforeEach(() => {
    changeWorkspaceNameCalls = [];
    changeWorkspaceColorCalls = [];
    editWebhookCalls = [];
    remoteName = "phoenix";
    remoteColor = undefined;
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

  test("a settings.yaml without a color key does not clear the workspace color", async () => {
    remoteColor = "#ff0000";
    remoteWebhook = "https://old";
    await pushWorkspaceSettings(ws, "settings", undefined, {
      name: "phoenix",
      webhook: "https://new",
    });
    expect(editWebhookCalls.length).toBe(1);
    expect(changeWorkspaceColorCalls.length).toBe(0);
  });

  test("a color in settings.yaml is applied when it differs from the workspace", async () => {
    remoteColor = "#ff0000";
    await pushWorkspaceSettings(ws, "settings", undefined, {
      name: "phoenix",
      color: "#00ff00",
    });
    expect(editWebhookCalls.length).toBe(0);
    expect(changeWorkspaceColorCalls).toEqual([
      { workspace: ws, requestBody: { color: "#00ff00" } },
    ]);
  });

  test("a color matching the workspace is a complete no-op", async () => {
    remoteColor = "#ff0000";
    await pushWorkspaceSettings(ws, "settings", undefined, {
      name: "phoenix",
      color: "#ff0000",
    });
    expect(editWebhookCalls.length).toBe(0);
    expect(changeWorkspaceColorCalls.length).toBe(0);
  });
});
