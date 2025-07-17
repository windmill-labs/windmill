import { assertEquals, assert, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

// =============================================================================
// MULTI-INSTANCE WORKSPACE TESTS
// Tests for handling multiple Windmill instances with same workspace IDs
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

Deno.test("Multi-Instance: gitsync-settings pull with new format", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile
    await setupWorkspaceProfile(backend, "multi_instance_test");

    // Create wmill.yaml with new format overrides for different instances
    const backendUrl = new URL(backend.baseUrl).toString(); // Normalize URL
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Current backend instance (should match)
  "${backendUrl}:${backend.workspace}:u/test/test_repo":
    includeTriggers: true
    includeSchedules: true
    skipVariables: true

  # Different instance (won't match)
  "https://app.windmill.dev/:${backend.workspace}:u/test/test_repo":
    includeTriggers: false
    includeSchedules: false
    skipVariables: false`);

    // Pull settings - should use the matching instance override (skipVariables: true)
    const pullResult = await backend.runCLICommand([
      'gitsync-settings', 'pull',
      '--repository', 'u/test/test_repo',
      '--diff'
    ], tempDir, "multi_instance_test");

    assertEquals(pullResult.code, 0);
    assertStringIncludes(pullResult.stdout, "Changes that would be applied locally:");
    // includeSchedules should show as a change since backend default is false
    assertStringIncludes(pullResult.stdout, "includeSchedules");
  });
});

Deno.test("Multi-Instance: gitsync-settings push with overrides", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile
    await setupWorkspaceProfile(backend, "push_override_test");

    // Create wmill.yaml with specific settings that differ from backend defaults
    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
skipVariables: false
includeSchedules: false

overrides:
  # Override for current backend instance - set includeSchedules: true (backend default is false)
  "${backendUrl}:${backend.workspace}:u/test/test_repo":
    includeSchedules: true
    skipVariables: true`);

    // Push settings - should show changes because includeSchedules differs from backend
    const pushResult = await backend.runCLICommand([
      'gitsync-settings', 'push',
      '--repository', 'u/test/test_repo',
      '--diff'
    ], tempDir, "push_override_test");

    assertEquals(pushResult.code, 0);
    assertStringIncludes(pushResult.stdout, "Changes that would be pushed to Windmill:");
    assertStringIncludes(pushResult.stdout, "includeSchedules");
  });
});

Deno.test("Multi-Instance: sync with repository-specific overrides", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "my-workspace_123");

    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  "${backendUrl}:${backend.workspace}:u/test/test_repo":
    skipApps: true`);

    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--dry-run',
      '--json-output'
    ], tempDir, "my-workspace_123");

    assertEquals(result.code, 0);

    const data = parseJsonFromCLIOutput(result.stdout);

    // Test is designed to verify that the new format works correctly

    // The test app should NOT appear in changes because skipApps: true
    const hasTestApp = (data.changes || []).some((change: any) =>
      change.path?.includes('f/test_dashboard')
    );
    assertEquals(hasTestApp, false, "Test app should be skipped due to skipApps override");
  });
});

Deno.test("Multi-Instance: auto-detection of single repository override", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "auto_detect_test");

    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  # Single repository override - should be auto-detected
  "${backendUrl}:${backend.workspace}:u/test/test_repo":
    skipApps: true
    includeSchedules: true`);

    // Don't specify --repository, it should auto-detect
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--dry-run',
      '--json-output'
    ], tempDir, "auto_detect_test");

    assertEquals(result.code, 0);
    assertStringIncludes(result.stdout, "Auto-selected repository: u/test/test_repo");

    const data = parseJsonFromCLIOutput(result.stdout);

    // The test app should NOT appear because of auto-detected skipApps: true
    const hasTestApp = (data.changes || []).some((change: any) =>
      change.path?.includes('f/test_dashboard')
    );
    assertEquals(hasTestApp, false, "Test app should be skipped due to auto-detected override");
  });
});

Deno.test("Multi-Instance: workspace wildcards with new format", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "wildcard_test");

    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []

overrides:
  # Wildcard for current backend instance
  "${backendUrl}:${backend.workspace}:*":
    skipVariables: true
    skipResources: true`);

    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--dry-run',
      '--json-output'
    ], tempDir, "wildcard_test");

    assertEquals(result.code, 0);

    const data = parseJsonFromCLIOutput(result.stdout);

    // Variables should be skipped due to wildcard override
    const hasTestVariable = (data.changes || []).some((change: any) =>
      change.path?.includes('u/admin/test_config.variable.yaml')
    );
    assertEquals(hasTestVariable, false, "Variables should be skipped due to wildcard override");
  });
});
