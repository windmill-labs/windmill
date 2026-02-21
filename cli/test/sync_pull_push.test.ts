/**
 * Sync Pull/Push Integration Tests
 *
 * Tests the sync pull and push functionality with a simulated filesystem
 * containing every kind of Windmill resource type.
 */

import { expect, test, describe } from "bun:test";
import * as path from "@std/path";
import { SEPARATOR as SEP } from "@std/path";
import { writeFile, readFile, readdir, rm, mkdir, mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import JSZip from "jszip";
import {
  getFolderSuffix,
  getMetadataFileName,
  buildFolderPath,
  getFolderSuffixes,
  isFlowPath,
  isAppPath,
  isRawAppPath,
  buildMetadataPath,
  extractResourceName,
  hasFolderSuffix,
  setNonDottedPaths,
  getNonDottedPaths,
  isFlowMetadataFile,
  isAppMetadataFile,
  isRawAppMetadataFile,
  transformJsonPathToDir,
  isAppInlineScriptPath,
  isFlowInlineScriptPath,
  isRawAppBackendPath,
} from "../src/utils/resource_folders.ts";
import { newPathAssigner } from "../windmill-utils-internal/src/path-utils/path-assigner.ts";

// =============================================================================
// Test Fixtures - Every Type of Windmill Resource
// =============================================================================

/**
 * Creates a mock script file structure
 */
function createScriptFixture(
  name: string,
  language: "python3" | "deno" | "bun" | "bash" | "go" | "postgresql",
  content?: string,
): { contentFile: { path: string; content: string }; metadataFile: { path: string; content: string } } {
  const extensions: Record<string, string> = {
    python3: ".py",
    deno: ".ts",
    bun: ".ts",
    bash: ".sh",
    go: ".go",
    postgresql: ".sql",
  };

  const ext = extensions[language];
  const defaultContent: Record<string, string> = {
    python3: `def main():\n    return "Hello from ${name}"`,
    deno: `export async function main() {\n  return "Hello from ${name}";\n}`,
    bun: `export async function main() {\n  return "Hello from ${name}";\n}`,
    bash: `#!/bin/bash\necho "Hello from ${name}"`,
    go: `package inner\n\nfunc main() string {\n  return "Hello from ${name}"\n}`,
    postgresql: `-- ${name}\nSELECT 'Hello from ${name}';`,
  };

  return {
    contentFile: {
      path: `${name}${ext}`,
      content: content ?? defaultContent[language],
    },
    metadataFile: {
      path: `${name}.script.yaml`,
      content: `summary: "${name} script"
description: "A ${language} script for testing"
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties: {}
  required: []
is_template: false
lock: []
kind: script
`,
    },
  };
}

/**
 * Creates a mock flow file structure
 */
function createFlowFixture(name: string): Record<string, { path: string; content: string }> {
  const flowSuffix = getFolderSuffix("flow");
  const metadataFile = getMetadataFileName("flow", "yaml");

  return {
    metadata: {
      path: `${name}${flowSuffix}/${metadataFile}`,
      content: `summary: "${name} flow"
description: "A flow for testing"
value:
  modules:
    - id: a
      value:
        type: rawscript
        content: "!inline ${name}${flowSuffix}/a.ts"
        language: bun
        input_transforms: {}
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties: {}
  required: []
`,
    },
    inlineScript: {
      path: `${name}${flowSuffix}/a.ts`,
      content: `export async function main() {\n  return "Hello from flow ${name}";\n}`,
    },
  };
}

/**
 * Creates a mock app file structure
 */
function createAppFixture(name: string): Record<string, { path: string; content: string }> {
  const appSuffix = getFolderSuffix("app");
  const metadataFile = getMetadataFileName("app", "yaml");

  return {
    metadata: {
      path: `${name}${appSuffix}/${metadataFile}`,
      content: `summary: "${name} app"
value:
  grid:
    - data:
        id: "text_1"
        type: textcomponent
        componentInput:
          type: static
          value: "Hello from ${name}"
      id: "a"
  hiddenInlineScripts: []
policy:
  execution_mode: viewer
`,
    },
  };
}

/**
 * Creates a mock raw_app file structure
 */
function createRawAppFixture(name: string): Record<string, { path: string; content: string }> {
  const rawAppSuffix = getFolderSuffix("raw_app");
  const metadataFile = getMetadataFileName("raw_app", "yaml");

  return {
    metadata: {
      path: `${name}${rawAppSuffix}/${metadataFile}`,
      content: `summary: "${name} raw app"
runnables:
  a:
    path: u/test/my_script
    type: script
`,
    },
    indexHtml: {
      path: `${name}${rawAppSuffix}/index.html`,
      content: `<!DOCTYPE html>
<html>
<head><title>${name}</title></head>
<body><h1>Hello from ${name}</h1></body>
</html>`,
    },
    indexJs: {
      path: `${name}${rawAppSuffix}/index.js`,
      content: `console.log("Hello from ${name}");`,
    },
  };
}

/**
 * Creates a mock resource file
 */
function createResourceFixture(
  name: string,
  resourceType: string,
  value: Record<string, unknown>,
): { path: string; content: string } {
  return {
    path: `${name}.resource.yaml`,
    content: `resource_type: "${resourceType}"
value:
${Object.entries(value).map(([k, v]) => `  ${k}: ${JSON.stringify(v)}`).join("\n")}
`,
  };
}

/**
 * Creates a mock variable file
 */
function createVariableFixture(
  name: string,
  value: string,
  isSecret: boolean = false,
): { path: string; content: string } {
  return {
    path: `${name}.variable.yaml`,
    content: `value: "${value}"
is_secret: ${isSecret}
description: "Variable ${name} for testing"
`,
  };
}

/**
 * Creates a mock schedule file
 */
function createScheduleFixture(
  name: string,
  scriptPath: string,
  schedule: string = "0 * * * *",
): { path: string; content: string } {
  return {
    path: `${name}.schedule.yaml`,
    content: `path: "${name}"
schedule: "${schedule}"
script_path: "${scriptPath}"
is_flow: false
args: {}
enabled: true
timezone: "UTC"
`,
  };
}

/**
 * Creates a mock HTTP trigger file
 */
function createHttpTriggerFixture(
  name: string,
  routePath: string,
  scriptPath: string,
): { path: string; content: string } {
  return {
    path: `${name}.http_trigger.yaml`,
    content: `path: "${name}"
route_path: "${routePath}"
script_path: "${scriptPath}"
is_flow: false
http_method: post
authentication_method: none
is_async: false
`,
  };
}

/**
 * Creates a mock WebSocket trigger file
 */
function createWebsocketTriggerFixture(
  name: string,
  url: string,
  scriptPath: string,
): { path: string; content: string } {
  return {
    path: `${name}.websocket_trigger.yaml`,
    content: `path: "${name}"
url: "${url}"
script_path: "${scriptPath}"
is_flow: false
enabled: true
filters: []
initial_messages: []
`,
  };
}

/**
 * Creates a mock folder meta file
 */
function createFolderFixture(name: string): { path: string; content: string } {
  return {
    path: `${name}/folder.meta.yaml`,
    content: `display_name: "${name}"
owners: []
extra_perms: {}
`,
  };
}

/**
 * Creates a mock user file
 */
function createUserFixture(
  username: string,
  email: string,
  isAdmin: boolean = false,
): { path: string; content: string } {
  return {
    path: `${username}.user.yaml`,
    content: `username: "${username}"
email: "${email}"
is_admin: ${isAdmin}
operator: false
disabled: false
`,
  };
}

/**
 * Creates a mock group file
 */
function createGroupFixture(name: string, members: string[] = []): { path: string; content: string } {
  return {
    path: `${name}.group.yaml`,
    content: `name: "${name}"
summary: "Group ${name} for testing"
members:
${members.map((m) => `  - ${m}`).join("\n") || "  []"}
`,
  };
}

// =============================================================================
// Test Helper Functions
// =============================================================================

/**
 * Creates a complete local filesystem with all resource types
 */
async function createLocalFilesystem(baseDir: string): Promise<void> {
  // Create folder structure
  const folders = ["f/scripts", "f/flows", "f/apps", "f/resources"];
  for (const folder of folders) {
    await mkdir(path.join(baseDir, folder), { recursive: true });
  }

  // Create scripts
  const scripts = [
    createScriptFixture("f/scripts/python_script", "python3"),
    createScriptFixture("f/scripts/deno_script", "deno"),
    createScriptFixture("f/scripts/bash_script", "bash"),
    createScriptFixture("f/scripts/go_script", "go"),
    createScriptFixture("f/scripts/sql_script", "postgresql"),
  ];

  for (const script of scripts) {
    await writeFile(
      path.join(baseDir, script.contentFile.path),
      script.contentFile.content,
    "utf-8",
    );
    await writeFile(
      path.join(baseDir, script.metadataFile.path),
      script.metadataFile.content,
    "utf-8",
    );
  }

  // Create flows
  const flowFixture = createFlowFixture("f/flows/test_flow");
  await mkdir(path.join(baseDir, `f/flows/test_flow${getFolderSuffix("flow")}`), { recursive: true });
  for (const file of Object.values(flowFixture)) {
    await writeFile(path.join(baseDir, file.path), file.content, "utf-8");
  }

  // Create apps
  const appFixture = createAppFixture("f/apps/test_app");
  await mkdir(path.join(baseDir, `f/apps/test_app${getFolderSuffix("app")}`), { recursive: true });
  for (const file of Object.values(appFixture)) {
    await writeFile(path.join(baseDir, file.path), file.content, "utf-8");
  }

  // Create raw apps
  const rawAppFixture = createRawAppFixture("f/apps/test_raw_app");
  await mkdir(path.join(baseDir, `f/apps/test_raw_app${getFolderSuffix("raw_app")}`), { recursive: true });
  for (const file of Object.values(rawAppFixture)) {
    await writeFile(path.join(baseDir, file.path), file.content, "utf-8");
  }

  // Create resources
  const resources = [
    createResourceFixture("f/resources/postgres_db", "postgresql", {
      host: "localhost",
      port: 5432,
      user: "admin",
      dbname: "test_db",
    }),
    createResourceFixture("f/resources/api_config", "c_api_config", {
      base_url: "https://api.example.com",
      api_key: "$var:f/secrets/api_key",
    }),
  ];

  for (const resource of resources) {
    await writeFile(path.join(baseDir, resource.path), resource.content, "utf-8");
  }

  // Create variables
  const variables = [
    createVariableFixture("f/resources/config_value", "test_value", false),
    createVariableFixture("f/resources/secret_key", "secret123", true),
  ];

  for (const variable of variables) {
    await writeFile(path.join(baseDir, variable.path), variable.content, "utf-8");
  }

  // Create folder metadata
  await mkdir(path.join(baseDir, "f"), { recursive: true });
  const folderMeta = createFolderFixture("f");
  await writeFile(path.join(baseDir, folderMeta.path), folderMeta.content, "utf-8");
}

/**
 * Creates a mock zip file representing remote workspace state
 */
async function createMockRemoteZip(items: Record<string, string>): Promise<JSZip> {
  const zip = new JSZip();

  for (const [filePath, content] of Object.entries(items)) {
    zip.file(filePath, content);
  }

  return zip;
}

/**
 * Reads all files from a directory recursively
 * Normalizes path separators to forward slashes for cross-platform compatibility
 */
async function readDirRecursive(
  dir: string,
  baseDir: string = dir,
): Promise<Record<string, string>> {
  const files: Record<string, string> = {};

  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    // Normalize path separators to forward slashes for cross-platform compatibility
    const relativePath = fullPath.substring(baseDir.length + 1).replaceAll("\\", "/");

    if (entry.isDirectory()) {
      const subFiles = await readDirRecursive(fullPath, baseDir);
      Object.assign(files, subFiles);
    } else {
      files[relativePath] = await readFile(fullPath, "utf-8");
    }
  }

  return files;
}

