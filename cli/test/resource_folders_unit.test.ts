/**
 * Unit tests for resource_folders.ts path detection and manipulation functions.
 * Tests both dotted (.flow, .app, .raw_app) and non-dotted (__flow, __app, __raw_app) modes.
 */

import { expect, test, describe, beforeEach } from "bun:test";
import {
  setNonDottedPaths,
  getNonDottedPaths,
  getFolderSuffixes,
  getFolderSuffix,
  getMetadataFileName,
  getMetadataPathSuffix,
  isFlowPath,
  isAppPath,
  isRawAppPath,
  isFolderResourcePath,
  detectFolderResourceType,
  isRawAppBackendPath,
  isAppInlineScriptPath,
  isFlowInlineScriptPath,
  extractResourceName,
  extractFolderPath,
  buildFolderPath,
  buildMetadataPath,
  hasFolderSuffix,
  validateFolderName,
  extractNameFromFolder,
  isFlowMetadataFile,
  isAppMetadataFile,
  isRawAppMetadataFile,
  isRawAppFolderMetadataFile,
  getDeleteSuffix,
  transformJsonPathToDir,
} from "../src/utils/resource_folders.ts";
import { removeWorkerPrefix } from "../src/commands/worker-groups/worker-groups.ts";

// =============================================================================
// Helper: reset to dotted mode before each test
// =============================================================================

beforeEach(() => {
  setNonDottedPaths(false);
});

// =============================================================================
// Configuration Functions
// =============================================================================

describe("setNonDottedPaths / getNonDottedPaths", () => {
  test("defaults to false (dotted)", () => {
    expect(getNonDottedPaths()).toBe(false);
  });

  test("can be set to true", () => {
    setNonDottedPaths(true);
    expect(getNonDottedPaths()).toBe(true);
  });

  test("can be toggled back to false", () => {
    setNonDottedPaths(true);
    setNonDottedPaths(false);
    expect(getNonDottedPaths()).toBe(false);
  });
});

describe("getFolderSuffixes", () => {
  test("returns dotted suffixes by default", () => {
    const suffixes = getFolderSuffixes();
    expect(suffixes.flow).toBe(".flow");
    expect(suffixes.app).toBe(".app");
    expect(suffixes.raw_app).toBe(".raw_app");
  });

  test("returns non-dotted suffixes when configured", () => {
    setNonDottedPaths(true);
    const suffixes = getFolderSuffixes();
    expect(suffixes.flow).toBe("__flow");
    expect(suffixes.app).toBe("__app");
    expect(suffixes.raw_app).toBe("__raw_app");
  });
});

describe("getFolderSuffix", () => {
  test("returns correct suffix for each type (dotted)", () => {
    expect(getFolderSuffix("flow")).toBe(".flow");
    expect(getFolderSuffix("app")).toBe(".app");
    expect(getFolderSuffix("raw_app")).toBe(".raw_app");
  });

  test("returns correct suffix for each type (non-dotted)", () => {
    setNonDottedPaths(true);
    expect(getFolderSuffix("flow")).toBe("__flow");
    expect(getFolderSuffix("app")).toBe("__app");
    expect(getFolderSuffix("raw_app")).toBe("__raw_app");
  });
});

// =============================================================================
// Metadata File Names
// =============================================================================

describe("getMetadataFileName", () => {
  test("returns correct metadata file names", () => {
    expect(getMetadataFileName("flow", "yaml")).toBe("flow.yaml");
    expect(getMetadataFileName("flow", "json")).toBe("flow.json");
    expect(getMetadataFileName("app", "yaml")).toBe("app.yaml");
    expect(getMetadataFileName("app", "json")).toBe("app.json");
    expect(getMetadataFileName("raw_app", "yaml")).toBe("raw_app.yaml");
    expect(getMetadataFileName("raw_app", "json")).toBe("raw_app.json");
  });
});

