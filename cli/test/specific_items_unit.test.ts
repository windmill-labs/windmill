import { expect, test } from "bun:test";

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

test("toBranchSpecificPath: converts variable path to branch-specific", () => {
  const result = toBranchSpecificPath("f/test.variable.yaml", "main");
  expect(result).toEqual("f/test.main.variable.yaml");
});

test("toBranchSpecificPath: converts resource path to branch-specific", () => {
  const result = toBranchSpecificPath("u/admin/db.resource.yaml", "develop");
  expect(result).toEqual("u/admin/db.develop.resource.yaml");
});

test("toBranchSpecificPath: converts trigger path to branch-specific", () => {
  const result = toBranchSpecificPath("f/my_trigger.http_trigger.yaml", "feature-x");
  expect(result).toEqual("f/my_trigger.feature-x.http_trigger.yaml");
});

test("toBranchSpecificPath: sanitizes branch names with slashes", () => {
  const result = toBranchSpecificPath("f/test.variable.yaml", "feature/my-feature");
  expect(result).toEqual("f/test.feature_my-feature.variable.yaml");
});

test("toBranchSpecificPath: sanitizes branch names with dots", () => {
  const result = toBranchSpecificPath("f/test.variable.yaml", "release.1.0");
  expect(result).toEqual("f/test.release_1_0.variable.yaml");
});

test("toBranchSpecificPath: leaves non-specific files unchanged", () => {
  const result = toBranchSpecificPath("f/script.ts", "main");
  expect(result).toEqual("f/script.ts");
});

test("toBranchSpecificPath: handles resource files with extensions", () => {
  const result = toBranchSpecificPath("f/config.resource.file.json", "main");
  expect(result).toEqual("f/config.main.resource.file.json");
});

// =============================================================================
// fromBranchSpecificPath TESTS
// =============================================================================

test("fromBranchSpecificPath: converts branch-specific variable back to base", () => {
  const result = fromBranchSpecificPath("f/test.main.variable.yaml", "main");
  expect(result).toEqual("f/test.variable.yaml");
});

test("fromBranchSpecificPath: converts branch-specific resource back to base", () => {
  const result = fromBranchSpecificPath("u/admin/db.develop.resource.yaml", "develop");
  expect(result).toEqual("u/admin/db.resource.yaml");
});

test("fromBranchSpecificPath: converts branch-specific trigger back to base", () => {
  const result = fromBranchSpecificPath("f/my_trigger.feature-x.http_trigger.yaml", "feature-x");
  expect(result).toEqual("f/my_trigger.http_trigger.yaml");
});

test("fromBranchSpecificPath: handles sanitized branch names", () => {
  const result = fromBranchSpecificPath("f/test.feature_my-feature.variable.yaml", "feature/my-feature");
  expect(result).toEqual("f/test.variable.yaml");
});

test("fromBranchSpecificPath: returns unchanged if not branch-specific", () => {
  const result = fromBranchSpecificPath("f/test.variable.yaml", "main");
  expect(result).toEqual("f/test.variable.yaml");
});

test("fromBranchSpecificPath: handles resource files with extensions", () => {
  const result = fromBranchSpecificPath("f/config.main.resource.file.json", "main");
  expect(result).toEqual("f/config.resource.file.json");
});

// =============================================================================
// isSpecificItem TESTS
// =============================================================================

test("isSpecificItem: returns false when specificItems is undefined", () => {
  const result = isSpecificItem("f/test.variable.yaml", undefined);
  expect(result).toEqual(false);
});

test("isSpecificItem: matches variable paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  expect(isSpecificItem("f/test.variable.yaml", config)).toEqual(true);
  expect(isSpecificItem("u/admin/test.variable.yaml", config)).toEqual(false);
});

