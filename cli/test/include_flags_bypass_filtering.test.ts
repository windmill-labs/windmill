import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
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

Deno.test("CLI include flags bypass restrictive path filtering", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);
    
    // Create wmill.yaml with very restrictive includes that would exclude special files
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "f/**"
excludes: []
skipVariables: true
skipResources: true
includeUsers: false
includeGroups: false
includeSettings: false
includeKey: false`);
    
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
    
    assertEquals(result.code, 0, `Command failed: ${result.stderr}`);
    
    const output = parseJsonFromCLIOutput(result.stdout);
    const changePaths = output.changes.map((c: any) => c.path);
    
    // Assert that special files are included despite restrictive path filtering
    const hasUser = changePaths.some((path: string) => path.includes('admin@windmill.dev.user.yaml'));
    const hasGroup = changePaths.some((path: string) => path.includes('groups/test_group.group.yaml'));
    const hasSettings = changePaths.some((path: string) => path === 'settings.yaml');
    const hasEncryptionKey = changePaths.some((path: string) => path === 'encryption_key.yaml');
    
    assert(hasUser, `Admin user should be included despite restrictive includes. Found paths: ${changePaths.join(', ')}`);
    assert(hasGroup, `'test_group' should be included despite restrictive includes. Found paths: ${changePaths.join(', ')}`);
    assert(hasSettings, `Settings should be included despite restrictive includes. Found paths: ${changePaths.join(', ')}`);
    assert(hasEncryptionKey, `Encryption key should be included despite restrictive includes. Found paths: ${changePaths.join(', ')}`);
  });
});

Deno.test("CLI flags override wmill.yaml include settings", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);
    
    // Config explicitly disables includes, but CLI should override
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []
includeUsers: false
includeGroups: false`);
    
    // CLI flags should override config file settings
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--include-users',
      '--include-groups', 
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    assertEquals(result.code, 0, `Command failed: ${result.stderr}`);
    
    const output = parseJsonFromCLIOutput(result.stdout);
    const changePaths = output.changes.map((c: any) => c.path);
    
    const hasUser = changePaths.some((path: string) => path.includes('admin@windmill.dev.user.yaml'));
    const hasGroup = changePaths.some((path: string) => path.includes('groups/test_group.group.yaml'));
    
    assert(hasUser, `CLI --include-users should override config includeUsers: false. Found paths: ${changePaths.join(', ')}`);
    assert(hasGroup, `CLI --include-groups should override config includeGroups: false. Found paths: ${changePaths.join(', ')}`);
  });
});

Deno.test("Skip flags work correctly with getTypeStrFromPath and lock files", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);
    
    // Create wmill.yaml with skip flags enabled
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []
skipScripts: true
skipFlows: false
includeUsers: true`);
    
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    assertEquals(result.code, 0, `Command failed: ${result.stderr}`);
    
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
    
    assert(!hasScript, `Standalone scripts should be skipped when skipScripts: true. Found paths: ${changePaths.join(', ')}`);
    assert(!hasScriptLock, `Script lock files should be skipped when skipScripts: true. Found paths: ${changePaths.join(', ')}`);
    assert(hasApp, `Apps should be included (inline scripts are part of apps). Found paths: ${changePaths.join(', ')}`);
    assert(hasUser, `Users should be included when includeUsers: true. Found paths: ${changePaths.join(', ')}`);
  });
});

Deno.test("Mixed include and skip flags work together", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend);
    
    // Create restrictive config with mixed settings
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "f/**"
excludes: []
skipScripts: true
includeUsers: false
includeSettings: false`);
    
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--skip-scripts',      // Reinforce script skipping
      '--include-users',     // Override config to include users
      '--dry-run',
      '--json-output'
    ], tempDir);
    
    assertEquals(result.code, 0, `Command failed: ${result.stderr}`);
    
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
    
    assert(!hasScript, `Scripts should be excluded due to skipScripts. Found paths: ${changePaths.join(', ')}`);
    assert(hasUser, `Users should be included due to CLI --include-users override. Found paths: ${changePaths.join(', ')}`);
    assert(!hasSettings, `Settings should be excluded (no CLI override + restrictive paths). Found paths: ${changePaths.join(', ')}`);
  });
});