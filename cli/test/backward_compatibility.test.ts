import { assertEquals, assertNotEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { readConfigFile, getEffectiveSettings } from "../conf.ts";
import type { SyncOptions } from "../conf.ts";

// =============================================================================
// BACKWARD COMPATIBILITY TESTS
// Tests to ensure existing wmill.yaml formats continue to work
// =============================================================================

Deno.test("Backward Compatibility: legacy format - simple config", async () => {
  const legacyConfig = `defaultTs: bun
includes:
  - f/**
  - u/shared/**
excludes:
  - "*.test.ts"
skipVariables: true
skipResources: true
skipSecrets: true
includeSchedules: true
includeTriggers: false`;

  await Deno.writeTextFile("wmill.yaml", legacyConfig);
  
  try {
    const config = await readConfigFile();
    
    // Should read legacy format correctly
    assertEquals(config.defaultTs, "bun");
    assertEquals(config.includes, ["f/**", "u/shared/**"]);
    assertEquals(config.excludes, ["*.test.ts"]);
    assertEquals(config.skipVariables, true);
    assertEquals(config.skipResources, true);
    assertEquals(config.skipSecrets, true);
    assertEquals(config.includeSchedules, true);
    assertEquals(config.includeTriggers, false);
    
    // Should work with getEffectiveSettings
    const effective = getEffectiveSettings(config, "test", "u/user/repo");
    assertEquals(effective.defaultTs, "bun");
    assertEquals(effective.includes, ["f/**", "u/shared/**"]);
    assertEquals(effective.skipVariables, true);
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Backward Compatibility: legacy format - minimal config", async () => {
  const minimalConfig = `defaultTs: deno`;

  await Deno.writeTextFile("wmill.yaml", minimalConfig);
  
  try {
    const config = await readConfigFile();
    
    // Should read minimal config and apply defaults
    assertEquals(config.defaultTs, "deno");
    assertEquals(config.includes, undefined); // Not specified
    
    // getEffectiveSettings should apply defaults
    const effective = getEffectiveSettings(config, "test", "u/user/repo");
    assertEquals(effective.defaultTs, "deno");
    assertEquals(effective.skipSecrets, false); // From DEFAULT_SYNC_OPTIONS
    assertEquals(effective.includeSchedules, false); // From DEFAULT_SYNC_OPTIONS
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Backward Compatibility: legacy format with all options", async () => {
  const fullLegacyConfig = `defaultTs: bun
includes:
  - f/**
  - u/**
excludes:
  - "**/.git/**"
  - "*.log"
skipVariables: false
skipResources: false
skipResourceTypes: true
skipSecrets: false
skipScripts: false
skipFlows: false
skipApps: false
skipFolders: false
includeSchedules: true
includeTriggers: true
includeUsers: true
includeGroups: false
includeSettings: true
includeKey: false
codebases:
  - relative_path: frontend
    includes:
      - src/**
    excludes:
      - "*.test.ts"`;

  await Deno.writeTextFile("wmill.yaml", fullLegacyConfig);
  
  try {
    const config = await readConfigFile();
    
    // Should preserve all settings
    assertEquals(config.defaultTs, "bun");
    assertEquals(config.includes, ["f/**", "u/**"]);
    assertEquals(config.excludes, ["**/.git/**", "*.log"]);
    assertEquals(config.skipVariables, false);
    assertEquals(config.skipResources, false);
    assertEquals(config.skipResourceTypes, true);
    assertEquals(config.skipSecrets, false);
    assertEquals(config.includeSchedules, true);
    assertEquals(config.includeTriggers, true);
    assertEquals(config.includeUsers, true);
    assertEquals(config.includeGroups, false);
    assertEquals(config.includeSettings, true);
    assertEquals(config.includeKey, false);
    assertEquals(config.codebases?.[0]?.relative_path, "frontend");
    assertEquals(config.codebases?.[0]?.includes, ["src/**"]);
    
    // getEffectiveSettings should preserve all settings
    const effective = getEffectiveSettings(config, "test", "u/user/repo");
    assertEquals(effective.defaultTs, "bun");
    assertEquals(effective.skipVariables, false);
    assertEquals(effective.includeUsers, true);
    assertEquals(effective.codebases?.[0]?.relative_path, "frontend");
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Backward Compatibility: new format with overrides works alongside legacy", async () => {
  const newFormatConfig = `# Top-level defaults (legacy-compatible)
defaultTs: bun
includes:
  - f/**
skipVariables: true
skipSecrets: true

# New override system
overrides:
  dev:*:
    defaultTs: deno
    skipSecrets: false
  dev:u/alice/special:
    includes:
      - alice/**
    skipVariables: false`;

  await Deno.writeTextFile("wmill.yaml", newFormatConfig);
  
  try {
    const config = await readConfigFile();
    
    // Should read both legacy and new format parts
    assertEquals(config.defaultTs, "bun");
    assertEquals(config.includes, ["f/**"]);
    assertEquals(config.skipVariables, true);
    assertEquals(config.overrides !== undefined, true);
    assertEquals(config.overrides!["dev:*"]?.defaultTs, "deno");
    assertEquals(config.overrides!["dev:*"]?.skipSecrets, false);
    
    // getEffectiveSettings should apply overrides correctly
    const effective1 = getEffectiveSettings(config, "dev", "u/bob/project");
    assertEquals(effective1.defaultTs, "deno"); // Workspace override
    assertEquals(effective1.skipSecrets, false); // Workspace override
    assertEquals(effective1.skipVariables, true); // Base config
    assertEquals(effective1.includes, ["f/**"]); // Base config
    
    const effective2 = getEffectiveSettings(config, "dev", "u/alice/special");
    assertEquals(effective2.defaultTs, "deno"); // Workspace override
    assertEquals(effective2.skipSecrets, false); // Workspace override
    assertEquals(effective2.skipVariables, false); // Repository override
    assertEquals(effective2.includes, ["alice/**"]); // Repository override
    
    const effective3 = getEffectiveSettings(config, "prod", "u/team/main");
    assertEquals(effective3.defaultTs, "bun"); // Base config (no overrides)
    assertEquals(effective3.skipSecrets, true); // Base config
    assertEquals(effective3.skipVariables, true); // Base config
    assertEquals(effective3.includes, ["f/**"]); // Base config
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Backward Compatibility: empty config file", async () => {
  const emptyConfig = `# Just comments
# No actual configuration`;

  await Deno.writeTextFile("wmill.yaml", emptyConfig);
  
  try {
    const config = await readConfigFile();
    
    // Should handle empty config gracefully (may return null for empty YAML)
    assertEquals(config?.defaultTs, undefined);
    assertEquals(config?.includes, undefined);
    
    // getEffectiveSettings should apply all defaults
    const effective = getEffectiveSettings(config || {}, "test", "u/user/repo");
    assertEquals(effective.defaultTs, "bun"); // From DEFAULT_SYNC_OPTIONS
    assertEquals(effective.skipSecrets, false); // From DEFAULT_SYNC_OPTIONS
    assertEquals(effective.includeSchedules, false); // From DEFAULT_SYNC_OPTIONS
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Backward Compatibility: config with only new fields", async () => {
  const newFieldsConfig = `skipScripts: true
skipFlows: true
skipApps: false
skipFolders: false
jsonOutput: true

overrides:
  test:u/user/repo:
    skipScripts: false
    jsonOutput: false`;

  await Deno.writeTextFile("wmill.yaml", newFieldsConfig);
  
  try {
    const config = await readConfigFile();
    
    // Should read new fields
    assertEquals(config.skipScripts, true);
    assertEquals(config.skipFlows, true);
    assertEquals(config.skipApps, false);
    assertEquals(config.skipFolders, false);
    assertEquals(config.jsonOutput, true);
    
    // getEffectiveSettings should handle new fields in overrides
    const effective = getEffectiveSettings(config, "test", "u/user/repo");
    assertEquals(effective.skipScripts, false); // Repository override
    assertEquals(effective.skipFlows, true); // Base config
    assertEquals(effective.jsonOutput, false); // Repository override
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

// =============================================================================
// MIGRATION PATH TESTS
// =============================================================================

Deno.test("Backward Compatibility: migration from simple to override-based", async () => {
  // Start with a simple legacy config
  const legacyConfig = `defaultTs: bun
includes:
  - f/**
skipVariables: true`;

  await Deno.writeTextFile("wmill.yaml", legacyConfig);
  
  try {
    // Read legacy config
    const config1 = await readConfigFile();
    const effective1 = getEffectiveSettings(config1, "dev", "u/user/repo");
    
    // Now "migrate" by adding overrides
    const migratedConfig = `defaultTs: bun
includes:
  - f/**
skipVariables: true

# Added overrides for different environments
overrides:
  dev:*:
    skipSecrets: false
    includeSchedules: true
  prod:*:
    skipVariables: true
    skipSecrets: true
    includeSchedules: false`;

    await Deno.writeTextFile("wmill.yaml", migratedConfig);
    
    // Read migrated config
    const config2 = await readConfigFile();
    const effective2Dev = getEffectiveSettings(config2, "dev", "u/user/repo");
    const effective2Prod = getEffectiveSettings(config2, "prod", "u/user/repo");
    
    // Original behavior should be preserved for other workspaces
    const effective2Other = getEffectiveSettings(config2, "other", "u/user/repo");
    assertEquals(effective2Other.defaultTs, effective1.defaultTs);
    assertEquals(effective2Other.skipVariables, effective1.skipVariables);
    assertEquals(effective2Other.includes, effective1.includes);
    
    // New overrides should work
    assertEquals(effective2Dev.skipSecrets, false);
    assertEquals(effective2Dev.includeSchedules, true);
    assertEquals(effective2Prod.skipVariables, true);
    assertEquals(effective2Prod.includeSchedules, false);
    
  } finally {
    await Deno.remove("wmill.yaml");
  }
});

Deno.test("Backward Compatibility: performance with mixed legacy and new format", () => {
  const mixedConfig: SyncOptions = {
    // Legacy format fields
    defaultTs: "bun",
    includes: ["f/**"],
    excludes: ["*.test.ts"],
    skipVariables: true,
    skipResources: true,
    includeSchedules: false,
    
    // New format fields
    skipScripts: false,
    skipFlows: false,
    skipApps: false,
    skipFolders: false,
    jsonOutput: true,
    
    // Override system
    overrides: {
      "dev:*": {
        skipSecrets: false,
        includeSchedules: true,
      },
      "prod:*": {
        skipVariables: true,
        skipSecrets: true,
      }
    }
  };

  const start = performance.now();
  
  // Test performance with mixed format
  for (let i = 0; i < 1000; i++) {
    getEffectiveSettings(mixedConfig, "dev", "u/user/repo");
    getEffectiveSettings(mixedConfig, "prod", "u/user/repo");
    getEffectiveSettings(mixedConfig, "test", "u/user/repo");
  }
  
  const end = performance.now();

  // Should handle mixed format efficiently
  assertEquals(end - start < 100, true);
});

// =============================================================================
// REAL-WORLD COMPATIBILITY SCENARIOS
// =============================================================================

Deno.test("Backward Compatibility: typical user config evolution", async () => {
  // Simulate how a user's config might evolve over time
  
  // Stage 1: Brand new user, minimal config
  const stage1 = `defaultTs: bun`;
  await Deno.writeTextFile("wmill.yaml", stage1);
  
  const config1 = await readConfigFile();
  const effective1 = getEffectiveSettings(config1, "main", "u/user/scripts");
  assertEquals(effective1.defaultTs, "bun");
  
  // Stage 2: User adds some basic settings  
  const stage2 = `defaultTs: bun
includes:
  - f/**
  - u/user/**
skipVariables: true
skipSecrets: true`;
  await Deno.writeTextFile("wmill.yaml", stage2);
  
  const config2 = await readConfigFile();
  const effective2 = getEffectiveSettings(config2, "main", "u/user/scripts");
  assertEquals(effective2.defaultTs, "bun");
  assertEquals(effective2.includes, ["f/**", "u/user/**"]);
  assertEquals(effective2.skipVariables, true);
  
  // Stage 3: User starts working with multiple workspaces/repos
  const stage3 = `defaultTs: bun
includes:
  - f/**
  - u/user/**
skipVariables: true
skipSecrets: true

# Added workspace-specific overrides
overrides:
  dev:*:
    skipSecrets: false
    includeSchedules: true
  
  staging:*:
    skipVariables: false
    includeSchedules: false
    
  prod:u/user/critical:
    skipVariables: true
    skipSecrets: true
    includes:
      - critical/**`;
  await Deno.writeTextFile("wmill.yaml", stage3);
  
  const config3 = await readConfigFile();
  
  // Original workspace should still work the same
  const effective3Main = getEffectiveSettings(config3, "main", "u/user/scripts");
  assertEquals(effective3Main.defaultTs, effective2.defaultTs);
  assertEquals(effective3Main.skipVariables, effective2.skipVariables);
  
  // New workspaces should use overrides
  const effective3Dev = getEffectiveSettings(config3, "dev", "u/user/scripts");
  assertEquals(effective3Dev.skipSecrets, false);
  assertEquals(effective3Dev.includeSchedules, true);
  
  const effective3ProdCritical = getEffectiveSettings(config3, "prod", "u/user/critical");
  assertEquals(effective3ProdCritical.includes, ["critical/**"]);
  assertEquals(effective3ProdCritical.skipVariables, true);
  
  await Deno.remove("wmill.yaml");
});