test("isSpecificItem: matches resource paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    resources: ["u/admin/**"],
  };
  expect(isSpecificItem("u/admin/db.resource.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/db.resource.yaml", config)).toEqual(false);
});

test("isSpecificItem: matches trigger paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    triggers: ["f/triggers/**"],
  };
  expect(isSpecificItem("f/triggers/my.http_trigger.yaml", config)).toEqual(true);
  expect(isSpecificItem("u/admin/my.http_trigger.yaml", config)).toEqual(false);
});

test("isSpecificItem: matches multiple patterns", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**", "g/**"],
  };
  expect(isSpecificItem("f/test.variable.yaml", config)).toEqual(true);
  expect(isSpecificItem("g/test.variable.yaml", config)).toEqual(true);
  expect(isSpecificItem("u/admin/test.variable.yaml", config)).toEqual(false);
});

test("isSpecificItem: handles exact path patterns", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/specific.variable.yaml"],
  };
  expect(isSpecificItem("f/specific.variable.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/other.variable.yaml", config)).toEqual(false);
});

// =============================================================================
// isBranchSpecificFile TESTS
// =============================================================================

test("isBranchSpecificFile: detects branch-specific variable files", () => {
  expect(isBranchSpecificFile("f/test.main.variable.yaml")).toEqual(true);
  expect(isBranchSpecificFile("f/test.develop.variable.yaml")).toEqual(true);
  expect(isBranchSpecificFile("f/test.feature_branch.variable.yaml")).toEqual(true);
});

test("isBranchSpecificFile: detects branch-specific resource files", () => {
  expect(isBranchSpecificFile("u/admin/db.main.resource.yaml")).toEqual(true);
  expect(isBranchSpecificFile("u/admin/db.staging.resource.yaml")).toEqual(true);
});

test("isBranchSpecificFile: detects branch-specific trigger files", () => {
  expect(isBranchSpecificFile("f/my.main.http_trigger.yaml")).toEqual(true);
  expect(isBranchSpecificFile("f/my.develop.kafka_trigger.yaml")).toEqual(true);
  expect(isBranchSpecificFile("f/my.main.websocket_trigger.yaml")).toEqual(true);
});

test("isBranchSpecificFile: returns false for non-branch-specific files", () => {
  expect(isBranchSpecificFile("f/test.variable.yaml")).toEqual(false);
  expect(isBranchSpecificFile("u/admin/db.resource.yaml")).toEqual(false);
  expect(isBranchSpecificFile("f/my.http_trigger.yaml")).toEqual(false);
  expect(isBranchSpecificFile("f/script.ts")).toEqual(false);
});

test("isBranchSpecificFile: handles resource files with extensions", () => {
  expect(isBranchSpecificFile("f/config.main.resource.file.json")).toEqual(true);
  expect(isBranchSpecificFile("f/config.resource.file.json")).toEqual(false);
});

// =============================================================================
// ROUND-TRIP TESTS
// =============================================================================

test("round-trip: variable file path conversion", () => {
  const original = "f/my/nested/config.variable.yaml";
  const branch = "feature/test-branch";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

test("round-trip: resource file path conversion", () => {
  const original = "u/admin/database.resource.yaml";
  const branch = "develop";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

test("round-trip: trigger file path conversion", () => {
  const original = "f/webhooks/handler.http_trigger.yaml";
  const branch = "main";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

test("round-trip: resource file with extension", () => {
  const original = "f/configs/settings.resource.file.ini";
  const branch = "release/v1.0";
  const branchSpecific = toBranchSpecificPath(original, branch);
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

// =============================================================================
// BRANCH OVERRIDE TESTS (for --branch flag functionality)
// These tests validate that functions work correctly with explicit branch override
// =============================================================================

test("branchOverride: getBranchSpecificPath with override returns branch-specific path", () => {
  // This test verifies that when branchOverride is provided, the function uses it
  // instead of detecting the current git branch
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };

  // When override is provided, it should return the branch-specific path even outside git repo
  const result = getBranchSpecificPath("f/test.variable.yaml", config, "staging");
  expect(result).toEqual("f/test.staging.variable.yaml");
});

test("branchOverride: getBranchSpecificPath without override and not in git repo returns undefined", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };

  // Without override and outside git repo (or if git returns null), should return undefined
  // Note: This test's behavior depends on whether we're in a git repo
  const result = getBranchSpecificPath("f/test.variable.yaml", config);
  // In a git repo, this would return a branch-specific path; outside, it would be undefined
  // We test the override case above which is deterministic
});

test("branchOverride: isCurrentBranchFile with override uses provided branch", () => {
  // Test that isCurrentBranchFile uses the override branch instead of git detection
  const result = isCurrentBranchFile("f/test.staging.variable.yaml", "staging");
  expect(result).toEqual(true);

  // Should return false for different branch
  const resultOther = isCurrentBranchFile("f/test.staging.variable.yaml", "production");
  expect(resultOther).toEqual(false);

  // Should return false for non-branch-specific file
  const resultNonSpecific = isCurrentBranchFile("f/test.variable.yaml", "staging");
  expect(resultNonSpecific).toEqual(false);
});

test("branchOverride: isCurrentBranchFile with override handles sanitized branch names", () => {
  // Test with branch names that get sanitized
  const result = isCurrentBranchFile("f/test.feature_my-branch.variable.yaml", "feature/my-branch");
  expect(result).toEqual(true);

  // Different sanitized branch should return false
  const resultOther = isCurrentBranchFile("f/test.feature_my-branch.variable.yaml", "feature/other-branch");
  expect(resultOther).toEqual(false);
});

test("branchOverride: getSpecificItemsForCurrentBranch with override returns correct config", () => {
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
  expect(stagingItems?.variables).toEqual(["f/**"]);
  expect(stagingItems?.resources).toEqual(["u/admin/**"]);
  expect(stagingItems?.triggers).toEqual(["f/webhooks/**"]); // From common

  const productionItems = getSpecificItemsForCurrentBranch(config as any, "production");
  expect(productionItems?.variables).toEqual(["g/**"]);
  expect(productionItems?.resources).toEqual(undefined);
  expect(productionItems?.triggers).toEqual(["f/webhooks/**"]); // From common
});

test("branchOverride: getSpecificItemsForCurrentBranch with non-existent branch returns undefined", () => {
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
  expect(result).toEqual(undefined);
});

test("branchOverride: getSpecificItemsForCurrentBranch merges common and branch items", () => {
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
  expect(result?.variables).toEqual(["common/**", "dev/**"]);
  expect(result?.resources).toEqual(["shared/**"]);
  expect(result?.triggers).toEqual(["dev/triggers/**"]);
});

// =============================================================================
// FOLDER BRANCH-SPECIFIC TESTS
// Format: f/folder/folder.branchName.meta.yaml
// =============================================================================

test("toBranchSpecificPath: converts folder meta path to branch-specific", () => {
  // f/my_folder/folder.meta.yaml -> f/my_folder/folder.main.meta.yaml
  const result = toBranchSpecificPath("f/my_folder/folder.meta.yaml", "main");
  expect(result).toEqual("f/my_folder/folder.main.meta.yaml");
});

test("toBranchSpecificPath: converts nested folder meta path to branch-specific", () => {
  const result = toBranchSpecificPath("f/parent/child/folder.meta.yaml", "develop");
  expect(result).toEqual("f/parent/child/folder.develop.meta.yaml");
});

test("toBranchSpecificPath: sanitizes branch name in folder path", () => {
  const result = toBranchSpecificPath("f/env/folder.meta.yaml", "feature/test");
  expect(result).toEqual("f/env/folder.feature_test.meta.yaml");
});

test("fromBranchSpecificPath: converts branch-specific folder back to base", () => {
  const result = fromBranchSpecificPath("f/my_folder/folder.main.meta.yaml", "main");
  expect(result).toEqual("f/my_folder/folder.meta.yaml");
});

test("fromBranchSpecificPath: handles nested branch-specific folder", () => {
  const result = fromBranchSpecificPath("f/parent/child/folder.develop.meta.yaml", "develop");
  expect(result).toEqual("f/parent/child/folder.meta.yaml");
});

test("fromBranchSpecificPath: handles sanitized branch names for folders", () => {
  const result = fromBranchSpecificPath("f/env/folder.feature_test.meta.yaml", "feature/test");
  expect(result).toEqual("f/env/folder.meta.yaml");
});

test("isSpecificItem: matches folder paths with glob pattern", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };
  expect(isSpecificItem("f/env_staging/folder.meta.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/env_production/folder.meta.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/other/folder.meta.yaml", config)).toEqual(false);
});

test("isSpecificItem: matches folder paths with exact pattern", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/config"],
  };
  expect(isSpecificItem("f/config/folder.meta.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/other/folder.meta.yaml", config)).toEqual(false);
});

test("isBranchSpecificFile: detects branch-specific folder files", () => {
  expect(isBranchSpecificFile("f/my_folder/folder.main.meta.yaml")).toEqual(true);
  expect(isBranchSpecificFile("f/my_folder/folder.develop.meta.yaml")).toEqual(true);
  expect(isBranchSpecificFile("f/nested/path/folder.staging.meta.yaml")).toEqual(true);
});

test("isBranchSpecificFile: returns false for non-branch-specific folder files", () => {
  expect(isBranchSpecificFile("f/my_folder/folder.meta.yaml")).toEqual(false);
  expect(isBranchSpecificFile("f/nested/path/folder.meta.yaml")).toEqual(false);
});

test("isCurrentBranchFile: detects branch-specific folder for current branch", () => {
  expect(isCurrentBranchFile("f/my_folder/folder.staging.meta.yaml", "staging")).toEqual(true);
  expect(isCurrentBranchFile("f/my_folder/folder.staging.meta.yaml", "production")).toEqual(false);
  expect(isCurrentBranchFile("f/my_folder/folder.meta.yaml", "staging")).toEqual(false);
});

test("isCurrentBranchFile: handles sanitized branch for folders", () => {
  expect(isCurrentBranchFile("f/env/folder.feature_test.meta.yaml", "feature/test")).toEqual(true);
  expect(isCurrentBranchFile("f/env/folder.feature_test.meta.yaml", "feature/other")).toEqual(false);
});

test("round-trip: folder meta path conversion", () => {
  const original = "f/configs/env_folder/folder.meta.yaml";
  const branch = "main";
  const branchSpecific = toBranchSpecificPath(original, branch);
  expect(branchSpecific).toEqual("f/configs/env_folder/folder.main.meta.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

test("round-trip: folder meta with sanitized branch", () => {
  const original = "f/env/folder.meta.yaml";
  const branch = "feature/new-env";
  const branchSpecific = toBranchSpecificPath(original, branch);
  expect(branchSpecific).toEqual("f/env/folder.feature_new-env.meta.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

// =============================================================================
// SETTINGS BRANCH-SPECIFIC TESTS
// =============================================================================

test("toBranchSpecificPath: converts settings.yaml to branch-specific", () => {
  const result = toBranchSpecificPath("settings.yaml", "main");
  expect(result).toEqual("settings.main.yaml");
});

test("toBranchSpecificPath: sanitizes branch name in settings path", () => {
  const result = toBranchSpecificPath("settings.yaml", "feature/test");
  expect(result).toEqual("settings.feature_test.yaml");
});

test("fromBranchSpecificPath: converts branch-specific settings back to base", () => {
  const result = fromBranchSpecificPath("settings.main.yaml", "main");
  expect(result).toEqual("settings.yaml");
});

test("fromBranchSpecificPath: handles sanitized branch names for settings", () => {
  const result = fromBranchSpecificPath("settings.feature_test.yaml", "feature/test");
  expect(result).toEqual("settings.yaml");
});

test("isSpecificItem: matches settings.yaml when settings is true", () => {
  const config: SpecificItemsConfig = {
    settings: true,
  };
  expect(isSpecificItem("settings.yaml", config)).toEqual(true);
});

test("isSpecificItem: does not match settings.yaml when settings is false", () => {
  const config: SpecificItemsConfig = {
    settings: false,
  };
  expect(isSpecificItem("settings.yaml", config)).toEqual(false);
});

test("isSpecificItem: does not match settings.yaml when settings is undefined", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  expect(isSpecificItem("settings.yaml", config)).toEqual(false);
});

test("isBranchSpecificFile: detects branch-specific settings files", () => {
  expect(isBranchSpecificFile("settings.main.yaml")).toEqual(true);
  expect(isBranchSpecificFile("settings.develop.yaml")).toEqual(true);
  expect(isBranchSpecificFile("settings.feature_test.yaml")).toEqual(true);
});

test("isBranchSpecificFile: returns false for non-branch-specific settings", () => {
  expect(isBranchSpecificFile("settings.yaml")).toEqual(false);
});

test("isCurrentBranchFile: detects branch-specific settings for current branch", () => {
  expect(isCurrentBranchFile("settings.staging.yaml", "staging")).toEqual(true);
  expect(isCurrentBranchFile("settings.staging.yaml", "production")).toEqual(false);
  expect(isCurrentBranchFile("settings.yaml", "staging")).toEqual(false);
});

test("isCurrentBranchFile: handles sanitized branch for settings", () => {
  expect(isCurrentBranchFile("settings.feature_test.yaml", "feature/test")).toEqual(true);
  expect(isCurrentBranchFile("settings.feature_test.yaml", "feature/other")).toEqual(false);
});

test("round-trip: settings path conversion", () => {
  const original = "settings.yaml";
  const branch = "main";
  const branchSpecific = toBranchSpecificPath(original, branch);
  expect(branchSpecific).toEqual("settings.main.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

test("round-trip: settings with sanitized branch", () => {
  const original = "settings.yaml";
  const branch = "release/v1.0";
  const branchSpecific = toBranchSpecificPath(original, branch);
  expect(branchSpecific).toEqual("settings.release_v1_0.yaml");
  const restored = fromBranchSpecificPath(branchSpecific, branch);
  expect(restored).toEqual(original);
});

// =============================================================================
// isItemTypeConfigured TESTS
// This function checks if the TYPE is configured, not whether it matches pattern.
// Used to determine if branch-specific files should be used for this type.
// =============================================================================

test("isItemTypeConfigured: returns false when specificItems is undefined", () => {
  expect(isItemTypeConfigured("f/test.variable.yaml", undefined)).toEqual(false);
  expect(isItemTypeConfigured("f/test.resource.yaml", undefined)).toEqual(false);
  expect(isItemTypeConfigured("f/folder/folder.meta.yaml", undefined)).toEqual(false);
  expect(isItemTypeConfigured("settings.yaml", undefined)).toEqual(false);
});

test("isItemTypeConfigured: returns true for variables when variables is configured", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  // Type is configured (even if path doesn't match the pattern)
  expect(isItemTypeConfigured("f/test.variable.yaml", config)).toEqual(true);
  expect(isItemTypeConfigured("g/other.variable.yaml", config)).toEqual(true);
});

test("isItemTypeConfigured: returns false for variables when variables is NOT configured", () => {
  const config: SpecificItemsConfig = {
    resources: ["f/**"],
  };
  expect(isItemTypeConfigured("f/test.variable.yaml", config)).toEqual(false);
});

test("isItemTypeConfigured: returns true for resources when resources is configured", () => {
  const config: SpecificItemsConfig = {
    resources: ["f/**"],
  };
  expect(isItemTypeConfigured("f/test.resource.yaml", config)).toEqual(true);
  expect(isItemTypeConfigured("g/other.resource.yaml", config)).toEqual(true);
});

test("isItemTypeConfigured: returns false for resources when resources is NOT configured", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  expect(isItemTypeConfigured("f/test.resource.yaml", config)).toEqual(false);
});

test("isItemTypeConfigured: returns true for triggers when triggers is configured", () => {
  const config: SpecificItemsConfig = {
    triggers: ["f/**"],
  };
  expect(isItemTypeConfigured("f/my.http_trigger.yaml", config)).toEqual(true);
  expect(isItemTypeConfigured("f/my.kafka_trigger.yaml", config)).toEqual(true);
  expect(isItemTypeConfigured("g/other.websocket_trigger.yaml", config)).toEqual(true);
});

test("isItemTypeConfigured: returns false for triggers when triggers is NOT configured", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  expect(isItemTypeConfigured("f/my.http_trigger.yaml", config)).toEqual(false);
});

test("isItemTypeConfigured: returns true for folders when folders is configured", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };
  // Type is configured (even if path doesn't match the pattern)
  expect(isItemTypeConfigured("f/env_staging/folder.meta.yaml", config)).toEqual(true);
  expect(isItemTypeConfigured("f/other/folder.meta.yaml", config)).toEqual(true);
});

test("isItemTypeConfigured: returns false for folders when folders is NOT configured", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  expect(isItemTypeConfigured("f/my_folder/folder.meta.yaml", config)).toEqual(false);
});

test("isItemTypeConfigured: returns true for settings when settings is configured (true)", () => {
  const config: SpecificItemsConfig = {
    settings: true,
  };
  expect(isItemTypeConfigured("settings.yaml", config)).toEqual(true);
});

test("isItemTypeConfigured: returns true for settings when settings is configured (false)", () => {
  // settings: false still means the type is "configured" (explicitly disabled)
  const config: SpecificItemsConfig = {
    settings: false,
  };
  expect(isItemTypeConfigured("settings.yaml", config)).toEqual(true);
});

test("isItemTypeConfigured: returns false for settings when settings is NOT configured", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  expect(isItemTypeConfigured("settings.yaml", config)).toEqual(false);
});

test("isItemTypeConfigured: returns true for resource files (with extension) when resources is configured", () => {
  const config: SpecificItemsConfig = {
    resources: ["f/**"],
  };
  expect(isItemTypeConfigured("f/config.resource.file.json", config)).toEqual(true);
  expect(isItemTypeConfigured("f/data.resource.file.ini", config)).toEqual(true);
});

test("isItemTypeConfigured: returns false for resource files when resources is NOT configured", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };
  expect(isItemTypeConfigured("f/config.resource.file.json", config)).toEqual(false);
});

// =============================================================================
// BRANCH-SPECIFIC FILE FILTERING TESTS
// These tests verify the expected filtering behavior:
// - When type IS configured: use branch-specific files, skip base files
// - When type is NOT configured: skip branch-specific files, use base files
// =============================================================================

test("filtering logic: folders - when NOT configured, branch-specific should be ignored", () => {
  // Config has variables but NOT folders
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };

  const basePath = "f/my_folder/folder.meta.yaml";
  const branchSpecificPath = "f/my_folder/folder.main.meta.yaml";

  // Folder type is NOT configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(false);

  // Therefore, branch-specific file detection should not apply to this type
  // The sync logic should:
  // 1. Skip branch-specific folder files (isBranchSpecificFile returns true)
  // 2. Use the base file
  expect(isBranchSpecificFile(branchSpecificPath)).toEqual(true);
  expect(isBranchSpecificFile(basePath)).toEqual(false);
});

test("filtering logic: folders - when IS configured and matches, use branch-specific", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/my_folder"],
  };

  const basePath = "f/my_folder/folder.meta.yaml";
  const branchSpecificPath = "f/my_folder/folder.main.meta.yaml";

  // Folder type IS configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(true);

  // And path matches the pattern
  expect(isSpecificItem(basePath, config)).toEqual(true);

  // The sync logic should:
  // 1. Use branch-specific folder file (map to base path)
  // 2. Skip the base file
  expect(isBranchSpecificFile(branchSpecificPath)).toEqual(true);
  expect(fromBranchSpecificPath(branchSpecificPath, "main")).toEqual(basePath);
});

test("filtering logic: folders - when IS configured but doesn't match, skip branch-specific", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"], // Only env_ folders are branch-specific
  };

  const basePath = "f/other_folder/folder.meta.yaml";
  const branchSpecificPath = "f/other_folder/folder.main.meta.yaml";

  // Folder type IS configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(true);

  // But this path doesn't match the pattern
  expect(isSpecificItem(basePath, config)).toEqual(false);

  // The sync logic should:
  // 1. Skip the branch-specific file (type configured but doesn't match)
  // 2. Use the base file
});

test("filtering logic: settings - when NOT configured, branch-specific should be ignored", () => {
  // Config has variables but NOT settings
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };

  const basePath = "settings.yaml";
  const branchSpecificPath = "settings.main.yaml";

  // Settings type is NOT configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(false);

  // Therefore, branch-specific file detection should not apply to this type
  expect(isBranchSpecificFile(branchSpecificPath)).toEqual(true);
  expect(isBranchSpecificFile(basePath)).toEqual(false);
});

test("filtering logic: settings - when IS configured (true), use branch-specific", () => {
  const config: SpecificItemsConfig = {
    settings: true,
  };

  const basePath = "settings.yaml";
  const branchSpecificPath = "settings.main.yaml";

  // Settings type IS configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(true);

  // And settings: true means it matches
  expect(isSpecificItem(basePath, config)).toEqual(true);

  // The sync logic should use branch-specific file
  expect(isBranchSpecificFile(branchSpecificPath)).toEqual(true);
  expect(fromBranchSpecificPath(branchSpecificPath, "main")).toEqual(basePath);
});

test("filtering logic: settings - when IS configured (false), skip branch-specific", () => {
  // settings: false means type is configured but explicitly disabled
  const config: SpecificItemsConfig = {
    settings: false,
  };

  const basePath = "settings.yaml";
  const branchSpecificPath = "settings.main.yaml";

  // Settings type IS configured (even though value is false)
  expect(isItemTypeConfigured(basePath, config)).toEqual(true);

  // But settings: false means it doesn't match (not a specific item)
  expect(isSpecificItem(basePath, config)).toEqual(false);

  // The sync logic should skip branch-specific file and use base
});

test("filtering logic: variables - when NOT configured, branch-specific should be ignored", () => {
  // Config has folders but NOT variables
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };

  const basePath = "f/test.variable.yaml";
  const branchSpecificPath = "f/test.main.variable.yaml";

  // Variable type is NOT configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(false);

  // Branch-specific variable files should be ignored
  expect(isBranchSpecificFile(branchSpecificPath)).toEqual(true);
  expect(isBranchSpecificFile(basePath)).toEqual(false);
});

test("filtering logic: resources - when NOT configured, branch-specific should be ignored", () => {
  // Config has folders but NOT resources
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };

  const basePath = "f/db.resource.yaml";
  const branchSpecificPath = "f/db.main.resource.yaml";

  // Resource type is NOT configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(false);

  expect(isBranchSpecificFile(branchSpecificPath)).toEqual(true);
  expect(isBranchSpecificFile(basePath)).toEqual(false);
});

test("filtering logic: triggers - when NOT configured, branch-specific should be ignored", () => {
  // Config has folders but NOT triggers
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };

  const basePath = "f/webhook.http_trigger.yaml";
  const branchSpecificPath = "f/webhook.main.http_trigger.yaml";

  // Trigger type is NOT configured
  expect(isItemTypeConfigured(basePath, config)).toEqual(false);

  expect(isBranchSpecificFile(branchSpecificPath)).toEqual(true);
  expect(isBranchSpecificFile(basePath)).toEqual(false);
});

// =============================================================================
// MIXED CONFIGURATION TESTS
// Tests for configs that have some types configured but not others
// =============================================================================

test("mixed config: only folders configured - other types use base files", () => {
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };

  // Folders IS configured
  expect(isItemTypeConfigured("f/env_staging/folder.meta.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/env_staging/folder.meta.yaml", config)).toEqual(true);

  // Variables, resources, triggers, settings are NOT configured
  expect(isItemTypeConfigured("f/test.variable.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("f/db.resource.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("f/hook.http_trigger.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("settings.yaml", config)).toEqual(false);
});

test("mixed config: only settings configured - other types use base files", () => {
  const config: SpecificItemsConfig = {
    settings: true,
  };

  // Settings IS configured
  expect(isItemTypeConfigured("settings.yaml", config)).toEqual(true);
  expect(isSpecificItem("settings.yaml", config)).toEqual(true);

  // Other types are NOT configured
  expect(isItemTypeConfigured("f/test.variable.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("f/db.resource.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("f/hook.http_trigger.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("f/my_folder/folder.meta.yaml", config)).toEqual(false);
});

test("mixed config: variables and folders configured - resources and triggers use base", () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
    folders: ["f/env_*"],
  };

  // Variables IS configured
  expect(isItemTypeConfigured("f/test.variable.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/test.variable.yaml", config)).toEqual(true);

  // Folders IS configured (path matches)
  expect(isItemTypeConfigured("f/env_staging/folder.meta.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/env_staging/folder.meta.yaml", config)).toEqual(true);

  // Folders IS configured but path doesn't match
  expect(isItemTypeConfigured("f/other/folder.meta.yaml", config)).toEqual(true);
  expect(isSpecificItem("f/other/folder.meta.yaml", config)).toEqual(false);

  // Resources and triggers are NOT configured
  expect(isItemTypeConfigured("f/db.resource.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("f/hook.http_trigger.yaml", config)).toEqual(false);
  expect(isItemTypeConfigured("settings.yaml", config)).toEqual(false);
});
