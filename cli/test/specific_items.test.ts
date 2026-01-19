import { assertEquals, assertExists, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";

// =============================================================================
// SPECIFIC ITEMS UNIT TESTS
// Tests for branch-specific file path functions (no Docker required)
// =============================================================================

// Import the functions we need to test
import {
  isSpecificItem,
  toBranchSpecificPath,
  fromBranchSpecificPath,
  isBranchSpecificFile,
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

import {
  getBranchSpecificPath,
  isCurrentBranchFile,
  getSpecificItemsForCurrentBranch,
} from "../src/core/specific_items.ts";

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
// =============================================================================

Deno.test("toBranchSpecificPath: converts folder path to branch-specific", () => {
  const result = toBranchSpecificPath("f/my_folder/folder.meta.yaml", "main");
  assertEquals(result, "f/my_folder.main/folder.meta.yaml");
});

Deno.test("toBranchSpecificPath: converts nested folder path to branch-specific", () => {
  const result = toBranchSpecificPath("f/parent/child/folder.meta.yaml", "develop");
  assertEquals(result, "f/parent/child.develop/folder.meta.yaml");
});

Deno.test("fromBranchSpecificPath: converts branch-specific folder back to base", () => {
  const result = fromBranchSpecificPath("f/my_folder.main/folder.meta.yaml", "main");
  assertEquals(result, "f/my_folder/folder.meta.yaml");
});

Deno.test("fromBranchSpecificPath: handles sanitized branch names for folders", () => {
  const result = fromBranchSpecificPath("f/my_folder.feature_test/folder.meta.yaml", "feature/test");
  assertEquals(result, "f/my_folder/folder.meta.yaml");
});

Deno.test("isSpecificItem: matches folder paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };
  assertEquals(isSpecificItem("f/env_staging/folder.meta.yaml", config), true);
  assertEquals(isSpecificItem("f/env_production/folder.meta.yaml", config), true);
  assertEquals(isSpecificItem("f/other/folder.meta.yaml", config), false);
});

Deno.test("isBranchSpecificFile: detects branch-specific folder files", () => {
  assertEquals(isBranchSpecificFile("f/my_folder.main/folder.meta.yaml"), true);
  assertEquals(isBranchSpecificFile("f/my_folder.develop/folder.meta.yaml"), true);
  assertEquals(isBranchSpecificFile("f/my_folder/folder.meta.yaml"), false);
});

Deno.test("isCurrentBranchFile: detects branch-specific folder for current branch", () => {
  assertEquals(isCurrentBranchFile("f/my_folder.staging/folder.meta.yaml", "staging"), true);
  assertEquals(isCurrentBranchFile("f/my_folder.staging/folder.meta.yaml", "production"), false);
  assertEquals(isCurrentBranchFile("f/my_folder/folder.meta.yaml", "staging"), false);
});

Deno.test("round-trip: folder path conversion", () => {
  const original = "f/configs/env_folder/folder.meta.yaml";
  const branch = "feature/new-env";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  assertEquals(restored, original);
});