describe("getMetadataPathSuffix", () => {
  test("returns correct path suffix (dotted)", () => {
    expect(getMetadataPathSuffix("flow", "yaml")).toBe(".flow/flow.yaml");
    expect(getMetadataPathSuffix("app", "json")).toBe(".app/app.json");
    expect(getMetadataPathSuffix("raw_app", "yaml")).toBe(".raw_app/raw_app.yaml");
  });

  test("returns correct path suffix (non-dotted)", () => {
    setNonDottedPaths(true);
    expect(getMetadataPathSuffix("flow", "yaml")).toBe("__flow/flow.yaml");
    expect(getMetadataPathSuffix("app", "json")).toBe("__app/app.json");
    expect(getMetadataPathSuffix("raw_app", "yaml")).toBe("__raw_app/raw_app.yaml");
  });
});

// =============================================================================
// Path Detection Functions (dotted mode)
// =============================================================================

describe("isFlowPath (dotted)", () => {
  test("detects flow paths", () => {
    expect(isFlowPath("f/my_flow.flow/flow.yaml")).toBe(true);
    expect(isFlowPath("u/admin/test.flow/step.ts")).toBe(true);
  });

  test("rejects non-flow paths", () => {
    expect(isFlowPath("f/my_script.ts")).toBe(false);
    expect(isFlowPath("f/my_app.app/app.yaml")).toBe(false);
  });
});

describe("isAppPath (dotted)", () => {
  test("detects app paths", () => {
    expect(isAppPath("f/my_app.app/app.yaml")).toBe(true);
    expect(isAppPath("u/admin/dashboard.app/inline.ts")).toBe(true);
  });

  test("rejects non-app paths", () => {
    expect(isAppPath("f/my_script.ts")).toBe(false);
    expect(isAppPath("f/my_flow.flow/flow.yaml")).toBe(false);
  });
});

describe("isRawAppPath (dotted)", () => {
  test("detects raw_app paths", () => {
    expect(isRawAppPath("f/my_raw.raw_app/raw_app.yaml")).toBe(true);
  });

  test("rejects non-raw_app paths", () => {
    expect(isRawAppPath("f/my_app.app/app.yaml")).toBe(false);
    expect(isRawAppPath("f/my_script.ts")).toBe(false);
  });
});

// =============================================================================
// Path Detection Functions (non-dotted mode)
// =============================================================================

describe("isFlowPath (non-dotted)", () => {
  test("detects non-dotted flow paths", () => {
    setNonDottedPaths(true);
    expect(isFlowPath("f/my_flow__flow/flow.yaml")).toBe(true);
  });

  test("rejects dotted flow paths in non-dotted mode", () => {
    setNonDottedPaths(true);
    expect(isFlowPath("f/my_flow.flow/flow.yaml")).toBe(false);
  });
});

describe("isAppPath (non-dotted)", () => {
  test("detects non-dotted app paths", () => {
    setNonDottedPaths(true);
    expect(isAppPath("f/my_app__app/app.yaml")).toBe(true);
  });
});

describe("isRawAppPath (non-dotted)", () => {
  test("detects non-dotted raw_app paths", () => {
    setNonDottedPaths(true);
    expect(isRawAppPath("f/my_raw__raw_app/raw_app.yaml")).toBe(true);
  });
});

// =============================================================================
// Composite Path Detection
// =============================================================================

describe("isFolderResourcePath", () => {
  test("returns true for any folder resource path", () => {
    expect(isFolderResourcePath("f/x.flow/flow.yaml")).toBe(true);
    expect(isFolderResourcePath("f/x.app/app.yaml")).toBe(true);
    expect(isFolderResourcePath("f/x.raw_app/raw_app.yaml")).toBe(true);
  });

  test("returns false for non-folder paths", () => {
    expect(isFolderResourcePath("f/script.ts")).toBe(false);
    expect(isFolderResourcePath("f/var.variable.yaml")).toBe(false);
  });
});