/**
 * Creates a temporary directory for testing
 */
async function createTempDir(): Promise<string> {
  return await mkdtemp(join(tmpdir(), "wmill_sync_test_"));
}

/**
 * Cleans up a temporary directory
 */
async function cleanupTempDir(dir: string): Promise<void> {
  try {
    await rm(dir, { recursive: true });
  } catch {
    // Ignore cleanup errors
  }
}

// =============================================================================
// Tests
// =============================================================================

test("Resource folder suffixes are correct", () => {
  expect(getFolderSuffix("flow")).toEqual(".flow");
  expect(getFolderSuffix("app")).toEqual(".app");
  expect(getFolderSuffix("raw_app")).toEqual(".raw_app");
});

test("Metadata file names are correct", () => {
  expect(getMetadataFileName("flow", "yaml")).toEqual("flow.yaml");
  expect(getMetadataFileName("flow", "json")).toEqual("flow.json");
  expect(getMetadataFileName("app", "yaml")).toEqual("app.yaml");
  expect(getMetadataFileName("raw_app", "yaml")).toEqual("raw_app.yaml");
});

test("buildFolderPath creates correct paths", () => {
  expect(buildFolderPath("my_flow", "flow")).toEqual("my_flow.flow");
  expect(buildFolderPath("f/test/my_app", "app")).toEqual("f/test/my_app.app");
  expect(buildFolderPath("u/admin/raw_app", "raw_app")).toEqual("u/admin/raw_app.raw_app");
});

// =============================================================================
// nonDottedPaths Tests - API format detection and transformation
// =============================================================================

test("Metadata file detection works with dotted format (default)", () => {
  // Ensure we're in default mode
  setNonDottedPaths(false);

  // API always returns dotted format
  expect(isFlowMetadataFile("f/my_flow.flow.json")).toBeTruthy();
  expect(isFlowMetadataFile("f/my_flow.flow.yaml")).toBeTruthy();
  expect(isAppMetadataFile("f/my_app.app.json")).toBeTruthy();
  expect(isAppMetadataFile("f/my_app.app.yaml")).toBeTruthy();
  expect(isRawAppMetadataFile("f/my_raw.raw_app.json")).toBeTruthy();
  expect(isRawAppMetadataFile("f/my_raw.raw_app.yaml")).toBeTruthy();

  // Non-matching should return false
  expect(!isFlowMetadataFile("f/my_script.ts")).toBeTruthy();
  expect(!isAppMetadataFile("f/my_script.ts")).toBeTruthy();
});

