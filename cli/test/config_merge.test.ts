import { assertEquals, assertNotEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { getEffectiveSettings, DEFAULT_SYNC_OPTIONS } from "../conf.ts";
import type { SyncOptions } from "../conf.ts";

// =============================================================================
// CONFIGURATION MERGING TESTS - NEW OVERRIDE SYSTEM
// Tests for getEffectiveSettings() and hierarchical override resolution
// =============================================================================

Deno.test("Config Merge: getEffectiveSettings - defaults only", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    skipVariables: true,
  };

  const result = getEffectiveSettings(config, "test-workspace", "u/user/repo");

  // Should merge defaults with config
  assertEquals(result.defaultTs, "bun");
  assertEquals(result.includes, ["f/**"]);
  assertEquals(result.skipVariables, true);
  assertEquals(result.skipSecrets, false); // From DEFAULT_SYNC_OPTIONS
});

Deno.test("Config Merge: getEffectiveSettings - workspace-level overrides", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    skipVariables: true,
    overrides: {
      "test-workspace:*": {
        defaultTs: "deno",
        skipSecrets: false,
        includeSchedules: true,
      }
    }
  };

  const result = getEffectiveSettings(config, "test-workspace", "u/user/repo");

  // Workspace-level overrides should take precedence
  assertEquals(result.defaultTs, "deno"); // Override
  assertEquals(result.includes, ["f/**"]); // From base config
  assertEquals(result.skipVariables, true); // From base config
  assertEquals(result.skipSecrets, false); // Override
  assertEquals(result.includeSchedules, true); // Override
});

Deno.test("Config Merge: getEffectiveSettings - repository-specific overrides", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    skipVariables: true,
    overrides: {
      "test-workspace:*": {
        defaultTs: "deno",
        skipSecrets: false,
      },
      "test-workspace:u/user/repo": {
        defaultTs: "bun", // Override workspace setting back to bun
        includeSchedules: true,
        includes: ["u/user/**"], // Override includes
      }
    }
  };

  const result = getEffectiveSettings(config, "test-workspace", "u/user/repo");

  // Repository-specific overrides should take highest precedence
  assertEquals(result.defaultTs, "bun"); // Repository override wins over workspace
  assertEquals(result.includes, ["u/user/**"]); // Repository override
  assertEquals(result.skipVariables, true); // From base config
  assertEquals(result.skipSecrets, false); // From workspace override
  assertEquals(result.includeSchedules, true); // From repository override
});

Deno.test("Config Merge: getEffectiveSettings - no matching overrides", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    overrides: {
      "other-workspace:*": {
        defaultTs: "deno",
      },
      "test-workspace:u/other/repo": {
        includeSchedules: true,
      }
    }
  };

  const result = getEffectiveSettings(config, "test-workspace", "u/user/repo");

  // Should only use base config and defaults (no matching overrides)
  assertEquals(result.defaultTs, "bun");
  assertEquals(result.includes, ["f/**"]);
  assertEquals(result.includeSchedules, false); // Default
});

Deno.test("Config Merge: getEffectiveSettings - hierarchy priority", () => {
  const config: SyncOptions = {
    // Base level
    defaultTs: "bun",
    skipVariables: true,
    skipSecrets: true,
    includeSchedules: false,
    includeUsers: false,
    
    overrides: {
      // Workspace level - should override base
      "prod:*": {
        defaultTs: "deno",
        skipSecrets: false,
        includeSchedules: true,
      },
      // Repository level - should override both base and workspace
      "prod:u/team/backend": {
        skipVariables: false, // Override base
        includeUsers: true,   // Override base
        includeSchedules: false, // Override workspace setting
      }
    }
  };

  const result = getEffectiveSettings(config, "prod", "u/team/backend");

  // Verify priority: repository > workspace > base > defaults
  assertEquals(result.defaultTs, "deno");        // From workspace (no repo override)
  assertEquals(result.skipVariables, false);    // From repository override
  assertEquals(result.skipSecrets, false);      // From workspace override  
  assertEquals(result.includeSchedules, false); // From repository override (overrides workspace)
  assertEquals(result.includeUsers, true);      // From repository override
});

Deno.test("Config Merge: getEffectiveSettings - complex nested arrays and objects", () => {
  const config: SyncOptions = {
    includes: ["f/**"],
    excludes: ["*.test.ts"],
    codebases: [{ relative_path: "frontend" }],
    
    overrides: {
      "dev:*": {
        includes: ["f/**", "dev/**"],
        excludes: ["*.test.ts", "*.spec.ts"],
      },
      "dev:u/alice/project": {
        includes: ["alice/**"], // Complete override, not merge
        codebases: [{ relative_path: "alice-frontend" }],
      }
    }
  };

  const result = getEffectiveSettings(config, "dev", "u/alice/project");

  // Arrays should be completely replaced, not merged
  assertEquals(result.includes, ["alice/**"]); // Repository override replaces workspace override
  assertEquals(result.excludes, ["*.test.ts", "*.spec.ts"]); // From workspace override
  assertEquals(result.codebases?.[0]?.relative_path, "alice-frontend"); // Repository override
});

