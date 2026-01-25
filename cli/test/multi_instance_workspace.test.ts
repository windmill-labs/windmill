import { assertEquals, assert, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

// =============================================================================
// MULTI-BRANCH WORKSPACE TESTS
// Tests for handling multiple Git branches with different configurations
// =============================================================================

// Helper function to set up workspace profile with specific name
async function setupWorkspaceProfile(backend: any, workspaceName: string): Promise<void> {
  const testWorkspace = {
    remote: backend.baseUrl,
    workspaceId: backend.workspace,
    name: workspaceName,
    token: backend.token
  };

  await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });
}

Deno.test({
  name: "Multi-Branch: sync pull with branch-specific overrides",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "multi_branch_test");

      // Create wmill.yaml with gitBranches configuration
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []

gitBranches:
  main:
    overrides:
      skipVariables: false
      skipResources: false
  staging:
    overrides:
      skipVariables: true
      skipResources: false
  prod:
    overrides:
      skipVariables: true
      skipResources: true`);

      // Test main branch - should include variables and resources
      const mainResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'main',
        '--dry-run',
        '--json-output'
      ], tempDir, "multi_branch_test");

      assertEquals(mainResult.code, 0, `Main branch sync should succeed: ${mainResult.stderr}`);

      const mainData = parseJsonFromCLIOutput(mainResult.stdout);
      const mainPaths = (mainData.changes || []).map((c: any) => c.path);

      const mainHasVariables = mainPaths.some((path: string) => path.includes('.variable.yaml'));
      const mainHasResources = mainPaths.some((path: string) => path.includes('.resource.yaml'));

      assertEquals(mainHasVariables, true, "Main branch should include variables");
      assertEquals(mainHasResources, true, "Main branch should include resources");

      // Test staging branch - should skip variables but include resources
      const stagingResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'staging',
        '--dry-run',
        '--json-output'
      ], tempDir, "multi_branch_test");

      assertEquals(stagingResult.code, 0, `Staging branch sync should succeed: ${stagingResult.stderr}`);

      const stagingData = parseJsonFromCLIOutput(stagingResult.stdout);
      const stagingPaths = (stagingData.changes || []).map((c: any) => c.path);

      const stagingHasVariables = stagingPaths.some((path: string) => path.includes('.variable.yaml'));
      const stagingHasResources = stagingPaths.some((path: string) => path.includes('.resource.yaml'));

      assertEquals(stagingHasVariables, false, "Staging branch should skip variables");
      assertEquals(stagingHasResources, true, "Staging branch should include resources");

      // Test prod branch - should skip both variables and resources
      const prodResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'prod',
        '--dry-run',
        '--json-output'
      ], tempDir, "multi_branch_test");

      assertEquals(prodResult.code, 0, `Prod branch sync should succeed: ${prodResult.stderr}`);

      const prodData = parseJsonFromCLIOutput(prodResult.stdout);
      const prodPaths = (prodData.changes || []).map((c: any) => c.path);

      const prodHasVariables = prodPaths.some((path: string) => path.includes('.variable.yaml'));
      const prodHasResources = prodPaths.some((path: string) => path.includes('.resource.yaml'));

      assertEquals(prodHasVariables, false, "Prod branch should skip variables");
      assertEquals(prodHasResources, false, "Prod branch should skip resources");
    });
  }
});

Deno.test({
  name: "Multi-Branch: branch override with includes filtering",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "includes_branch_test");

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"

gitBranches:
  feature:
    overrides:
      includes:
        - "f/**"
      skipVariables: true
  release:
    overrides:
      includes:
        - "f/**"
        - "users/**"
      skipVariables: false`);

      // Test feature branch - should skip variables and only include f/**
      const featureResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'feature',
        '--dry-run',
        '--json-output'
      ], tempDir, "includes_branch_test");

      assertEquals(featureResult.code, 0, `Feature branch sync should succeed: ${featureResult.stderr}`);

      const featureData = parseJsonFromCLIOutput(featureResult.stdout);
      const featurePaths = (featureData.changes || []).map((c: any) => c.path);

      const featureHasVariables = featurePaths.some((path: string) => path.includes('.variable.yaml'));
      const featureHasUsers = featurePaths.some((path: string) => path.startsWith('users/'));

      assertEquals(featureHasVariables, false, "Feature branch should skip variables");
      assertEquals(featureHasUsers, false, "Feature branch should not include users (not in includes)");

      // Test release branch - should include variables and users
      const releaseResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'release',
        '--include-users',
        '--dry-run',
        '--json-output'
      ], tempDir, "includes_branch_test");

      assertEquals(releaseResult.code, 0, `Release branch sync should succeed: ${releaseResult.stderr}`);

      const releaseData = parseJsonFromCLIOutput(releaseResult.stdout);
      const releasePaths = (releaseData.changes || []).map((c: any) => c.path);

      const releaseHasVariables = releasePaths.some((path: string) => path.includes('.variable.yaml'));
      const releaseHasUsers = releasePaths.some((path: string) => path.startsWith('users/'));

      assertEquals(releaseHasVariables, true, "Release branch should include variables");
      assertEquals(releaseHasUsers, true, "Release branch should include users");
    });
  }
});

