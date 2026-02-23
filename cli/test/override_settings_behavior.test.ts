import { expect, test } from "bun:test";
import { writeFile } from "node:fs/promises";
import { getEffectiveSettings } from "../src/core/conf.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

// =============================================================================
// OVERRIDE SETTINGS BEHAVIOR TESTS
// Tests for gitBranches override inheritance and file filtering behavior
// =============================================================================

test("Override Settings: branch override inherits non-overridden settings from base config", async () => {
    const config = {
      includes: ["default/**"],
      skipVariables: true,      // Base has this as true
      skipResources: true,      // Base has this as true
      skipApps: false,          // Base has this as false
      defaultTs: "bun" as const,
      gitBranches: {
        main: {
          overrides: {
            includes: ["override/**"],
            skipApps: true           // Override only changes skipApps, should inherit other skip flags
          }
        }
      }
    };

    const effective = await getEffectiveSettings(
      config,
      undefined,  // promotion
      true,       // skipBranchValidation
      true,       // suppressLogs
      "main"      // branchOverride
    );

    // Override values should be used
    expect(effective.includes).toEqual(["override/**"]);
    expect(effective.skipApps).toEqual(true);

    // Should inherit skip flags from base config
    expect(effective.skipVariables).toEqual(true);
    expect(effective.skipResources).toEqual(true);
    expect(effective.defaultTs).toEqual("bun");
});

test("Override Settings: branch-specific settings take precedence", async () => {
    const config = {
      includes: ["default/**"],
      skipVariables: false,
      gitBranches: {
        main: {
          overrides: {
            skipVariables: true,
            includes: ["main/**"]
          }
        },
        dev: {
          overrides: {
            skipVariables: false,
            includes: ["dev/**"]
          }
        }
      }
    };

    // Test main branch
    const mainEffective = await getEffectiveSettings(
      config,
      undefined,
      true,
      true,
      "main"
    );
    expect(mainEffective.includes).toEqual(["main/**"]);
    expect(mainEffective.skipVariables).toEqual(true);

    // Test dev branch
    const devEffective = await getEffectiveSettings(
      config,
      undefined,
      true,
      true,
      "dev"
    );
    expect(devEffective.includes).toEqual(["dev/**"]);
    expect(devEffective.skipVariables).toEqual(false);
});

// =============================================================================
// INTEGRATION TESTS - File Filtering Behavior with gitBranches
// =============================================================================

test("Integration: sync pull with skipVariables branch override excludes variable files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "skip_variables_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml with gitBranches override that skips variables
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

gitBranches:
  test_branch:
    overrides:
      skipVariables: true`, "utf-8");

      // Run sync pull with --branch to force using test_branch config
      const result = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'test_branch',
        '--dry-run',
        '--json-output'
      ], tempDir);

      expect(result.code).toEqual(0);

      // Parse output and verify variable files are NOT included
      const output = parseJsonFromCLIOutput(result.stdout);
      const changePaths = (output.changes || []).map((c: any) => c.path);

      const hasVariableFile = changePaths.some((path: string) => path.includes('.variable.yaml'));
      expect(hasVariableFile).toEqual(false);

      // Verify other files ARE included
      const hasOtherFiles = changePaths.some((path: string) =>
        !path.includes('.variable.yaml') && !path.includes('wmill.yaml')
      );
      expect(hasOtherFiles).toBeTruthy();
    });
});

test("Integration: sync pull respects includes branch override for file filtering", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "includes_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml with gitBranches override for includes
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"

gitBranches:
  restricted_branch:
    overrides:
      includes:
        - "users/**"
        - "groups/**"`, "utf-8");

      // Run sync pull with --branch to use restricted includes
      const result = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'restricted_branch',
        '--include-users',
        '--include-groups',
        '--dry-run',
        '--json-output'
      ], tempDir);

      expect(result.code).toEqual(0);

      // Parse output
      const output = parseJsonFromCLIOutput(result.stdout);
      const changePaths = (output.changes || []).map((c: any) => c.path);
      // Normalize paths for cross-platform comparison (Windows uses backslashes)
      const normalizedPaths = changePaths.map((p: string) => p.replace(/\\/g, '/'));

      // Verify users/groups are included
      const hasUserFiles = normalizedPaths.some((path: string) => path.includes('users/'));
      const hasGroupFiles = normalizedPaths.some((path: string) => path.includes('groups/'));

      expect(hasUserFiles || hasGroupFiles).toBeTruthy();

      // Verify f/** files are NOT included (due to restrictive includes)
      const hasFolderFiles = normalizedPaths.some((path: string) => path.startsWith('f/'));
      expect(hasFolderFiles).toEqual(false);
    });
});

test("Integration: different branches have different settings", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "multi_branch_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml with different settings per branch
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false
skipResources: false

gitBranches:
  prod:
    overrides:
      skipVariables: true
      skipResources: true
  dev:
    overrides:
      skipVariables: false
      skipResources: false`, "utf-8");

      // Test prod branch - should skip variables and resources
      const prodResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'prod',
        '--dry-run',
        '--json-output'
      ], tempDir);

      expect(prodResult.code).toEqual(0);

      const prodOutput = parseJsonFromCLIOutput(prodResult.stdout);
      const prodPaths = (prodOutput.changes || []).map((c: any) => c.path);

      const prodHasVariables = prodPaths.some((path: string) => path.includes('.variable.yaml'));
      const prodHasResources = prodPaths.some((path: string) => path.includes('.resource.yaml'));

      expect(prodHasVariables).toEqual(false);
      expect(prodHasResources).toEqual(false);

      // Test dev branch - should include variables and resources
      const devResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'dev',
        '--dry-run',
        '--json-output'
      ], tempDir);

      expect(devResult.code).toEqual(0);

      const devOutput = parseJsonFromCLIOutput(devResult.stdout);
      const devPaths = (devOutput.changes || []).map((c: any) => c.path);

      const devHasVariables = devPaths.some((path: string) => path.includes('.variable.yaml'));
      const devHasResources = devPaths.some((path: string) => path.includes('.resource.yaml'));

      expect(devHasVariables).toEqual(true);
      expect(devHasResources).toEqual(true);
    });
});