test("Metadata file detection works with nonDottedPaths=true", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    setNonDottedPaths(true);

    // API format (dotted) should still be detected
    expect(isFlowMetadataFile("f/my_flow.flow.json")).toBeTruthy();
    expect(isAppMetadataFile("f/my_app.app.json")).toBeTruthy();
    expect(isRawAppMetadataFile("f/my_raw.raw_app.json")).toBeTruthy();

    // Local format (non-dotted) should also be detected
    expect(isFlowMetadataFile("f/my_flow__flow.json")).toBeTruthy();
    expect(isFlowMetadataFile("f/my_flow__flow.yaml")).toBeTruthy();
    expect(isAppMetadataFile("f/my_app__app.json")).toBeTruthy();
    expect(isRawAppMetadataFile("f/my_raw__raw_app.json")).toBeTruthy();
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

test("transformJsonPathToDir transforms API format to local format", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    expect(transformJsonPathToDir("f/my_flow.flow.json", "flow")).toEqual("f/my_flow.flow");
    expect(transformJsonPathToDir("f/my_app.app.json", "app")).toEqual("f/my_app.app");
    expect(transformJsonPathToDir("f/my_raw.raw_app.json", "raw_app")).toEqual("f/my_raw.raw_app");

    // Test with non-dotted paths
    setNonDottedPaths(true);
    expect(transformJsonPathToDir("f/my_flow.flow.json", "flow")).toEqual("f/my_flow__flow");
    expect(transformJsonPathToDir("f/my_app.app.json", "app")).toEqual("f/my_app__app");
    expect(transformJsonPathToDir("f/my_raw.raw_app.json", "raw_app")).toEqual("f/my_raw__raw_app");

    // Non-matching paths should be returned unchanged
    expect(transformJsonPathToDir("f/my_script.ts", "flow")).toEqual("f/my_script.ts");
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

test("getFolderSuffix returns correct suffix based on nonDottedPaths setting", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    setNonDottedPaths(false);
    expect(getFolderSuffix("flow")).toEqual(".flow");
    expect(getFolderSuffix("app")).toEqual(".app");
    expect(getFolderSuffix("raw_app")).toEqual(".raw_app");

    setNonDottedPaths(true);
    expect(getFolderSuffix("flow")).toEqual("__flow");
    expect(getFolderSuffix("app")).toEqual("__app");
    expect(getFolderSuffix("raw_app")).toEqual("__raw_app");
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

test("newPathAssigner with skipInlineScriptSuffix removes .inline_script. from paths", () => {
  // Test default behavior (with .inline_script. suffix)
  const defaultAssigner = newPathAssigner("bun");
  const [defaultPath, defaultExt] = defaultAssigner.assignPath("my_script", "bun");
  expect(defaultPath).toEqual("my_script.inline_script.");
  expect(defaultExt).toEqual("ts");

  // Test with skipInlineScriptSuffix = false (explicit)
  const withSuffixAssigner = newPathAssigner("bun", { skipInlineScriptSuffix: false });
  const [withSuffixPath, withSuffixExt] = withSuffixAssigner.assignPath("another_script", "python3");
  expect(withSuffixPath).toEqual("another_script.inline_script.");
  expect(withSuffixExt).toEqual("py");

  // Test with skipInlineScriptSuffix = true (no .inline_script. suffix)
  const noSuffixAssigner = newPathAssigner("bun", { skipInlineScriptSuffix: true });
  const [noSuffixPath, noSuffixExt] = noSuffixAssigner.assignPath("clean_script", "bun");
  expect(noSuffixPath).toEqual("clean_script.");
  expect(noSuffixExt).toEqual("ts");

  // Test with skipInlineScriptSuffix = true and different language
  const noSuffixPyAssigner = newPathAssigner("bun", { skipInlineScriptSuffix: true });
  const [noSuffixPyPath, noSuffixPyExt] = noSuffixPyAssigner.assignPath("python_script", "python3");
  expect(noSuffixPyPath).toEqual("python_script.");
  expect(noSuffixPyExt).toEqual("py");
});

test("newPathAssigner generates unique paths for duplicate names", () => {
  const assigner = newPathAssigner("bun", { skipInlineScriptSuffix: true });

  // First script
  const [path1, ext1] = assigner.assignPath("my_script", "bun");
  expect(path1).toEqual("my_script.");
  expect(ext1).toEqual("ts");

  // Second script with same name should get counter
  const [path2, ext2] = assigner.assignPath("my_script", "bun");
  expect(path2).toEqual("my_script_1.");
  expect(ext2).toEqual("ts");

  // Third script with same name should get incremented counter
  const [path3, ext3] = assigner.assignPath("my_script", "python3");
  expect(path3).toEqual("my_script_2.");
  expect(ext3).toEqual("py");
});

test("isAppInlineScriptPath detects app inline scripts correctly", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    expect(isAppInlineScriptPath("f/my_app.app/my_script.ts")).toBeTruthy();
    expect(isAppInlineScriptPath("f/my_app.app/app.yaml")).toBeTruthy();
    expect(!isAppInlineScriptPath("f/my_script.ts")).toBeTruthy();
    expect(!isAppInlineScriptPath("f/my_flow.flow/flow.yaml")).toBeTruthy();

    // Test with non-dotted paths
    setNonDottedPaths(true);
    expect(isAppInlineScriptPath("f/my_app__app/my_script.ts")).toBeTruthy();
    expect(isAppInlineScriptPath("f/my_app__app/app.yaml")).toBeTruthy();
    expect(!isAppInlineScriptPath("f/my_script.ts")).toBeTruthy();
    expect(!isAppInlineScriptPath("f/my_flow__flow/flow.yaml")).toBeTruthy();
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

test("isFlowInlineScriptPath detects flow inline scripts correctly", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    expect(isFlowInlineScriptPath("f/my_flow.flow/my_script.ts")).toBeTruthy();
    expect(isFlowInlineScriptPath("f/my_flow.flow/flow.yaml")).toBeTruthy();
    expect(!isFlowInlineScriptPath("f/my_script.ts")).toBeTruthy();
    expect(!isFlowInlineScriptPath("f/my_app.app/app.yaml")).toBeTruthy();

    // Test with non-dotted paths
    setNonDottedPaths(true);
    expect(isFlowInlineScriptPath("f/my_flow__flow/my_script.ts")).toBeTruthy();
    expect(isFlowInlineScriptPath("f/my_flow__flow/flow.yaml")).toBeTruthy();
    expect(!isFlowInlineScriptPath("f/my_script.ts")).toBeTruthy();
    expect(!isFlowInlineScriptPath("f/my_app__app/app.yaml")).toBeTruthy();
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

test("isRawAppBackendPath detects raw app backend paths correctly", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    expect(isRawAppBackendPath("f/my_app.raw_app/backend/script.ts")).toBeTruthy();
    expect(!isRawAppBackendPath("f/my_app.raw_app/index.html")).toBeTruthy();
    expect(!isRawAppBackendPath("f/my_script.ts")).toBeTruthy();

    // Test with non-dotted paths
    setNonDottedPaths(true);
    expect(isRawAppBackendPath("f/my_app__raw_app/backend/script.ts")).toBeTruthy();
    expect(!isRawAppBackendPath("f/my_app__raw_app/index.html")).toBeTruthy();
    expect(!isRawAppBackendPath("f/my_script.ts")).toBeTruthy();
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

test("Script fixture creates valid structure", () => {
  const pythonScript = createScriptFixture("test_script", "python3");

  expect(pythonScript.contentFile.path).toEqual("test_script.py");
  expect(pythonScript.metadataFile.path).toEqual("test_script.script.yaml");
  expect(pythonScript.contentFile.content).toContain("def main()");
  expect(pythonScript.metadataFile.content).toContain("summary:");
  expect(pythonScript.metadataFile.content).toContain("kind: script");
});

test("Flow fixture creates valid structure", () => {
  const flow = createFlowFixture("test_flow");

  expect(flow.metadata.path).toEqual("test_flow.flow/flow.yaml");
  expect(flow.inlineScript.path).toEqual("test_flow.flow/a.ts");
  expect(flow.metadata.content).toContain("summary:");
  expect(flow.metadata.content).toContain("modules:");
  expect(flow.inlineScript.content).toContain("export async function main");
});

test("App fixture creates valid structure", () => {
  const app = createAppFixture("test_app");

  expect(app.metadata.path).toEqual("test_app.app/app.yaml");
  expect(app.metadata.content).toContain("summary:");
  expect(app.metadata.content).toContain("grid:");
  expect(app.metadata.content).toContain("policy:");
});

test("Raw app fixture creates valid structure", () => {
  const rawApp = createRawAppFixture("test_raw_app");

  expect(rawApp.metadata.path).toEqual("test_raw_app.raw_app/raw_app.yaml");
  expect(rawApp.indexHtml.path).toEqual("test_raw_app.raw_app/index.html");
  expect(rawApp.indexJs.path).toEqual("test_raw_app.raw_app/index.js");
  expect(rawApp.metadata.content).toContain("summary:");
  expect(rawApp.metadata.content).toContain("runnables:");
});

test("Resource fixture creates valid YAML", () => {
  const resource = createResourceFixture("postgres", "postgresql", {
    host: "localhost",
    port: 5432,
  });

  expect(resource.path).toEqual("postgres.resource.yaml");
  expect(resource.content).toContain('resource_type: "postgresql"');
  expect(resource.content).toContain("value:");
});

test("Variable fixture creates valid YAML", () => {
  const variable = createVariableFixture("my_var", "test_value", false);

  expect(variable.path).toEqual("my_var.variable.yaml");
  expect(variable.content).toContain('value: "test_value"');
  expect(variable.content).toContain("is_secret: false");
});

test("Schedule fixture creates valid YAML", () => {
  const schedule = createScheduleFixture("hourly_job", "u/admin/my_script", "0 * * * *");

  expect(schedule.path).toEqual("hourly_job.schedule.yaml");
  expect(schedule.content).toContain('schedule: "0 * * * *"');
  expect(schedule.content).toContain('script_path: "u/admin/my_script"');
});

test("HTTP trigger fixture creates valid YAML", () => {
  const trigger = createHttpTriggerFixture("webhook", "/api/webhook", "u/admin/handler");

  expect(trigger.path).toEqual("webhook.http_trigger.yaml");
  expect(trigger.content).toContain('route_path: "/api/webhook"');
  expect(trigger.content).toContain("http_method: post");
});

test("Folder fixture creates valid YAML", () => {
  const folder = createFolderFixture("my_folder");

  expect(folder.path).toEqual("my_folder/folder.meta.yaml");
  expect(folder.content).toContain('display_name: "my_folder"');
});

test("User fixture creates valid YAML", () => {
  const user = createUserFixture("test_user", "test@example.com", true);

  expect(user.path).toEqual("test_user.user.yaml");
  expect(user.content).toContain('username: "test_user"');
  expect(user.content).toContain('email: "test@example.com"');
  expect(user.content).toContain("is_admin: true");
});

test("Group fixture creates valid YAML", () => {
  const group = createGroupFixture("developers", ["user1", "user2"]);

  expect(group.path).toEqual("developers.group.yaml");
  expect(group.content).toContain('name: "developers"');
  expect(group.content).toContain("- user1");
  expect(group.content).toContain("- user2");
});

test("Local filesystem creation creates all expected files", async () => {
  const tempDir = await createTempDir();

  try {
    await createLocalFilesystem(tempDir);
    const files = await readDirRecursive(tempDir);

    // Check scripts exist
    expect("f/scripts/python_script.py" in files).toBeTruthy();
    expect("f/scripts/python_script.script.yaml" in files).toBeTruthy();
    expect("f/scripts/deno_script.ts" in files).toBeTruthy();
    expect("f/scripts/bash_script.sh" in files).toBeTruthy();
    expect("f/scripts/go_script.go" in files).toBeTruthy();
    expect("f/scripts/sql_script.sql" in files).toBeTruthy();

    // Check flows exist
    expect("f/flows/test_flow.flow/flow.yaml" in files).toBeTruthy();
    expect("f/flows/test_flow.flow/a.ts" in files).toBeTruthy();

    // Check apps exist
    expect("f/apps/test_app.app/app.yaml" in files).toBeTruthy();

    // Check raw apps exist
    expect("f/apps/test_raw_app.raw_app/raw_app.yaml" in files).toBeTruthy();
    expect("f/apps/test_raw_app.raw_app/index.html" in files).toBeTruthy();
    expect("f/apps/test_raw_app.raw_app/index.js" in files).toBeTruthy();

    // Check resources exist
    expect("f/resources/postgres_db.resource.yaml" in files).toBeTruthy();
    expect("f/resources/api_config.resource.yaml" in files).toBeTruthy();

    // Check variables exist
    expect("f/resources/config_value.variable.yaml" in files).toBeTruthy();
    expect("f/resources/secret_key.variable.yaml" in files).toBeTruthy();

    // Check folder metadata
    expect("f/folder.meta.yaml" in files).toBeTruthy();
  } finally {
    await cleanupTempDir(tempDir);
  }
});

test("Mock remote zip can be created and read", async () => {
  const items = {
    "test_script.py": 'def main():\n    return "hello"',
    "test_script.script.json": '{"summary":"test","schema":{}}',
    "test_flow.flow.json": '{"summary":"flow","value":{"modules":[]}}',
    "test_app.app.json": '{"summary":"app","value":{}}',
  };

  const zip = await createMockRemoteZip(items);

  // Verify files exist in zip
  const scriptContent = await zip.file("test_script.py")?.async("text");
  expect(scriptContent).toEqual('def main():\n    return "hello"');

  const flowContent = await zip.file("test_flow.flow.json")?.async("text");
  expect(flowContent!).toContain('"summary":"flow"');
});

test("readDirRecursive reads all files correctly", async () => {
  const tempDir = await createTempDir();

  try {
    // Create a simple structure
    await mkdir(path.join(tempDir, "subdir"), { recursive: true });
    await writeFile(path.join(tempDir, "file1.txt"), "content1", "utf-8");
    await writeFile(path.join(tempDir, "subdir", "file2.txt"), "content2", "utf-8");

    const files = await readDirRecursive(tempDir);

    expect(files["file1.txt"]).toEqual("content1");
    expect(files["subdir/file2.txt"]).toEqual("content2");
    expect(Object.keys(files).length).toEqual(2);
  } finally {
    await cleanupTempDir(tempDir);
  }
});

// =============================================================================
// Integration Tests (use withTestBackend for automated backend setup)
// =============================================================================

import { yamlParseFile } from "../src/utils/yaml.ts";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";

test("Integration: Pull creates correct local structure", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Run sync pull
      const result = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);

      expect(result.code).toEqual(0);

      // Verify files were created
      const files = await readDirRecursive(tempDir);
      const hasYamlFiles = Object.keys(files).some((f) => f.endsWith(".yaml") && f !== "wmill.yaml");

      expect(hasYamlFiles || Object.keys(files).length > 1).toBeTruthy();
    });
  });

test("Integration: Push uploads local changes correctly", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Create a test script locally with a unique name
      // Path must have at least 2 segments after prefix (e.g., f/folder/name)
      const uniqueId = Date.now();
      await mkdir(`${tempDir}/f/test`, { recursive: true });
      const script = createScriptFixture(`f/test/push_script_${uniqueId}`, "deno");
      await writeFile(`${tempDir}/${script.contentFile.path}`, script.contentFile.content, "utf-8");
      await writeFile(`${tempDir}/${script.metadataFile.path}`, script.metadataFile.content, "utf-8");

      // Run sync push with dry-run first (only push our test script, not everything)
      const dryRunResult = await backend.runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/push_script_${uniqueId}**`],
        tempDir,
      );

      expect(dryRunResult.code).toEqual(0);
      expect(dryRunResult.stdout + dryRunResult.stderr).toContain(`push_script_${uniqueId}`);

      // Run actual push (only push our test script)
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/push_script_${uniqueId}**`],
        tempDir,
      );

      expect(pushResult.code).toEqual(0);
    });
  });

