import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { getEffectiveSettings } from "../src/core/conf.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

// =============================================================================
// OVERRIDE SETTINGS BEHAVIOR TESTS
// Tests for gitBranches override inheritance and file filtering behavior
// =============================================================================

Deno.test({
  name: "Override Settings: branch override inherits non-overridden settings from base config",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
    assertEquals(effective.includes, ["override/**"], "Must use override includes");
    assertEquals(effective.skipApps, true, "Must use override skipApps");

    // Should inherit skip flags from base config
    assertEquals(effective.skipVariables, true, "Must inherit skipVariables=true from base config");
    assertEquals(effective.skipResources, true, "Must inherit skipResources=true from base config");
    assertEquals(effective.defaultTs, "bun", "Must inherit defaultTs from base config");
  }
});

Deno.test({
  name: "Override Settings: branch-specific settings take precedence",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
    assertEquals(mainEffective.includes, ["main/**"], "Main branch must use its own includes");
    assertEquals(mainEffective.skipVariables, true, "Main branch must use its own skipVariables");

    // Test dev branch
    const devEffective = await getEffectiveSettings(
      config,
      undefined,
      true,
      true,
      "dev"
    );
    assertEquals(devEffective.includes, ["dev/**"], "Dev branch must use its own includes");
    assertEquals(devEffective.skipVariables, false, "Dev branch must use its own skipVariables");
  }
});

// =============================================================================
// INTEGRATION TESTS - File Filtering Behavior with gitBranches
// =============================================================================

Deno.test({
  name: "Integration: sync pull with skipVariables branch override excludes variable files",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

gitBranches:
  test_branch:
    overrides:
      skipVariables: true`);

      // Run sync pull with --branch to force using test_branch config
      const result = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'test_branch',
        '--dry-run',
        '--json-output'
      ], tempDir);

      assertEquals(result.code, 0, `Sync pull should succeed: ${result.stderr}`);

      // Parse output and verify variable files are NOT included
      const output = parseJsonFromCLIOutput(result.stdout);
      const changePaths = (output.changes || []).map((c: any) => c.path);

      const hasVariableFile = changePaths.some((path: string) => path.includes('.variable.yaml'));
      assertEquals(hasVariableFile, false, "Variable files should NOT be included due to skipVariables override");

      // Verify other files ARE included
      const hasOtherFiles = changePaths.some((path: string) =>
        !path.includes('.variable.yaml') && !path.includes('wmill.yaml')
      );
      assert(hasOtherFiles, `Other files should be included. Found paths: ${changePaths.join(', ')}`);
    });
  }
});

Deno.test({
  name: "Integration: sync pull respects includes branch override for file filtering",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"

gitBranches:
  restricted_branch:
    overrides:
      includes:
        - "users/**"
        - "groups/**"`);

      // Run sync pull with --branch to use restricted includes
      const result = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'restricted_branch',
        '--include-users',
        '--include-groups',
        '--dry-run',
        '--json-output'
      ], tempDir);

      assertEquals(result.code, 0, `Sync pull should succeed: ${result.stderr}`);

      // Parse output
      const output = parseJsonFromCLIOutput(result.stdout);
      const changePaths = (output.changes || []).map((c: any) => c.path);
      // Normalize paths for cross-platform comparison (Windows uses backslashes)
      const normalizedPaths = changePaths.map((p: string) => p.replace(/\\/g, '/'));

      // Verify users/groups are included
      const hasUserFiles = normalizedPaths.some((path: string) => path.includes('users/'));
      const hasGroupFiles = normalizedPaths.some((path: string) => path.includes('groups/'));

      assert(hasUserFiles || hasGroupFiles, `User or group files should be included. Found: ${normalizedPaths.join(', ')}`);

      // Verify f/** files are NOT included (due to restrictive includes)
      const hasFolderFiles = normalizedPaths.some((path: string) => path.startsWith('f/'));
      assertEquals(hasFolderFiles, false, `f/ files should NOT be included due to restrictive includes. Found: ${normalizedPaths.join(', ')}`);
    });
  }
});

Deno.test({
  name: "Integration: different branches have different settings",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
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
      skipResources: false`);

      // Test prod branch - should skip variables and resources
      const prodResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'prod',
        '--dry-run',
        '--json-output'
      ], tempDir);

      assertEquals(prodResult.code, 0, `Prod sync pull should succeed: ${prodResult.stderr}`);

      const prodOutput = parseJsonFromCLIOutput(prodResult.stdout);
      const prodPaths = (prodOutput.changes || []).map((c: any) => c.path);

      const prodHasVariables = prodPaths.some((path: string) => path.includes('.variable.yaml'));
      const prodHasResources = prodPaths.some((path: string) => path.includes('.resource.yaml'));

      assertEquals(prodHasVariables, false, "Prod branch should skip variables");
      assertEquals(prodHasResources, false, "Prod branch should skip resources");

      // Test dev branch - should include variables and resources
      const devResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'dev',
        '--dry-run',
        '--json-output'
      ], tempDir);

      assertEquals(devResult.code, 0, `Dev sync pull should succeed: ${devResult.stderr}`);

      const devOutput = parseJsonFromCLIOutput(devResult.stdout);
      const devPaths = (devOutput.changes || []).map((c: any) => c.path);

      const devHasVariables = devPaths.some((path: string) => path.includes('.variable.yaml'));
      const devHasResources = devPaths.some((path: string) => path.includes('.resource.yaml'));

      assertEquals(devHasVariables, true, "Dev branch should include variables");
      assertEquals(devHasResources, true, "Dev branch should include resources");
    });
  }
});