Deno.test("Config Merge: getEffectiveSettings - empty overrides", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    overrides: {}
  };

  const result = getEffectiveSettings(config, "test", "u/user/repo");

  // Should work normally with empty overrides
  assertEquals(result.defaultTs, "bun");
  assertEquals(result.includes, ["f/**"]);
  assertEquals(result.skipSecrets, false); // From defaults
});

Deno.test("Config Merge: getEffectiveSettings - undefined overrides", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    // overrides field not present
  };

  const result = getEffectiveSettings(config, "test", "u/user/repo");

  // Should work normally without overrides field
  assertEquals(result.defaultTs, "bun");
  assertEquals(result.includes, ["f/**"]);
  assertEquals(result.skipSecrets, false); // From defaults
});

// =============================================================================
// EDGE CASES AND ERROR CONDITIONS
// =============================================================================

Deno.test("Config Merge: getEffectiveSettings - malformed override keys", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    overrides: {
      "no-colon-key": { defaultTs: "deno" },
      "too:many:colons:here": { skipVariables: false },
      ":missing-workspace": { includeSchedules: true },
      "workspace:": { includeUsers: true },
    }
  };

  const result = getEffectiveSettings(config, "test", "u/user/repo");

  // Should ignore malformed keys and only use base config
  assertEquals(result.defaultTs, "bun");
  assertEquals(result.skipVariables, false); // Default should remain
  assertEquals(result.includeSchedules, false); // Default should remain
});

Deno.test("Config Merge: getEffectiveSettings - wildcard repository matching", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    overrides: {
      "test:*": {
        defaultTs: "deno",
        skipSecrets: false,
      }
    }
  };

  // Should match any repository in the workspace
  const result1 = getEffectiveSettings(config, "test", "u/alice/repo1");
  const result2 = getEffectiveSettings(config, "test", "u/bob/repo2");
  const result3 = getEffectiveSettings(config, "test", "f/shared");

  assertEquals(result1.defaultTs, "deno");
  assertEquals(result1.skipSecrets, false);
  assertEquals(result2.defaultTs, "deno");
  assertEquals(result2.skipSecrets, false);
  assertEquals(result3.defaultTs, "deno");
  assertEquals(result3.skipSecrets, false);
});

Deno.test("Config Merge: getEffectiveSettings - case sensitivity", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    overrides: {
      "Test:u/User/Repo": { defaultTs: "deno" },
      "test:u/user/repo": { skipSecrets: false },
    }
  };

  // Should be case-sensitive
  const result1 = getEffectiveSettings(config, "Test", "u/User/Repo");
  const result2 = getEffectiveSettings(config, "test", "u/user/repo");
  const result3 = getEffectiveSettings(config, "test", "u/User/Repo");

  assertEquals(result1.defaultTs, "deno");
  assertEquals(result1.skipSecrets, false); // Default, no matching override

  assertEquals(result2.defaultTs, "bun");
  assertEquals(result2.skipSecrets, false);

  assertEquals(result3.defaultTs, "bun");
  assertEquals(result3.skipSecrets, false); // Default, no matching override
});

// =============================================================================
// PERFORMANCE TESTS
// =============================================================================

Deno.test("Config Merge: Performance - large number of overrides", () => {
  const overrides: { [key: string]: Partial<SyncOptions> } = {};
  
  // Create 1000 override entries
  for (let i = 0; i < 1000; i++) {
    overrides[`workspace${i}:u/user/repo${i}`] = {
      defaultTs: i % 2 === 0 ? "bun" : "deno",
      skipVariables: i % 3 === 0,
    };
  }

  const config: SyncOptions = {
    defaultTs: "bun",
    overrides
  };

  const start = performance.now();
  
  // Test lookup performance
  for (let i = 0; i < 100; i++) {
    getEffectiveSettings(config, `workspace${i}`, `u/user/repo${i}`);
  }
  
  const end = performance.now();

  // Should complete 100 lookups in less than 50ms even with 1000 overrides
  assertEquals(end - start < 50, true);
});

Deno.test("Config Merge: Performance - deep override nesting", () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    excludes: [],
    codebases: [],
    skipVariables: true,
    skipResources: true,
    skipResourceTypes: true,
    skipSecrets: true,
    skipScripts: false,
    skipFlows: false,
    skipApps: false,
    skipFolders: false,
    includeSchedules: false,
    includeTriggers: false,
    includeUsers: false,
    includeGroups: false,
    includeSettings: false,
    includeKey: false,
    
    overrides: {
      "test:*": {
        defaultTs: "deno",
        includes: ["f/**", "shared/**"],
        skipSecrets: false,
        includeSchedules: true,
      },
      "test:u/user/repo": {
        includes: ["u/user/**"],
        skipVariables: false,
        includeUsers: true,
      }
    }
  };

  const start = performance.now();
  
  // Test performance with complex config merging
  for (let i = 0; i < 1000; i++) {
    getEffectiveSettings(config, "test", "u/user/repo");
  }
  
  const end = performance.now();

  // Should complete 1000 merge operations in less than 100ms
  assertEquals(end - start < 100, true);
});