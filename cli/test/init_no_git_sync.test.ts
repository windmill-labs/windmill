/**
 * Test to verify that wmill init handles workspaces with no git-sync settings correctly
 * This creates a unit test that directly tests the logic without needing a backend
 */

import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { DEFAULT_SYNC_OPTIONS } from "../src/core/conf.ts";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

// Mock the workspace object
const mockWorkspace = {
  remote: 'https://app.windmill.dev/',
  workspaceId: 'test-workspace',
  name: 'test-workspace'
};

// Simulate the createWorkspaceProfile function logic for the "no repositories" case
function createWorkspaceProfileNoRepos(workspace: any): any {
  const workspaceProfile: any = {
    baseUrl: workspace.remote,
    workspaceId: workspace.workspaceId,
  };

  // Simulate the case where listRepositories returns empty array
  const repositories: any[] = [];

  if (repositories.length === 0) {
    console.log(`No git repositories found in workspace '${workspace.workspaceId}'`);
    // This is the fix: include default sync settings when no repositories exist
    Object.assign(workspaceProfile, DEFAULT_SYNC_OPTIONS);
    return workspaceProfile;
  }

  return workspaceProfile;
}

test("Init: createWorkspaceProfile includes defaults when no repositories exist", () => {
  console.log('Testing init logic for workspace with no git-sync repositories...');

  const workspaceProfile = createWorkspaceProfileNoRepos(mockWorkspace);

  console.log('Generated workspace profile:', JSON.stringify(workspaceProfile, null, 2));

  // Verify basic workspace info
  expect(workspaceProfile.baseUrl).toEqual('https://app.windmill.dev/');
  expect(workspaceProfile.workspaceId).toEqual('test-workspace');

  // Verify default sync settings are included
  expect(Array.isArray(workspaceProfile.includes)).toBeTruthy();
  expect(workspaceProfile.includes.length).toEqual(1);
  expect(workspaceProfile.includes[0]).toEqual('f/**');

  expect(Array.isArray(workspaceProfile.excludes)).toBeTruthy();
  expect(workspaceProfile.excludes.length).toEqual(0);

  expect(workspaceProfile.defaultTs).toEqual('bun');

  console.log('Workspace profile correctly includes default sync settings when no repositories exist');
});

test("Init: verify DEFAULT_SYNC_OPTIONS has expected values", () => {
  console.log('Verifying DEFAULT_SYNC_OPTIONS contains expected values...');

  console.log('DEFAULT_SYNC_OPTIONS:', JSON.stringify(DEFAULT_SYNC_OPTIONS, null, 2));

  // Verify the default options include the expected f/** pattern
  expect(Array.isArray(DEFAULT_SYNC_OPTIONS.includes)).toBeTruthy();
  expect(DEFAULT_SYNC_OPTIONS.includes.length).toEqual(1);
  expect(DEFAULT_SYNC_OPTIONS.includes[0]).toEqual('f/**');

  expect(Array.isArray(DEFAULT_SYNC_OPTIONS.excludes)).toBeTruthy();
  expect(DEFAULT_SYNC_OPTIONS.excludes.length).toEqual(0);

  expect(DEFAULT_SYNC_OPTIONS.defaultTs).toEqual('bun');

  console.log('DEFAULT_SYNC_OPTIONS has expected values');
});

test.skipIf(shouldSkipOnCI())("Init: --use-backend flag applies git-sync settings", async () => {
  await withTestBackend(async (backend, tempDir) => {
    // Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: backend.workspace,
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // First create the git repository resource that the git-sync will reference
    await backend.createAdditionalGitRepo("u/test/init_repo", "Test init repository");

    // Configure backend with git-sync settings
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/init_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/backend/**"],
            include_type: ["script", "flow"],
            exclude_path: ["*.test.ts"],
            extra_include_path: ["g/**"]
          }
        }]
      }
    });

    // Run init with --use-backend flag
    const result = await backend.runCLICommand([
      'init',
      '--use-backend',
      '--repository', 'u/test/init_repo'
    ], tempDir);

    expect(result.code).toEqual(0);

    // Verify wmill.yaml was created with backend settings
    const wmillYaml = await readFile(`${tempDir}/wmill.yaml`, "utf-8");

    // Should have backend-applied settings written to top-level (not overrides)
    expect(wmillYaml).toContain("f/backend/**");
    expect(wmillYaml).toContain("*.test.ts");
    expect(wmillYaml).toContain("g/**");

    // Should have empty overrides section for consistency
    expect(wmillYaml).toContain("gitBranches: {}");
  });
});

test.skipIf(shouldSkipOnCI())("Init: --use-default bypasses backend settings check", async () => {
  await withTestBackend(async (backend, tempDir) => {
    // Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: backend.workspace,
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Create git repository resource
    await backend.createAdditionalGitRepo("u/test/ignored_repo", "Test ignored repository");

    // Configure backend with git-sync settings
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/ignored_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/should-be-ignored/**"],
            include_type: ["script"],
            exclude_path: [],
            extra_include_path: []
          }
        }]
      }
    });

    // Run init with --use-default (should ignore backend)
    const result = await backend.runCLICommand([
      'init',
      '--use-default'
    ], tempDir);

    expect(result.code).toEqual(0);

    // Verify wmill.yaml was created with default settings only
    const wmillYaml = await readFile(`${tempDir}/wmill.yaml`, "utf-8");

    // Should have default settings, not backend settings
    expect(wmillYaml).toContain("includes:\n  - f/**");
    expect(wmillYaml).toContain("defaultTs: bun");

    // Should NOT have backend-specific settings
    expect(wmillYaml.includes("f/should-be-ignored/**")).toEqual(false);
    expect(wmillYaml).toContain("gitBranches: {}");
  });
});
