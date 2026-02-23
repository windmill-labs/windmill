import { expect, test } from "bun:test";

// Import the function we need to test
import { elementsToMap } from "../src/commands/sync/sync.ts";
import type { SpecificItemsConfig } from "../src/core/specific_items.ts";

// =============================================================================
// elementsToMap TESTS FOR BRANCH-SPECIFIC ITEMS
// Tests for the regression where remote base files were incorrectly skipped
// when configured as branch-specific, causing them to be marked for deletion
// on pull operations.
//
// Regression: PR #7643 (commit 287b7e7d9, Jan 21, 2026)
// =============================================================================

/**
 * Mock DynFSElement implementation for testing
 */
interface MockFile {
  path: string;
  content: string;
  isDirectory?: boolean;
}

function createMockDynFSElement(files: MockFile[]): {
  isDirectory: boolean;
  path: string;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<{
    isDirectory: boolean;
    path: string;
    getContentText(): Promise<string>;
    getChildren(): AsyncIterable<unknown>;
  }>;
} {
  return {
    isDirectory: true,
    path: "",
    async getContentText() {
      return "";
    },
    async *getChildren() {
      for (const file of files) {
        yield {
          isDirectory: file.isDirectory ?? false,
          path: file.path,
          async getContentText() {
            return file.content;
          },
          async *getChildren() {
            // No children for files
          },
        };
      }
    },
  };
}

const noIgnore = () => false;
const defaultSkips = {};

// =============================================================================
// REGRESSION TEST: Remote base files should NOT be skipped
// =============================================================================

test("elementsToMap: remote base file is NOT skipped when configured as branch-specific (isRemote=true)", async () => {
  // This is the key regression test.
  // When pulling from remote, the workspace only has base paths (e.g., TestVar.variable.yaml)
  // These should NOT be skipped even if configured as branch-specific, because the remote
  // workspace doesn't have branch-specific file naming.

  const config: SpecificItemsConfig = {
    variables: ["f/Shared/Variable/**"],
  };

  const remoteFiles: MockFile[] = [
    {
      path: "f/Shared/Variable/TestVar.variable.yaml",
      content: "value: test\nis_secret: false",
    },
  ];

  const mockElement = createMockDynFSElement(remoteFiles);

  // When isRemote=true, base files should be included even if they match branch-specific config
  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging", // branchOverride
    true, // isRemote = true
  );

  // The base file should be in the map
  expect(Object.keys(result).includes("f/Shared/Variable/TestVar.variable.yaml")).toEqual(true);
});

test("elementsToMap: local base file IS skipped when configured as branch-specific (isRemote=false)", async () => {
  // When processing local files, if a base file is configured as branch-specific,
  // it should be skipped because we expect the branch-specific version to be used instead.

  const config: SpecificItemsConfig = {
    variables: ["f/Shared/Variable/**"],
  };

  const localFiles: MockFile[] = [
    {
      path: "f/Shared/Variable/TestVar.variable.yaml",
      content: "value: test\nis_secret: false",
    },
  ];

  const mockElement = createMockDynFSElement(localFiles);

  // When isRemote=false, base files should be skipped if configured as branch-specific
  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging", // branchOverride
    false, // isRemote = false
  );

  // The base file should NOT be in the map (skipped because branch-specific expected)
  expect(Object.keys(result).includes("f/Shared/Variable/TestVar.variable.yaml")).toEqual(false);
});

test("elementsToMap: local branch-specific file is mapped to base path (isRemote=false)", async () => {
  // When processing local files with branch-specific naming, they should be mapped to base paths

  const config: SpecificItemsConfig = {
    variables: ["f/Shared/Variable/**"],
  };

  const localFiles: MockFile[] = [
    {
      path: "f/Shared/Variable/TestVar.staging.variable.yaml",
      content: "value: staging-test\nis_secret: false",
    },
  ];

  const mockElement = createMockDynFSElement(localFiles);

  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging", // branchOverride
    false, // isRemote = false
  );

  // The branch-specific file should be mapped to the base path
  expect(Object.keys(result).includes("f/Shared/Variable/TestVar.variable.yaml")).toEqual(true);
  expect(result["f/Shared/Variable/TestVar.variable.yaml"]).toEqual("value: staging-test\nis_secret: false");
});

// =============================================================================
// PULL SCENARIO: Remote (base path) vs Local (branch-specific path)
// This simulates the actual pull scenario where:
// - Remote has: f/Shared/Variable/TestVar.variable.yaml
// - Local has: f/Shared/Variable/TestVar.staging.variable.yaml
// - Expected: No deletion, the files should match
// =============================================================================

