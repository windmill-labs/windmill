import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { readConfigFile, getEffectiveSettings } from "../conf.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";

// =============================================================================
// SYNC CONFIGURATION RESOLUTION TESTS
// Tests for configuration resolution and integration with backend
// =============================================================================

// Helper function to set up workspace profile with localhost_test name
async function setupWorkspaceProfile(backend: any): Promise<void> {
  const testWorkspace = {
    remote: backend.baseUrl,           // "http://localhost:8001/"
    workspaceId: backend.workspace,    // "test"
    name: "localhost_test",           // This is what the tests expect!
    token: backend.token
  };

  await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });
}


// =============================================================================
// INTEGRATION TESTS WITH REAL BACKEND
// =============================================================================

Deno.test("Integration: wmill.yaml configuration produces expected results", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile with name "localhost_test"
    await setupWorkspaceProfile(backend);
    
    // Create wmill.yaml with settings
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
  - settings.yaml
excludes:
  - "*.test.ts"
skipVariables: true
skipResources: true
includeSettings: true
includeSchedules: true
includeTriggers: true`);
    
    // Test pull with wmill.yaml configuration
    const yamlResult = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    if (yamlResult.code !== 0) {
      console.log("YAML command failed!");
      console.log("Exit code:", yamlResult.code);
      console.log("Stdout:", yamlResult.stdout);
      console.log("Stderr:", yamlResult.stderr);
    }
    assertEquals(yamlResult.code, 0);
    
    // Extract JSON from CLI output (skip log messages)
    const jsonStart = yamlResult.stdout.indexOf('{');
    assert(jsonStart >= 0, "Should have JSON output");
    const jsonOutput = yamlResult.stdout.substring(jsonStart);
    const yamlData = JSON.parse(jsonOutput);
    
    // Should include settings.yaml due to includeSettings: true
    const hasSettings = (yamlData.changes || []).some((change: any) => 
      change.type === 'added' && change.path === 'settings.yaml'
    );
    assertEquals(hasSettings, true);
    
    // Should NOT include resources or variables (due to skip flags)
    const hasResources = (yamlData.changes || []).some((change: any) => 
      change.type === 'added' && change.path?.includes('.resource.yaml')
    );
    const hasVariables = (yamlData.changes || []).some((change: any) => 
      change.type === 'added' && change.path?.includes('.variable.yaml')
    );
    assertEquals(hasResources, false);
    assertEquals(hasVariables, false);
  });
});

Deno.test("Integration: settings.yaml inclusion respects includeSettings flag", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile with name "localhost_test"
    await setupWorkspaceProfile(backend);
    
    // Test 1: includeSettings: true should include settings.yaml  
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`);
    
    const includeResult = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    assertEquals(includeResult.code, 0);
    
    // Extract JSON from CLI output (skip log messages)
    const includeJsonStart = includeResult.stdout.indexOf('{');
    assert(includeJsonStart >= 0, "Should have JSON output");
    const includeJsonOutput = includeResult.stdout.substring(includeJsonStart);
    const includeData = JSON.parse(includeJsonOutput);
    const hasSettingsInclude = (includeData.changes || []).some((change: any) => 
      change.type === 'added' && change.path === 'settings.yaml'
    );
    assertEquals(hasSettingsInclude, true);
    
    // Test 2: includeSettings: false should NOT include settings.yaml
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: false`);
    
    const excludeResult = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    assertEquals(excludeResult.code, 0);
    
    // Extract JSON from CLI output (skip log messages)
    const excludeJsonStart = excludeResult.stdout.indexOf('{');
    assert(excludeJsonStart >= 0, "Should have JSON output");
    const excludeJsonOutput = excludeResult.stdout.substring(excludeJsonStart);
    const excludeData = JSON.parse(excludeJsonOutput);
    const hasSettingsExclude = (excludeData.changes || []).some((change: any) => 
      change.type === 'added' && change.path === 'settings.yaml'
    );
    assertEquals(hasSettingsExclude, false);
  });
});

Deno.test("Integration: resource/variable filtering respects skip flags", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile with name "localhost_test"
    await setupWorkspaceProfile(backend);
    
    // Test skipResources: true  
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipResources: true
skipVariables: false`);
    
    const result = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    assertEquals(result.code, 0);
    
    // Extract JSON from CLI output (skip log messages)
    const jsonStart = result.stdout.indexOf('{');
    assert(jsonStart >= 0, "Should have JSON output");
    const jsonOutput = result.stdout.substring(jsonStart);
    const data = JSON.parse(jsonOutput);
    
    // Should NOT include resources
    const hasResources = (data.changes || []).some((change: any) => 
      change.type === 'added' && change.path?.includes('.resource.yaml')
    );
    assertEquals(hasResources, false);
    
    // Should include variables (not skipped)
    const hasVariables = (data.changes || []).some((change: any) => 
      change.type === 'added' && change.path?.includes('.variable.yaml')
    );
    assertEquals(hasVariables, true);
  });
});