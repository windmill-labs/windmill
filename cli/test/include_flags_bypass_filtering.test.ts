import { expect, test } from "bun:test";
import { writeFile } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

// =============================================================================
// INCLUDE FLAGS BYPASS FILTERING TESTS
// Tests that CLI include flags properly bypass path-based filtering
// =============================================================================

// Helper function to set up workspace profile
async function setupWorkspaceProfile(backend: any): Promise<void> {
  const testWorkspace = {
    remote: backend.baseUrl,
    workspaceId: backend.workspace,
    name: "localhost_test",
    token: backend.token
  };

  await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });
}

// The ContainerizedBackend already creates test data we can use:
// - admin user (admin@windmill.dev)
// - test_group (created by seedTestData())
// - workspace encryption key
// - test apps, resources, variables via seedTestData()
// No additional setup needed!

test("CLI include flags bypass restrictive path filtering", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);

    // Create wmill.yaml with very restrictive includes that would exclude special files
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "f/**"
excludes: []
skipVariables: true
skipResources: true
includeUsers: false
includeGroups: false
includeSettings: false
includeKey: false`, "utf-8");

    // Test: CLI flags should override config and bypass path filtering
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--include-users',
      '--include-groups',
      '--include-settings',
      '--include-key',
      '--dry-run',
      '--json-output'
    ], tempDir);

    expect(result.code).toEqual(0);

    const output = parseJsonFromCLIOutput(result.stdout);
    const changePaths = output.changes.map((c: any) => c.path);

    // Assert that special files are included despite restrictive path filtering
    // Normalize paths for cross-platform comparison (Windows uses backslashes)
    const normalizedPaths = changePaths.map((p: string) => p.replace(/\\/g, '/'));
    const hasUser = normalizedPaths.some((path: string) => path.includes('admin@windmill.dev.user.yaml'));
    const hasGroup = normalizedPaths.some((path: string) => path.includes('groups/test_group.group.yaml'));
    const hasSettings = changePaths.some((path: string) => path === 'settings.yaml');
    const hasEncryptionKey = changePaths.some((path: string) => path === 'encryption_key.yaml');

    expect(hasUser).toBe(true);
    expect(hasGroup).toBe(true);
    expect(hasSettings).toBe(true);
    expect(hasEncryptionKey).toBe(true);
  });
});

test("CLI flags override wmill.yaml include settings", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);

    // Config explicitly disables includes, but CLI should override
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []
includeUsers: false
includeGroups: false`, "utf-8");

    // CLI flags should override config file settings
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--include-users',
      '--include-groups',
      '--dry-run',
      '--json-output'
    ], tempDir);

    expect(result.code).toEqual(0);

    const output = parseJsonFromCLIOutput(result.stdout);
    const changePaths = output.changes.map((c: any) => c.path);

    // Normalize paths for cross-platform comparison (Windows uses backslashes)
    const normalizedPaths = changePaths.map((p: string) => p.replace(/\\/g, '/'));
    const hasUser = normalizedPaths.some((path: string) => path.includes('admin@windmill.dev.user.yaml'));
    const hasGroup = normalizedPaths.some((path: string) => path.includes('groups/test_group.group.yaml'));

    expect(hasUser).toBe(true);
    expect(hasGroup).toBe(true);
  });
});

test("Skip flags work correctly with getTypeStrFromPath and lock files", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);

    // Create wmill.yaml with skip flags enabled
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []
skipScripts: true
skipFlows: false
includeUsers: true`, "utf-8");

    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--dry-run',
      '--json-output'
    ], tempDir);

    expect(result.code).toEqual(0);

    const output = parseJsonFromCLIOutput(result.stdout);
    const changePaths = output.changes.map((c: any) => c.path);

    // Scripts should be skipped (including lock files) - the backend doesn't create scripts by default
    const hasScript = changePaths.some((path: string) =>
      path.endsWith('.py') || path.endsWith('.ts') || path.endsWith('.go') || path.endsWith('.sh')
    );
    const hasScriptLock = changePaths.some((path: string) => path.endsWith('.script.lock'));

    // Apps should be included (the backend creates test apps)
    const hasApp = changePaths.some((path: string) => path.includes('test_dashboard') || path.endsWith('.app.yaml'));

    // Users should still be included
    const hasUser = changePaths.some((path: string) => path.includes('admin@windmill.dev.user.yaml'));

    expect(hasScript).toBe(false);
    expect(hasScriptLock).toBe(false);
    expect(hasApp).toBe(true);
    expect(hasUser).toBe(true);
  });
});

test("Mixed include and skip flags work together", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);

    // Create restrictive config with mixed settings
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "f/**"
excludes: []
skipScripts: true
includeUsers: false
includeSettings: false`, "utf-8");

    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--skip-scripts',      // Reinforce script skipping
      '--include-users',     // Override config to include users
      '--dry-run',
      '--json-output'
    ], tempDir);

    expect(result.code).toEqual(0);

    const output = parseJsonFromCLIOutput(result.stdout);
    const changePaths = output.changes.map((c: any) => c.path);

    // Scripts should be excluded
    const hasScript = changePaths.some((path: string) =>
      path.endsWith('.py') || path.endsWith('.ts') || path.endsWith('.go') || path.endsWith('.sh')
    );

    // Users should be included (CLI override)
    const hasUser = changePaths.some((path: string) => path.includes('admin@windmill.dev.user.yaml'));

    // Settings should be excluded (no CLI override, restrictive path filtering)
    const hasSettings = changePaths.some((path: string) => path === 'settings.yaml');

    expect(hasScript).toBe(false);
    expect(hasUser).toBe(true);
    expect(hasSettings).toBe(false);
  });
});
