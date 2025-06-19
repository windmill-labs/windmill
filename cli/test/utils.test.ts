import { assertEquals, assertNotEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { uiStateToSyncOptions, syncOptionsToUIState, parseJsonInput } from "../settings_utils.ts";

// =============================================================================
// UTILITY FUNCTION TESTS - SETTINGS AND SYNC UTILITIES
// =============================================================================

// Settings Utils Tests (from settings_unit.test.ts)
Deno.test("Utils: uiStateToSyncOptions - converts UI state to SyncOptions correctly", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret", "schedule", "trigger", "user", "group", "settings", "key"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.defaultTs, 'bun');
  assertEquals(syncOptions.includes, ['f/**']);
  assertEquals(syncOptions.skipVariables, false);
  assertEquals(syncOptions.skipResources, false);
  assertEquals(syncOptions.skipResourceTypes, false);
  assertEquals(syncOptions.skipSecrets, false);
  assertEquals(syncOptions.includeSchedules, true);
  assertEquals(syncOptions.includeTriggers, true);
  assertEquals(syncOptions.includeUsers, true);
  assertEquals(syncOptions.includeGroups, true);
  assertEquals(syncOptions.includeSettings, true);
  assertEquals(syncOptions.includeKey, true);
});

Deno.test("Utils: uiStateToSyncOptions - handles partial UI state", () => {
  const uiState = {
    include_path: ["u/**"],
    include_type: ["script", "flow"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.defaultTs, 'bun');
  assertEquals(syncOptions.includes, ['u/**']);
  assertEquals(syncOptions.skipVariables, true);
  assertEquals(syncOptions.skipResources, true);
  assertEquals(syncOptions.includeSchedules, false);
  assertEquals(syncOptions.includeTriggers, false);
  assertEquals(syncOptions.includeUsers, false);
  assertEquals(syncOptions.includeGroups, false);
  assertEquals(syncOptions.includeSettings, false);
});

Deno.test("Utils: syncOptionsToUIState - converts SyncOptions to UI state correctly", () => {
  const syncOptions = {
    defaultTs: 'bun' as const,
    includes: ['f/**'],
    excludes: [],
    codebases: [],
    skipVariables: false,
    skipResources: false,
    skipResourceTypes: false,
    skipSecrets: false,
    includeSchedules: true,
    includeTriggers: true,
    includeUsers: true,
    includeGroups: true,
    includeSettings: true,
    includeKey: true
  };

  const uiState = syncOptionsToUIState(syncOptions);

  assertEquals(uiState.include_path, ['f/**']);
  assertEquals(uiState.include_type, ['script', 'flow', 'app', 'folder', 'variable', 'resource', 'resourcetype', 'secret', 'schedule', 'trigger', 'user', 'group', 'settings', 'key']);
});

Deno.test("Utils: syncOptionsToUIState - handles skip flags correctly", () => {
  const syncOptions = {
    defaultTs: 'bun' as const,
    includes: ['f/**'],
    excludes: [],
    codebases: [],
    skipVariables: true,
    skipResources: true,
    skipResourceTypes: true,
    skipSecrets: true,
    includeSchedules: false,
    includeTriggers: false,
    includeUsers: false,
    includeGroups: false,
    includeSettings: false,
    includeKey: false
  };

  const uiState = syncOptionsToUIState(syncOptions);

  assertEquals(uiState.include_path, ['f/**']);
  assertEquals(uiState.include_type, ['script', 'flow', 'app', 'folder']);
});

// Performance tests
Deno.test("Utils: Performance - uiStateToSyncOptions conversion", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret", "schedule", "trigger", "user", "group", "settings", "key"]
  };

  const start = performance.now();
  for (let i = 0; i < 1000; i++) {
    uiStateToSyncOptions(uiState);
  }
  const end = performance.now();

  // Should complete 1000 operations in less than 100ms
  assertEquals(end - start < 100, true);
});

Deno.test("Utils: Performance - syncOptionsToUIState conversion", () => {
  const syncOptions = {
    defaultTs: 'bun' as const,
    includes: ['f/**'],
    excludes: [],
    codebases: [],
    skipVariables: false,
    skipResources: false,
    skipResourceTypes: false,
    skipSecrets: false,
    includeSchedules: true,
    includeTriggers: true,
    includeUsers: true,
    includeGroups: true,
    includeSettings: true,
    includeKey: true
  };

  const start = performance.now();
  for (let i = 0; i < 1000; i++) {
    syncOptionsToUIState(syncOptions);
  }
  const end = performance.now();

  // Should complete 1000 operations in less than 100ms
  assertEquals(end - start < 100, true);
});