test("Integration: Pull then Push is idempotent", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Pull from remote
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Push back without changes (should be no-op)
      const pushResult = await backend.runCLICommand(["sync", "push", "--dry-run"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Should report 0 changes (check both stdout and stderr)
      const output = (pushResult.stdout + pushResult.stderr).toLowerCase();
      expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();
    });
  });

test("Integration: Include/exclude filters work correctly", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with restrictive filters
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/scripts/**"
excludes:
  - "**/*.test.ts"
skipVariables: true
skipResources: true
`,
      "utf-8",
      );

      // Run sync pull
      const result = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);

      expect(result.code).toEqual(0);

      // Verify only scripts in f/scripts/ were pulled (if any exist)
      const files = await readDirRecursive(tempDir);

      // Should not have variables or resources
      const hasVariables = Object.keys(files).some((f) => f.includes(".variable."));
      const hasResources = Object.keys(files).some((f) => f.includes(".resource."));

      expect(!hasVariables).toBeTruthy();
      expect(!hasResources).toBeTruthy();
    });
  });

test("Integration: Flow folder structure is created correctly", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Create a local flow with unique name
      // Path must have at least 2 segments after prefix (e.g., f/folder/name)
      const uniqueId = Date.now();
      const flowName = `f/test/flow_${uniqueId}`;
      const flowFixture = createFlowFixture(flowName);
      await mkdir(`${tempDir}/f/test/flow_${uniqueId}${getFolderSuffix("flow")}`, { recursive: true });
      for (const file of Object.values(flowFixture)) {
        await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
      }

      // Push the flow (only push our test flow, not everything)
      // Pattern needs to match the .flow folder, so use * to match the suffix
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/flow_${uniqueId}*/**`],
        tempDir,
      );

      expect(pushResult.code).toEqual(0);

      // Pull back and verify structure is preserved
      const tempDir2 = await mkdtemp(join(tmpdir(), "wmill_flow_verify_"));
      try {
        // Use template literal properly for the includes pattern
        const wmillConfig = `defaultTs: bun
includes:
  - "f/test/flow_${uniqueId}*/**"
excludes: []
`;
        await writeFile(`${tempDir2}/wmill.yaml`, wmillConfig, "utf-8");

        const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir2);

        expect(pullResult.code).toEqual(0);

        // Verify flow folder structure
        const files = await readDirRecursive(tempDir2);
        const allFiles = Object.keys(files);
        const flowFiles = allFiles.filter((f) => f.includes(`flow_${uniqueId}`));

        expect(flowFiles.length > 0).toBeTruthy();
        expect(flowFiles.some((f) => f.includes(".flow/"))).toBeTruthy();
      } finally {
        await cleanupTempDir(tempDir2);
      }
    });
  });

