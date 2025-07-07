import { assertEquals, assert, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";

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

Deno.test("Multi-Instance: gitsync-settings pull with disambiguated format", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile
    await setupWorkspaceProfile(backend, "multi_instance_test");
    
    // Create wmill.yaml with disambiguated overrides for different instances
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
    
    // Pull settings - should use disambiguated format override (skipVariables: true)
    const pullResult = await backend.runCLICommand([
      'gitsync-settings', 'pull',
      '--repository', 'u/test/test_repo',
      '--diff'
    ], tempDir, "multi_instance_test");
    
    assertEquals(pullResult.code, 0);
    assertStringIncludes(pullResult.stdout, "Changes that would be made:");
    assertStringIncludes(pullResult.stdout, "skipVariables");
  });
});

Deno.test("Multi-Instance: gitsync-settings push with overrides", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile  
    await setupWorkspaceProfile(backend, "push_override_test");
    
    // Create wmill.yaml with specific settings that differ from backend defaults
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []
skipVariables: false
includeSchedules: false

overrides:
  # Override for current backend instance - set includeSchedules: true (backend default is false)
  "push_override_test:u/test/test_repo":
    includeSchedules: true
    skipVariables: true`);
    
    // Push settings - should show changes because includeSchedules differs from backend
    const pushResult = await backend.runCLICommand([
      'gitsync-settings', 'push',
      '--repository', 'u/test/test_repo',
      '--diff'
    ], tempDir, "push_override_test");
    
    assertEquals(pushResult.code, 0);
    assertStringIncludes(pushResult.stdout, "Changes that would be pushed:");
    assertStringIncludes(pushResult.stdout, "includeSchedules");
  });
});

Deno.test("Multi-Instance: workspace names with special characters", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "my-workspace_123");
    
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []

overrides:
  "my-workspace_123:u/test/test_repo":
    skipApps: true`);
    
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--repository', 'u/test/test_repo',
      '--dry-run',
      '--json-output'
    ], tempDir, "my-workspace_123");
    
    assertEquals(result.code, 0);
    
    const jsonMatch = result.stdout.match(/\{[\s\S]*\}/);
    assert(jsonMatch, `Must have JSON output in: ${result.stdout}`);
    const data = JSON.parse(jsonMatch[0]);
    
    // Test is designed to verify that workspace names with special characters work
    
    // The test app should NOT appear in changes because skipApps: true
    const hasTestApp = (data.changes || []).some((change: any) => 
      change.path?.includes('f/test_dashboard')
    );
    assertEquals(hasTestApp, false, "Test app should be skipped due to skipApps override with special character workspace name");
  });
});

Deno.test("Multi-Instance: workspace wildcards with disambiguated format", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "wildcard_test");
    
    const backendUrl = new URL(backend.baseUrl).toString();
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []

overrides:
  # Disambiguated wildcard for current backend instance
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
    
    const jsonMatch = result.stdout.match(/\{[\s\S]*\}/);
    assert(jsonMatch, `Must have JSON output in: ${result.stdout}`);
    const data = JSON.parse(jsonMatch[0]);
    
    // Variables should be skipped due to disambiguated wildcard override
    const hasTestVariable = (data.changes || []).some((change: any) => 
      change.path?.includes('u/admin/test_config.variable.yaml')
    );
    assertEquals(hasTestVariable, false, "Variables should be skipped due to disambiguated wildcard override");
  });
});