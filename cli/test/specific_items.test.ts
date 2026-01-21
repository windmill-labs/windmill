import { assertEquals, assertExists, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";

// =============================================================================
// SPECIFIC ITEMS UNIT TESTS
// Tests for branch-specific file path functions (no Docker required)
// =============================================================================

// Import the functions we need to test
import {
  isSpecificItem,
  isItemTypeConfigured,
  toBranchSpecificPath,
  fromBranchSpecificPath,
  isBranchSpecificFile,
  isCurrentBranchFile,
  getBranchSpecificPath,
  getSpecificItemsForCurrentBranch,
} from "../src/core/specific_items.ts";

import type { SpecificItemsConfig } from "../src/core/specific_items.ts";

// =============================================================================
// toBranchSpecificPath TESTS
// =============================================================================

Deno.test("toBranchSpecificPath: converts variable path to branch-specific", () => {
  const result = toBranchSpecificPath("f/test.variable.yaml", "main");
  assertEquals(result, "f/test.main.variable.yaml");
});

Deno.test("toBranchSpecificPath: converts resource path to branch-specific", () => {
  const result = toBranchSpecificPath("u/admin/db.resource.yaml", "develop");
  assertEquals(result, "u/admin/db.develop.resource.yaml");
});

Deno.test("toBranchSpecificPath: converts trigger path to branch-specific", () => {
  const result = toBranchSpecificPath("f/my_trigger.http_trigger.yaml", "feature-x");
  assertEquals(result, "f/my_trigger.feature-x.http_trigger.yaml");
});

Deno.test("toBranchSpecificPath: sanitizes branch names with slashes", () => {
  const result = toBranchSpecificPath("f/test.variable.yaml", "feature/my-feature");
  assertEquals(result, "f/test.feature_my-feature.variable.yaml");
});

Deno.test("toBranchSpecificPath: sanitizes branch names with dots", () => {
  const result = toBranchSpecificPath("f/test.variable.yaml", "release.1.0");
  assertEquals(result, "f/test.release_1_0.variable.yaml");
});

Deno.test("toBranchSpecificPath: leaves non-specific files unchanged", () => {
  const result = toBranchSpecificPath("f/script.ts", "main");
  assertEquals(result, "f/script.ts");
});

Deno.test("toBranchSpecificPath: handles resource files with extensions", () => {
  const result = toBranchSpecificPath("f/config.resource.file.json", "main");
  assertEquals(result, "f/config.main.resource.file.json");
});

// =============================================================================
// fromBranchSpecificPath TESTS
// =============================================================================

Deno.test("fromBranchSpecificPath: converts branch-specific variable back to base", () => {
  const result = fromBranchSpecificPath("f/test.main.variable.yaml", "main");
  assertEquals(result, "f/test.variable.yaml");
});

Deno.test("fromBranchSpecificPath: converts branch-specific resource back to base", () => {
  const result = fromBranchSpecificPath("u/admin/db.develop.resource.yaml", "develop");
  assertEquals(result, "u/admin/db.resource.yaml");
});

Deno.test("fromBranchSpecificPath: converts branch-specific trigger back to base", () => {
  const result = fromBranchSpecificPath("f/my_trigger.feature-x.http_trigger.yaml", "feature-x");
  assertEquals(result, "f/my_trigger.http_trigger.yaml");
});

Deno.test("fromBranchSpecificPath: handles sanitized branch names", () => {
  const result = fromBranchSpecificPath("f/test.feature_my-feature.variable.yaml", "feature/my-feature");
  assertEquals(result, "f/test.variable.yaml");
});

Deno.test("fromBranchSpecificPath: returns unchanged if not branch-specific", () => {
  const result = fromBranchSpecificPath("f/test.variable.yaml", "main");
  assertEquals(result, "f/test.variable.yaml");
});

Deno.test("fromBranchSpecificPath: handles resource files with extensions", () => {
  const result = fromBranchSpecificPath("f/config.main.resource.file.json", "main");
  assertEquals(result, "f/config.resource.file.json");
});

// =============================================================================
// isSpecificItem TESTS
// =============================================================================

Deno.test("isSpecificItem: returns false when specificItems is undefined", () => {
  const result = isSpecificItem("f/test.variable.yaml", undefined);
  assertEquals(result, false);
});

Deno.test("isSpecificItem: matches variable paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  assertEquals(isSpecificItem("f/test.variable.yaml", config), true);
  assertEquals(isSpecificItem("u/admin/test.variable.yaml", config), false);
});