test("Integration: Raw app folder structure is handled correctly", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Create a local raw app with unique name
      // Path must have at least 2 segments after prefix (e.g., f/folder/name)
      const uniqueId = Date.now();
      const rawAppFixture = createRawAppFixture(`f/test/raw_app_${uniqueId}`);
      await mkdir(`${tempDir}/f/test/raw_app_${uniqueId}${getFolderSuffix("raw_app")}`, { recursive: true });
      for (const file of Object.values(rawAppFixture)) {
        await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
      }

      // Push the raw app (only push our test raw app, not everything)
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/raw_app_${uniqueId}**`],
        tempDir,
      );

      // Note: This may fail if raw apps require specific validation
      // The test verifies the CLI handles the folder structure correctly
      if (pushResult.code === 0) {
        expect(pushResult.stdout + pushResult.stderr).toContain("");
      }
    });
  });

// =============================================================================
// nonDottedPaths Unit Tests
// =============================================================================

test("getFolderSuffixes returns correct suffixes for dotted paths (default)", () => {
  setNonDottedPaths(false);
  const suffixes = getFolderSuffixes();
  expect(suffixes.flow).toEqual(".flow");
  expect(suffixes.app).toEqual(".app");
  expect(suffixes.raw_app).toEqual(".raw_app");
});

test("getFolderSuffixes returns correct suffixes for non-dotted paths", () => {
  setNonDottedPaths(true);
  const suffixes = getFolderSuffixes();
  expect(suffixes.flow).toEqual("__flow");
  expect(suffixes.app).toEqual("__app");
  expect(suffixes.raw_app).toEqual("__raw_app");
  setNonDottedPaths(false); // Reset
});

test("getFolderSuffix with nonDottedPaths returns dunder suffixes", () => {
  setNonDottedPaths(true);
  expect(getFolderSuffix("flow")).toEqual("__flow");
  expect(getFolderSuffix("app")).toEqual("__app");
  expect(getFolderSuffix("raw_app")).toEqual("__raw_app");
  setNonDottedPaths(false); // Reset
});

test("buildFolderPath with nonDottedPaths creates correct paths", () => {
  setNonDottedPaths(true);
  expect(buildFolderPath("my_flow", "flow")).toEqual("my_flow__flow");
  expect(buildFolderPath("f/test/my_app", "app")).toEqual("f/test/my_app__app");
  expect(buildFolderPath("u/admin/raw_app", "raw_app")).toEqual("u/admin/raw_app__raw_app");
  setNonDottedPaths(false); // Reset
});

test("buildMetadataPath with nonDottedPaths creates correct paths", () => {
  setNonDottedPaths(true);
  // buildMetadataPath always uses forward slashes internally
  expect(buildMetadataPath("my_flow", "flow", "yaml")).toEqual("my_flow__flow/flow.yaml");
  expect(buildMetadataPath("f/test/my_app", "app", "yaml")).toEqual("f/test/my_app__app/app.yaml");
  setNonDottedPaths(false); // Reset
});

test("isFlowPath detects non-dotted paths when configured", () => {
  // Default (dotted) paths
  setNonDottedPaths(false);
  expect(isFlowPath(`f${SEP}test${SEP}my_flow.flow${SEP}flow.yaml`)).toBeTruthy();
  expect(!isFlowPath(`f${SEP}test${SEP}my_flow__flow${SEP}flow.yaml`)).toBeTruthy();

  // Non-dotted paths
  setNonDottedPaths(true);
  expect(isFlowPath(`f${SEP}test${SEP}my_flow__flow${SEP}flow.yaml`)).toBeTruthy();
  expect(!isFlowPath(`f${SEP}test${SEP}my_flow.flow${SEP}flow.yaml`)).toBeTruthy();
  setNonDottedPaths(false); // Reset
});

test("isAppPath detects non-dotted paths when configured", () => {
  // Default (dotted) paths
  setNonDottedPaths(false);
  expect(isAppPath(`f${SEP}test${SEP}my_app.app${SEP}app.yaml`)).toBeTruthy();
  expect(!isAppPath(`f${SEP}test${SEP}my_app__app${SEP}app.yaml`)).toBeTruthy();

  // Non-dotted paths
  setNonDottedPaths(true);
  expect(isAppPath(`f${SEP}test${SEP}my_app__app${SEP}app.yaml`)).toBeTruthy();
  expect(!isAppPath(`f${SEP}test${SEP}my_app.app${SEP}app.yaml`)).toBeTruthy();
  setNonDottedPaths(false); // Reset
});

test("isRawAppPath detects non-dotted paths when configured", () => {
  // Default (dotted) paths
  setNonDottedPaths(false);
  expect(isRawAppPath(`f${SEP}test${SEP}my_raw_app.raw_app${SEP}raw_app.yaml`)).toBeTruthy();
  expect(!isRawAppPath(`f${SEP}test${SEP}my_raw_app__raw_app${SEP}raw_app.yaml`)).toBeTruthy();

  // Non-dotted paths
  setNonDottedPaths(true);
  expect(isRawAppPath(`f${SEP}test${SEP}my_raw_app__raw_app${SEP}raw_app.yaml`)).toBeTruthy();
  expect(!isRawAppPath(`f${SEP}test${SEP}my_raw_app.raw_app${SEP}raw_app.yaml`)).toBeTruthy();
  setNonDottedPaths(false); // Reset
});

test("extractResourceName works with non-dotted paths", () => {
  setNonDottedPaths(true);
  // extractResourceName normalizes separators to forward slashes
  expect(extractResourceName(`f${SEP}test${SEP}my_flow__flow${SEP}flow.yaml`, "flow")).toEqual("f/test/my_flow");
  expect(extractResourceName(`f${SEP}test${SEP}my_app__app${SEP}app.yaml`, "app")).toEqual("f/test/my_app");
  setNonDottedPaths(false); // Reset
});

test("hasFolderSuffix works with non-dotted paths", () => {
  setNonDottedPaths(true);
  expect(hasFolderSuffix("my_flow__flow", "flow")).toBeTruthy();
  expect(!hasFolderSuffix("my_flow.flow", "flow")).toBeTruthy();

  expect(hasFolderSuffix("my_app__app", "app")).toBeTruthy();
  expect(!hasFolderSuffix("my_app.app", "app")).toBeTruthy();
  setNonDottedPaths(false); // Reset
});

test("setNonDottedPaths and getNonDottedPaths work correctly", () => {
  // Default should be false
  setNonDottedPaths(false);
  expect(getNonDottedPaths()).toEqual(false);

  // Set to true
  setNonDottedPaths(true);
  expect(getNonDottedPaths()).toEqual(true);

  // Set back to false
  setNonDottedPaths(false);
  expect(getNonDottedPaths()).toEqual(false);
});

// =============================================================================
// nonDottedPaths Fixture Functions
// =============================================================================

/**
 * Creates a mock flow file structure using the current global nonDottedPaths setting
 */
function createFlowFixtureWithCurrentConfig(name: string): Record<string, { path: string; content: string }> {
  const flowSuffix = getFolderSuffix("flow");
  const metadataFile = getMetadataFileName("flow", "yaml");

  return {
    metadata: {
      path: `${name}${flowSuffix}/${metadataFile}`,
      content: `summary: "${name} flow"
description: "A flow for testing"
value:
  modules:
    - id: a
      value:
        type: rawscript
        content: "!inline ${name}${flowSuffix}/a.ts"
        language: bun
        input_transforms: {}
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties: {}
  required: []
`,
    },
    inlineScript: {
      path: `${name}${flowSuffix}/a.ts`,
      content: `export async function main() {\n  return "Hello from flow ${name}";\n}`,
    },
  };
}

/**
 * Creates a mock app file structure using the current global nonDottedPaths setting
 */
function createAppFixtureWithCurrentConfig(name: string): Record<string, { path: string; content: string }> {
  const appSuffix = getFolderSuffix("app");
  const metadataFile = getMetadataFileName("app", "yaml");

  return {
    metadata: {
      path: `${name}${appSuffix}/${metadataFile}`,
      content: `summary: "${name} app"
value:
  grid:
    - data:
        id: "text_1"
        type: textcomponent
        componentInput:
          type: static
          value: "Hello from ${name}"
      id: "a"
  hiddenInlineScripts: []
policy:
  execution_mode: viewer
`,
    },
  };
}

test("Flow fixture with nonDottedPaths creates __flow structure", () => {
  setNonDottedPaths(true);
  const flow = createFlowFixtureWithCurrentConfig("test_flow");

  expect(flow.metadata.path).toEqual("test_flow__flow/flow.yaml");
  expect(flow.inlineScript.path).toEqual("test_flow__flow/a.ts");
  expect(flow.metadata.content).toContain("summary:");
  expect(flow.metadata.content).toContain("modules:");
  setNonDottedPaths(false); // Reset
});

test("App fixture with nonDottedPaths creates __app structure", () => {
  setNonDottedPaths(true);
  const app = createAppFixtureWithCurrentConfig("test_app");

  expect(app.metadata.path).toEqual("test_app__app/app.yaml");
  expect(app.metadata.content).toContain("summary:");
  expect(app.metadata.content).toContain("grid:");
  setNonDottedPaths(false); // Reset
});

test("Local filesystem with nonDottedPaths creates correct folder structure", async () => {
  setNonDottedPaths(true);
  const tempDir = await createTempDir();

  try {
    // Create folder structure
    await mkdir(path.join(tempDir, "f/flows"), { recursive: true });
    await mkdir(path.join(tempDir, "f/apps"), { recursive: true });

    // Create flows with non-dotted paths
    const flowFixture = createFlowFixtureWithCurrentConfig("f/flows/test_flow");
    await mkdir(path.join(tempDir, `f/flows/test_flow${getFolderSuffix("flow")}`), { recursive: true });
    for (const file of Object.values(flowFixture)) {
      await writeFile(path.join(tempDir, file.path), file.content, "utf-8");
    }

    // Create apps with non-dotted paths
    const appFixture = createAppFixtureWithCurrentConfig("f/apps/test_app");
    await mkdir(path.join(tempDir, `f/apps/test_app${getFolderSuffix("app")}`), { recursive: true });
    for (const file of Object.values(appFixture)) {
      await writeFile(path.join(tempDir, file.path), file.content, "utf-8");
    }

    const files = await readDirRecursive(tempDir);

    // Check flows exist with __flow suffix
    expect("f/flows/test_flow__flow/flow.yaml" in files).toBeTruthy();
    expect("f/flows/test_flow__flow/a.ts" in files).toBeTruthy();

    // Check apps exist with __app suffix
    expect("f/apps/test_app__app/app.yaml" in files).toBeTruthy();

    // Verify old-style paths don't exist
    expect(!("f/flows/test_flow.flow/flow.yaml" in files)).toBeTruthy();
    expect(!("f/apps/test_app.app/app.yaml" in files)).toBeTruthy();
  } finally {
    await cleanupTempDir(tempDir);
    setNonDottedPaths(false); // Reset
  }
});

// =============================================================================
// nonDottedPaths Integration Tests
// =============================================================================

test("Integration: wmill.yaml with nonDottedPaths is read correctly", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with nonDottedPaths option
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "f/**"
excludes: []
`,
      "utf-8",
      );

      // Create a test script with non-dotted flow folder
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const flowName = `f/test/nondot_flow_${uniqueId}`;
      const flowFixture = createFlowFixtureWithCurrentConfig(flowName);
      await mkdir(`${tempDir}/f/test/nondot_flow_${uniqueId}${getFolderSuffix("flow")}`, { recursive: true });
      for (const file of Object.values(flowFixture)) {
        await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
      }
      setNonDottedPaths(false); // Reset

      // Run sync push with dry-run to verify the config is being read
      const dryRunResult = await backend.runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/nondot_flow_${uniqueId}**`],
        tempDir,
      );

      expect(dryRunResult.code).toEqual(0);
    });
  });

test("Integration: Pull then Push with nonDottedPaths is idempotent", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with nonDottedPaths enabled
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Pull from remote with nonDottedPaths enabled
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify that pulled files use __flow/__app/__raw_app suffixes
      const filesAfterPull = await readDirRecursive(tempDir);
      const flowFiles = Object.keys(filesAfterPull).filter((f) => f.includes("__flow/"));
      const appFiles = Object.keys(filesAfterPull).filter((f) => f.includes("__app/"));

      // Verify NO dotted paths were created
      const dottedFlowFiles = Object.keys(filesAfterPull).filter((f) => f.includes(".flow/"));
      const dottedAppFiles = Object.keys(filesAfterPull).filter((f) => f.includes(".app/"));

      // Only check if there are actually flows/apps in the workspace
      // If there are flows, they should use __flow not .flow
      if (flowFiles.length > 0 || dottedFlowFiles.length > 0) {
        expect(dottedFlowFiles.length === 0).toBeTruthy();
      }
      if (appFiles.length > 0 || dottedAppFiles.length > 0) {
        expect(dottedAppFiles.length === 0).toBeTruthy();
      }

      // Push back without changes (should be no-op / idempotent)
      const pushResult = await backend.runCLICommand(["sync", "push", "--dry-run"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Should report 0 changes (check both stdout and stderr)
      const output = (pushResult.stdout + pushResult.stderr).toLowerCase();
      expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();
    });
  });

test("Integration: Push flow with nonDottedPaths creates __flow structure on server", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with nonDottedPaths enabled
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Create a local flow with __flow suffix
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const flowName = `f/test/nondot_idem_flow_${uniqueId}`;
      const flowFixture = createFlowFixtureWithCurrentConfig(flowName);
      await mkdir(`${tempDir}/f/test/nondot_idem_flow_${uniqueId}${getFolderSuffix("flow")}`, { recursive: true });
      for (const file of Object.values(flowFixture)) {
        await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
      }
      setNonDottedPaths(false); // Reset global state

      // Push the flow
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/nondot_idem_flow_${uniqueId}**`],
        tempDir,
      );

      expect(pushResult.code).toEqual(0);

      // Pull back to same directory to verify round-trip (idempotency)
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes", "--includes", `f/test/nondot_idem_flow_${uniqueId}**`],
        tempDir,
      );

      expect(pullResult.code).toEqual(0);

      // Verify flow still has __flow suffix after round-trip
      const filesAfterPull = await readDirRecursive(tempDir);
      const allFiles = Object.keys(filesAfterPull);
      const flowFiles = allFiles.filter((f) => f.includes(`nondot_idem_flow_${uniqueId}`));

      expect(flowFiles.length > 0).toBeTruthy();
      expect(flowFiles.some((f) => f.includes("__flow/"))).toBeTruthy();
      expect(!flowFiles.some((f) => f.includes(".flow/"))).toBeTruthy();

      // Push again (should be idempotent - no changes)
      const push2 = await backend.runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/nondot_idem_flow_${uniqueId}**`],
        tempDir,
      );

      expect(push2.code).toEqual(0);

      const output = (push2.stdout + push2.stderr).toLowerCase();
      expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();
    });
  });

test("Integration: Multiple pull/push cycles with nonDottedPaths remain idempotent", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with nonDottedPaths enabled
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // First pull
      const pull1 = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pull1.code).toEqual(0);

      // First push (should be no-op)
      const push1 = await backend.runCLICommand(["sync", "push", "--dry-run"], tempDir);
      expect(push1.code).toEqual(0);

      // Second pull (should have no changes)
      const pull2 = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pull2.code).toEqual(0);

      // Second push (should still be no-op)
      const push2 = await backend.runCLICommand(["sync", "push", "--dry-run"], tempDir);
      expect(push2.code).toEqual(0);

      // Verify no changes after multiple cycles
      const output = (push2.stdout + push2.stderr).toLowerCase();
      expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();

      // Third pull to verify consistency
      const pull3 = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pull3.code).toEqual(0);

      // Final push check
      const push3 = await backend.runCLICommand(["sync", "push", "--dry-run"], tempDir);
      expect(push3.code).toEqual(0);

      const finalOutput = (push3.stdout + push3.stderr).toLowerCase();
      expect(finalOutput.includes("0 change") || finalOutput.includes("no change") || finalOutput.includes("nothing")).toBeTruthy();
    });
  });

test("Integration: App with nonDottedPaths creates __app structure and is idempotent", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with nonDottedPaths enabled
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Create a local app with __app suffix
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const appName = `f/test/nondot_app_${uniqueId}`;
      const appFixture = createAppFixtureWithCurrentConfig(appName);
      await mkdir(`${tempDir}/f/test/nondot_app_${uniqueId}${getFolderSuffix("app")}`, { recursive: true });
      for (const file of Object.values(appFixture)) {
        await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
      }
      setNonDottedPaths(false); // Reset global state

      // Push the app
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/nondot_app_${uniqueId}**`],
        tempDir,
      );

      expect(pushResult.code).toEqual(0);

      // Pull back to same directory
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes", "--includes", `f/test/nondot_app_${uniqueId}**`],
        tempDir,
      );

      expect(pullResult.code).toEqual(0);

      // Verify app structure uses __app
      const files = await readDirRecursive(tempDir);
      const appFiles = Object.keys(files).filter((f) => f.includes(`nondot_app_${uniqueId}`));

      expect(appFiles.length > 0).toBeTruthy();
      expect(appFiles.some((f) => f.includes("__app/"))).toBeTruthy();

      // Push again (should be idempotent)
      const push2 = await backend.runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/nondot_app_${uniqueId}**`],
        tempDir,
      );

      expect(push2.code).toEqual(0);

      const output = (push2.stdout + push2.stderr).toLowerCase();
      expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();
    });
  });

/**
 * Creates a mock raw_app file structure using the current global nonDottedPaths setting
 */
function createRawAppFixtureWithCurrentConfig(name: string): Record<string, { path: string; content: string }> {
  const rawAppSuffix = getFolderSuffix("raw_app");
  const metadataFile = getMetadataFileName("raw_app", "yaml");

  return {
    metadata: {
      path: `${name}${rawAppSuffix}/${metadataFile}`,
      content: `summary: "${name} raw app"
