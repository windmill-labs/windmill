import { assertEquals, assertThrows } from "https://deno.land/std@0.208.0/testing/asserts.ts";
import { UIState, uiStateToSyncOptions, parseJsonInput } from "./settings_utils.ts";

// Tests
Deno.test("uiStateToSyncOptions - basic conversion", () => {
  const uiState: UIState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder", "variable", "schedule"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.defaultTs, 'bun');
  assertEquals(syncOptions.includes, ["f/**"]);
  assertEquals(syncOptions.excludes, []);
  assertEquals(syncOptions.codebases, []);
  assertEquals(syncOptions.skipVariables, false);
  assertEquals(syncOptions.skipResources, true);
  assertEquals(syncOptions.skipSecrets, true);
  assertEquals(syncOptions.includeSchedules, true);
  assertEquals(syncOptions.includeTriggers, false);
});

Deno.test("uiStateToSyncOptions - empty include_path defaults to f/**", () => {
  const uiState: UIState = {
    include_path: [],
    include_type: ["script", "flow", "app", "folder"]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.includes, ["f/**"]);
});

Deno.test("uiStateToSyncOptions - all skip flags correctly set", () => {
  const uiState: UIState = {
    include_path: ["f/**"],
    include_type: ["script", "flow", "app", "folder"] // Only core types, no variables/resources/etc
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

Deno.test("uiStateToSyncOptions - all types included", () => {
  const uiState: UIState = {
    include_path: ["f/**", "u/**"],
    include_type: [
      "script", "flow", "app", "folder",
      "variable", "resource", "resourcetype", "secret",
      "schedule", "trigger", "user", "group", "settings", "key"
    ]
  };

  const syncOptions = uiStateToSyncOptions(uiState);

  assertEquals(syncOptions.includes, ["f/**", "u/**"]);
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

Deno.test("parseJsonInput - valid JSON", () => {
  const jsonString = '{"include_path":["f/**"],"include_type":["script","flow"]}';

  const result = parseJsonInput(jsonString);

  assertEquals(result.include_path, ["f/**"]);
  assertEquals(result.include_type, ["script", "flow"]);
});

Deno.test("parseJsonInput - invalid JSON throws error", () => {
  const invalidJson = '{"include_path":["f/**"],"include_type":["script","flow"}'; // Missing closing bracket

  assertThrows(
    () => parseJsonInput(invalidJson),
    Error,
    "Invalid JSON in settings parameter"
  );
});

Deno.test("parseJsonInput - empty object", () => {
  const jsonString = '{"include_path":[],"include_type":[]}';

  const result = parseJsonInput(jsonString);

  assertEquals(result.include_path, []);
  assertEquals(result.include_type, []);
});