test("elementsToMap: pull scenario - remote and local maps should align correctly", async () => {
  const config: SpecificItemsConfig = {
    variables: ["f/Shared/Variable/**"],
  };

  // Remote workspace has base path
  const remoteFiles: MockFile[] = [
    {
      path: "f/Shared/Variable/TestVar.variable.yaml",
      content: "value: test\nis_secret: false",
    },
  ];

  // Local has branch-specific path
  const localFiles: MockFile[] = [
    {
      path: "f/Shared/Variable/TestVar.staging.variable.yaml",
      content: "value: staging-test\nis_secret: false",
    },
  ];

  const remoteElement = createMockDynFSElement(remoteFiles);
  const localElement = createMockDynFSElement(localFiles);

  // Process remote (isRemote=true)
  const remoteMap = await elementsToMap(
    remoteElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging",
    true, // isRemote
  );

  // Process local (isRemote=false)
  const localMap = await elementsToMap(
    localElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging",
    false, // isRemote
  );

  // Both maps should have the same base path key
  const remoteKeys = Object.keys(remoteMap);
  const localKeys = Object.keys(localMap);

  expect(remoteKeys.includes("f/Shared/Variable/TestVar.variable.yaml")).toEqual(true);
  expect(localKeys.includes("f/Shared/Variable/TestVar.variable.yaml")).toEqual(true);
});

// =============================================================================
// NON-CONFIGURED ITEMS: Should work the same regardless of isRemote
// =============================================================================

test("elementsToMap: non-configured items included regardless of isRemote", async () => {
  const config: SpecificItemsConfig = {
    variables: ["f/Other/**"], // Only "Other" folder is branch-specific
  };

  const files: MockFile[] = [
    {
      path: "f/Shared/Variable/TestVar.variable.yaml",
      content: "value: test\nis_secret: false",
    },
  ];

  const mockElement = createMockDynFSElement(files);

  // Test with isRemote=true
  const remoteResult = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging",
    true,
  );

  // Test with isRemote=false
  const localResult = await elementsToMap(
    createMockDynFSElement(files),
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging",
    false,
  );

  // Both should include the file since it's not in the branch-specific config
  expect(Object.keys(remoteResult).includes("f/Shared/Variable/TestVar.variable.yaml")).toEqual(true);
  expect(Object.keys(localResult).includes("f/Shared/Variable/TestVar.variable.yaml")).toEqual(true);
});

// =============================================================================
// RESOURCE TYPE TESTS
// =============================================================================

test("elementsToMap: remote resource base file not skipped when configured", async () => {
  const config: SpecificItemsConfig = {
    resources: ["f/db/**"],
  };

  const remoteFiles: MockFile[] = [
    {
      path: "f/db/connection.resource.yaml",
      content: "value: { host: localhost }",
    },
  ];

  const mockElement = createMockDynFSElement(remoteFiles);

  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging",
    true, // isRemote
  );

  expect(Object.keys(result).includes("f/db/connection.resource.yaml")).toEqual(true);
});

// =============================================================================
// TRIGGER TYPE TESTS
// =============================================================================

test("elementsToMap: remote trigger base file not skipped when configured", async () => {
  const config: SpecificItemsConfig = {
    triggers: ["f/webhooks/**"],
  };

  const remoteFiles: MockFile[] = [
    {
      path: "f/webhooks/handler.http_trigger.yaml",
      content: "path: /webhook",
    },
  ];

  const mockElement = createMockDynFSElement(remoteFiles);

  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    { includeTriggers: true }, // Must include triggers explicitly
    config,
    "staging",
    true, // isRemote
  );

  expect(Object.keys(result).includes("f/webhooks/handler.http_trigger.yaml")).toEqual(true);
});

// =============================================================================
// SETTINGS TYPE TESTS
// =============================================================================

test("elementsToMap: remote settings.yaml not skipped when configured", async () => {
  const config: SpecificItemsConfig = {
    settings: true,
  };

  const remoteFiles: MockFile[] = [
    {
      path: "settings.yaml",
      content: "openai_resource_path: null",
    },
  ];

  const mockElement = createMockDynFSElement(remoteFiles);

  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    { includeSettings: true },
    config,
    "staging",
    true, // isRemote
  );

  expect(Object.keys(result).includes("settings.yaml")).toEqual(true);
});

// =============================================================================
// FOLDER TYPE TESTS
// =============================================================================

test("elementsToMap: remote folder meta not skipped when configured", async () => {
  const config: SpecificItemsConfig = {
    folders: ["f/env_*"],
  };

  const remoteFiles: MockFile[] = [
    {
      path: "f/env_staging/folder.meta.yaml",
      content: "display_name: Staging Environment",
    },
  ];

  const mockElement = createMockDynFSElement(remoteFiles);

  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging",
    true, // isRemote
  );

  expect(Object.keys(result).includes("f/env_staging/folder.meta.yaml")).toEqual(true);
});

// =============================================================================
// BACKWARD COMPATIBILITY: isRemote undefined behaves like local (false)
// =============================================================================

test("elementsToMap: isRemote undefined behaves like local (backward compatible)", async () => {
  const config: SpecificItemsConfig = {
    variables: ["f/**"],
  };

  const files: MockFile[] = [
    {
      path: "f/test.variable.yaml",
      content: "value: test\nis_secret: false",
    },
  ];

  const mockElement = createMockDynFSElement(files);

  // When isRemote is undefined (backward compatibility), it should behave like local
  const result = await elementsToMap(
    mockElement,
    noIgnore,
    false,
    defaultSkips,
    config,
    "staging",
    // isRemote omitted
  );

  // Base file should be skipped (same behavior as isRemote=false)
  expect(Object.keys(result).includes("f/test.variable.yaml")).toEqual(false);
});
