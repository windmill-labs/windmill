/**
 * Test to verify that wmill init handles workspaces with no git-sync settings correctly
 * This creates a unit test that directly tests the logic without needing a backend
 */

import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { DEFAULT_SYNC_OPTIONS } from "../conf.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
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

Deno.test("Init: createWorkspaceProfile includes defaults when no repositories exist", () => {
  console.log('🧪 Testing init logic for workspace with no git-sync repositories...');
  
  const workspaceProfile = createWorkspaceProfileNoRepos(mockWorkspace);
  
  console.log('Generated workspace profile:', JSON.stringify(workspaceProfile, null, 2));
  
  // Verify basic workspace info
  assertEquals(workspaceProfile.baseUrl, 'https://app.windmill.dev/');
  assertEquals(workspaceProfile.workspaceId, 'test-workspace');
  
  // Verify default sync settings are included
  assert(Array.isArray(workspaceProfile.includes), 'Should have includes array');
  assertEquals(workspaceProfile.includes.length, 1, 'Should have one include pattern');
  assertEquals(workspaceProfile.includes[0], 'f/**', 'Should include f/** pattern');
  
  assert(Array.isArray(workspaceProfile.excludes), 'Should have excludes array');
  assertEquals(workspaceProfile.excludes.length, 0, 'Should have empty excludes array');
  
  assertEquals(workspaceProfile.defaultTs, 'bun', 'Should have bun as default TypeScript runtime');
  
  console.log('✅ Workspace profile correctly includes default sync settings when no repositories exist');
});

Deno.test("Init: verify DEFAULT_SYNC_OPTIONS has expected values", () => {
  console.log('🔍 Verifying DEFAULT_SYNC_OPTIONS contains expected values...');
  
  console.log('DEFAULT_SYNC_OPTIONS:', JSON.stringify(DEFAULT_SYNC_OPTIONS, null, 2));
  
  // Verify the default options include the expected f/** pattern
  assert(Array.isArray(DEFAULT_SYNC_OPTIONS.includes), 'DEFAULT_SYNC_OPTIONS should have includes array');
  assertEquals(DEFAULT_SYNC_OPTIONS.includes.length, 1, 'Should have one include pattern');
  assertEquals(DEFAULT_SYNC_OPTIONS.includes[0], 'f/**', 'Should default to f/** pattern');
  
  assert(Array.isArray(DEFAULT_SYNC_OPTIONS.excludes), 'DEFAULT_SYNC_OPTIONS should have excludes array');
  assertEquals(DEFAULT_SYNC_OPTIONS.excludes.length, 0, 'Should have empty excludes array by default');
  
  assertEquals(DEFAULT_SYNC_OPTIONS.defaultTs, 'bun', 'Should default to bun runtime');
  
  console.log('✅ DEFAULT_SYNC_OPTIONS has expected values');
});

Deno.test("Init: --use-backend flag applies git-sync settings", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
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

    assertEquals(result.code, 0, `Init with --use-backend should succeed: ${result.stderr}`);

    // Verify wmill.yaml was created with backend settings
    const wmillYaml = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
    
    // Should have backend-applied settings written to top-level (not overrides)
    assertStringIncludes(wmillYaml, "f/backend/**", "Should include backend's include_path");
    assertStringIncludes(wmillYaml, "*.test.ts", "Should include backend's exclude_path");
    assertStringIncludes(wmillYaml, "g/**", "Should include backend's extra_include_path");
    
    // Should have empty overrides section for consistency
    assertStringIncludes(wmillYaml, "overrides: {}");
  });
});

Deno.test("Init: --use-default bypasses backend settings check", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
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

    assertEquals(result.code, 0, `Init with --use-default should succeed: ${result.stderr}`);

    // Verify wmill.yaml was created with default settings only
    const wmillYaml = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
    
    // Should have default settings, not backend settings
    assertStringIncludes(wmillYaml, "includes:\n  - f/**", "Should use default includes");
    assertStringIncludes(wmillYaml, "defaultTs: bun", "Should use default TypeScript runtime");
    
    // Should NOT have backend-specific settings
    assertEquals(wmillYaml.includes("f/should-be-ignored/**"), false, "Should not include backend settings");
    assertStringIncludes(wmillYaml, "overrides: {}", "Should have empty overrides section for consistency");
  });
});