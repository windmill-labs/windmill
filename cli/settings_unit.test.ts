import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import type { SyncOptions } from "./conf.ts";
import { UIState, uiStateToSyncOptions, syncOptionsToUIState } from "./settings_utils.ts";

// Mock data for tests
const mockUIState: UIState = {
  include_path: ["f/**"],
  include_type: ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret", "schedule", "trigger", "user", "group", "settings", "key"]
};

const mockSyncOptions: SyncOptions = {
  defaultTs: 'bun',
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

const mockPartialUIState: UIState = {
  include_path: ["f/**"],
  include_type: ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret"]
};

// Unit tests
Deno.test("uiStateToSyncOptions - converts UI state to SyncOptions correctly", () => {
  const result = uiStateToSyncOptions(mockUIState);

  assertEquals(result.defaultTs, 'bun');
  assertEquals(result.includes, ['f/**']);
  assertEquals(result.excludes, []);
  assertEquals(result.codebases, []);
  assertEquals(result.skipVariables, false);
  assertEquals(result.skipResources, false);
  assertEquals(result.skipResourceTypes, false);
  assertEquals(result.skipSecrets, false);
  assertEquals(result.includeSchedules, true);
  assertEquals(result.includeTriggers, true);
  assertEquals(result.includeUsers, true);
  assertEquals(result.includeGroups, true);
  assertEquals(result.includeSettings, true);
  assertEquals(result.includeKey, true);
});

Deno.test("uiStateToSyncOptions - handles partial UI state", () => {
  const result = uiStateToSyncOptions(mockPartialUIState);

  assertEquals(result.skipVariables, false);
  assertEquals(result.skipResources, false);
  assertEquals(result.skipResourceTypes, false);
  assertEquals(result.skipSecrets, false);
  assertEquals(result.includeSchedules, false); // Not in include_type
  assertEquals(result.includeTriggers, false); // Not in include_type
  assertEquals(result.includeUsers, false); // Not in include_type
  assertEquals(result.includeGroups, false); // Not in include_type
  assertEquals(result.includeSettings, false); // Not in include_type
  assertEquals(result.includeKey, false); // Not in include_type
});

Deno.test("syncOptionsToUIState - converts SyncOptions to UI state correctly", () => {
  const result = syncOptionsToUIState(mockSyncOptions);

  assertEquals(result.include_path, ['f/**']);
  assertEquals(result.include_type.includes('script'), true);
  assertEquals(result.include_type.includes('flow'), true);
  assertEquals(result.include_type.includes('app'), true);
  assertEquals(result.include_type.includes('folder'), true);
  assertEquals(result.include_type.includes('variable'), true);
  assertEquals(result.include_type.includes('resource'), true);
  assertEquals(result.include_type.includes('resourcetype'), true);
  assertEquals(result.include_type.includes('secret'), true);
  assertEquals(result.include_type.includes('schedule'), true);
  assertEquals(result.include_type.includes('trigger'), true);
  assertEquals(result.include_type.includes('user'), true);
  assertEquals(result.include_type.includes('group'), true);
  assertEquals(result.include_type.includes('settings'), true);
  assertEquals(result.include_type.includes('key'), true);
});

Deno.test("syncOptionsToUIState - handles skip flags correctly", () => {
  const optionsWithSkips: SyncOptions = {
    ...mockSyncOptions,
    skipVariables: true,
    skipResources: true,
    includeSchedules: false,
    includeTriggers: false
  };

  const result = syncOptionsToUIState(optionsWithSkips);

  assertEquals(result.include_type.includes('variable'), false);
  assertEquals(result.include_type.includes('resource'), false);
  assertEquals(result.include_type.includes('schedule'), false);
  assertEquals(result.include_type.includes('trigger'), false);
});

// Performance tests
Deno.test("Performance: uiStateToSyncOptions conversion", () => {
  const iterations = 1000;
  const start = performance.now();

  for (let i = 0; i < iterations; i++) {
    uiStateToSyncOptions(mockUIState);
  }

  const end = performance.now();
  const duration = end - start;

  // Should complete 1000 conversions in under 100ms
  assertEquals(duration < 100, true, `Conversion took ${duration}ms for ${iterations} iterations`);
});

Deno.test("Performance: syncOptionsToUIState conversion", () => {
  const iterations = 1000;
  const start = performance.now();

  for (let i = 0; i < iterations; i++) {
    syncOptionsToUIState(mockSyncOptions);
  }

  const end = performance.now();
  const duration = end - start;

  // Should complete 1000 conversions in under 100ms
  assertEquals(duration < 100, true, `Conversion took ${duration}ms for ${iterations} iterations`);
});

// Edge case tests
Deno.test("Edge case: empty UI state", () => {
  const emptyUIState: UIState = {
    include_path: [],
    include_type: []
  };

  const result = uiStateToSyncOptions(emptyUIState);

  assertEquals(result.includes, ['f/**']); // Should default to ['f/**']
  assertEquals(result.skipVariables, true); // Should skip everything when not included
  assertEquals(result.includeSchedules, false);
});

Deno.test("Edge case: UI state with only base types", () => {
  const baseOnlyUIState: UIState = {
    include_path: ["custom/**"],
    include_type: ["script", "flow", "app", "folder"]
  };

  const result = uiStateToSyncOptions(baseOnlyUIState);

  assertEquals(result.includes, ['custom/**']);
  assertEquals(result.skipVariables, true);
  assertEquals(result.skipResources, true);
  assertEquals(result.skipResourceTypes, true);
  assertEquals(result.skipSecrets, true);
  assertEquals(result.includeSchedules, false);
  assertEquals(result.includeTriggers, false);
  assertEquals(result.includeUsers, false);
  assertEquals(result.includeGroups, false);
  assertEquals(result.includeSettings, false);
  assertEquals(result.includeKey, false);
});

Deno.test("Edge case: invalid include_type values are ignored", () => {
  const invalidUIState: UIState = {
    include_path: ["f/**"],
    include_type: ["script", "invalid_type", "flow", "another_invalid"]
  };

  const result = uiStateToSyncOptions(invalidUIState);

  // Should only process valid types
  assertEquals(result.skipVariables, true); // 'variable' not included
  assertEquals(result.includeSchedules, false); // 'schedule' not included
});

// Roundtrip tests - make sure conversion is bidirectional
Deno.test("Roundtrip: uiState -> syncOptions -> uiState", () => {
  const originalUIState = mockUIState;
  const syncOptions = uiStateToSyncOptions(originalUIState);
  const resultUIState = syncOptionsToUIState(syncOptions);

  assertEquals(resultUIState.include_path, originalUIState.include_path);
  // Sort both arrays to ensure consistent comparison
  const originalTypes = [...originalUIState.include_type].sort();
  const resultTypes = [...resultUIState.include_type].sort();
  assertEquals(resultTypes, originalTypes);
});

Deno.test("Roundtrip: syncOptions -> uiState -> syncOptions", () => {
  const originalSyncOptions = mockSyncOptions;
  const uiState = syncOptionsToUIState(originalSyncOptions);
  const resultSyncOptions = uiStateToSyncOptions(uiState);

  assertEquals(resultSyncOptions.includes, originalSyncOptions.includes);
  assertEquals(resultSyncOptions.skipVariables, originalSyncOptions.skipVariables);
  assertEquals(resultSyncOptions.skipResources, originalSyncOptions.skipResources);
  assertEquals(resultSyncOptions.skipResourceTypes, originalSyncOptions.skipResourceTypes);
  assertEquals(resultSyncOptions.skipSecrets, originalSyncOptions.skipSecrets);
  assertEquals(resultSyncOptions.includeSchedules, originalSyncOptions.includeSchedules);
  assertEquals(resultSyncOptions.includeTriggers, originalSyncOptions.includeTriggers);
  assertEquals(resultSyncOptions.includeUsers, originalSyncOptions.includeUsers);
  assertEquals(resultSyncOptions.includeGroups, originalSyncOptions.includeGroups);
  assertEquals(resultSyncOptions.includeSettings, originalSyncOptions.includeSettings);
  assertEquals(resultSyncOptions.includeKey, originalSyncOptions.includeKey);
});

// Test specific field mappings
Deno.test("Field mapping: include_path correctly maps", () => {
  const customPaths = ["app/**", "scripts/**", "flows/**"];
  const uiState: UIState = {
    include_path: customPaths,
    include_type: ["script", "flow"]
  };

  const result = uiStateToSyncOptions(uiState);
  assertEquals(result.includes, customPaths);
});

Deno.test("Field mapping: all skip flags work correctly", () => {
  const baseUIState: UIState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder"] // Only base types, should skip everything else
  };

  const result = uiStateToSyncOptions(baseUIState);

  assertEquals(result.skipVariables, true);
  assertEquals(result.skipResources, true);
  assertEquals(result.skipResourceTypes, true);
  assertEquals(result.skipSecrets, true);

  // New core skip flags should be applied (they default to include when not specified)
  assertEquals(result.skipScripts, false);
  assertEquals(result.skipFlows, false);
  assertEquals(result.skipApps, false);
  assertEquals(result.skipFolders, false);
});

