import { expect, test } from "bun:test";
import { writeFile, readFile } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

async function setupProfile(backend: any, name: string): Promise<void> {
  await addWorkspace({
    remote: backend.baseUrl,
    workspaceId: backend.workspace,
    name,
    token: backend.token,
  }, { force: true, configDir: backend.testConfigDir });
}

// =============================================================================
// Test 1: Full sync flow with new workspaces config — overrides, gitBranch ≠ key,
//         workspace-specific file suffix, legacy back-compat, and promotion
// =============================================================================

test("Full workspaces config: overrides, gitBranch mapping, specific items suffix, legacy compat, promotion", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupProfile(backend, "test_profile");

    // --- Part A: New workspaces config with gitBranch ≠ key ---
    await writeFile(join(tempDir, "wmill.yaml"), `defaultTs: bun
includes:
  - "**"
skipVariables: false

workspaces:
  production:
    gitBranch: prod_branch
    baseUrl: ${backend.baseUrl}
    workspaceId: ${backend.workspace}
    overrides:
      skipVariables: true
    promotionOverrides:
      skipVariables: true
      skipResources: true
    specificItems:
      variables:
        - "f/**"
  development:
    gitBranch: dev_branch
    baseUrl: ${backend.baseUrl}
    workspaceId: ${backend.workspace}
    overrides:
      skipVariables: false`, "utf-8");

    // A1: --branch prod_branch → resolves to workspace "production", skipVariables: true
    const prodResult = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'prod_branch', '--dry-run', '--json-output',
    ], tempDir);
    expect(prodResult.code).toEqual(0);
    const prodChanges = parseJsonFromCLIOutput(prodResult.stdout).changes || [];
    const prodPaths = prodChanges.map((c: any) => c.path);
    expect(prodPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);

    // A2: --branch dev_branch → resolves to workspace "development", skipVariables: false
    const devResult = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'dev_branch', '--dry-run', '--json-output',
    ], tempDir);
    expect(devResult.code).toEqual(0);
    const devPaths = (parseJsonFromCLIOutput(devResult.stdout).changes || []).map((c: any) => c.path);
    expect(devPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(true);

    // A3: workspace-specific file suffix uses workspace name "production", not "prod_branch"
    const wsSpecificPaths = prodChanges
      .filter((c: any) => c.workspace_specific_path)
      .map((c: any) => c.workspace_specific_path);
    for (const p of wsSpecificPaths) {
      expect(p).toContain(".production.");
      expect(p).not.toContain(".prod_branch.");
    }

    // A4: --promotion prod_branch applies promotionOverrides (skipResources: true)
    const promoResult = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'prod_branch', '--promotion', 'prod_branch',
      '--dry-run', '--json-output',
    ], tempDir);
    expect(promoResult.code).toEqual(0);
    const promoPaths = (parseJsonFromCLIOutput(promoResult.stdout).changes || []).map((c: any) => c.path);
    expect(promoPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);
    expect(promoPaths.some((p: string) => p.includes('.resource.yaml'))).toEqual(false);

    // --- Part B: Legacy gitBranches config works via normalization ---
    await writeFile(join(tempDir, "wmill.yaml"), `defaultTs: bun
includes:
  - "**"
skipVariables: false

gitBranches:
  legacy_branch:
    overrides:
      skipVariables: true`, "utf-8");

    const legacyResult = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'legacy_branch', '--dry-run', '--json-output',
    ], tempDir);
    expect(legacyResult.code).toEqual(0);
    const legacyPaths = (parseJsonFromCLIOutput(legacyResult.stdout).changes || []).map((c: any) => c.path);
    expect(legacyPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);

    // --- Part C: Legacy environments config ---
    await writeFile(join(tempDir, "wmill.yaml"), `defaultTs: bun
includes:
  - "**"
skipVariables: false

environments:
  env_branch:
    overrides:
      skipVariables: true`, "utf-8");

    const envResult = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'env_branch', '--dry-run', '--json-output',
    ], tempDir);
    expect(envResult.code).toEqual(0);
    const envPaths = (parseJsonFromCLIOutput(envResult.stdout).changes || []).map((c: any) => c.path);
    expect(envPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);
  });
});

// =============================================================================
// Test 2: Config migrate and workspace resolution fallbacks
// =============================================================================

test("Config migrate and workspace resolution fallbacks", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupProfile(backend, "fallback_test");

    // --- Part A: config migrate converts gitBranches → workspaces ---
    await writeFile(join(tempDir, "wmill.yaml"), `defaultTs: bun
includes:
  - "f/**"
gitBranches:
  main:
    baseUrl: https://app.windmill.dev
    workspaceId: production
    overrides:
      skipSecrets: false
  commonSpecificItems:
    variables:
      - "f/shared/**"`, "utf-8");

    const migrateResult = await backend.runCLICommand(['config', 'migrate'], tempDir);
    expect(migrateResult.code).toEqual(0);

    const migrated = await readFile(join(tempDir, "wmill.yaml"), "utf-8");
    expect(migrated).toContain("workspaces:");
    expect(migrated).not.toContain("gitBranches:");
    expect(migrated).toContain("production");
    expect(migrated).toContain("commonSpecificItems");

    // config migrate is idempotent
    const migrateAgain = await backend.runCLICommand(['config', 'migrate'], tempDir);
    expect(migrateAgain.code).toEqual(0);

    // --- Part B: single workspace auto-selected ---
    await writeFile(join(tempDir, "wmill.yaml"), `defaultTs: bun
includes:
  - "**"
skipVariables: false
workspaces:
  only_ws:
    baseUrl: ${backend.baseUrl}
    workspaceId: ${backend.workspace}
    overrides:
      skipVariables: true`, "utf-8");

    // No --branch, no --workspace: should auto-select "only_ws"
    const singleResult = await backend.runCLICommand([
      'sync', 'pull', '--dry-run', '--json-output',
    ], tempDir);
    expect(singleResult.code).toEqual(0);
    const singlePaths = (parseJsonFromCLIOutput(singleResult.stdout).changes || []).map((c: any) => c.path);
    // skipVariables: true should be applied from auto-selected workspace
    expect(singlePaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);

    // --- Part C: no workspaces config falls back to active profile ---
    await writeFile(join(tempDir, "wmill.yaml"), `defaultTs: bun
includes:
  - "**"`, "utf-8");

    const noWsResult = await backend.runCLICommand([
      'sync', 'pull', '--dry-run', '--json-output',
    ], tempDir);
    expect(noWsResult.code).toEqual(0);
    // Should succeed using active profile, no overrides applied (all defaults)
    const noWsPaths = (parseJsonFromCLIOutput(noWsResult.stdout).changes || []).map((c: any) => c.path);
    expect(noWsPaths.length).toBeGreaterThan(0);
    // Variables should be included (no skipVariables override)
    expect(noWsPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(true);
  });
});