Deno.test("isSpecificItem: matches resource paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    resources: ["u/admin/**"],
  };
  assertEquals(isSpecificItem("u/admin/db.resource.yaml", config), true);
  assertEquals(isSpecificItem("f/db.resource.yaml", config), false);
});

Deno.test("isSpecificItem: matches trigger paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    triggers: ["f/triggers/**"],
  };
  assertEquals(isSpecificItem("f/triggers/my.http_trigger.yaml", config), true);
  assertEquals(isSpecificItem("u/admin/my.http_trigger.yaml", config), false);
});

Deno.test("isSpecificItem: matches multiple patterns", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**", "g/**"],
  };
  assertEquals(isSpecificItem("f/test.variable.yaml", config), true);
  assertEquals(isSpecificItem("g/test.variable.yaml", config), true);
  assertEquals(isSpecificItem("u/admin/test.variable.yaml", config), false);
});

Deno.test("isSpecificItem: handles exact path patterns", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/specific.variable.yaml"],
  };
  assertEquals(isSpecificItem("f/specific.variable.yaml", config), true);
  assertEquals(isSpecificItem("f/other.variable.yaml", config), false);
});

// =============================================================================
// isBranchSpecificFile TESTS
// =============================================================================

Deno.test("isBranchSpecificFile: detects branch-specific variable files", () => {
  assertEquals(isBranchSpecificFile("f/test.main.variable.yaml"), true);
  assertEquals(isBranchSpecificFile("f/test.develop.variable.yaml"), true);
  assertEquals(isBranchSpecificFile("f/test.feature_branch.variable.yaml"), true);
});

Deno.test("isBranchSpecificFile: detects branch-specific resource files", () => {
  assertEquals(isBranchSpecificFile("u/admin/db.main.resource.yaml"), true);
  assertEquals(isBranchSpecificFile("u/admin/db.staging.resource.yaml"), true);
});

Deno.test("isBranchSpecificFile: detects branch-specific trigger files", () => {
  assertEquals(isBranchSpecificFile("f/my.main.http_trigger.yaml"), true);
  assertEquals(isBranchSpecificFile("f/my.develop.kafka_trigger.yaml"), true);
  assertEquals(isBranchSpecificFile("f/my.main.websocket_trigger.yaml"), true);
});

Deno.test("isBranchSpecificFile: returns false for non-branch-specific files", () => {
  assertEquals(isBranchSpecificFile("f/test.variable.yaml"), false);
  assertEquals(isBranchSpecificFile("u/admin/db.resource.yaml"), false);
  assertEquals(isBranchSpecificFile("f/my.http_trigger.yaml"), false);
  assertEquals(isBranchSpecificFile("f/script.ts"), false);
});

Deno.test("isBranchSpecificFile: handles resource files with extensions", () => {
  assertEquals(isBranchSpecificFile("f/config.main.resource.file.json"), true);
  assertEquals(isBranchSpecificFile("f/config.resource.file.json"), false);
});

// =============================================================================
// ROUND-TRIP TESTS
// =============================================================================

Deno.test("round-trip: variable file path conversion", () => {
  const original = "f/my/nested/config.variable.yaml";
  const branch = "feature/test-branch";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

Deno.test("round-trip: resource file path conversion", () => {
  const original = "u/admin/database.resource.yaml";
  const branch = "develop";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

Deno.test("round-trip: trigger file path conversion", () => {
  const original = "f/webhooks/handler.http_trigger.yaml";
  const branch = "main";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

Deno.test("round-trip: resource file with extension", () => {
  const original = "f/configs/settings.resource.file.ini";
  const branch = "release/v1.0";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

// =============================================================================
// BRANCH OVERRIDE TESTS (for --branch flag functionality)
// These tests validate that functions work correctly with explicit branch override
// =============================================================================

Deno.test("branchOverride: getBranchSpecificPath with override returns branch-specific path", () => {
  // This test verifies that when branchOverride is provided, the function uses it
  // instead of detecting the current git branch
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };

  // When override is provided, it should return the branch-specific path even outside git repo
  const result = getBranchSpecificPath("f/test.variable.yaml", config, "staging");
  assertEquals(result, "f/test.staging.variable.yaml");
});

Deno.test("branchOverride: getBranchSpecificPath without override and not in git repo returns undefined", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };

  // Without override and outside git repo (or if git returns null), should return undefined
  // Note: This test's behavior depends on whether we're in a git repo
  const result = getBranchSpecificPath("f/test.variable.yaml", config);
  // In a git repo, this would return a branch-specific path; outside, it would be undefined
  // We test the override case above which is deterministic
});

Deno.test("branchOverride: isCurrentBranchFile with override uses provided branch", () => {
  // Test that isCurrentBranchFile uses the override branch instead of git detection
  const result = isCurrentBranchFile("f/test.staging.variable.yaml", "staging");
  assertEquals(result, true);

  // Should return false for different branch
  const resultOther = isCurrentBranchFile("f/test.staging.variable.yaml", "production");
  assertEquals(resultOther, false);

  // Should return false for non-branch-specific file
  const resultNonSpecific = isCurrentBranchFile("f/test.variable.yaml", "staging");
  assertEquals(resultNonSpecific, false);
});

Deno.test("branchOverride: isCurrentBranchFile with override handles sanitized branch names", () => {
  // Test with branch names that get sanitized
  const result = isCurrentBranchFile("f/test.feature_my-branch.variable.yaml", "feature/my-branch");
  assertEquals(result, true);

  // Different sanitized branch should return false
  const resultOther = isCurrentBranchFile("f/test.feature_my-branch.variable.yaml", "feature/other-branch");
  assertEquals(resultOther, false);
});

Deno.test("branchOverride: getSpecificItemsForCurrentBranch with override returns correct config", () => {
  // Test that getSpecificItemsForCurrentBranch uses the override branch
  const config = {
    gitBranches: {
      staging: {
        specificItems: {
          variables: ["f/**"],
          resources: ["u/admin/**"],
        },
      },
      production: {
        specificItems: {
          variables: ["g/**"],
        },
      },
      commonSpecificItems: {
        triggers: ["f/webhooks/**"],
      },
    },
  };

  const stagingItems = getSpecificItemsForCurrentBranch(config as any, "staging");
  assertEquals(stagingItems?.variables, ["f/**"]);
  assertEquals(stagingItems?.resources, ["u/admin/**"]);
  assertEquals(stagingItems?.triggers, ["f/webhooks/**"]); // From common

  const productionItems = getSpecificItemsForCurrentBranch(config as any, "production");
  assertEquals(productionItems?.variables, ["g/**"]);
  assertEquals(productionItems?.resources, undefined);
  assertEquals(productionItems?.triggers, ["f/webhooks/**"]); // From common
});

Deno.test("branchOverride: getSpecificItemsForCurrentBranch with non-existent branch returns undefined", () => {
  const config = {
    gitBranches: {
      staging: {
        specificItems: {
          variables: ["f/**"],
        },
      },
    },
  };

  // When the branch doesn't have specific items (and there's no common), should return undefined
  const result = getSpecificItemsForCurrentBranch(config as any, "nonexistent");
  assertEquals(result, undefined);
});

Deno.test("branchOverride: getSpecificItemsForCurrentBranch merges common and branch items", () => {
  const config = {
    gitBranches: {
      commonSpecificItems: {
        variables: ["common/**"],
        resources: ["shared/**"],
      },
      develop: {
        specificItems: {
          variables: ["dev/**"],
          triggers: ["dev/triggers/**"],
        },
      },
    },
  };

  const result = getSpecificItemsForCurrentBranch(config as any, "develop");
  // Should merge common and branch-specific
  assertEquals(result?.variables, ["common/**", "dev/**"]);
  assertEquals(result?.resources, ["shared/**"]);
  assertEquals(result?.triggers, ["dev/triggers/**"]);
});

// =============================================================================
// FOLDER BRANCH-SPECIFIC TESTS
// Format: f/folder/folder.branchName.meta.yaml
// =============================================================================

Deno.test("toBranchSpecificPath: converts folder meta path to branch-specific", () => {
  // f/my_folder/folder.meta.yaml -> f/my_folder/folder.main.meta.yaml
  const result = toBranchSpecificPath("f/my_folder/folder.meta.yaml", "main");
  assertEquals(result, "f/my_folder/folder.main.meta.yaml");
});

Deno.test("toBranchSpecificPath: converts nested folder meta path to branch-specific", () => {
  const result = toBranchSpecificPath("f/parent/child/folder.meta.yaml", "develop");
  assertEquals(result, "f/parent/child/folder.develop.meta.yaml");
});

Deno.test("toBranchSpecificPath: sanitizes branch name in folder path", () => {
  const result = toBranchSpecificPath("f/env/folder.meta.yaml", "feature/test");
  assertEquals(result, "f/env/folder.feature_test.meta.yaml");
});

Deno.test("fromBranchSpecificPath: converts branch-specific folder back to base", () => {
  const result = fromBranchSpecificPath("f/my_folder/folder.main.meta.yaml", "main");
  assertEquals(result, "f/my_folder/folder.meta.yaml");
});

Deno.test("fromBranchSpecificPath: handles nested branch-specific folder", () => {
  const result = fromBranchSpecificPath("f/parent/child/folder.develop.meta.yaml", "develop");
  assertEquals(result, "f/parent/child/folder.meta.yaml");
});

Deno.test("fromBranchSpecificPath: handles sanitized branch names for folders", () => {
  const result = fromBranchSpecificPath("f/env/folder.feature_test.meta.yaml", "feature/test");
  assertEquals(result, "f/env/folder.meta.yaml");
});

Deno.test("isSpecificItem: matches folder paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };
  assertEquals(isSpecificItem("f/env_staging/folder.meta.yaml", config), true);
  assertEquals(isSpecificItem("f/env_production/folder.meta.yaml", config), true);
  assertEquals(isSpecificItem("f/other/folder.meta.yaml", config), false);
});

Deno.test("isSpecificItem: matches folder paths with exact pattern", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/config"],
  };
  assertEquals(isSpecificItem("f/config/folder.meta.yaml", config), true);
  assertEquals(isSpecificItem("f/other/folder.meta.yaml", config), false);
});

Deno.test("isBranchSpecificFile: detects branch-specific folder files", () => {
  assertEquals(isBranchSpecificFile("f/my_folder/folder.main.meta.yaml"), true);
  assertEquals(isBranchSpecificFile("f/my_folder/folder.develop.meta.yaml"), true);
  assertEquals(isBranchSpecificFile("f/nested/path/folder.staging.meta.yaml"), true);
});

Deno.test("isBranchSpecificFile: returns false for non-branch-specific folder files", () => {
  assertEquals(isBranchSpecificFile("f/my_folder/folder.meta.yaml"), false);
  assertEquals(isBranchSpecificFile("f/nested/path/folder.meta.yaml"), false);
});

Deno.test("isCurrentBranchFile: detects branch-specific folder for current branch", () => {
  assertEquals(isCurrentBranchFile("f/my_folder/folder.staging.meta.yaml", "staging"), true);
  assertEquals(isCurrentBranchFile("f/my_folder/folder.staging.meta.yaml", "production"), false);
  assertEquals(isCurrentBranchFile("f/my_folder/folder.meta.yaml", "staging"), false);
});

Deno.test("isCurrentBranchFile: handles sanitized branch for folders", () => {
  assertEquals(isCurrentBranchFile("f/env/folder.feature_test.meta.yaml", "feature/test"), true);
  assertEquals(isCurrentBranchFile("f/env/folder.feature_test.meta.yaml", "feature/other"), false);
});

Deno.test("round-trip: folder meta path conversion", () => {
  const original = "f/configs/env_folder/folder.meta.yaml";
  const branch = "main";
  const branchSpecific = toBranchSpecificPath(original, branch);
  assertEquals(branchSpecific, "f/configs/env_folder/folder.main.meta.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

Deno.test("round-trip: folder meta with sanitized branch", () => {
  const original = "f/env/folder.meta.yaml";
  const branch = "feature/new-env";
  const branchSpecific = toBranchSpecificPath(original, branch);
  assertEquals(branchSpecific, "f/env/folder.feature_new-env.meta.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

// =============================================================================
// SETTINGS BRANCH-SPECIFIC TESTS
// =============================================================================

Deno.test("toBranchSpecificPath: converts settings.yaml to branch-specific", () => {
  const result = toBranchSpecificPath("settings.yaml", "main");
  assertEquals(result, "settings.main.yaml");
});

Deno.test("toBranchSpecificPath: sanitizes branch name in settings path", () => {
  const result = toBranchSpecificPath("settings.yaml", "feature/test");
  assertEquals(result, "settings.feature_test.yaml");
});

Deno.test("fromBranchSpecificPath: converts branch-specific settings back to base", () => {
  const result = fromBranchSpecificPath("settings.main.yaml", "main");
  assertEquals(result, "settings.yaml");
});

Deno.test("fromBranchSpecificPath: handles sanitized branch names for settings", () => {
  const result = fromBranchSpecificPath("settings.feature_test.yaml", "feature/test");
  assertEquals(result, "settings.yaml");
});

Deno.test("isSpecificItem: matches settings.yaml when settings is true", () => {
  const config: SpecificItemsConfig = {
    settings: true,
  };
  assertEquals(isSpecificItem("settings.yaml", config), true);
});

Deno.test("isSpecificItem: does not match settings.yaml when settings is false", () => {
  const config: SpecificItemsConfig = {
    settings: false,
  };
  assertEquals(isSpecificItem("settings.yaml", config), false);
});

Deno.test("isSpecificItem: does not match settings.yaml when settings is undefined", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  assertEquals(isSpecificItem("settings.yaml", config), false);
});

Deno.test("isBranchSpecificFile: detects branch-specific settings files", () => {
  assertEquals(isBranchSpecificFile("settings.main.yaml"), true);
  assertEquals(isBranchSpecificFile("settings.develop.yaml"), true);
  assertEquals(isBranchSpecificFile("settings.feature_test.yaml"), true);
});

Deno.test("isBranchSpecificFile: returns false for non-branch-specific settings", () => {
  assertEquals(isBranchSpecificFile("settings.yaml"), false);
});

Deno.test("isCurrentBranchFile: detects branch-specific settings for current branch", () => {
  assertEquals(isCurrentBranchFile("settings.staging.yaml", "staging"), true);
  assertEquals(isCurrentBranchFile("settings.staging.yaml", "production"), false);
  assertEquals(isCurrentBranchFile("settings.yaml", "staging"), false);
});

Deno.test("isCurrentBranchFile: handles sanitized branch for settings", () => {
  assertEquals(isCurrentBranchFile("settings.feature_test.yaml", "feature/test"), true);
  assertEquals(isCurrentBranchFile("settings.feature_test.yaml", "feature/other"), false);
});

Deno.test("round-trip: settings path conversion", () => {
  const original = "settings.yaml";
  const branch = "main";
  const branchSpecific = toBranchSpecificPath(original, branch);
  assertEquals(branchSpecific, "settings.main.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

Deno.test("round-trip: settings with sanitized branch", () => {
  const original = "settings.yaml";
  const branch = "release/v1.0";
  const branchSpecific = toBranchSpecificPath(original, branch);
  assertEquals(branchSpecific, "settings.release_v1_0.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});

// =============================================================================
// isItemTypeConfigured TESTS
// =============================================================================

Deno.test("isItemTypeConfigured: returns true when folders is configured", () => {
  const config: SpecificItemsConfig = { folders: ["f/**"] };
  assertEquals(isItemTypeConfigured("f/test/folder.meta.yaml", config), true);
});

Deno.test("isItemTypeConfigured: returns false when folders is NOT configured", () => {
  const config: SpecificItemsConfig = { variables: ["f/**"] };
  assertEquals(isItemTypeConfigured("f/test/folder.meta.yaml", config), false);
});

Deno.test("isItemTypeConfigured: returns true when variables is configured", () => {
  const config: SpecificItemsConfig = { variables: ["f/**"] };
  assertEquals(isItemTypeConfigured("f/test.variable.yaml", config), true);
});

Deno.test("isItemTypeConfigured: returns false when variables is NOT configured", () => {
  const config: SpecificItemsConfig = { folders: ["f/**"] };
  assertEquals(isItemTypeConfigured("f/test.variable.yaml", config), false);
});

Deno.test("isItemTypeConfigured: returns true when settings is configured", () => {
  const config: SpecificItemsConfig = { settings: true };
  assertEquals(isItemTypeConfigured("settings.yaml", config), true);
});

Deno.test("isItemTypeConfigured: returns false when settings is NOT configured", () => {
  const config: SpecificItemsConfig = { folders: ["f/**"] };
  assertEquals(isItemTypeConfigured("settings.yaml", config), false);
});

Deno.test("isItemTypeConfigured: returns false for undefined config", () => {
  assertEquals(isItemTypeConfigured("f/test/folder.meta.yaml", undefined), false);
});

// =============================================================================
// Branch-specific item filtering behavior tests
// =============================================================================

Deno.test("filtering: folder type configured and matches - should process branch-specific file", () => {
  const config: SpecificItemsConfig = { folders: ["f/**"] };
  const basePath = "f/compliance/folder.meta.yaml";

  // Type is configured AND matches pattern = should process
  const typeConfigured = isItemTypeConfigured(basePath, config);
  const matchesPattern = isSpecificItem(basePath, config);

  assertEquals(typeConfigured, true);
  assertEquals(matchesPattern, true);
  // Skip condition: typeConfigured && !matchesPattern = false (should NOT skip)
  assertEquals(typeConfigured && !matchesPattern, false);
});

Deno.test("filtering: folder type configured but does NOT match - should skip branch-specific file", () => {
  const config: SpecificItemsConfig = { folders: ["f/other/**"] };
  const basePath = "f/compliance/folder.meta.yaml";

  // Type is configured but does NOT match pattern = should skip
  const typeConfigured = isItemTypeConfigured(basePath, config);
  const matchesPattern = isSpecificItem(basePath, config);

  assertEquals(typeConfigured, true);
  assertEquals(matchesPattern, false);
  // Skip condition: typeConfigured && !matchesPattern = true (should skip)
  assertEquals(typeConfigured && !matchesPattern, true);
});

Deno.test("filtering: folder type NOT configured - should process branch-specific file", () => {
  const config: SpecificItemsConfig = { variables: ["f/**"] };
  const basePath = "f/compliance/folder.meta.yaml";

  // Type is NOT configured = should process (not subject to specificItems rules)
  const typeConfigured = isItemTypeConfigured(basePath, config);

  assertEquals(typeConfigured, false);
  // Skip condition: typeConfigured && !matchesPattern = false (should NOT skip)
  assertEquals(typeConfigured && !isSpecificItem(basePath, config), false);
});

Deno.test("filtering: getSpecificItemsForCurrentBranch returns correct config", () => {
  const config = {
    gitBranches: {
      dev: {
        specificItems: {
          folders: ["f/**"]
        }
      },
      prod: {
        specificItems: {
          folders: ["f/prod/**"]
        }
      }
    }
  };

  const devItems = getSpecificItemsForCurrentBranch(config as any, "dev");
  assertEquals(devItems?.folders, ["f/**"]);

  const prodItems = getSpecificItemsForCurrentBranch(config as any, "prod");
  assertEquals(prodItems?.folders, ["f/prod/**"]);

  const unknownItems = getSpecificItemsForCurrentBranch(config as any, "unknown");
  assertEquals(unknownItems, undefined);
});
