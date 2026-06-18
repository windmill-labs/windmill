/**
 * Unit tests for GitSyncSettingsConverter.
 * Tests conversion between backend format (include_type array) and
 * SyncOptions format (boolean flags like skipWorkspaceDependencies).
 */

import { expect, test, describe } from "bun:test";
import { GitSyncSettingsConverter } from "../src/commands/gitsync-settings/converter.ts";

// =============================================================================
// fromBackendFormat - converts backend include_type array to SyncOptions
// =============================================================================

describe("GitSyncSettingsConverter.fromBackendFormat", () => {
  test("converts workspacedependencies in include_type to skipWorkspaceDependencies: false", () => {
    const backend = {
      include_path: ["f/**"],
      include_type: ["script", "flow", "workspacedependencies"],
    };
    const result = GitSyncSettingsConverter.fromBackendFormat(backend);
    expect(result.skipWorkspaceDependencies).toBe(false);
  });

  test("sets skipWorkspaceDependencies: true when workspacedependencies is absent", () => {
    const backend = {
      include_path: ["f/**"],
      include_type: ["script", "flow"],
    };
    const result = GitSyncSettingsConverter.fromBackendFormat(backend);
    expect(result.skipWorkspaceDependencies).toBe(true);
  });

  test("handles empty include_type array", () => {
    const backend = {
      include_path: [],
      include_type: [],
    };
    const result = GitSyncSettingsConverter.fromBackendFormat(backend);
    expect(result.skipWorkspaceDependencies).toBe(true);
    expect(result.skipScripts).toBe(true);
    expect(result.skipFlows).toBe(true);
  });

  test("converts all standard types correctly", () => {
    const backend = {
      include_path: ["f/**"],
      include_type: ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret", "schedule", "trigger", "user", "group", "settings", "key", "workspacedependencies"],
    };
    const result = GitSyncSettingsConverter.fromBackendFormat(backend);

    expect(result.skipScripts).toBe(false);
    expect(result.skipFlows).toBe(false);
    expect(result.skipApps).toBe(false);
    expect(result.skipFolders).toBe(false);
    expect(result.skipVariables).toBe(false);
    expect(result.skipResources).toBe(false);
    expect(result.skipResourceTypes).toBe(false);
    expect(result.skipSecrets).toBe(false);
    expect(result.includeSchedules).toBe(true);
    expect(result.includeTriggers).toBe(true);
    expect(result.includeUsers).toBe(true);
    expect(result.includeGroups).toBe(true);
    expect(result.includeSettings).toBe(true);
    expect(result.includeKey).toBe(true);
    expect(result.skipWorkspaceDependencies).toBe(false);
  });
});

// =============================================================================
// toBackendFormat - converts SyncOptions to backend include_type array
// =============================================================================

describe("GitSyncSettingsConverter.toBackendFormat", () => {
  test("adds workspacedependencies when skipWorkspaceDependencies is false", () => {
    const opts = {
      includes: ["f/**"],
      skipWorkspaceDependencies: false,
    };
    const result = GitSyncSettingsConverter.toBackendFormat(opts);
    expect(result.include_type).toContain("workspacedependencies");
  });

  test("does not add workspacedependencies when skipWorkspaceDependencies is true", () => {
    const opts = {
      includes: ["f/**"],
      skipWorkspaceDependencies: true,
    };
    const result = GitSyncSettingsConverter.toBackendFormat(opts);
    expect(result.include_type).not.toContain("workspacedependencies");
  });

  test("adds workspacedependencies when skipWorkspaceDependencies is undefined (defaults to false)", () => {
    const opts = {
      includes: ["f/**"],
      // skipWorkspaceDependencies not set
    };
    const normalized = GitSyncSettingsConverter.normalize(opts);
    const result = GitSyncSettingsConverter.toBackendFormat(normalized);
    expect(result.include_type).toContain("workspacedependencies");
  });

  test("converts all boolean flags to include_type correctly", () => {
    const opts = {
      includes: ["f/**"],
      skipScripts: false,
      skipFlows: false,
      skipApps: false,
      skipFolders: false,
      skipVariables: false,
      skipResources: false,
      skipResourceTypes: false,
      skipSecrets: false,
      includeSchedules: true,
      includeTriggers: true,
      includeUsers: true,
      includeGroups: true,
      includeSettings: true,
      includeKey: true,
      skipWorkspaceDependencies: false,
    };
    const result = GitSyncSettingsConverter.toBackendFormat(opts);

    expect(result.include_type).toContain("script");
    expect(result.include_type).toContain("flow");
    expect(result.include_type).toContain("app");
    expect(result.include_type).toContain("folder");
    expect(result.include_type).toContain("variable");
    expect(result.include_type).toContain("resource");
    expect(result.include_type).toContain("resourcetype");
    expect(result.include_type).toContain("secret");
    expect(result.include_type).toContain("schedule");
    expect(result.include_type).toContain("trigger");
    expect(result.include_type).toContain("user");
    expect(result.include_type).toContain("group");
    expect(result.include_type).toContain("settings");
    expect(result.include_type).toContain("key");
    expect(result.include_type).toContain("workspacedependencies");
  });
});

