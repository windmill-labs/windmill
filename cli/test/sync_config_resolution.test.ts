import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { readConfigFile, getEffectiveSettings } from "../conf.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";

// =============================================================================
// SYNC CONFIGURATION RESOLUTION TESTS
// Tests for configuration resolution and integration with backend
// =============================================================================

Deno.test("Config Resolution: getEffectiveSettings applies overrides correctly", async () => {
  const config = {
    defaultTs: 'bun' as const,
    includeSettings: false,
    skipResources: false,
    skipVariables: true,
    
    overrides: {
      'test:u/user/repo': {
        includeSettings: true,
        skipResources: true,
        skipVariables: false,
        defaultTs: 'deno' as const
      }
    }
  };
  
  const result = getEffectiveSettings(config, "test", "u/user/repo");
  
  // Should merge config with repository overrides, with overrides taking precedence
  assertEquals(result.includeSettings, true);   // Override
  assertEquals(result.skipResources, true);     // Override
  assertEquals(result.skipVariables, false);    // Override
  assertEquals(result.defaultTs, 'deno');       // Override
});

Deno.test("Config Resolution: getEffectiveSettings falls back gracefully", async () => {
  const config = {
    defaultTs: 'bun' as const,
    includeSettings: false,
    skipVariables: true
  };
  
  // Test with non-existent workspace/repo combination (no overrides)
  const result1 = getEffectiveSettings(config, "nonexistent", "u/user/repo");
  
  // Should return base config merged with defaults
  assertEquals(result1.defaultTs, 'bun');
  assertEquals(result1.includeSettings, false);
  assertEquals(result1.skipVariables, true);
  assertEquals(result1.skipSecrets, true); // From DEFAULT_SYNC_OPTIONS
  
  // Test with config that has overrides but no matching ones
  const configWithOverrides = {
    defaultTs: 'bun' as const,
    includeSettings: false,
    overrides: {
      'other:u/other/repo': {
        defaultTs: 'deno' as const
      }
    }
  };
  
  const result2 = getEffectiveSettings(configWithOverrides, "test", "u/user/repo");
  
  // Should ignore non-matching overrides
  assertEquals(result2.defaultTs, 'bun');
  assertEquals(result2.includeSettings, false);
});

// =============================================================================
// INTEGRATION TESTS WITH REAL BACKEND
// =============================================================================

Deno.test("Integration: wmill.yaml configuration produces expected results", async () => {
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
    
    // Test pull with wmill.yaml configuration
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
    
    // Should include settings.yaml due to includeSettings: true
    assertEquals(yamlData.added.includes('settings.yaml'), true);
    
    // Should NOT include resources or variables (due to skip flags)
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