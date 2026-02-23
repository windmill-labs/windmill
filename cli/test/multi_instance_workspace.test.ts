import { expect, test } from "bun:test";
import { writeFile } from "node:fs/promises";
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

test("Multi-Branch: sync pull with branch-specific overrides", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "multi_branch_test");

      // Create wmill.yaml with gitBranches configuration
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
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
      skipResources: true`, "utf-8");

      // Test main branch - should include variables and resources
      const mainResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'main',
        '--dry-run',
        '--json-output'
      ], tempDir, "multi_branch_test");

      expect(mainResult.code).toEqual(0);

      const mainData = parseJsonFromCLIOutput(mainResult.stdout);
      const mainPaths = (mainData.changes || []).map((c: any) => c.path);

      const mainHasVariables = mainPaths.some((path: string) => path.includes('.variable.yaml'));
      const mainHasResources = mainPaths.some((path: string) => path.includes('.resource.yaml'));

      expect(mainHasVariables).toEqual(true);
      expect(mainHasResources).toEqual(true);

      // Test staging branch - should skip variables but include resources
      const stagingResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'staging',
        '--dry-run',
        '--json-output'
      ], tempDir, "multi_branch_test");

      expect(stagingResult.code).toEqual(0);

      const stagingData = parseJsonFromCLIOutput(stagingResult.stdout);
      const stagingPaths = (stagingData.changes || []).map((c: any) => c.path);

      const stagingHasVariables = stagingPaths.some((path: string) => path.includes('.variable.yaml'));
      const stagingHasResources = stagingPaths.some((path: string) => path.includes('.resource.yaml'));

      expect(stagingHasVariables).toEqual(false);
      expect(stagingHasResources).toEqual(true);

      // Test prod branch - should skip both variables and resources
      const prodResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'prod',
        '--dry-run',
        '--json-output'
      ], tempDir, "multi_branch_test");

      expect(prodResult.code).toEqual(0);

      const prodData = parseJsonFromCLIOutput(prodResult.stdout);
      const prodPaths = (prodData.changes || []).map((c: any) => c.path);

      const prodHasVariables = prodPaths.some((path: string) => path.includes('.variable.yaml'));
      const prodHasResources = prodPaths.some((path: string) => path.includes('.resource.yaml'));

      expect(prodHasVariables).toEqual(false);
      expect(prodHasResources).toEqual(false);
    });
});

test("Multi-Branch: branch override with includes filtering", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "includes_branch_test");

      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
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
      skipVariables: false`, "utf-8");

      // Test feature branch - should skip variables and only include f/**
      const featureResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'feature',
        '--dry-run',
        '--json-output'
      ], tempDir, "includes_branch_test");

      expect(featureResult.code).toEqual(0);

      const featureData = parseJsonFromCLIOutput(featureResult.stdout);
      const featurePaths = (featureData.changes || []).map((c: any) => c.path);
      // Normalize paths for cross-platform comparison (Windows uses backslashes)
      const normalizedFeaturePaths = featurePaths.map((p: string) => p.replace(/\\/g, '/'));

      const featureHasVariables = normalizedFeaturePaths.some((path: string) => path.includes('.variable.yaml'));
      const featureHasUsers = normalizedFeaturePaths.some((path: string) => path.startsWith('users/'));

      expect(featureHasVariables).toEqual(false);
      expect(featureHasUsers).toEqual(false);

      // Test release branch - should include variables and users
      const releaseResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'release',
        '--include-users',
        '--dry-run',
        '--json-output'
      ], tempDir, "includes_branch_test");

      expect(releaseResult.code).toEqual(0);

      const releaseData = parseJsonFromCLIOutput(releaseResult.stdout);
      const releasePaths = (releaseData.changes || []).map((c: any) => c.path);
      // Normalize paths for cross-platform comparison (Windows uses backslashes)
      const normalizedReleasePaths = releasePaths.map((p: string) => p.replace(/\\/g, '/'));

      const releaseHasVariables = normalizedReleasePaths.some((path: string) => path.includes('.variable.yaml'));
      const releaseHasUsers = normalizedReleasePaths.some((path: string) => path.startsWith('users/'));

      expect(releaseHasVariables).toEqual(true);
      expect(releaseHasUsers).toEqual(true);
    });
});

test("Multi-Branch: fallback to base config when branch not defined", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "fallback_test");

      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: true
skipResources: true

gitBranches:
  main:
    overrides:
      skipVariables: false
      skipResources: false`, "utf-8");

      // Test undefined branch - should use base config (skip variables and resources)
      const undefinedResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'undefined_branch',
        '--dry-run',
        '--json-output'
      ], tempDir, "fallback_test");

      expect(undefinedResult.code).toEqual(0);

      const undefinedData = parseJsonFromCLIOutput(undefinedResult.stdout);
      const undefinedPaths = (undefinedData.changes || []).map((c: any) => c.path);

      const undefinedHasVariables = undefinedPaths.some((path: string) => path.includes('.variable.yaml'));
      const undefinedHasResources = undefinedPaths.some((path: string) => path.includes('.resource.yaml'));

      // Should use base config since branch is not defined
      expect(undefinedHasVariables).toEqual(false);
      expect(undefinedHasResources).toEqual(false);

      // Test defined main branch - should use branch overrides
      const mainResult = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'main',
        '--dry-run',
        '--json-output'
      ], tempDir, "fallback_test");

      expect(mainResult.code).toEqual(0);

      const mainData = parseJsonFromCLIOutput(mainResult.stdout);
      const mainPaths = (mainData.changes || []).map((c: any) => c.path);

      const mainHasVariables = mainPaths.some((path: string) => path.includes('.variable.yaml'));
      const mainHasResources = mainPaths.some((path: string) => path.includes('.resource.yaml'));

      expect(mainHasVariables).toEqual(true);
      expect(mainHasResources).toEqual(true);
    });
});

test("Multi-Branch: branch inherits unspecified settings from base", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend, "inherit_test");

      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: true
skipResources: true
skipApps: true

gitBranches:
  partial:
    overrides:
      skipVariables: false`, "utf-8");

      // Test partial branch - should inherit skipResources and skipApps from base
      const result = await backend.runCLICommand([
        'sync', 'pull',
        '--branch', 'partial',
        '--dry-run',
        '--json-output'
      ], tempDir, "inherit_test");

      expect(result.code).toEqual(0);

      const data = parseJsonFromCLIOutput(result.stdout);
      const paths = (data.changes || []).map((c: any) => c.path);

      const hasVariables = paths.some((path: string) => path.includes('.variable.yaml'));
      const hasResources = paths.some((path: string) => path.includes('.resource.yaml'));
      const hasApps = paths.some((path: string) => path.includes('.app/') || path.endsWith('.app.yaml'));

      // skipVariables is overridden to false
      expect(hasVariables).toEqual(true);
      // skipResources and skipApps are inherited from base (true)
      expect(hasResources).toEqual(false);
      expect(hasApps).toEqual(false);
    });
});