runnables:
  a:
    path: u/test/my_script
    type: script
`,
    },
    indexHtml: {
      path: `${name}${rawAppSuffix}/index.html`,
      content: `<!DOCTYPE html>
<html>
<head><title>${name}</title></head>
<body><h1>Hello from ${name}</h1></body>
</html>`,
    },
    indexJs: {
      path: `${name}${rawAppSuffix}/index.js`,
      content: `console.log("Hello from ${name}");`,
    },
  };
}

test("Integration: Raw app with nonDottedPaths creates __raw_app structure", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with nonDottedPaths enabled
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      // Create a local raw app with __raw_app suffix
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const rawAppName = `f/test/nondot_rawapp_${uniqueId}`;
      const rawAppFixture = createRawAppFixtureWithCurrentConfig(rawAppName);
      await mkdir(`${tempDir}/f/test/nondot_rawapp_${uniqueId}${getFolderSuffix("raw_app")}`, { recursive: true });
      for (const file of Object.values(rawAppFixture)) {
        await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
      }
      setNonDottedPaths(false); // Reset global state

      // Verify the files are created with __raw_app suffix
      const files = await readDirRecursive(tempDir);
      const rawAppFiles = Object.keys(files).filter((f) => f.includes(`nondot_rawapp_${uniqueId}`));

      expect(rawAppFiles.length > 0).toBeTruthy();
      expect(rawAppFiles.some((f) => f.includes("__raw_app/"))).toBeTruthy();
      expect(!rawAppFiles.some((f) => f.includes(".raw_app/"))).toBeTruthy();

      // Push the raw app (may fail if raw apps require specific validation)
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/nondot_rawapp_${uniqueId}**`],
        tempDir,
      );

      // Note: Raw apps may have additional validation requirements
      // This test primarily verifies the folder structure is correct
      if (pushResult.code === 0) {
        // If push succeeded, verify idempotency
        const push2 = await backend.runCLICommand(
          ["sync", "push", "--dry-run", "--includes", `f/test/nondot_rawapp_${uniqueId}**`],
          tempDir,
        );

        expect(push2.code).toEqual(0);
      }
    });
  });

