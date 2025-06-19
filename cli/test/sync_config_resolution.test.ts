import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { resolveWorkspaceAndRepositoryForSync } from "../settings_utils.ts";
import { getWorkspaceRepositorySettings, readConfigFile } from "../conf.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";

// =============================================================================
// SYNC CONFIGURATION RESOLUTION TESTS
// Tests for the bugs found in workspace/repository resolution
// =============================================================================

Deno.test("Config Resolution: resolveWorkspaceAndRepositoryForSync loads wmill.yaml automatically", async () => {
  // Create test wmill.yaml
  const testConfig = `workspaces:
  test:
    baseUrl: 'http://localhost:8000/'
    workspaceId: test
    repositories:
      u/test/repo:
        defaultTs: bun
        includes: []
        excludes: []
        skipScripts: false
        skipFlows: false
        skipApps: false
        skipFolders: false
        skipVariables: true
        skipResources: true
        skipResourceTypes: true
        skipSecrets: true
        includeSchedules: true
        includeTriggers: true
        includeUsers: false
        includeGroups: false
        includeSettings: true
        includeKey: false
defaultWorkspace: test`;

  await Deno.writeTextFile("wmill.yaml", testConfig);
  
  try {
    // Call without providing config - should load from wmill.yaml
    const result = await resolveWorkspaceAndRepositoryForSync("test", "u/test/repo");
    
    // Should load workspace and repository correctly
    assertEquals(result.workspaceName, "test");
    assertEquals(result.repositoryPath, "u/test/repo");
    
    // Should extract repository settings to top level
    assertEquals(result.syncOptions.includeSettings, true);
    assertEquals(result.syncOptions.skipResources, true);
    assertEquals(result.syncOptions.skipVariables, true);
    assertEquals(result.syncOptions.defaultTs, "bun");
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Config Resolution: resolveWorkspaceAndRepositoryForSync with undefined params uses defaultWorkspace", async () => {
  // Create test wmill.yaml with defaultWorkspace
  const testConfig = `workspaces:
  production:
    baseUrl: 'http://localhost:8000/'
    workspaceId: production
    repositories:
      u/prod/main:
        includeSettings: true
        skipResources: false
defaultWorkspace: production`;

  await Deno.writeTextFile("wmill.yaml", testConfig);
  
  try {
    // Call with undefined params - should use defaultWorkspace  
    const result = await resolveWorkspaceAndRepositoryForSync(undefined, undefined);
    
    assertEquals(result.workspaceName, "production");
    assertEquals(result.repositoryPath, "u/prod/main");
    assertEquals(result.syncOptions.includeSettings, true);
    assertEquals(result.syncOptions.skipResources, false);
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Config Resolution: getWorkspaceRepositorySettings extracts repository settings correctly", async () => {
  const config = {
    workspaces: {
      test: {
        baseUrl: 'http://localhost:8000/',
        workspaceId: 'test',
        repositories: {
          'u/user/repo': {
            includeSettings: true,
            skipResources: true,
            skipVariables: false,
            defaultTs: 'deno' as const
          }
        }
      }
    }
  };
  
  const result = getWorkspaceRepositorySettings(config, "test", "u/user/repo");
  
  // Should merge config with repository settings, with repository settings taking precedence
  assertEquals(result.includeSettings, true);
  assertEquals(result.skipResources, true); 
  assertEquals(result.skipVariables, false);
  assertEquals(result.defaultTs, 'deno');
  
  // Should still contain original config structure
  assertEquals(result.workspaces?.test?.baseUrl, 'http://localhost:8000/');
});

Deno.test("Config Resolution: getWorkspaceRepositorySettings falls back gracefully", async () => {
  const config = {
    defaultTs: 'bun' as const,
    includeSettings: false
  };
  
  // Test with non-existent workspace
  const result1 = getWorkspaceRepositorySettings(config, "nonexistent", "u/user/repo");
  assertEquals(result1, config); // Should return original config
  
  // Test with existent workspace but non-existent repository
  const configWithWorkspace = {
    workspaces: {
      test: {
        baseUrl: 'http://localhost:8000/',
        workspaceId: 'test',
        repositories: {}
      }
    },
    defaultTs: 'bun' as const
  };
  
  const result2 = getWorkspaceRepositorySettings(configWithWorkspace, "test", "nonexistent");
  assertEquals(result2, configWithWorkspace); // Should return original config
});

// =============================================================================
// INTEGRATION TESTS WITH REAL BACKEND
// =============================================================================

Deno.test("Integration: wmill.yaml vs JSON override produces identical results", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
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
    
    // Test 1: Pull with wmill.yaml only (no repository restriction)
    const yamlResult = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    if (yamlResult.code !== 0) {
      console.log("YAML command failed!");
      console.log("Exit code:", yamlResult.code);
      console.log("Stdout:", yamlResult.stdout);
      console.log("Stderr:", yamlResult.stderr);
    }
    assertEquals(yamlResult.code, 0);
    
    const yamlOutput = yamlResult.stdout.split('\n').find(line => line.trim().startsWith('{'));
    if (!yamlOutput) {
      console.log("YAML Result stdout:", yamlResult.stdout);
      console.log("YAML Result stderr:", yamlResult.stderr);
      throw new Error("No JSON output found in YAML result");
    }
    const yamlData = JSON.parse(yamlOutput);
    
    // Test 2: Pull with JSON override (same settings)  
    const jsonSettings = JSON.stringify({
      include_path: ["f/**", "settings.yaml"],
      include_type: ["script", "flow", "app", "folder", "schedule", "trigger", "settings"]
    });
    
    const jsonResult = await backend.runCLICommand([
      'sync', 'pull', '--dry-run', '--json-output',
      '--settings-from-json', jsonSettings
    ], tempDir);
    assertEquals(jsonResult.code, 0);
    
    const jsonOutput = jsonResult.stdout.split('\n').find(line => line.trim().startsWith('{'));
    if (!jsonOutput) {
      console.log("JSON Result stdout:", jsonResult.stdout);
      console.log("JSON Result stderr:", jsonResult.stderr);
      throw new Error("No JSON output found in JSON result");
    }
    const jsonData = JSON.parse(jsonOutput);
    
    // Both should produce identical results
    assertEquals(yamlData.added.length, jsonData.added.length);
    assertEquals(yamlData.added.includes('settings.yaml'), jsonData.added.includes('settings.yaml'));
    assertEquals(yamlData.added.includes('settings.yaml'), true); // Both should include settings.yaml
    
    // Neither should include resources or variables (due to skip flags)
    const hasResources = yamlData.added.some((file: string) => file.includes('.resource.yaml'));
    const hasVariables = yamlData.added.some((file: string) => file.includes('.variable.yaml'));
    assertEquals(hasResources, false);
    assertEquals(hasVariables, false);
  });
});

