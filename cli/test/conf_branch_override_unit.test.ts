import { expect, test } from "bun:test";
import { getEffectiveSettings, type SyncOptions } from "../src/core/conf.ts";

// =============================================================================
// CONF.TS WORKSPACE OVERRIDE TESTS
// Tests for getEffectiveSettings with workspaceNameOverride parameter
// =============================================================================

test("getEffectiveSettings: applies workspace overrides when workspaceNameOverride is provided", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    workspaces: {
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

  // Test with staging workspace override
  const stagingSettings = await getEffectiveSettings(config, undefined, true, true, "staging");
  expect(stagingSettings.includes).toEqual(["staging/**"]);
  expect(stagingSettings.skipVariables).toEqual(true);
  expect(stagingSettings.skipSecrets).toEqual(undefined);

  // Test with production workspace override
  const prodSettings = await getEffectiveSettings(config, undefined, true, true, "production");
  expect(prodSettings.includes).toEqual(["prod/**"]);
  expect(prodSettings.skipSecrets).toEqual(true);
  expect(prodSettings.skipVariables).toEqual(undefined);
});

test("getEffectiveSettings: uses top-level settings when workspace has no overrides", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    skipVariables: true,
    workspaces: {
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

test("getEffectiveSettings: uses top-level settings for unknown workspace", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
    workspaces: {
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
    workspaces: {
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

test("getEffectiveSettings: workspaceNameOverride works without workspaces config", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    includes: ["f/**"],
  };

  // Should not throw even with workspaceNameOverride but no workspaces
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
    workspaces: {
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