describe("detectFolderResourceType", () => {
  test("detects flow type", () => {
    expect(detectFolderResourceType("f/x.flow/flow.yaml")).toBe("flow");
  });

  test("detects app type", () => {
    expect(detectFolderResourceType("f/x.app/app.yaml")).toBe("app");
  });

  test("detects raw_app type", () => {
    expect(detectFolderResourceType("f/x.raw_app/raw_app.yaml")).toBe("raw_app");
  });

  test("returns null for non-folder paths", () => {
    expect(detectFolderResourceType("f/script.ts")).toBeNull();
  });
});

// =============================================================================
// Inline Script / Backend Path Detection
// =============================================================================

describe("isRawAppBackendPath", () => {
  test("detects raw app backend paths (dotted)", () => {
    expect(isRawAppBackendPath("f/my_app.raw_app/backend/handler.ts")).toBe(true);
  });

  test("rejects non-backend raw app paths", () => {
    expect(isRawAppBackendPath("f/my_app.raw_app/raw_app.yaml")).toBe(false);
  });

  test("detects raw app backend paths (non-dotted)", () => {
    setNonDottedPaths(true);
    expect(isRawAppBackendPath("f/my_app__raw_app/backend/handler.ts")).toBe(true);
  });
});

describe("isAppInlineScriptPath", () => {
  test("detects inline script paths in apps", () => {
    expect(isAppInlineScriptPath("f/dashboard.app/inline_0.ts")).toBe(true);
  });

  test("rejects non-app paths", () => {
    expect(isAppInlineScriptPath("f/script.ts")).toBe(false);
  });
});

describe("isFlowInlineScriptPath", () => {
  test("detects inline script paths in flows", () => {
    expect(isFlowInlineScriptPath("f/pipeline.flow/step_0.ts")).toBe(true);
  });

  test("rejects non-flow paths", () => {
    expect(isFlowInlineScriptPath("f/script.ts")).toBe(false);
  });
});

// =============================================================================
// Path Manipulation Functions
// =============================================================================

describe("extractResourceName", () => {
  test("extracts name from flow path", () => {
    expect(extractResourceName("f/my_flow.flow/flow.yaml", "flow")).toBe("f/my_flow");
  });

  test("extracts name from app path", () => {
    expect(extractResourceName("f/dashboard.app/app.yaml", "app")).toBe("f/dashboard");
  });

  test("extracts name from raw_app path", () => {
    expect(extractResourceName("f/my_raw.raw_app/raw_app.yaml", "raw_app")).toBe("f/my_raw");
  });

  test("returns null when type doesn't match", () => {
    expect(extractResourceName("f/script.ts", "flow")).toBeNull();
  });

  test("works in non-dotted mode", () => {
    setNonDottedPaths(true);
    expect(extractResourceName("f/my_flow__flow/flow.yaml", "flow")).toBe("f/my_flow");
  });
});

describe("extractFolderPath", () => {
  test("extracts folder path from flow", () => {
    expect(extractFolderPath("f/my_flow.flow/flow.yaml", "flow")).toBe("f/my_flow.flow/");
  });

  test("returns null when type doesn't match", () => {
    expect(extractFolderPath("f/script.ts", "flow")).toBeNull();
  });
});

describe("buildFolderPath", () => {
  test("builds folder path (dotted)", () => {
    expect(buildFolderPath("f/my_flow", "flow")).toBe("f/my_flow.flow");
    expect(buildFolderPath("f/dashboard", "app")).toBe("f/dashboard.app");
    expect(buildFolderPath("f/my_raw", "raw_app")).toBe("f/my_raw.raw_app");
  });

  test("builds folder path (non-dotted)", () => {
    setNonDottedPaths(true);
    expect(buildFolderPath("f/my_flow", "flow")).toBe("f/my_flow__flow");
    expect(buildFolderPath("f/dashboard", "app")).toBe("f/dashboard__app");
    expect(buildFolderPath("f/my_raw", "raw_app")).toBe("f/my_raw__raw_app");
  });
});

