/**
 * Regression guard: `sync push` (pushWorkspaceSettings) applies the domain invite and
 * the instance groups of `auto_invite` through their own endpoints, and never clears
 * instance groups that settings.yaml does not declare.
 */

import { expect, test, describe, beforeEach, mock } from "bun:test";

let editAutoInviteCalls: unknown[] = [];
let editInstanceGroupsCalls: unknown[] = [];
const remoteAutoInvite = {
  enabled: true,
  domain: "*",
  operator: false,
  mode: "invite",
  instance_groups: ["eng"],
  instance_groups_roles: { eng: "developer" },
};

// Every wmill.* call reachable from pushWorkspaceSettings is stubbed: bun shares one
// mocked module across test files, and names missing from whichever mock loads first
// stay missing for the others.
mock.module("../gen/services.gen.ts", () => ({
  getSettings: async () => ({ auto_invite: remoteAutoInvite }),
  getWorkspaceName: async () => "phoenix",
  changeWorkspaceName: async () => {},
  changeWorkspaceColor: async () => {},
  editWebhook: async () => {},
  editAutoInvite: async (a: unknown) => {
    editAutoInviteCalls.push(a);
  },
  editInstanceGroups: async (a: unknown) => {
    editInstanceGroupsCalls.push(a);
  },
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

describe("pushWorkspaceSettings auto_invite", () => {
  beforeEach(() => {
    editAutoInviteCalls = [];
    editInstanceGroupsCalls = [];
  });

  test("an instance-group-only change updates the groups and leaves the domain invite", async () => {
    await pushWorkspaceSettings("phoenix", "settings", undefined, {
      name: "phoenix",
      auto_invite: { ...remoteAutoInvite, instance_groups_roles: { eng: "admin" } },
    });
    expect(editAutoInviteCalls.length).toBe(0);
    expect(editInstanceGroupsCalls).toEqual([
      {
        workspace: "phoenix",
        requestBody: { groups: ["eng"], roles: { eng: "admin" } },
      },
    ]);
  });

  test("a settings.yaml without instance_groups does not clear them", async () => {
    const { instance_groups: _g, instance_groups_roles: _r, ...domainInvite } =
      remoteAutoInvite;
    await pushWorkspaceSettings("phoenix", "settings", undefined, {
      name: "phoenix",
      auto_invite: { ...domainInvite, operator: true },
    });
    expect(editAutoInviteCalls.length).toBe(1);
    expect(editInstanceGroupsCalls.length).toBe(0);
  });
});