Deno.test("Integration: settings.yaml inclusion respects includeSettings flag", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Test 1: includeSettings: true should include settings.yaml  
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: true`);
    
    const includeResult = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    assertEquals(includeResult.code, 0);
    
    const includeOutput = includeResult.stdout.split('\n').find(line => line.trim().startsWith('{'));
    if (!includeOutput) {
      console.log("Include Result stdout:", includeResult.stdout);
      throw new Error("No JSON output found in include result");
    }
    const includeData = JSON.parse(includeOutput);
    assertEquals(includeData.added.includes('settings.yaml'), true);
    
    // Test 2: includeSettings: false should NOT include settings.yaml
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
includeSettings: false`);
    
    const excludeResult = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    assertEquals(excludeResult.code, 0);
    
    const excludeOutput = excludeResult.stdout.split('\n').find(line => line.trim().startsWith('{'));
    if (!excludeOutput) {
      console.log("Exclude Result stdout:", excludeResult.stdout);
      throw new Error("No JSON output found in exclude result");
    }
    const excludeData = JSON.parse(excludeOutput);
    assertEquals(excludeData.added.includes('settings.yaml'), false);
  });
});

Deno.test("Integration: resource/variable filtering respects skip flags", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    // Test skipResources: true  
    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipResources: true
skipVariables: false`);
    
    const result = await backend.runCLICommand(['sync', 'pull', '--dry-run', '--json-output'], tempDir);
    assertEquals(result.code, 0);
    
    const output = result.stdout.split('\n').find(line => line.trim().startsWith('{'));
    if (!output) {
      console.log("Result stdout:", result.stdout);
      throw new Error("No JSON output found in result");
    }
    const data = JSON.parse(output);
    
    // Should NOT include resources
    const hasResources = data.added.some((file: string) => file.includes('.resource.yaml'));
    assertEquals(hasResources, false);
    
    // Should include variables (not skipped)
    const hasVariables = data.added.some((file: string) => file.includes('.variable.yaml'));
    assertEquals(hasVariables, true);
  });
});