describe("buildMetadataPath", () => {
  test("builds metadata path (dotted, yaml)", () => {
    expect(buildMetadataPath("f/my_flow", "flow", "yaml")).toBe("f/my_flow.flow/flow.yaml");
  });

  test("builds metadata path (dotted, json)", () => {
    expect(buildMetadataPath("f/dashboard", "app", "json")).toBe("f/dashboard.app/app.json");
  });

  test("builds metadata path (non-dotted)", () => {
    setNonDottedPaths(true);
    expect(buildMetadataPath("f/my_flow", "flow", "yaml")).toBe("f/my_flow__flow/flow.yaml");
  });
});

// =============================================================================
// Folder Validation Functions
// =============================================================================

describe("hasFolderSuffix", () => {
  test("returns true for matching suffix", () => {
    expect(hasFolderSuffix("my_flow.flow", "flow")).toBe(true);
    expect(hasFolderSuffix("dashboard.app", "app")).toBe(true);
    expect(hasFolderSuffix("my_raw.raw_app", "raw_app")).toBe(true);
  });

  test("returns false for non-matching suffix", () => {
    expect(hasFolderSuffix("my_flow.app", "flow")).toBe(false);
    expect(hasFolderSuffix("script.ts", "flow")).toBe(false);
  });

  test("works in non-dotted mode", () => {
    setNonDottedPaths(true);
    expect(hasFolderSuffix("my_flow__flow", "flow")).toBe(true);
    expect(hasFolderSuffix("my_flow.flow", "flow")).toBe(false);
  });
});

describe("validateFolderName", () => {
  test("returns null for valid folder name", () => {
    expect(validateFolderName("my_flow.flow", "flow")).toBeNull();
  });

  test("returns error message for invalid folder name", () => {
    const result = validateFolderName("my_flow.app", "flow");
    expect(result).not.toBeNull();
    expect(result).toContain("my_flow.app");
    expect(result).toContain(".flow");
  });
});

describe("extractNameFromFolder", () => {
  test("extracts name by removing suffix (dotted)", () => {
    expect(extractNameFromFolder("my_flow.flow", "flow")).toBe("my_flow");
    expect(extractNameFromFolder("dashboard.app", "app")).toBe("dashboard");
    expect(extractNameFromFolder("my_raw.raw_app", "raw_app")).toBe("my_raw");
  });

  test("returns original name if suffix doesn't match", () => {
    expect(extractNameFromFolder("my_script", "flow")).toBe("my_script");
  });

  test("extracts name (non-dotted)", () => {
    setNonDottedPaths(true);
    expect(extractNameFromFolder("my_flow__flow", "flow")).toBe("my_flow");
  });
});

// =============================================================================
// Metadata File Detection Functions
// =============================================================================

describe("isFlowMetadataFile", () => {
  test("detects dotted flow metadata files", () => {
    expect(isFlowMetadataFile("f/my_flow.flow.json")).toBe(true);
    expect(isFlowMetadataFile("f/my_flow.flow.yaml")).toBe(true);
  });

  test("rejects non-flow metadata files", () => {
    expect(isFlowMetadataFile("f/my_app.app.json")).toBe(false);
    expect(isFlowMetadataFile("f/script.ts")).toBe(false);
  });

  test("detects non-dotted flow metadata files when configured", () => {
    setNonDottedPaths(true);
    expect(isFlowMetadataFile("f/my_flow__flow.json")).toBe(true);
    expect(isFlowMetadataFile("f/my_flow__flow.yaml")).toBe(true);
    // API format (dotted) is always detected
    expect(isFlowMetadataFile("f/my_flow.flow.json")).toBe(true);
  });
});

describe("isAppMetadataFile", () => {
  test("detects dotted app metadata files", () => {
    expect(isAppMetadataFile("f/dashboard.app.json")).toBe(true);
    expect(isAppMetadataFile("f/dashboard.app.yaml")).toBe(true);
  });

  test("rejects non-app metadata files", () => {
    expect(isAppMetadataFile("f/my_flow.flow.json")).toBe(false);
  });

  test("detects non-dotted app metadata files when configured", () => {
    setNonDottedPaths(true);
    expect(isAppMetadataFile("f/dashboard__app.json")).toBe(true);
    // API format always detected
    expect(isAppMetadataFile("f/dashboard.app.json")).toBe(true);
  });
});