Deno.test({
  name: "Multi-Branch: fallback to base config when branch not defined",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "fallback_test");

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: true
skipResources: true

gitBranches:
  main:
    overrides:
      skipVariables: false
      skipResources: false`);

      // Test undefined branch - should use base config (skip variables and resources)
      const undefinedResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'undefined_branch',
        '--dry-run',
        '--json-output'
      ], tempDir, "fallback_test");

      assertEquals(undefinedResult.code, 0, `Undefined branch sync should succeed: ${undefinedResult.stderr}`);

      const undefinedData = parseJsonFromCLIOutput(undefinedResult.stdout);
      const undefinedPaths = (undefinedData.changes || []).map((c: any) => c.path);

      const undefinedHasVariables = undefinedPaths.some((path: string) => path.includes('.variable.yaml'));
      const undefinedHasResources = undefinedPaths.some((path: string) => path.includes('.resource.yaml'));

      // Should use base config since branch is not defined
      assertEquals(undefinedHasVariables, false, "Undefined branch should use base config skipVariables: true");
      assertEquals(undefinedHasResources, false, "Undefined branch should use base config skipResources: true");

      // Test defined main branch - should use branch overrides
      const mainResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'main',
        '--dry-run',
        '--json-output'
      ], tempDir, "fallback_test");

      assertEquals(mainResult.code, 0, `Main branch sync should succeed: ${mainResult.stderr}`);

      const mainData = parseJsonFromCLIOutput(mainResult.stdout);
      const mainPaths = (mainData.changes || []).map((c: any) => c.path);

      const mainHasVariables = mainPaths.some((path: string) => path.includes('.variable.yaml'));
      const mainHasResources = mainPaths.some((path: string) => path.includes('.resource.yaml'));

      assertEquals(mainHasVariables, true, "Main branch should use override skipVariables: false");
      assertEquals(mainHasResources, true, "Main branch should use override skipResources: false");
    });
  }
});

Deno.test({
  name: "Multi-Branch: branch inherits unspecified settings from base",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "inherit_test");

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: true
skipResources: true
skipApps: true

gitBranches:
  partial:
    overrides:
      skipVariables: false`);

      // Test partial branch - should inherit skipResources and skipApps from base
      const result = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'partial',
        '--dry-run',
        '--json-output'
      ], tempDir, "inherit_test");

      assertEquals(result.code, 0, `Partial branch sync should succeed: ${result.stderr}`);

      const data = parseJsonFromCLIOutput(result.stdout);
      const paths = (data.changes || []).map((c: any) => c.path);

      const hasVariables = paths.some((path: string) => path.includes('.variable.yaml'));
      const hasResources = paths.some((path: string) => path.includes('.resource.yaml'));
      const hasApps = paths.some((path: string) => path.includes('.app/') || path.endsWith('.app.yaml'));

      // skipVariables is overridden to false
      assertEquals(hasVariables, true, "Partial branch should include variables (override)");
      // skipResources and skipApps are inherited from base (true)
      assertEquals(hasResources, false, "Partial branch should skip resources (inherited)");
      assertEquals(hasApps, false, "Partial branch should skip apps (inherited)");
    });
  }
});