// =============================================================================
// normalize - applies defaults for undefined fields
// =============================================================================

describe("GitSyncSettingsConverter.normalize", () => {
  test("defaults skipWorkspaceDependencies to false", () => {
    const opts = { includes: ["f/**"] };
    const result = GitSyncSettingsConverter.normalize(opts);
    expect(result.skipWorkspaceDependencies).toBe(false);
  });

  test("preserves explicit skipWorkspaceDependencies: true", () => {
    const opts = { includes: ["f/**"], skipWorkspaceDependencies: true };
    const result = GitSyncSettingsConverter.normalize(opts);
    expect(result.skipWorkspaceDependencies).toBe(true);
  });

  test("preserves explicit skipWorkspaceDependencies: false", () => {
    const opts = { includes: ["f/**"], skipWorkspaceDependencies: false };
    const result = GitSyncSettingsConverter.normalize(opts);
    expect(result.skipWorkspaceDependencies).toBe(false);
  });
});

// =============================================================================
// Round-trip conversion tests
// =============================================================================

describe("GitSyncSettingsConverter round-trip", () => {
  test("backend -> SyncOptions -> backend preserves workspacedependencies", () => {
    const original = {
      include_path: ["f/**"],
      include_type: ["script", "flow", "workspacedependencies"],
    };

    const syncOpts = GitSyncSettingsConverter.fromBackendFormat(original);
    const backAgain = GitSyncSettingsConverter.toBackendFormat(syncOpts);

    expect(backAgain.include_type).toContain("workspacedependencies");
    expect(backAgain.include_type).toContain("script");
    expect(backAgain.include_type).toContain("flow");
  });

  test("backend without workspacedependencies -> SyncOptions -> backend still excludes it", () => {
    const original = {
      include_path: ["f/**"],
      include_type: ["script", "flow"],
    };

    const syncOpts = GitSyncSettingsConverter.fromBackendFormat(original);
    const backAgain = GitSyncSettingsConverter.toBackendFormat(syncOpts);

    expect(backAgain.include_type).not.toContain("workspacedependencies");
    expect(backAgain.include_type).toContain("script");
    expect(backAgain.include_type).toContain("flow");
  });

  test("SyncOptions with defaults -> backend includes workspacedependencies", () => {
    const opts = {
      includes: ["f/**"],
      skipScripts: false,
      skipFlows: false,
      // skipWorkspaceDependencies not set - should default to false
    };

    const normalized = GitSyncSettingsConverter.normalize(opts);
    const backend = GitSyncSettingsConverter.toBackendFormat(normalized);

    expect(backend.include_type).toContain("workspacedependencies");
  });
});

// =============================================================================
// extractGitSyncFields
// =============================================================================

describe("GitSyncSettingsConverter.extractGitSyncFields", () => {
  test("includes skipWorkspaceDependencies in extracted fields", () => {
    const opts = {
      includes: ["f/**"],
      skipWorkspaceDependencies: true,
      someOtherField: "ignored",
    };
    const result = GitSyncSettingsConverter.extractGitSyncFields(opts);
    expect(result.skipWorkspaceDependencies).toBe(true);
  });
});