Deno.test("Field mapping: all include flags work correctly", () => {
  const fullUIState: UIState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder", "schedule", "trigger", "user", "group", "settings", "key"]
  };

  const result = uiStateToSyncOptions(fullUIState);

  assertEquals(result.includeSchedules, true);
  assertEquals(result.includeTriggers, true);
  assertEquals(result.includeUsers, true);
  assertEquals(result.includeGroups, true);
  assertEquals(result.includeSettings, true);
  assertEquals(result.includeKey, true);
});

Deno.test("Core skip flags: skipScripts/skipFlows/skipApps/skipFolders", () => {
  const uiState: UIState = {
    include_path: ["f/**"],
    include_type: ["variable"] // exclude core types on purpose
  };

  const result = uiStateToSyncOptions(uiState);

  // Core types not included means skip flags should be true
  assertEquals(result.skipScripts, true);
  assertEquals(result.skipFlows, true);
  assertEquals(result.skipApps, true);
  assertEquals(result.skipFolders, true);
});

Deno.test("Core include flags roundtrip", () => {
  const options: SyncOptions = {
    ...mockSyncOptions,
    skipScripts: true,
    skipFlows: true,
    skipApps: false,
    skipFolders: false,
  };

  const uiState = syncOptionsToUIState(options);

  // scripts & flows should be absent, apps & folders present
  assertEquals(uiState.include_type.includes("script"), false);
  assertEquals(uiState.include_type.includes("flow"), false);
  assertEquals(uiState.include_type.includes("app"), true);
  assertEquals(uiState.include_type.includes("folder"), true);

  // convert back
  const roundtrip = uiStateToSyncOptions(uiState);
  assertEquals(roundtrip.skipScripts, true);
  assertEquals(roundtrip.skipFlows, true);
  assertEquals(roundtrip.skipApps, false);
  assertEquals(roundtrip.skipFolders, false);
});