// Edge case tests
Deno.test("Utils: Edge case - empty UI state", () => {
  const uiState = {
    include_path: [],
    include_type: []
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.defaultTs, 'bun');
  assertEquals(syncOptions.includes, []); // Empty array stays empty
  assertEquals(syncOptions.skipVariables, true);
  assertEquals(syncOptions.skipResources, true);
  assertEquals(syncOptions.includeSettings, false);
});

Deno.test("Utils: Edge case - UI state with only base types", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.includes, ['f/**']);
  assertEquals(syncOptions.skipVariables, true);
  assertEquals(syncOptions.skipResources, true);
  assertEquals(syncOptions.includeSchedules, false);
  assertEquals(syncOptions.includeUsers, false);
});

Deno.test("Utils: Edge case - invalid include_type values are ignored", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "invalid_type", "flow", "another_invalid"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  // Should only include valid types
  assertEquals(syncOptions.skipVariables, true);
  assertEquals(syncOptions.skipResources, true);
});

// Roundtrip tests
Deno.test("Utils: Roundtrip - uiState -> syncOptions -> uiState", () => {
  const originalUiState = {
    include_path: ["f/**", "u/**"],
    include_type: ["script", "flow", "app", "folder", "variable", "resource", "secret", "schedule", "user", "group", "settings"]
  };

  const syncOptions = uiStateToSyncOptions(originalUiState);
  const convertedUiState = syncOptionsToUIState(syncOptions);

  assertEquals(convertedUiState.include_path, originalUiState.include_path);
  // Note: include_type may have different ordering or additional base types
  assertEquals(convertedUiState.include_type.includes("script"), true);
  assertEquals(convertedUiState.include_type.includes("flow"), true);
  assertEquals(convertedUiState.include_type.includes("variable"), true);
  assertEquals(convertedUiState.include_type.includes("settings"), true);
});

Deno.test("Utils: Roundtrip - syncOptions -> uiState -> syncOptions", () => {
  const originalSyncOptions = {
    defaultTs: 'bun' as const,
    includes: ['f/**'],
    excludes: [],
    codebases: [],
    skipVariables: false,
    skipResources: false,
    skipResourceTypes: false,
    skipSecrets: false,
    includeSchedules: true,
    includeTriggers: true,
    includeUsers: true,
    includeGroups: true,
    includeSettings: true,
    includeKey: true
  };

  const uiState = syncOptionsToUIState(originalSyncOptions);
  const convertedSyncOptions = uiStateToSyncOptions(uiState);

  assertEquals(convertedSyncOptions.includes, originalSyncOptions.includes);
  assertEquals(convertedSyncOptions.skipVariables, originalSyncOptions.skipVariables);
  assertEquals(convertedSyncOptions.skipResources, originalSyncOptions.skipResources);
  assertEquals(convertedSyncOptions.includeSchedules, originalSyncOptions.includeSchedules);
  assertEquals(convertedSyncOptions.includeSettings, originalSyncOptions.includeSettings);
});

// Field mapping tests
Deno.test("Utils: Field mapping - include_path correctly maps", () => {
  const uiState = {
    include_path: ["custom/**", "another/**"],
    include_type: ["script"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);
  assertEquals(syncOptions.includes, ["custom/**", "another/**"]);
});

Deno.test("Utils: Field mapping - all skip flags work correctly", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder"] // Only base types
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.skipVariables, true);
  assertEquals(syncOptions.skipResources, true);
  assertEquals(syncOptions.skipResourceTypes, true);
  assertEquals(syncOptions.skipSecrets, true);
});

Deno.test("Utils: Field mapping - all include flags work correctly", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder", "schedule", "trigger", "user", "group", "settings", "key"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.includeSchedules, true);
  assertEquals(syncOptions.includeTriggers, true);
  assertEquals(syncOptions.includeUsers, true);
  assertEquals(syncOptions.includeGroups, true);
  assertEquals(syncOptions.includeSettings, true);
  assertEquals(syncOptions.includeKey, true);
});