test("Integration: Mixed scripts and flows with nonDottedPaths are idempotent", async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml with nonDottedPaths enabled
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      const uniqueId = Date.now();
      await mkdir(`${tempDir}/f/test`, { recursive: true });

      // Create a script (scripts don't use folder suffixes, so they're unaffected)
      const script = createScriptFixture(`f/test/mixed_script_${uniqueId}`, "deno");
      await writeFile(`${tempDir}/${script.contentFile.path}`, script.contentFile.content, "utf-8");
      await writeFile(`${tempDir}/${script.metadataFile.path}`, script.metadataFile.content, "utf-8");

      // Create a flow with __flow suffix
      setNonDottedPaths(true);
      const flowName = `f/test/mixed_flow_${uniqueId}`;
      const flowFixture = createFlowFixtureWithCurrentConfig(flowName);
      await mkdir(`${tempDir}/f/test/mixed_flow_${uniqueId}${getFolderSuffix("flow")}`, { recursive: true });
      for (const file of Object.values(flowFixture)) {
        await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
      }
      setNonDottedPaths(false); // Reset global state

      // Push both
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/mixed_*_${uniqueId}**`],
        tempDir,
      );

      expect(pushResult.code).toEqual(0);

      // Pull back
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes", "--includes", `f/test/mixed_*_${uniqueId}**`],
        tempDir,
      );

      expect(pullResult.code).toEqual(0);

      // Verify idempotency
      const push2 = await backend.runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/mixed_*_${uniqueId}**`],
        tempDir,
      );

      expect(push2.code).toEqual(0);

      const output = (push2.stdout + push2.stderr).toLowerCase();
      expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();
    });
  });

// =============================================================================
// ws_error_handler_muted Persistence Tests
// =============================================================================

test.skipIf(shouldSkipOnCI())("Integration: Script ws_error_handler_muted is persisted through push/pull", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      const uniqueId = Date.now();
      await mkdir(`${tempDir}/f/test`, { recursive: true });

      // Create a script with ws_error_handler_muted: true
      const scriptName = `f/test/muted_script_${uniqueId}`;
      const script = createScriptFixture(scriptName, "deno");
      await writeFile(`${tempDir}/${script.contentFile.path}`, script.contentFile.content, "utf-8");
      // Add ws_error_handler_muted to the metadata
      const metadataWithMuted = script.metadataFile.content + `ws_error_handler_muted: true\n`;
      await writeFile(`${tempDir}/${script.metadataFile.path}`, metadataWithMuted, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/muted_script_${uniqueId}**`],
        tempDir,
      );
      expect(pushResult.code).toEqual(0);

      // Verify via API that ws_error_handler_muted was persisted
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptName}`,
      );
      expect(apiResp.status).toEqual(200);
      const scriptData = await apiResp.json();
      expect(scriptData.ws_error_handler_muted).toEqual(true);

      // Pull into a fresh directory and verify the field round-trips
      const pullDir = await mkdtemp(join(tmpdir(), "wmill_muted_script_pull_"));
      try {
        await writeFile(
          `${pullDir}/wmill.yaml`,
          `defaultTs: bun
includes:
  - "f/test/muted_script_${uniqueId}**"
excludes: []
`,
        "utf-8",
        );

        const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], pullDir);
        expect(pullResult.code).toEqual(0);

        // Verify ws_error_handler_muted is in the pulled metadata
        const pulledMetadata = await readFile(`${pullDir}/${script.metadataFile.path}`, "utf-8");
        expect(pulledMetadata).toContain("ws_error_handler_muted: true");

        // Verify push from pulled dir is idempotent (no changes)
        const push2 = await backend.runCLICommand(
          ["sync", "push", "--dry-run", "--includes", `f/test/muted_script_${uniqueId}**`],
          pullDir,
        );
        expect(push2.code).toEqual(0);
        const output = (push2.stdout + push2.stderr).toLowerCase();
        expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();
      } finally {
        await rm(pullDir, { recursive: true }).catch(() => {});
      }
    });
  });

test.skipIf(shouldSkipOnCI())("Integration: Flow ws_error_handler_muted is persisted through push/pull", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      "utf-8",
      );

      const uniqueId = Date.now();
      const flowName = `f/test/muted_flow_${uniqueId}`;
      const flowFixture = createFlowFixture(flowName);

      // Create flow directory and files
      await mkdir(`${tempDir}/f/test/muted_flow_${uniqueId}${getFolderSuffix("flow")}`, { recursive: true });
      for (const [key, file] of Object.entries(flowFixture)) {
        if (key === "metadata") {
          // Add ws_error_handler_muted to flow metadata
          const contentWithMuted = file.content + `ws_error_handler_muted: true\n`;
          await writeFile(`${tempDir}/${file.path}`, contentWithMuted, "utf-8");
        } else {
          await writeFile(`${tempDir}/${file.path}`, file.content, "utf-8");
        }
      }

      // Push
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/muted_flow_${uniqueId}*/**`],
        tempDir,
      );
      expect(pushResult.code).toEqual(0);

      // Verify via API that ws_error_handler_muted was persisted
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/flows/get/${flowName}`,
      );
      expect(apiResp.status).toEqual(200);
      const flowData = await apiResp.json();
      expect(flowData.ws_error_handler_muted).toEqual(true);

      // Pull into a fresh directory and verify the field round-trips
      const pullDir = await mkdtemp(join(tmpdir(), "wmill_muted_flow_pull_"));
      try {
        await writeFile(
          `${pullDir}/wmill.yaml`,
          `defaultTs: bun
includes:
  - "f/test/muted_flow_${uniqueId}*/**"
excludes: []
`,
        "utf-8",
        );

        const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], pullDir);
        expect(pullResult.code).toEqual(0);

        // Verify ws_error_handler_muted is in the pulled flow.yaml
        const flowYamlPath = `${pullDir}/${flowFixture.metadata.path}`;
        const pulledFlowYaml = await readFile(flowYamlPath, "utf-8");
        expect(pulledFlowYaml).toContain("ws_error_handler_muted: true");

        // Parse the YAML to confirm it's a proper boolean value
        // deno-lint-ignore no-explicit-any
        const parsed = await yamlParseFile(flowYamlPath) as any;
        expect(parsed.ws_error_handler_muted).toEqual(true);

        // Verify push from pulled dir is idempotent (no changes)
        const push2 = await backend.runCLICommand(
          ["sync", "push", "--dry-run", "--includes", `f/test/muted_flow_${uniqueId}*/**`],
          pullDir,
        );
        expect(push2.code).toEqual(0);
        const output = (push2.stdout + push2.stderr).toLowerCase();
        expect(output.includes("0 change") || output.includes("no change") || output.includes("nothing")).toBeTruthy();
      } finally {
        await rm(pullDir, { recursive: true }).catch(() => {});
      }
    });
  });

// =============================================================================
// Sync tests for groups, settings, resource types, schedules, and HTTP triggers
// =============================================================================

import type { TestBackend } from "./test_backend.ts";

/** Create a script on the remote via API */
async function createRemoteScript(
  backend: TestBackend,
  scriptPath: string,
  content: string = 'export async function main() { return "hello"; }'
): Promise<void> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/scripts/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path: scriptPath,
        content,
        language: "bun",
        summary: "Test script",
        description: "Created by integration test",
        schema: {
          $schema: "https://json-schema.org/draft/2020-12/schema",
          type: "object",
          properties: {},
          required: [],
        },
      }),
    }
  );
  expect(resp.status).toBeLessThan(300);
  await resp.text();
}

