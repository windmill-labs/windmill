import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { readConfigFile, getEffectiveSettings } from "../conf.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

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
    const yamlData = parseJsonFromCLIOutput(yamlResult.stdout);

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
    const includeData = parseJsonFromCLIOutput(includeResult.stdout);
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
    const excludeData = parseJsonFromCLIOutput(excludeResult.stdout);
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
    const data = parseJsonFromCLIOutput(result.stdout);

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

// =============================================================================
// CLI FLAG OVERRIDE TESTS
// Tests for CLI flags overriding configuration file settings
// =============================================================================

Deno.test("CLI skip flags override wmill.yaml configuration", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Set up workspace profile with name "localhost_test"
    await setupWorkspaceProfile(backend);

    // Create wmill.yaml that INCLUDES resources by default (skipResources: false)
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
  - u/**
skipResources: false
skipResourceTypes: false
includeSettings: true`);

    // Test 1: Without CLI flags - should respect wmill.yaml (include resources)
    const configResult = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    assertEquals(configResult.code, 0);

    const configData = parseJsonFromCLIOutput(configResult.stdout);


    // Should include resources (not skipped by config)
    const hasResources = (configData.changes || []).some((change: any) =>
      change.type === 'added' && change.path?.includes('.resource.yaml')
    );
    assertEquals(hasResources, true, "Resources should be included by wmill.yaml config");

    // Test 2: With CLI --skip-resources flag - should override wmill.yaml to skip resources
    const overrideResult = await backend.runCLICommand([
      'sync', 'pull', '--dry-run', '--json-output',
      '--skip-resources',  // CLI flag should override config to skip resources
      '--skip-resource-types'  // CLI flag should override config to skip resource types
    ], tempDir);
    assertEquals(overrideResult.code, 0);

    const overrideData = parseJsonFromCLIOutput(overrideResult.stdout);

    // Should NOT include resources (CLI flag overrides config)
    const hasResourcesOverride = (overrideData.changes || []).some((change: any) =>
      change.type === 'added' && change.path?.includes('.resource.yaml')
    );
    assertEquals(hasResourcesOverride, false, "CLI --skip-resources flag should override wmill.yaml to exclude resources");

    // Should NOT include resource types (CLI flag overrides config)
    const hasResourceTypesOverride = (overrideData.changes || []).some((change: any) =>
      change.type === 'added' && change.path?.includes('.resource-type.yaml')
    );
    assertEquals(hasResourceTypesOverride, false, "CLI --skip-resource-types flag should override wmill.yaml to exclude resource types");
  });
});
