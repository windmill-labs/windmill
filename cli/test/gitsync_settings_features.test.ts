import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";

// =============================================================================
// GITSYNC-SETTINGS COMMAND FEATURES
// Tests for additional gitsync-settings command functionality
// =============================================================================

Deno.test({
  name: "GitSync Settings: default mode writes to top-level",
  ignore: true, // TODO: Git sync settings API requires EE features
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "default_mode_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Configure backend with specific settings
      await backend.updateGitSyncConfig!({
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
    });
  }
});

Deno.test({
  name: "GitSync Settings: pull shows correct diff output",
  ignore: true, // TODO: Git sync settings API requires EE features
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "diff_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Configure backend
      await backend.updateGitSyncConfig!({
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

      assertEquals(result.code, 0, `Diff mode should succeed: ${result.stderr}`);

      // Should show differences
      assertStringIncludes(result.stdout, "Changes that would be applied locally:");
      // Should show the change for skipResources (ignoring ANSI color codes)
      assertStringIncludes(result.stdout, "skipResources:");
    });
  }
});

Deno.test({
  name: "GitSync Settings: replace mode overwrites existing config",
  ignore: true, // TODO: Git sync settings API requires EE features
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "replace_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Configure backend with specific settings
      await backend.updateGitSyncConfig!({
        git_sync_settings: {
          repositories: [{
            git_repo_resource_path: "u/test/replace_repo",
            script_path: "f/**",
            group_by_folder: false,
            use_individual_branch: false,
            settings: {
              include_path: ["f/replaced/**"],
              include_type: ["script", "flow"],
              exclude_path: ["*.backup.ts"],
              extra_include_path: []
            }
          }]
        }
      });

      // Create initial wmill.yaml with settings that should be replaced
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/old/**
excludes:
  - "*.old.ts"
skipVariables: true`);

      // Pull with replace flag
      const result = await backend.runCLICommand([
        'gitsync-settings', 'pull',
        '--repository', 'u/test/replace_repo',
        '--replace'
      ], tempDir);

      assertEquals(result.code, 0, `Replace mode pull should succeed: ${result.stderr}`);

      // Read updated config
      const updatedConfig = await Deno.readTextFile(`${tempDir}/wmill.yaml`);

      // Should have replaced settings from backend
      assertStringIncludes(updatedConfig, "f/replaced/**");
      assertStringIncludes(updatedConfig, "*.backup.ts");
    });
  }
});