// Core skip flags test
Deno.test("Utils: Core skip flags - skipScripts/skipFlows/skipApps/skipFolders", () => {
  // When include_type is empty, all types are excluded (skip = true)
  const uiState = {
    include_path: ["f/**"],
    include_type: []
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  // Empty include_type means all types are skipped
  assertEquals(syncOptions.skipScripts, true);
  assertEquals(syncOptions.skipFlows, true);
  assertEquals(syncOptions.skipApps, true);
  assertEquals(syncOptions.skipFolders, true);
});

Deno.test("Utils: Core include flags roundtrip", () => {
  const originalSyncOptions = {
    defaultTs: 'bun' as const,
    includes: ['f/**'],
    excludes: [],
    codebases: [],
    skipVariables: true,
    skipResources: true,
    skipResourceTypes: true,
    skipSecrets: true,
    includeSchedules: false,
    includeTriggers: false,
    includeUsers: false,
    includeGroups: false,
    includeSettings: false,
    includeKey: false
  };

  const uiState = syncOptionsToUIState(originalSyncOptions);
  const convertedSyncOptions = uiStateToSyncOptions(uiState);

  assertEquals(convertedSyncOptions.skipVariables, true);
  assertEquals(convertedSyncOptions.skipResources, true);
  assertEquals(convertedSyncOptions.includeSchedules, false);
  assertEquals(convertedSyncOptions.includeSettings, false);
});

// Sync Settings Tests (from sync_settings.test.ts)
Deno.test("Utils: Sync - basic conversion", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.defaultTs, 'bun');
  assertEquals(syncOptions.includes, ['f/**']);
  assertEquals(syncOptions.skipVariables, true);
  assertEquals(syncOptions.skipResources, true);
});

Deno.test("Utils: Sync - empty include_path stays empty", () => {
  const uiState = {
    include_path: [],
    include_type: ["script", "flow"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.includes, []); // Empty array stays empty
});

Deno.test("Utils: Sync - all skip flags correctly set", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.skipVariables, true);
  assertEquals(syncOptions.skipResources, true);
  assertEquals(syncOptions.skipResourceTypes, true);
  assertEquals(syncOptions.skipSecrets, true);
  assertEquals(syncOptions.includeSchedules, false);
  assertEquals(syncOptions.includeTriggers, false);
  assertEquals(syncOptions.includeUsers, false);
  assertEquals(syncOptions.includeGroups, false);
  assertEquals(syncOptions.includeSettings, false);
  assertEquals(syncOptions.includeKey, false);
});

Deno.test("Utils: Sync - all types included", () => {
  const uiState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret", "schedule", "trigger", "user", "group", "settings", "key"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.skipVariables, false);
  assertEquals(syncOptions.skipResources, false);
  assertEquals(syncOptions.skipResourceTypes, false);
  assertEquals(syncOptions.skipSecrets, false);
  assertEquals(syncOptions.includeSchedules, true);
  assertEquals(syncOptions.includeTriggers, true);
  assertEquals(syncOptions.includeUsers, true);
  assertEquals(syncOptions.includeGroups, true);
  assertEquals(syncOptions.includeSettings, true);
  assertEquals(syncOptions.includeKey, true);
});

// JSON parsing tests
Deno.test("Utils: parseJsonInput - valid JSON", () => {
  const jsonString = '{"include_path": ["f/**"], "include_type": ["script", "flow"]}';
  const result = parseJsonInput(jsonString);

  assertEquals(result.include_path, ["f/**"]);
  assertEquals(result.include_type, ["script", "flow"]);
});

Deno.test("Utils: parseJsonInput - invalid JSON throws error", () => {
  const invalidJson = '{"include_path": ["f/**"], "include_type": [invalid]}';
  
  let errorThrown = false;
  try {
    parseJsonInput(invalidJson);
  } catch (error) {
    errorThrown = true;
    assertEquals((error as Error).message.includes("Invalid JSON"), true);
  }
  
  assertEquals(errorThrown, true);
});

Deno.test("Utils: parseJsonInput - empty object", () => {
  const emptyJson = '{}';
  const result = parseJsonInput(emptyJson);

  assertEquals(typeof result, 'object');
  assertEquals(result !== null, true);
});