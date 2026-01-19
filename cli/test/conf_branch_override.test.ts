import { assertEquals, assertExists } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { getEffectiveSettings, type SyncOptions } from "../src/core/conf.ts";

// =============================================================================
// CONF.TS BRANCH OVERRIDE TESTS
// Tests for getEffectiveSettings with branchOverride parameter
// =============================================================================

Deno.test("getEffectiveSettings: applies branch overrides when branchOverride is provided", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    gitBranches: {
      staging: {
        overrides: {
          includes: ["staging/**"],
          skipVariables: true,
        },
      },
      production: {
        overrides: {
          includes: ["prod/**"],
          skipSecrets: true,
        },
      },
    },
  };

  // Test with staging branch override
  const stagingSettings = await getEffectiveSettings(config, undefined, true, true, "staging");
  assertEquals(stagingSettings.includes, ["staging/**"]);
  assertEquals(stagingSettings.skipVariables, true);
  assertEquals(stagingSettings.skipSecrets, undefined);

  // Test with production branch override
  const prodSettings = await getEffectiveSettings(config, undefined, true, true, "production");
  assertEquals(prodSettings.includes, ["prod/**"]);
  assertEquals(prodSettings.skipSecrets, true);
  assertEquals(prodSettings.skipVariables, undefined);
});

Deno.test("getEffectiveSettings: uses top-level settings when branchOverride has no overrides", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    skipVariables: true,
    gitBranches: {
      staging: {
        // No overrides defined
      },
    },
  };

  const settings = await getEffectiveSettings(config, undefined, true, true, "staging");
  assertEquals(settings.includes, ["f/**"]);
  assertEquals(settings.skipVariables, true);
  assertEquals(settings.defaultTs, "bun");
});

Deno.test("getEffectiveSettings: uses top-level settings for unknown branch", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    gitBranches: {
      staging: {
        overrides: {
          includes: ["staging/**"],
        },
      },
    },
  };

  const settings = await getEffectiveSettings(config, undefined, true, true, "nonexistent");
  assertEquals(settings.includes, ["f/**"]);
  assertEquals(settings.defaultTs, "bun");
});

Deno.test("getEffectiveSettings: promotionOverrides take precedence when promotion specified", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    gitBranches: {
      production: {
        overrides: {
          includes: ["prod/**"],
        },
        promotionOverrides: {
          includes: ["promoted/**"],
          skipVariables: true,
        },
      },
    },
  };

  // Test without promotion flag - should use regular overrides
  const normalSettings = await getEffectiveSettings(config, undefined, true, true, "production");
  assertEquals(normalSettings.includes, ["prod/**"]);
  assertEquals(normalSettings.skipVariables, undefined);

  // Test with promotion flag - should use promotionOverrides
  const promoSettings = await getEffectiveSettings(config, "production", true, true);
  assertEquals(promoSettings.includes, ["promoted/**"]);
  assertEquals(promoSettings.skipVariables, true);
});

Deno.test("getEffectiveSettings: branchOverride works without gitBranches config", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
  };

  // Should not throw even with branchOverride but no gitBranches
  const settings = await getEffectiveSettings(config, undefined, true, true, "staging");
  assertEquals(settings.includes, ["f/**"]);
  assertEquals(settings.defaultTs, "bun");
});

Deno.test("getEffectiveSettings: preserves all top-level settings in merged result", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    excludes: ["*.test.ts"],
    skipVariables: false,
    skipResources: false,
    skipFlows: false,
    parallel: 4,
    gitBranches: {
      staging: {
        overrides: {
          skipVariables: true, // Override just this one
        },
      },
    },
  };

  const settings = await getEffectiveSettings(config, undefined, true, true, "staging");
  assertEquals(settings.defaultTs, "bun");
  assertEquals(settings.includes, ["f/**"]);
  assertEquals(settings.excludes, ["*.test.ts"]);
  assertEquals(settings.skipVariables, true); // Overridden
  assertEquals(settings.skipResources, false);
  assertEquals(settings.skipFlows, false);
  assertEquals(settings.parallel, 4);
});