describe("isRawAppMetadataFile", () => {
  test("detects dotted raw_app metadata files", () => {
    expect(isRawAppMetadataFile("f/my_raw.raw_app.json")).toBe(true);
    expect(isRawAppMetadataFile("f/my_raw.raw_app.yaml")).toBe(true);
  });

  test("rejects non-raw_app metadata files", () => {
    expect(isRawAppMetadataFile("f/my_app.app.json")).toBe(false);
  });

  test("detects non-dotted raw_app metadata files when configured", () => {
    setNonDottedPaths(true);
    expect(isRawAppMetadataFile("f/my_raw__raw_app.json")).toBe(true);
    expect(isRawAppMetadataFile("f/my_raw.raw_app.json")).toBe(true);
  });
});

describe("isRawAppFolderMetadataFile", () => {
  test("detects raw_app folder metadata file (dotted)", () => {
    expect(isRawAppFolderMetadataFile("f/my_raw.raw_app/raw_app.yaml")).toBe(true);
    expect(isRawAppFolderMetadataFile("f/my_raw.raw_app/raw_app.json")).toBe(true);
  });

  test("rejects non-metadata files", () => {
    expect(isRawAppFolderMetadataFile("f/my_raw.raw_app/backend/handler.ts")).toBe(false);
  });
});

// =============================================================================
// Sync-related Path Functions
// =============================================================================

describe("getDeleteSuffix", () => {
  test("returns correct delete suffix", () => {
    expect(getDeleteSuffix("flow", "yaml")).toBe(".flow/flow.yaml");
    expect(getDeleteSuffix("app", "json")).toBe(".app/app.json");
    expect(getDeleteSuffix("raw_app", "yaml")).toBe(".raw_app/raw_app.yaml");
  });

  test("returns correct delete suffix (non-dotted)", () => {
    setNonDottedPaths(true);
    expect(getDeleteSuffix("flow", "yaml")).toBe("__flow/flow.yaml");
  });
});

describe("transformJsonPathToDir", () => {
  test("transforms API dotted .flow.json to dotted dir", () => {
    expect(transformJsonPathToDir("f/my_flow.flow.json", "flow")).toBe("f/my_flow.flow");
  });

  test("transforms API dotted .app.json to dotted dir", () => {
    expect(transformJsonPathToDir("f/dashboard.app.json", "app")).toBe("f/dashboard.app");
  });

  test("transforms API dotted to non-dotted dir when configured", () => {
    setNonDottedPaths(true);
    expect(transformJsonPathToDir("f/my_flow.flow.json", "flow")).toBe("f/my_flow__flow");
  });

  test("handles already-configured format", () => {
    setNonDottedPaths(true);
    expect(transformJsonPathToDir("f/my_flow__flow.json", "flow")).toBe("f/my_flow__flow");
  });

  test("returns unchanged path when suffix doesn't match", () => {
    expect(transformJsonPathToDir("f/script.ts", "flow")).toBe("f/script.ts");
  });
});

// =============================================================================
// removeWorkerPrefix (from worker-groups.ts)
// =============================================================================

describe("removeWorkerPrefix", () => {
  test("removes worker__ prefix", () => {
    expect(removeWorkerPrefix("worker__default")).toBe("default");
    expect(removeWorkerPrefix("worker__gpu")).toBe("gpu");
  });

  test("returns name unchanged if no prefix", () => {
    expect(removeWorkerPrefix("default")).toBe("default");
    expect(removeWorkerPrefix("gpu")).toBe("gpu");
  });

  test("handles empty string", () => {
    expect(removeWorkerPrefix("")).toBe("");
  });

  test("handles worker__ as the entire name", () => {
    expect(removeWorkerPrefix("worker__")).toBe("");
  });
});
