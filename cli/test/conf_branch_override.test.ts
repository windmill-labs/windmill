import { expect, test } from "bun:test";
import { getEffectiveSettings, type SyncOptions } from "../src/core/conf.ts";

// =============================================================================
// CONF.TS BRANCH OVERRIDE TESTS
// Tests for getEffectiveSettings with branchOverride parameter
// =============================================================================

test("getEffectiveSettings: applies branch overrides when branchOverride is provided", async () => {
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
  expect(stagingSettings.includes).toEqual(["staging/**"]);
  expect(stagingSettings.skipVariables).toEqual(true);
  expect(stagingSettings.skipSecrets).toEqual(undefined);

  // Test with production branch override
  const prodSettings = await getEffectiveSettings(config, undefined, true, true, "production");
  expect(prodSettings.includes).toEqual(["prod/**"]);
  expect(prodSettings.skipSecrets).toEqual(true);
  expect(prodSettings.skipVariables).toEqual(undefined);
});

test("getEffectiveSettings: uses top-level settings when branchOverride has no overrides", async () => {
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
  expect(settings.includes).toEqual(["f/**"]);
  expect(settings.skipVariables).toEqual(true);
  expect(settings.defaultTs).toEqual("bun");
});

test("getEffectiveSettings: uses top-level settings for unknown branch", async () => {
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
  expect(settings.includes).toEqual(["f/**"]);
  expect(settings.defaultTs).toEqual("bun");
});

test("getEffectiveSettings: promotionOverrides take precedence when promotion specified", async () => {
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
  expect(normalSettings.includes).toEqual(["prod/**"]);
  expect(normalSettings.skipVariables).toEqual(undefined);

  // Test with promotion flag - should use promotionOverrides
  const promoSettings = await getEffectiveSettings(config, "production", true, true);
  expect(promoSettings.includes).toEqual(["promoted/**"]);
  expect(promoSettings.skipVariables).toEqual(true);
});

test("getEffectiveSettings: branchOverride works without gitBranches config", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
  };

  // Should not throw even with branchOverride but no gitBranches
  const settings = await getEffectiveSettings(config, undefined, true, true, "staging");
  expect(settings.includes).toEqual(["f/**"]);
  expect(settings.defaultTs).toEqual("bun");
});

test("getEffectiveSettings: preserves all top-level settings in merged result", async () => {
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
  expect(settings.defaultTs).toEqual("bun");
  expect(settings.includes).toEqual(["f/**"]);
  expect(settings.excludes).toEqual(["*.test.ts"]);
  expect(settings.skipVariables).toEqual(true); // Overridden
  expect(settings.skipResources).toEqual(false);
  expect(settings.skipFlows).toEqual(false);
  expect(settings.parallel).toEqual(4);
});
