import { expect, test } from "bun:test";
import { writeFile, readFile } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

// =============================================================================
// GITSYNC-SETTINGS COMMAND FEATURES
// Tests for additional gitsync-settings command functionality
// These tests require EE features (private, enterprise) and are skipped in CI
// =============================================================================

test.skipIf(shouldSkipOnCI())("GitSync Settings: default mode writes to top-level", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
skipVariables: false`, "utf-8");

      // Pull with default flag
      const result = await backend.runCLICommand([
        'gitsync-settings', 'pull',
        '--repository', 'u/test/default_repo',
        '--default'
      ], tempDir);

      expect(result.code).toEqual(0);

      // Read updated config
      const updatedConfig = await readFile(`${tempDir}/wmill.yaml`, "utf-8");

      // Should update top-level settings, not create overrides
      expect(updatedConfig).toContain("includes:\n  - f/special/**");
      expect(updatedConfig).toContain("excludes:\n  - '*.test.ts'");
      expect(updatedConfig).toContain("extraIncludes:\n  - g/**");
    });
});

test.skipIf(shouldSkipOnCI())("GitSync Settings: pull shows correct diff output", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
skipVariables: true
skipResources: false`, "utf-8");

      // Pull with diff flag
      const result = await backend.runCLICommand([
        'gitsync-settings', 'pull',
        '--repository', 'u/test/diff_repo',
        '--diff'
      ], tempDir);

      expect(result.code).toEqual(0);

      // Should show differences
      expect(result.stdout).toContain("Changes that would be applied locally:");
      // Should show the change for skipResources (ignoring ANSI color codes)
      expect(result.stdout).toContain("skipResources:");
    });
});

test.skipIf(shouldSkipOnCI())("GitSync Settings: replace mode overwrites existing config", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/old/**
excludes:
  - "*.old.ts"
skipVariables: true`, "utf-8");

      // Pull with replace flag
      const result = await backend.runCLICommand([
        'gitsync-settings', 'pull',
        '--repository', 'u/test/replace_repo',
        '--replace'
      ], tempDir);

      expect(result.code).toEqual(0);

      // Read updated config
      const updatedConfig = await readFile(`${tempDir}/wmill.yaml`, "utf-8");

      // Should have replaced settings from backend
      expect(updatedConfig).toContain("f/replaced/**");
      expect(updatedConfig).toContain("*.backup.ts");
    });
});
