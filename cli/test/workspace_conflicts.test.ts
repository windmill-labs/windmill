import { expect, test } from "bun:test";
import { addWorkspace, allWorkspaces } from "../workspace.ts";
import { withTestConfig, clearTestRemotes } from "./test_config_helpers.ts";

// Test workspace conflict detection
test("addWorkspace: prevents duplicate workspace names", async () => {
  await withTestConfig(async (testConfigDir) => {
    await clearTestRemotes(testConfigDir);

    // Add first workspace
    const workspace1 = {
      name: "test_workspace",
      remote: "http://localhost:8001/",
      workspaceId: "workspace1",
      token: "token1"
    };

    await addWorkspace(workspace1, { force: true, configDir: testConfigDir });

    // Try to add workspace with same name but different details
    const workspace2 = {
      name: "test_workspace", // Same name
      remote: "http://localhost:8002/", // Different remote
      workspaceId: "workspace2", // Different ID
      token: "token2"
    };

    // Force non-interactive mode so addWorkspace throws instead of prompting
    const origStdinTTY = process.stdin.isTTY;
    const origStdoutTTY = process.stdout.isTTY;
    try {
      process.stdin.isTTY = false as any;
      process.stdout.isTTY = false as any;

      // Should throw error in non-interactive mode without force
      await expect(
        addWorkspace(workspace2, { configDir: testConfigDir })
      ).rejects.toThrow("Workspace name conflict. Use --force to overwrite or choose a different name.");
    } finally {
      process.stdin.isTTY = origStdinTTY;
      process.stdout.isTTY = origStdoutTTY;
    }

    // Should succeed with force flag
    await addWorkspace(workspace2, { force: true, configDir: testConfigDir });

    // Verify the workspace was overwritten
    const workspaces = await allWorkspaces(testConfigDir);
    expect(workspaces.length).toEqual(1);
    expect(workspaces[0].name).toEqual("test_workspace");
    expect(workspaces[0].remote).toEqual("http://localhost:8002/");
    expect(workspaces[0].workspaceId).toEqual("workspace2");
  });
});

test("addWorkspace: prevents duplicate (remote, workspaceId) tuples", async () => {
  await withTestConfig(async (testConfigDir) => {
    await clearTestRemotes(testConfigDir);

    // Add first workspace
    const workspace1 = {
      name: "first_workspace",
      remote: "http://localhost:8001/",
      workspaceId: "test",
      token: "token1"
    };

    await addWorkspace(workspace1, { force: true, configDir: testConfigDir });

    // Try to add workspace with same (remote, workspaceId) but different name
    const workspace2 = {
      name: "second_workspace", // Different name
      remote: "http://localhost:8001/", // Same remote
      workspaceId: "test", // Same workspaceId
      token: "token2"
    };

    // Should throw error in non-interactive mode without force
    await expect(
      addWorkspace(workspace2, { configDir: testConfigDir })
    ).rejects.toThrow('Backend constraint violation: (http://localhost:8001/, test) already exists as "first_workspace". Use --force to overwrite.');

    // Should succeed with force flag (overwrites first workspace)
    await addWorkspace(workspace2, { force: true, configDir: testConfigDir });

    // Verify the first workspace was removed and second was added
    const workspaces = await allWorkspaces(testConfigDir);
    expect(workspaces.length).toEqual(1);
    expect(workspaces[0].name).toEqual("second_workspace");
    expect(workspaces[0].remote).toEqual("http://localhost:8001/");
    expect(workspaces[0].workspaceId).toEqual("test");
  });
});

test("addWorkspace: allows same workspace (name, remote, workspaceId) with token update", async () => {
  await withTestConfig(async (testConfigDir) => {
    await clearTestRemotes(testConfigDir);

    // Add first workspace
    const workspace1 = {
      name: "same_workspace",
      remote: "http://localhost:8001/",
      workspaceId: "test",
      token: "old_token"
    };

    await addWorkspace(workspace1, { force: true, configDir: testConfigDir });

    // Add same workspace with updated token
    const workspace2 = {
      name: "same_workspace", // Same name
      remote: "http://localhost:8001/", // Same remote
      workspaceId: "test", // Same workspaceId
      token: "new_token" // Different token
    };

    // Should succeed without force (just token update)
    await addWorkspace(workspace2, { configDir: testConfigDir });

    // Verify token was updated
    const workspaces = await allWorkspaces(testConfigDir);
    expect(workspaces.length).toEqual(1);
    expect(workspaces[0].name).toEqual("same_workspace");
    expect(workspaces[0].token).toEqual("new_token");
  });
});

test("addWorkspace: returns true on successful add", async () => {
  await withTestConfig(async (testConfigDir) => {
    await clearTestRemotes(testConfigDir);

    const workspace = {
      name: "return_test",
      remote: "http://localhost:8001/",
      workspaceId: "test",
      token: "token1"
    };

    const result = await addWorkspace(workspace, { force: true, configDir: testConfigDir });
    expect(result).toEqual(true);
  });
});

test("addWorkspace: returns true when force-overwriting conflict", async () => {
  await withTestConfig(async (testConfigDir) => {
    await clearTestRemotes(testConfigDir);

    const workspace1 = {
      name: "force_test",
      remote: "http://localhost:8001/",
      workspaceId: "workspace1",
      token: "token1"
    };
    await addWorkspace(workspace1, { force: true, configDir: testConfigDir });

    const workspace2 = {
      name: "force_test",
      remote: "http://localhost:8002/",
      workspaceId: "workspace2",
      token: "token2"
    };
    const result = await addWorkspace(workspace2, { force: true, configDir: testConfigDir });
    expect(result).toEqual(true);
  });
});

test("addWorkspace: allows different workspaces on different remotes", async () => {
  await withTestConfig(async (testConfigDir) => {
    await clearTestRemotes(testConfigDir);

    // Add workspace on first remote
    const workspace1 = {
      name: "workspace_remote1",
      remote: "http://localhost:8001/",
      workspaceId: "test",
      token: "token1"
    };

    await addWorkspace(workspace1, { force: true, configDir: testConfigDir });

    // Add workspace with same workspaceId on different remote (should be allowed)
    const workspace2 = {
      name: "workspace_remote2",
      remote: "http://localhost:8002/", // Different remote
      workspaceId: "test", // Same workspaceId (OK on different remote)
      token: "token2"
    };

    // Should succeed (different remotes)
    await addWorkspace(workspace2, { configDir: testConfigDir });

    // Verify both workspaces exist
    const workspaces = await allWorkspaces(testConfigDir);
    expect(workspaces.length).toEqual(2);

    const names = workspaces.map(w => w.name).sort();
    expect(names).toEqual(["workspace_remote1", "workspace_remote2"]);
  });
});
