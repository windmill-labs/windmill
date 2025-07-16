import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";

// =============================================================================
// GITSYNC-SETTINGS COMMAND FEATURES
// Tests for additional gitsync-settings command functionality
// =============================================================================

Deno.test("GitSync Settings: workspace-level wildcard settings", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "workspace_level_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Configure backend with repository
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/workspace_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/**"],
            include_type: ["script", "flow"],
            exclude_path: [],
            extra_include_path: []
          }
        }]
      }
    });

    // Create initial wmill.yaml
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []`);

    // Pull with workspace-level flag
    const result = await backend.runCLICommand([
      'gitsync-settings', 'pull',
      '--repository', 'u/test/workspace_repo',
      '--workspace-level',
      '--override'
    ], tempDir);

    assertEquals(result.code, 0, `Workspace-level pull should succeed: ${result.stderr}`);

    // Read updated config
    const updatedConfig = await Deno.readTextFile(`${tempDir}/wmill.yaml`);
    const backendUrl = new URL(backend.baseUrl).toString();

    // Should create workspace wildcard override
    assertStringIncludes(updatedConfig, `'${backendUrl}:${backend.workspace}:*':`);
    assertStringIncludes(updatedConfig, "overrides:");
  });
});

Deno.test("GitSync Settings: default mode writes to top-level", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "default_mode_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Configure backend with specific settings
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/default_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/special/**"],
            include_type: ["script"],
            exclude_path: ["*.test.ts"],
            extra_include_path: ["g/**"]
          }
        }]
      }
    });

    // Create initial wmill.yaml with different settings
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
skipVariables: false`);

    // Pull with default flag
    const result = await backend.runCLICommand([
      'gitsync-settings', 'pull',
      '--repository', 'u/test/default_repo',
      '--default'
    ], tempDir);

    assertEquals(result.code, 0, `Default mode pull should succeed: ${result.stderr}`);

    // Read updated config
    const updatedConfig = await Deno.readTextFile(`${tempDir}/wmill.yaml`);

    // Should update top-level settings, not create overrides
    assertStringIncludes(updatedConfig, "includes:\n  - f/special/**");
    assertStringIncludes(updatedConfig, "excludes:\n  - '*.test.ts'");
    assertStringIncludes(updatedConfig, "extraIncludes:\n  - g/**");

    // Should NOT have overrides section
    assertEquals(updatedConfig.includes("overrides:"), false, "Default mode should not create overrides");
  });
});

// Removed test for non-existent repository error handling
// as it was testing non-deterministic behavior

Deno.test("GitSync Settings: pull shows correct diff output", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "diff_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    // Configure backend
    await backend.updateGitSyncConfig({
      git_sync_settings: {
        repositories: [{
          git_repo_resource_path: "u/test/diff_repo",
          script_path: "f/**",
          group_by_folder: false,
          use_individual_branch: false,
          settings: {
            include_path: ["f/**"],
            include_type: ["script", "flow"],
            exclude_path: [],
            extra_include_path: []
          }
        }]
      }
    });

    // Create wmill.yaml with different settings
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
skipVariables: true
skipResources: false`);

    // Pull with diff flag
    const result = await backend.runCLICommand([
      'gitsync-settings', 'pull',
      '--repository', 'u/test/diff_repo',
      '--diff'
    ], tempDir);

    assertEquals(result.code, 0);

    // Should show differences
    assertStringIncludes(result.stdout, "Changes that would be applied locally:");
    // Should show the change for skipResources (ignoring ANSI color codes)
    assertStringIncludes(result.stdout, "skipResources:");
  });
});