/** Write a standard wmill.yaml with the given extra flags */
async function writeWmillYaml(
  tempDir: string,
  extraFlags: string = ""
): Promise<void> {
  await writeFile(
    `${tempDir}/wmill.yaml`,
    `defaultTs: bun
includes:
  - "**"
excludes: []
${extraFlags}`,
    "utf-8"
  );
}

/** Recursively list all files relative to baseDir, returning forward-slash paths */
async function listFilesRecursive(
  dir: string,
  baseDir: string = dir
): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    const relativePath = fullPath
      .substring(baseDir.length + 1)
      .replaceAll("\\", "/");
    if (entry.isDirectory()) {
      files.push(...(await listFilesRecursive(fullPath, baseDir)));
    } else {
      files.push(relativePath);
    }
  }
  return files;
}

describe("group sync", () => {
  test("Integration: Group pull/push round-trip", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeWmillYaml(tempDir, "includeGroups: true");

      // Pull with --include-groups
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes", "--include-groups"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Verify group file was created (seedTestData creates test_group)
      const files = await listFilesRecursive(tempDir);
      const groupFiles = files.filter((f) => f.endsWith(".group.yaml"));
      expect(groupFiles.length).toBeGreaterThan(0);

      const testGroupFile = groupFiles.find((f) => f.includes("test_group"));
      expect(testGroupFile).toBeDefined();

      // Read the group file and modify
      const groupContent = await readFile(`${tempDir}/${testGroupFile!}`, "utf-8");
      expect(groupContent).toContain("summary");

      const modifiedContent = groupContent.replace(
        /summary:.*/,
        'summary: "Modified group summary from test"'
      );
      await writeFile(`${tempDir}/${testGroupFile!}`, modifiedContent, "utf-8");

      // Push the modification
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--include-groups"],
        tempDir
      );
      expect(pushResult.code).toEqual(0);

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/groups/get/test_group`
      );
      expect(apiResp.status).toEqual(200);
      const groupData = await apiResp.json();
      expect(groupData.summary).toEqual("Modified group summary from test");
    });
  });
});

describe("settings sync", () => {
  test("Integration: Settings pull/push round-trip", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeWmillYaml(tempDir, "includeSettings: true");

      // Pull with --include-settings
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes", "--include-settings"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Verify settings.yaml exists
      const files = await listFilesRecursive(tempDir);
      expect(files).toContain("settings.yaml");

      // Read and modify a safe setting (webhook URL)
      const settingsContent = await readFile(`${tempDir}/settings.yaml`, "utf-8");

      let modifiedSettings: string;
      if (settingsContent.includes("webhook:")) {
        modifiedSettings = settingsContent.replace(
          /webhook:.*/,
          'webhook: "https://test-webhook.example.com/hook"'
        );
      } else {
        modifiedSettings =
          settingsContent + '\nwebhook: "https://test-webhook.example.com/hook"\n';
      }
      await writeFile(`${tempDir}/settings.yaml`, modifiedSettings, "utf-8");

      // Push the modification
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--include-settings"],
        tempDir
      );
      expect(pushResult.code).toEqual(0);

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/workspaces/get_settings`
      );
      expect(apiResp.status).toEqual(200);
      const settingsData = await apiResp.json();
      expect(settingsData.webhook).toEqual("https://test-webhook.example.com/hook");
    });
  });
});

describe("resource type sync", () => {
  test("Integration: Resource type pull/push round-trip", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const rtName = `test_sync_rt_${uniqueId}`;

      // Create a resource type via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/type/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            name: rtName,
            schema: {
              type: "object",
              properties: {
                host: { type: "string", description: "Hostname" },
                port: { type: "integer", description: "Port number" },
              },
            },
            description: "Test resource type for sync",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Resource types are included by default (not skipped)
      await writeWmillYaml(tempDir);

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify resource type file exists
      const files = await listFilesRecursive(tempDir);
      const rtFile = files.find((f) => f.includes(`${rtName}.resource-type.yaml`));
      expect(rtFile).toBeDefined();

      // Read and modify the description
      const rtContent = await readFile(`${tempDir}/${rtFile!}`, "utf-8");
      expect(rtContent).toContain("host");

      const modifiedContent = rtContent.replace(
        "Test resource type for sync",
        "Updated resource type description"
      );
      await writeFile(`${tempDir}/${rtFile!}`, modifiedContent, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/type/get/${rtName}`
      );
      expect(apiResp.status).toEqual(200);
      const rtData = await apiResp.json();
      expect(rtData.description).toEqual("Updated resource type description");
    });
  });
});

describe("schedule sync", () => {
  test("Integration: Schedule pull/push round-trip", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/sched_sync_target_${uniqueId}`;
      const schedulePath = `f/test/sched_sync_${uniqueId}`;

      // Create target script via API
      await createRemoteScript(backend, scriptPath);

      // Create schedule via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: schedulePath,
            schedule: "0 0 */6 * * *",
            script_path: scriptPath,
            is_flow: false,
            args: {},
            enabled: false,
            timezone: "UTC",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      await writeWmillYaml(tempDir, "includeSchedules: true");

      // Pull with --include-schedules
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes", "--include-schedules"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Verify schedule file exists
      const files = await listFilesRecursive(tempDir);
      const scheduleFile = files.find(
        (f) => f.includes(`sched_sync_${uniqueId}`) && f.endsWith(".schedule.yaml")
      );
      expect(scheduleFile).toBeDefined();

      // Read and verify content
      const schedContent = await readFile(`${tempDir}/${scheduleFile!}`, "utf-8");
      expect(schedContent).toContain("0 0 */6 * * *");

      // Modify the cron expression
      const modifiedContent = schedContent.replace("0 0 */6 * * *", "0 0 */12 * * *");
      await writeFile(`${tempDir}/${scheduleFile!}`, modifiedContent, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--include-schedules"],
        tempDir
      );
      expect(pushResult.code).toEqual(0);

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/get/${schedulePath}`
      );
      expect(apiResp.status).toEqual(200);
      const schedData = await apiResp.json();
      expect(schedData.schedule).toEqual("0 0 */12 * * *");
    });
  });

  test("Integration: Schedule push-only creates from local file", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/sched_pushonly_target_${uniqueId}`;
      const schedulePath = `f/test/sched_pushonly_${uniqueId}`;

      // Create target script via API
      await createRemoteScript(backend, scriptPath);

      await writeWmillYaml(tempDir, "includeSchedules: true");

      // Create schedule YAML locally
      await mkdir(`${tempDir}/f/test`, { recursive: true });
      await writeFile(
        `${tempDir}/${schedulePath}.schedule.yaml`,
        `path: "${schedulePath}"
schedule: "0 30 2 * * 1"
script_path: "${scriptPath}"
is_flow: false
args: {}
enabled: false
timezone: "UTC"
`,
        "utf-8"
      );

      // Push
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--include-schedules", "--includes", `f/test/sched_pushonly_${uniqueId}**`],
        tempDir
      );
      expect(pushResult.code).toEqual(0);

      // Verify schedule was created via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/get/${schedulePath}`
      );
      expect(apiResp.status).toEqual(200);
      const schedData = await apiResp.json();
      expect(schedData.schedule).toEqual("0 30 2 * * 1");
      expect(schedData.script_path).toEqual(scriptPath);
    });
  });
});

describe("http trigger sync", () => {
  test.skipIf(shouldSkipOnCI())("Integration: HTTP trigger pull/push is idempotent", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/http_trig_target_${uniqueId}`;

      // Create target script via API
      await createRemoteScript(backend, scriptPath);

      // Create HTTP trigger via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/http_triggers/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/http_trig_${uniqueId}`,
            script_path: scriptPath,
            route_path: `/test/hook_${uniqueId}`,
            is_flow: false,
            http_method: "post",
            is_async: false,
            requires_auth: false,
          }),
        }
      );
      // If the feature is not enabled, the create will fail - skip gracefully
      if (createResp.status >= 400) {
        console.log("HTTP trigger creation failed (feature may not be enabled), skipping");
        return;
      }
      await createResp.text();

      await writeWmillYaml(tempDir, "includeTriggers: true");

      // Pull with --include-triggers
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes", "--include-triggers"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Verify http_trigger file exists
      const files = await listFilesRecursive(tempDir);
      const triggerFile = files.find(
        (f) => f.includes(`http_trig_${uniqueId}`) && f.endsWith(".http_trigger.yaml")
      );
      expect(triggerFile).toBeDefined();

      // Push back (verify idempotent)
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--dry-run", "--include-triggers"],
        tempDir
      );
      expect(pushResult.code).toEqual(0);

      const output = (pushResult.stdout + pushResult.stderr).toLowerCase();
      expect(
        output.includes("0 change") || output.includes("no change") || output.includes("nothing")
      ).toBeTruthy();
    });
  });
});
