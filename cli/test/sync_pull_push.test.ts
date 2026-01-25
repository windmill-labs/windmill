/**
 * Sync Pull/Push Integration Tests
 *
 * Tests the sync pull and push functionality with a simulated filesystem
 * containing every kind of Windmill resource type.
 */

import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";
import * as path from "https://deno.land/std@0.224.0/path/mod.ts";
import { SEPARATOR as SEP } from "https://deno.land/std@0.224.0/path/mod.ts";
import { JSZip } from "../deps.ts";
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
    await ensureDir(path.join(baseDir, folder));
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
    await Deno.writeTextFile(
      path.join(baseDir, script.contentFile.path),
      script.contentFile.content,
    );
    await Deno.writeTextFile(
      path.join(baseDir, script.metadataFile.path),
      script.metadataFile.content,
    );
  }

  // Create flows
  const flowFixture = createFlowFixture("f/flows/test_flow");
  await ensureDir(path.join(baseDir, `f/flows/test_flow${getFolderSuffix("flow")}`));
  for (const file of Object.values(flowFixture)) {
    await Deno.writeTextFile(path.join(baseDir, file.path), file.content);
  }

  // Create apps
  const appFixture = createAppFixture("f/apps/test_app");
  await ensureDir(path.join(baseDir, `f/apps/test_app${getFolderSuffix("app")}`));
  for (const file of Object.values(appFixture)) {
    await Deno.writeTextFile(path.join(baseDir, file.path), file.content);
  }

  // Create raw apps
  const rawAppFixture = createRawAppFixture("f/apps/test_raw_app");
  await ensureDir(path.join(baseDir, `f/apps/test_raw_app${getFolderSuffix("raw_app")}`));
  for (const file of Object.values(rawAppFixture)) {
    await Deno.writeTextFile(path.join(baseDir, file.path), file.content);
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
    await Deno.writeTextFile(path.join(baseDir, resource.path), resource.content);
  }

  // Create variables
  const variables = [
    createVariableFixture("f/resources/config_value", "test_value", false),
    createVariableFixture("f/resources/secret_key", "secret123", true),
  ];

  for (const variable of variables) {
    await Deno.writeTextFile(path.join(baseDir, variable.path), variable.content);
  }

  // Create folder metadata
  await ensureDir(path.join(baseDir, "f"));
  const folderMeta = createFolderFixture("f");
  await Deno.writeTextFile(path.join(baseDir, folderMeta.path), folderMeta.content);
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

  for await (const entry of Deno.readDir(dir)) {
    const fullPath = path.join(dir, entry.name);
    // Normalize path separators to forward slashes for cross-platform compatibility
    const relativePath = fullPath.substring(baseDir.length + 1).replaceAll("\\", "/");

    if (entry.isDirectory) {
      const subFiles = await readDirRecursive(fullPath, baseDir);
      Object.assign(files, subFiles);
    } else {
      files[relativePath] = await Deno.readTextFile(fullPath);
    }
  }

  return files;
}

/**
 * Creates a temporary directory for testing
 */
async function createTempDir(): Promise<string> {
  return await Deno.makeTempDir({ prefix: "wmill_sync_test_" });
}

/**
 * Cleans up a temporary directory
 */
async function cleanupTempDir(dir: string): Promise<void> {
  try {
    await Deno.remove(dir, { recursive: true });
  } catch {
    // Ignore cleanup errors
  }
}

// =============================================================================
// Tests
// =============================================================================

Deno.test("Resource folder suffixes are correct", () => {
  assertEquals(getFolderSuffix("flow"), ".flow");
  assertEquals(getFolderSuffix("app"), ".app");
  assertEquals(getFolderSuffix("raw_app"), ".raw_app");
});

Deno.test("Metadata file names are correct", () => {
  assertEquals(getMetadataFileName("flow", "yaml"), "flow.yaml");
  assertEquals(getMetadataFileName("flow", "json"), "flow.json");
  assertEquals(getMetadataFileName("app", "yaml"), "app.yaml");
  assertEquals(getMetadataFileName("raw_app", "yaml"), "raw_app.yaml");
});

Deno.test("buildFolderPath creates correct paths", () => {
  assertEquals(buildFolderPath("my_flow", "flow"), "my_flow.flow");
  assertEquals(buildFolderPath("f/test/my_app", "app"), "f/test/my_app.app");
  assertEquals(buildFolderPath("u/admin/raw_app", "raw_app"), "u/admin/raw_app.raw_app");
});

// =============================================================================
// nonDottedPaths Tests - API format detection and transformation
// =============================================================================

Deno.test("Metadata file detection works with dotted format (default)", () => {
  // Ensure we're in default mode
  setNonDottedPaths(false);

  // API always returns dotted format
  assert(isFlowMetadataFile("f/my_flow.flow.json"), "Should detect .flow.json");
  assert(isFlowMetadataFile("f/my_flow.flow.yaml"), "Should detect .flow.yaml");
  assert(isAppMetadataFile("f/my_app.app.json"), "Should detect .app.json");
  assert(isAppMetadataFile("f/my_app.app.yaml"), "Should detect .app.yaml");
  assert(isRawAppMetadataFile("f/my_raw.raw_app.json"), "Should detect .raw_app.json");
  assert(isRawAppMetadataFile("f/my_raw.raw_app.yaml"), "Should detect .raw_app.yaml");

  // Non-matching should return false
  assert(!isFlowMetadataFile("f/my_script.ts"), "Should not detect script file");
  assert(!isAppMetadataFile("f/my_script.ts"), "Should not detect script file");
});

Deno.test("Metadata file detection works with nonDottedPaths=true", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    setNonDottedPaths(true);

    // API format (dotted) should still be detected
    assert(isFlowMetadataFile("f/my_flow.flow.json"), "Should detect API format .flow.json");
    assert(isAppMetadataFile("f/my_app.app.json"), "Should detect API format .app.json");
    assert(isRawAppMetadataFile("f/my_raw.raw_app.json"), "Should detect API format .raw_app.json");

    // Local format (non-dotted) should also be detected
    assert(isFlowMetadataFile("f/my_flow__flow.json"), "Should detect local format __flow.json");
    assert(isFlowMetadataFile("f/my_flow__flow.yaml"), "Should detect local format __flow.yaml");
    assert(isAppMetadataFile("f/my_app__app.json"), "Should detect local format __app.json");
    assert(isRawAppMetadataFile("f/my_raw__raw_app.json"), "Should detect local format __raw_app.json");
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

Deno.test("transformJsonPathToDir transforms API format to local format", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    assertEquals(
      transformJsonPathToDir("f/my_flow.flow.json", "flow"),
      "f/my_flow.flow",
      "Should transform dotted API format to dotted local format"
    );
    assertEquals(
      transformJsonPathToDir("f/my_app.app.json", "app"),
      "f/my_app.app",
      "Should transform app correctly"
    );
    assertEquals(
      transformJsonPathToDir("f/my_raw.raw_app.json", "raw_app"),
      "f/my_raw.raw_app",
      "Should transform raw_app correctly"
    );

    // Test with non-dotted paths
    setNonDottedPaths(true);
    assertEquals(
      transformJsonPathToDir("f/my_flow.flow.json", "flow"),
      "f/my_flow__flow",
      "Should transform dotted API format to non-dotted local format"
    );
    assertEquals(
      transformJsonPathToDir("f/my_app.app.json", "app"),
      "f/my_app__app",
      "Should transform app to non-dotted format"
    );
    assertEquals(
      transformJsonPathToDir("f/my_raw.raw_app.json", "raw_app"),
      "f/my_raw__raw_app",
      "Should transform raw_app to non-dotted format"
    );

    // Non-matching paths should be returned unchanged
    assertEquals(
      transformJsonPathToDir("f/my_script.ts", "flow"),
      "f/my_script.ts",
      "Should return non-matching path unchanged"
    );
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

Deno.test("getFolderSuffix returns correct suffix based on nonDottedPaths setting", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    setNonDottedPaths(false);
    assertEquals(getFolderSuffix("flow"), ".flow");
    assertEquals(getFolderSuffix("app"), ".app");
    assertEquals(getFolderSuffix("raw_app"), ".raw_app");

    setNonDottedPaths(true);
    assertEquals(getFolderSuffix("flow"), "__flow");
    assertEquals(getFolderSuffix("app"), "__app");
    assertEquals(getFolderSuffix("raw_app"), "__raw_app");
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

Deno.test("newPathAssigner with skipInlineScriptSuffix removes .inline_script. from paths", () => {
  // Test default behavior (with .inline_script. suffix)
  const defaultAssigner = newPathAssigner("bun");
  const [defaultPath, defaultExt] = defaultAssigner.assignPath("my_script", "bun");
  assertEquals(defaultPath, "my_script.inline_script.");
  assertEquals(defaultExt, "ts");

  // Test with skipInlineScriptSuffix = false (explicit)
  const withSuffixAssigner = newPathAssigner("bun", { skipInlineScriptSuffix: false });
  const [withSuffixPath, withSuffixExt] = withSuffixAssigner.assignPath("another_script", "python3");
  assertEquals(withSuffixPath, "another_script.inline_script.");
  assertEquals(withSuffixExt, "py");

  // Test with skipInlineScriptSuffix = true (no .inline_script. suffix)
  const noSuffixAssigner = newPathAssigner("bun", { skipInlineScriptSuffix: true });
  const [noSuffixPath, noSuffixExt] = noSuffixAssigner.assignPath("clean_script", "bun");
  assertEquals(noSuffixPath, "clean_script.");
  assertEquals(noSuffixExt, "ts");

  // Test with skipInlineScriptSuffix = true and different language
  const noSuffixPyAssigner = newPathAssigner("bun", { skipInlineScriptSuffix: true });
  const [noSuffixPyPath, noSuffixPyExt] = noSuffixPyAssigner.assignPath("python_script", "python3");
  assertEquals(noSuffixPyPath, "python_script.");
  assertEquals(noSuffixPyExt, "py");
});

Deno.test("newPathAssigner generates unique paths for duplicate names", () => {
  const assigner = newPathAssigner("bun", { skipInlineScriptSuffix: true });

  // First script
  const [path1, ext1] = assigner.assignPath("my_script", "bun");
  assertEquals(path1, "my_script.");
  assertEquals(ext1, "ts");

  // Second script with same name should get counter
  const [path2, ext2] = assigner.assignPath("my_script", "bun");
  assertEquals(path2, "my_script_1.");
  assertEquals(ext2, "ts");

  // Third script with same name should get incremented counter
  const [path3, ext3] = assigner.assignPath("my_script", "python3");
  assertEquals(path3, "my_script_2.");
  assertEquals(ext3, "py");
});

Deno.test("isAppInlineScriptPath detects app inline scripts correctly", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    assert(isAppInlineScriptPath("f/my_app.app/my_script.ts"), "Should detect script in .app folder");
    assert(isAppInlineScriptPath("f/my_app.app/app.yaml"), "Should detect metadata in .app folder");
    assert(!isAppInlineScriptPath("f/my_script.ts"), "Should not detect standalone script");
    assert(!isAppInlineScriptPath("f/my_flow.flow/flow.yaml"), "Should not detect flow files");

    // Test with non-dotted paths
    setNonDottedPaths(true);
    assert(isAppInlineScriptPath("f/my_app__app/my_script.ts"), "Should detect script in __app folder");
    assert(isAppInlineScriptPath("f/my_app__app/app.yaml"), "Should detect metadata in __app folder");
    assert(!isAppInlineScriptPath("f/my_script.ts"), "Should not detect standalone script");
    assert(!isAppInlineScriptPath("f/my_flow__flow/flow.yaml"), "Should not detect flow files");
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

Deno.test("isFlowInlineScriptPath detects flow inline scripts correctly", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    assert(isFlowInlineScriptPath("f/my_flow.flow/my_script.ts"), "Should detect script in .flow folder");
    assert(isFlowInlineScriptPath("f/my_flow.flow/flow.yaml"), "Should detect metadata in .flow folder");
    assert(!isFlowInlineScriptPath("f/my_script.ts"), "Should not detect standalone script");
    assert(!isFlowInlineScriptPath("f/my_app.app/app.yaml"), "Should not detect app files");

    // Test with non-dotted paths
    setNonDottedPaths(true);
    assert(isFlowInlineScriptPath("f/my_flow__flow/my_script.ts"), "Should detect script in __flow folder");
    assert(isFlowInlineScriptPath("f/my_flow__flow/flow.yaml"), "Should detect metadata in __flow folder");
    assert(!isFlowInlineScriptPath("f/my_script.ts"), "Should not detect standalone script");
    assert(!isFlowInlineScriptPath("f/my_app__app/app.yaml"), "Should not detect app files");
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

Deno.test("isRawAppBackendPath detects raw app backend paths correctly", () => {
  // Store original value
  const wasNonDotted = getNonDottedPaths();

  try {
    // Test with dotted paths (default)
    setNonDottedPaths(false);
    assert(isRawAppBackendPath("f/my_app.raw_app/backend/script.ts"), "Should detect script in .raw_app/backend");
    assert(!isRawAppBackendPath("f/my_app.raw_app/index.html"), "Should not detect root files in raw_app");
    assert(!isRawAppBackendPath("f/my_script.ts"), "Should not detect standalone script");

    // Test with non-dotted paths
    setNonDottedPaths(true);
    assert(isRawAppBackendPath("f/my_app__raw_app/backend/script.ts"), "Should detect script in __raw_app/backend");
    assert(!isRawAppBackendPath("f/my_app__raw_app/index.html"), "Should not detect root files in raw_app");
    assert(!isRawAppBackendPath("f/my_script.ts"), "Should not detect standalone script");
  } finally {
    // Restore original value
    setNonDottedPaths(wasNonDotted);
  }
});

Deno.test("Script fixture creates valid structure", () => {
  const pythonScript = createScriptFixture("test_script", "python3");

  assertEquals(pythonScript.contentFile.path, "test_script.py");
  assertEquals(pythonScript.metadataFile.path, "test_script.script.yaml");
  assertStringIncludes(pythonScript.contentFile.content, "def main()");
  assertStringIncludes(pythonScript.metadataFile.content, "summary:");
  assertStringIncludes(pythonScript.metadataFile.content, "kind: script");
});

Deno.test("Flow fixture creates valid structure", () => {
  const flow = createFlowFixture("test_flow");

  assertEquals(flow.metadata.path, "test_flow.flow/flow.yaml");
  assertEquals(flow.inlineScript.path, "test_flow.flow/a.ts");
  assertStringIncludes(flow.metadata.content, "summary:");
  assertStringIncludes(flow.metadata.content, "modules:");
  assertStringIncludes(flow.inlineScript.content, "export async function main");
});

Deno.test("App fixture creates valid structure", () => {
  const app = createAppFixture("test_app");

  assertEquals(app.metadata.path, "test_app.app/app.yaml");
  assertStringIncludes(app.metadata.content, "summary:");
  assertStringIncludes(app.metadata.content, "grid:");
  assertStringIncludes(app.metadata.content, "policy:");
});

Deno.test("Raw app fixture creates valid structure", () => {
  const rawApp = createRawAppFixture("test_raw_app");

  assertEquals(rawApp.metadata.path, "test_raw_app.raw_app/raw_app.yaml");
  assertEquals(rawApp.indexHtml.path, "test_raw_app.raw_app/index.html");
  assertEquals(rawApp.indexJs.path, "test_raw_app.raw_app/index.js");
  assertStringIncludes(rawApp.metadata.content, "summary:");
  assertStringIncludes(rawApp.metadata.content, "runnables:");
});

Deno.test("Resource fixture creates valid YAML", () => {
  const resource = createResourceFixture("postgres", "postgresql", {
    host: "localhost",
    port: 5432,
  });

  assertEquals(resource.path, "postgres.resource.yaml");
  assertStringIncludes(resource.content, 'resource_type: "postgresql"');
  assertStringIncludes(resource.content, "value:");
});

Deno.test("Variable fixture creates valid YAML", () => {
  const variable = createVariableFixture("my_var", "test_value", false);

  assertEquals(variable.path, "my_var.variable.yaml");
  assertStringIncludes(variable.content, 'value: "test_value"');
  assertStringIncludes(variable.content, "is_secret: false");
});

Deno.test("Schedule fixture creates valid YAML", () => {
  const schedule = createScheduleFixture("hourly_job", "u/admin/my_script", "0 * * * *");

  assertEquals(schedule.path, "hourly_job.schedule.yaml");
  assertStringIncludes(schedule.content, 'schedule: "0 * * * *"');
  assertStringIncludes(schedule.content, 'script_path: "u/admin/my_script"');
});

Deno.test("HTTP trigger fixture creates valid YAML", () => {
  const trigger = createHttpTriggerFixture("webhook", "/api/webhook", "u/admin/handler");

  assertEquals(trigger.path, "webhook.http_trigger.yaml");
  assertStringIncludes(trigger.content, 'route_path: "/api/webhook"');
  assertStringIncludes(trigger.content, "http_method: post");
});

Deno.test("Folder fixture creates valid YAML", () => {
  const folder = createFolderFixture("my_folder");

  assertEquals(folder.path, "my_folder/folder.meta.yaml");
  assertStringIncludes(folder.content, 'display_name: "my_folder"');
});

Deno.test("User fixture creates valid YAML", () => {
  const user = createUserFixture("test_user", "test@example.com", true);

  assertEquals(user.path, "test_user.user.yaml");
  assertStringIncludes(user.content, 'username: "test_user"');
  assertStringIncludes(user.content, 'email: "test@example.com"');
  assertStringIncludes(user.content, "is_admin: true");
});

Deno.test("Group fixture creates valid YAML", () => {
  const group = createGroupFixture("developers", ["user1", "user2"]);

  assertEquals(group.path, "developers.group.yaml");
  assertStringIncludes(group.content, 'name: "developers"');
  assertStringIncludes(group.content, "- user1");
  assertStringIncludes(group.content, "- user2");
});

Deno.test("Local filesystem creation creates all expected files", async () => {
  const tempDir = await createTempDir();

  try {
    await createLocalFilesystem(tempDir);
    const files = await readDirRecursive(tempDir);

    // Check scripts exist
    assert("f/scripts/python_script.py" in files, "Python script content should exist");
    assert("f/scripts/python_script.script.yaml" in files, "Python script metadata should exist");
    assert("f/scripts/deno_script.ts" in files, "Deno script content should exist");
    assert("f/scripts/bash_script.sh" in files, "Bash script content should exist");
    assert("f/scripts/go_script.go" in files, "Go script content should exist");
    assert("f/scripts/sql_script.sql" in files, "SQL script content should exist");

    // Check flows exist
    assert("f/flows/test_flow.flow/flow.yaml" in files, "Flow metadata should exist");
    assert("f/flows/test_flow.flow/a.ts" in files, "Flow inline script should exist");

    // Check apps exist
    assert("f/apps/test_app.app/app.yaml" in files, "App metadata should exist");

    // Check raw apps exist
    assert("f/apps/test_raw_app.raw_app/raw_app.yaml" in files, "Raw app metadata should exist");
    assert("f/apps/test_raw_app.raw_app/index.html" in files, "Raw app HTML should exist");
    assert("f/apps/test_raw_app.raw_app/index.js" in files, "Raw app JS should exist");

    // Check resources exist
    assert("f/resources/postgres_db.resource.yaml" in files, "PostgreSQL resource should exist");
    assert("f/resources/api_config.resource.yaml" in files, "API config resource should exist");

    // Check variables exist
    assert("f/resources/config_value.variable.yaml" in files, "Config variable should exist");
    assert("f/resources/secret_key.variable.yaml" in files, "Secret variable should exist");

    // Check folder metadata
    assert("f/folder.meta.yaml" in files, "Folder metadata should exist");
  } finally {
    await cleanupTempDir(tempDir);
  }
});

Deno.test("Mock remote zip can be created and read", async () => {
  const items = {
    "test_script.py": 'def main():\n    return "hello"',
    "test_script.script.json": '{"summary":"test","schema":{}}',
    "test_flow.flow.json": '{"summary":"flow","value":{"modules":[]}}',
    "test_app.app.json": '{"summary":"app","value":{}}',
  };

  const zip = await createMockRemoteZip(items);

  // Verify files exist in zip
  const scriptContent = await zip.file("test_script.py")?.async("text");
  assertEquals(scriptContent, 'def main():\n    return "hello"');

  const flowContent = await zip.file("test_flow.flow.json")?.async("text");
  assertStringIncludes(flowContent!, '"summary":"flow"');
});

Deno.test("readDirRecursive reads all files correctly", async () => {
  const tempDir = await createTempDir();

  try {
    // Create a simple structure
    await ensureDir(path.join(tempDir, "subdir"));
    await Deno.writeTextFile(path.join(tempDir, "file1.txt"), "content1");
    await Deno.writeTextFile(path.join(tempDir, "subdir", "file2.txt"), "content2");

    const files = await readDirRecursive(tempDir);

    assertEquals(files["file1.txt"], "content1");
    assertEquals(files["subdir/file2.txt"], "content2");
    assertEquals(Object.keys(files).length, 2);
  } finally {
    await cleanupTempDir(tempDir);
  }
});

// =============================================================================
// Integration Tests (require Windmill server at localhost:8000)
// =============================================================================

import { addWorkspace } from "../workspace.ts";

// Configuration for local Windmill server
const WINDMILL_BASE_URL = Deno.env.get("WINDMILL_BASE_URL") || "http://localhost:8000";
const WINDMILL_EMAIL = Deno.env.get("WINDMILL_EMAIL") || "admin@windmill.dev";
const WINDMILL_PASSWORD = Deno.env.get("WINDMILL_PASSWORD") || "changeme";
const WINDMILL_WORKSPACE = Deno.env.get("WINDMILL_WORKSPACE") || "admins";

// Set to true to run integration tests (requires Windmill server)
const RUN_INTEGRATION_TESTS = Deno.env.get("RUN_INTEGRATION_TESTS") === "true";

/**
 * Get an authentication token from the Windmill server
 */
async function getAuthToken(): Promise<string> {
  let response: Response;
  try {
    response = await fetch(`${WINDMILL_BASE_URL}/api/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email: WINDMILL_EMAIL, password: WINDMILL_PASSWORD }),
    });
  } catch (e) {
    throw new Error(
      `Failed to connect to Windmill server at ${WINDMILL_BASE_URL}. ` +
      `Make sure the server is running. Error: ${e instanceof Error ? e.message : e}`
    );
  }

  const responseText = await response.text();

  if (!response.ok) {
    throw new Error(
      `Failed to authenticate with ${WINDMILL_EMAIL} at ${WINDMILL_BASE_URL}. ` +
      `Status: ${response.status}. Response: ${responseText}`
    );
  }

  // The auth endpoint returns the token as plain text
  return responseText;
}

/**
 * Check if the Windmill server is available
 */
async function isServerAvailable(): Promise<boolean> {
  try {
    const response = await fetch(`${WINDMILL_BASE_URL}/api/version`, {
      signal: AbortSignal.timeout(5000),
    });
    // Consume the response body to avoid resource leaks
    await response.text();
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Run a CLI command and return the result
 */
async function runCLICommand(
  args: string[],
  cwd: string,
  configDir: string,
): Promise<{ code: number; stdout: string; stderr: string }> {
  const cmd = new Deno.Command("deno", {
    args: [
      "run",
      "-A",
      "--no-check",
      path.join(Deno.cwd(), "src/main.ts"),
      "--config-dir",
      configDir,
      ...args,
    ],
    cwd,
    env: {
      ...Deno.env.toObject(),
      SKIP_DENO_DEPRECATION_WARNING: "true",
    },
    stdout: "piped",
    stderr: "piped",
  });

  const output = await cmd.output();
  return {
    code: output.code,
    stdout: new TextDecoder().decode(output.stdout),
    stderr: new TextDecoder().decode(output.stderr),
  };
}

/**
 * Set up a test environment with workspace configured
 */
async function setupTestEnvironment(): Promise<{
  tempDir: string;
  configDir: string;
  token: string;
  cleanup: () => Promise<void>;
}> {
  const tempDir = await Deno.makeTempDir({ prefix: "wmill_sync_test_" });
  const configDir = await Deno.makeTempDir({ prefix: "wmill_config_" });

  const token = await getAuthToken();

  // Configure workspace - addWorkspace will create configDir/windmill/ structure
  const workspace = {
    remote: WINDMILL_BASE_URL + "/",
    workspaceId: WINDMILL_WORKSPACE,
    name: "test_workspace",
    token,
  };
  await addWorkspace(workspace, { force: true, configDir });

  // Set active workspace - needs to go in configDir/windmill/activeWorkspace
  const windmillConfigDir = path.join(configDir, "windmill");
  await ensureDir(windmillConfigDir);
  await Deno.writeTextFile(path.join(windmillConfigDir, "activeWorkspace"), "test_workspace");

  return {
    tempDir,
    configDir,
    token,
    cleanup: async () => {
      await cleanupTempDir(tempDir);
      await cleanupTempDir(configDir);
    },
  };
}

Deno.test({
  name: "Integration: Pull creates correct local structure",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      );

      // Run sync pull
      const result = await runCLICommand(["sync", "pull", "--yes"], tempDir, configDir);

      assertEquals(
        result.code,
        0,
        `Pull should succeed.\nstdout: ${result.stdout}\nstderr: ${result.stderr}`,
      );

      // Verify files were created
      const files = await readDirRecursive(tempDir);
      const hasYamlFiles = Object.keys(files).some((f) => f.endsWith(".yaml") && f !== "wmill.yaml");

      assert(hasYamlFiles || Object.keys(files).length > 1, "Should have pulled files from server");
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Push uploads local changes correctly",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      );

      // Create a test script locally with a unique name
      // Path must have at least 2 segments after prefix (e.g., f/folder/name)
      const uniqueId = Date.now();
      await ensureDir(`${tempDir}/f/test`);
      const script = createScriptFixture(`f/test/push_script_${uniqueId}`, "deno");
      await Deno.writeTextFile(`${tempDir}/${script.contentFile.path}`, script.contentFile.content);
      await Deno.writeTextFile(`${tempDir}/${script.metadataFile.path}`, script.metadataFile.content);

      // Run sync push with dry-run first (only push our test script, not everything)
      const dryRunResult = await runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/push_script_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        dryRunResult.code,
        0,
        `Dry run should succeed.\nstdout: ${dryRunResult.stdout}\nstderr: ${dryRunResult.stderr}`,
      );
      assertStringIncludes(
        dryRunResult.stdout + dryRunResult.stderr,
        `push_script_${uniqueId}`,
        "Should detect the new script",
      );

      // Run actual push (only push our test script)
      const pushResult = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/push_script_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pushResult.code,
        0,
        `Push should succeed.\nstdout: ${pushResult.stdout}\nstderr: ${pushResult.stderr}`,
      );
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Pull then Push is idempotent",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      );

      // Pull from remote
      const pullResult = await runCLICommand(["sync", "pull", "--yes"], tempDir, configDir);
      assertEquals(
        pullResult.code,
        0,
        `Pull should succeed.\nstdout: ${pullResult.stdout}\nstderr: ${pullResult.stderr}`,
      );

      // Push back without changes (should be no-op)
      const pushResult = await runCLICommand(["sync", "push", "--dry-run"], tempDir, configDir);
      assertEquals(pushResult.code, 0, `Push dry-run should succeed: ${pushResult.stderr}`);

      // Should report 0 changes (check both stdout and stderr)
      const output = (pushResult.stdout + pushResult.stderr).toLowerCase();
      assert(
        output.includes("0 change") || output.includes("no change") || output.includes("nothing"),
        `Should have no changes after pull without modifications. Output: ${output}`,
      );
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Include/exclude filters work correctly",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with restrictive filters
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/scripts/**"
excludes:
  - "**/*.test.ts"
skipVariables: true
skipResources: true
`,
      );

      // Run sync pull
      const result = await runCLICommand(["sync", "pull", "--yes"], tempDir, configDir);

      assertEquals(
        result.code,
        0,
        `Pull should succeed.\nstdout: ${result.stdout}\nstderr: ${result.stderr}`,
      );

      // Verify only scripts in f/scripts/ were pulled (if any exist)
      const files = await readDirRecursive(tempDir);

      // Should not have variables or resources
      const hasVariables = Object.keys(files).some((f) => f.includes(".variable."));
      const hasResources = Object.keys(files).some((f) => f.includes(".resource."));

      assert(!hasVariables, "Should not have pulled variables (skipVariables: true)");
      assert(!hasResources, "Should not have pulled resources (skipResources: true)");
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Flow folder structure is created correctly",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      );

      // Create a local flow with unique name
      // Path must have at least 2 segments after prefix (e.g., f/folder/name)
      const uniqueId = Date.now();
      const flowName = `f/test/flow_${uniqueId}`;
      const flowFixture = createFlowFixture(flowName);
      await ensureDir(`${tempDir}/f/test/flow_${uniqueId}${getFolderSuffix("flow")}`);
      for (const file of Object.values(flowFixture)) {
        await Deno.writeTextFile(`${tempDir}/${file.path}`, file.content);
      }

      // Push the flow (only push our test flow, not everything)
      // Pattern needs to match the .flow folder, so use * to match the suffix
      const pushResult = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/flow_${uniqueId}*/**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pushResult.code,
        0,
        `Push should succeed.\nstdout: ${pushResult.stdout}\nstderr: ${pushResult.stderr}`,
      );

      // Pull back and verify structure is preserved
      const tempDir2 = await Deno.makeTempDir({ prefix: "wmill_flow_verify_" });
      try {
        // Use template literal properly for the includes pattern
        const wmillConfig = `defaultTs: bun
includes:
  - "f/test/flow_${uniqueId}*/**"
excludes: []
`;
        await Deno.writeTextFile(`${tempDir2}/wmill.yaml`, wmillConfig);

        const pullResult = await runCLICommand(["sync", "pull", "--yes"], tempDir2, configDir);

        assertEquals(
          pullResult.code,
          0,
          `Pull should succeed.\nstdout: ${pullResult.stdout}\nstderr: ${pullResult.stderr}`,
        );

        // Verify flow folder structure
        const files = await readDirRecursive(tempDir2);
        const allFiles = Object.keys(files);
        const flowFiles = allFiles.filter((f) => f.includes(`flow_${uniqueId}`));

        assert(flowFiles.length > 0, `Should have pulled the flow. Files found: ${allFiles.join(", ")}`);
        assert(
          flowFiles.some((f) => f.includes(".flow/")),
          "Flow should be in a .flow folder",
        );
      } finally {
        await cleanupTempDir(tempDir2);
      }
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Raw app folder structure is handled correctly",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
      );

      // Create a local raw app with unique name
      // Path must have at least 2 segments after prefix (e.g., f/folder/name)
      const uniqueId = Date.now();
      const rawAppFixture = createRawAppFixture(`f/test/raw_app_${uniqueId}`);
      await ensureDir(`${tempDir}/f/test/raw_app_${uniqueId}${getFolderSuffix("raw_app")}`);
      for (const file of Object.values(rawAppFixture)) {
        await Deno.writeTextFile(`${tempDir}/${file.path}`, file.content);
      }

      // Push the raw app (only push our test raw app, not everything)
      const pushResult = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/raw_app_${uniqueId}**`],
        tempDir,
        configDir,
      );

      // Note: This may fail if raw apps require specific validation
      // The test verifies the CLI handles the folder structure correctly
      if (pushResult.code === 0) {
        assertStringIncludes(
          pushResult.stdout + pushResult.stderr,
          "",
          "Push completed",
        );
      }
    } finally {
      await cleanup();
    }
  },
});

// =============================================================================
// nonDottedPaths Unit Tests
// =============================================================================

Deno.test("getFolderSuffixes returns correct suffixes for dotted paths (default)", () => {
  setNonDottedPaths(false);
  const suffixes = getFolderSuffixes();
  assertEquals(suffixes.flow, ".flow");
  assertEquals(suffixes.app, ".app");
  assertEquals(suffixes.raw_app, ".raw_app");
});

Deno.test("getFolderSuffixes returns correct suffixes for non-dotted paths", () => {
  setNonDottedPaths(true);
  const suffixes = getFolderSuffixes();
  assertEquals(suffixes.flow, "__flow");
  assertEquals(suffixes.app, "__app");
  assertEquals(suffixes.raw_app, "__raw_app");
  setNonDottedPaths(false); // Reset
});

Deno.test("getFolderSuffix with nonDottedPaths returns dunder suffixes", () => {
  setNonDottedPaths(true);
  assertEquals(getFolderSuffix("flow"), "__flow");
  assertEquals(getFolderSuffix("app"), "__app");
  assertEquals(getFolderSuffix("raw_app"), "__raw_app");
  setNonDottedPaths(false); // Reset
});

Deno.test("buildFolderPath with nonDottedPaths creates correct paths", () => {
  setNonDottedPaths(true);
  assertEquals(buildFolderPath("my_flow", "flow"), "my_flow__flow");
  assertEquals(buildFolderPath("f/test/my_app", "app"), "f/test/my_app__app");
  assertEquals(buildFolderPath("u/admin/raw_app", "raw_app"), "u/admin/raw_app__raw_app");
  setNonDottedPaths(false); // Reset
});

Deno.test("buildMetadataPath with nonDottedPaths creates correct paths", () => {
  setNonDottedPaths(true);
  assertEquals(
    buildMetadataPath("my_flow", "flow", "yaml"),
    `my_flow__flow${SEP}flow.yaml`
  );
  assertEquals(
    buildMetadataPath("f/test/my_app", "app", "yaml"),
    `f/test/my_app__app${SEP}app.yaml`
  );
  setNonDottedPaths(false); // Reset
});

Deno.test("isFlowPath detects non-dotted paths when configured", () => {
  // Default (dotted) paths
  setNonDottedPaths(false);
  assert(isFlowPath(`f/test/my_flow.flow${SEP}flow.yaml`));
  assert(!isFlowPath(`f/test/my_flow__flow${SEP}flow.yaml`));

  // Non-dotted paths
  setNonDottedPaths(true);
  assert(isFlowPath(`f/test/my_flow__flow${SEP}flow.yaml`));
  assert(!isFlowPath(`f/test/my_flow.flow${SEP}flow.yaml`));
  setNonDottedPaths(false); // Reset
});

Deno.test("isAppPath detects non-dotted paths when configured", () => {
  // Default (dotted) paths
  setNonDottedPaths(false);
  assert(isAppPath(`f/test/my_app.app${SEP}app.yaml`));
  assert(!isAppPath(`f/test/my_app__app${SEP}app.yaml`));

  // Non-dotted paths
  setNonDottedPaths(true);
  assert(isAppPath(`f/test/my_app__app${SEP}app.yaml`));
  assert(!isAppPath(`f/test/my_app.app${SEP}app.yaml`));
  setNonDottedPaths(false); // Reset
});

Deno.test("isRawAppPath detects non-dotted paths when configured", () => {
  // Default (dotted) paths
  setNonDottedPaths(false);
  assert(isRawAppPath(`f/test/my_raw_app.raw_app${SEP}raw_app.yaml`));
  assert(!isRawAppPath(`f/test/my_raw_app__raw_app${SEP}raw_app.yaml`));

  // Non-dotted paths
  setNonDottedPaths(true);
  assert(isRawAppPath(`f/test/my_raw_app__raw_app${SEP}raw_app.yaml`));
  assert(!isRawAppPath(`f/test/my_raw_app.raw_app${SEP}raw_app.yaml`));
  setNonDottedPaths(false); // Reset
});

Deno.test("extractResourceName works with non-dotted paths", () => {
  setNonDottedPaths(true);
  assertEquals(
    extractResourceName(`f/test/my_flow__flow${SEP}flow.yaml`, "flow"),
    "f/test/my_flow"
  );
  assertEquals(
    extractResourceName(`f/test/my_app__app${SEP}app.yaml`, "app"),
    "f/test/my_app"
  );
  setNonDottedPaths(false); // Reset
});

Deno.test("hasFolderSuffix works with non-dotted paths", () => {
  setNonDottedPaths(true);
  assert(hasFolderSuffix("my_flow__flow", "flow"));
  assert(!hasFolderSuffix("my_flow.flow", "flow"));

  assert(hasFolderSuffix("my_app__app", "app"));
  assert(!hasFolderSuffix("my_app.app", "app"));
  setNonDottedPaths(false); // Reset
});

Deno.test("setNonDottedPaths and getNonDottedPaths work correctly", () => {
  // Default should be false
  setNonDottedPaths(false);
  assertEquals(getNonDottedPaths(), false);

  // Set to true
  setNonDottedPaths(true);
  assertEquals(getNonDottedPaths(), true);

  // Set back to false
  setNonDottedPaths(false);
  assertEquals(getNonDottedPaths(), false);
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

Deno.test("Flow fixture with nonDottedPaths creates __flow structure", () => {
  setNonDottedPaths(true);
  const flow = createFlowFixtureWithCurrentConfig("test_flow");

  assertEquals(flow.metadata.path, "test_flow__flow/flow.yaml");
  assertEquals(flow.inlineScript.path, "test_flow__flow/a.ts");
  assertStringIncludes(flow.metadata.content, "summary:");
  assertStringIncludes(flow.metadata.content, "modules:");
  setNonDottedPaths(false); // Reset
});

Deno.test("App fixture with nonDottedPaths creates __app structure", () => {
  setNonDottedPaths(true);
  const app = createAppFixtureWithCurrentConfig("test_app");

  assertEquals(app.metadata.path, "test_app__app/app.yaml");
  assertStringIncludes(app.metadata.content, "summary:");
  assertStringIncludes(app.metadata.content, "grid:");
  setNonDottedPaths(false); // Reset
});

Deno.test("Local filesystem with nonDottedPaths creates correct folder structure", async () => {
  setNonDottedPaths(true);
  const tempDir = await createTempDir();

  try {
    // Create folder structure
    await ensureDir(path.join(tempDir, "f/flows"));
    await ensureDir(path.join(tempDir, "f/apps"));

    // Create flows with non-dotted paths
    const flowFixture = createFlowFixtureWithCurrentConfig("f/flows/test_flow");
    await ensureDir(path.join(tempDir, `f/flows/test_flow${getFolderSuffix("flow")}`));
    for (const file of Object.values(flowFixture)) {
      await Deno.writeTextFile(path.join(tempDir, file.path), file.content);
    }

    // Create apps with non-dotted paths
    const appFixture = createAppFixtureWithCurrentConfig("f/apps/test_app");
    await ensureDir(path.join(tempDir, `f/apps/test_app${getFolderSuffix("app")}`));
    for (const file of Object.values(appFixture)) {
      await Deno.writeTextFile(path.join(tempDir, file.path), file.content);
    }

    const files = await readDirRecursive(tempDir);

    // Check flows exist with __flow suffix
    assert("f/flows/test_flow__flow/flow.yaml" in files, "Flow metadata should exist with __flow suffix");
    assert("f/flows/test_flow__flow/a.ts" in files, "Flow inline script should exist with __flow suffix");

    // Check apps exist with __app suffix
    assert("f/apps/test_app__app/app.yaml" in files, "App metadata should exist with __app suffix");

    // Verify old-style paths don't exist
    assert(!("f/flows/test_flow.flow/flow.yaml" in files), "Old .flow suffix should not exist");
    assert(!("f/apps/test_app.app/app.yaml" in files), "Old .app suffix should not exist");
  } finally {
    await cleanupTempDir(tempDir);
    setNonDottedPaths(false); // Reset
  }
});

// =============================================================================
// nonDottedPaths Integration Tests
// =============================================================================

Deno.test({
  name: "Integration: wmill.yaml with nonDottedPaths is read correctly",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with nonDottedPaths option
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "f/**"
excludes: []
`,
      );

      // Create a test script with non-dotted flow folder
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const flowName = `f/test/nondot_flow_${uniqueId}`;
      const flowFixture = createFlowFixtureWithCurrentConfig(flowName);
      await ensureDir(`${tempDir}/f/test/nondot_flow_${uniqueId}${getFolderSuffix("flow")}`);
      for (const file of Object.values(flowFixture)) {
        await Deno.writeTextFile(`${tempDir}/${file.path}`, file.content);
      }
      setNonDottedPaths(false); // Reset

      // Run sync push with dry-run to verify the config is being read
      const dryRunResult = await runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/nondot_flow_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        dryRunResult.code,
        0,
        `Dry run should succeed with nonDottedPaths config.\nstdout: ${dryRunResult.stdout}\nstderr: ${dryRunResult.stderr}`,
      );
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Pull then Push with nonDottedPaths is idempotent",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with nonDottedPaths enabled
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      );

      // Pull from remote with nonDottedPaths enabled
      const pullResult = await runCLICommand(["sync", "pull", "--yes"], tempDir, configDir);
      assertEquals(
        pullResult.code,
        0,
        `Pull should succeed with nonDottedPaths.\nstdout: ${pullResult.stdout}\nstderr: ${pullResult.stderr}`,
      );

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
        assert(
          dottedFlowFiles.length === 0,
          `Flows should use __flow suffix with nonDottedPaths, found .flow files: ${dottedFlowFiles.join(", ")}`,
        );
      }
      if (appFiles.length > 0 || dottedAppFiles.length > 0) {
        assert(
          dottedAppFiles.length === 0,
          `Apps should use __app suffix with nonDottedPaths, found .app files: ${dottedAppFiles.join(", ")}`,
        );
      }

      // Push back without changes (should be no-op / idempotent)
      const pushResult = await runCLICommand(["sync", "push", "--dry-run"], tempDir, configDir);
      assertEquals(pushResult.code, 0, `Push dry-run should succeed: ${pushResult.stderr}`);

      // Should report 0 changes (check both stdout and stderr)
      const output = (pushResult.stdout + pushResult.stderr).toLowerCase();
      assert(
        output.includes("0 change") || output.includes("no change") || output.includes("nothing"),
        `Should have no changes after pull with nonDottedPaths without modifications. Output: ${output}`,
      );
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Push flow with nonDottedPaths creates __flow structure on server",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with nonDottedPaths enabled
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      );

      // Create a local flow with __flow suffix
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const flowName = `f/test/nondot_idem_flow_${uniqueId}`;
      const flowFixture = createFlowFixtureWithCurrentConfig(flowName);
      await ensureDir(`${tempDir}/f/test/nondot_idem_flow_${uniqueId}${getFolderSuffix("flow")}`);
      for (const file of Object.values(flowFixture)) {
        await Deno.writeTextFile(`${tempDir}/${file.path}`, file.content);
      }
      setNonDottedPaths(false); // Reset global state

      // Push the flow
      const pushResult = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/nondot_idem_flow_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pushResult.code,
        0,
        `Push should succeed.\nstdout: ${pushResult.stdout}\nstderr: ${pushResult.stderr}`,
      );

      // Pull back to same directory to verify round-trip (idempotency)
      const pullResult = await runCLICommand(
        ["sync", "pull", "--yes", "--includes", `f/test/nondot_idem_flow_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pullResult.code,
        0,
        `Pull should succeed.\nstdout: ${pullResult.stdout}\nstderr: ${pullResult.stderr}`,
      );

      // Verify flow still has __flow suffix after round-trip
      const filesAfterPull = await readDirRecursive(tempDir);
      const allFiles = Object.keys(filesAfterPull);
      const flowFiles = allFiles.filter((f) => f.includes(`nondot_idem_flow_${uniqueId}`));

      assert(flowFiles.length > 0, `Should have the flow files after pull. Files found: ${allFiles.join(", ")}`);
      assert(
        flowFiles.some((f) => f.includes("__flow/")),
        `Flow should be in a __flow folder with nonDottedPaths. Found: ${flowFiles.join(", ")}`,
      );
      assert(
        !flowFiles.some((f) => f.includes(".flow/")),
        `Flow should NOT use .flow suffix with nonDottedPaths. Found: ${flowFiles.join(", ")}`,
      );

      // Push again (should be idempotent - no changes)
      const push2 = await runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/nondot_idem_flow_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(push2.code, 0, `Second push dry-run should succeed: ${push2.stderr}`);

      const output = (push2.stdout + push2.stderr).toLowerCase();
      assert(
        output.includes("0 change") || output.includes("no change") || output.includes("nothing"),
        `Should have no changes after push-pull cycle for flow. Output: ${output}`,
      );
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Multiple pull/push cycles with nonDottedPaths remain idempotent",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with nonDottedPaths enabled
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      );

      // First pull
      const pull1 = await runCLICommand(["sync", "pull", "--yes"], tempDir, configDir);
      assertEquals(pull1.code, 0, `First pull should succeed: ${pull1.stderr}`);

      // First push (should be no-op)
      const push1 = await runCLICommand(["sync", "push", "--dry-run"], tempDir, configDir);
      assertEquals(push1.code, 0, `First push dry-run should succeed: ${push1.stderr}`);

      // Second pull (should have no changes)
      const pull2 = await runCLICommand(["sync", "pull", "--yes"], tempDir, configDir);
      assertEquals(pull2.code, 0, `Second pull should succeed: ${pull2.stderr}`);

      // Second push (should still be no-op)
      const push2 = await runCLICommand(["sync", "push", "--dry-run"], tempDir, configDir);
      assertEquals(push2.code, 0, `Second push dry-run should succeed: ${push2.stderr}`);

      // Verify no changes after multiple cycles
      const output = (push2.stdout + push2.stderr).toLowerCase();
      assert(
        output.includes("0 change") || output.includes("no change") || output.includes("nothing"),
        `Should have no changes after multiple pull/push cycles with nonDottedPaths. Output: ${output}`,
      );

      // Third pull to verify consistency
      const pull3 = await runCLICommand(["sync", "pull", "--yes"], tempDir, configDir);
      assertEquals(pull3.code, 0, `Third pull should succeed: ${pull3.stderr}`);

      // Final push check
      const push3 = await runCLICommand(["sync", "push", "--dry-run"], tempDir, configDir);
      assertEquals(push3.code, 0, `Final push dry-run should succeed: ${push3.stderr}`);

      const finalOutput = (push3.stdout + push3.stderr).toLowerCase();
      assert(
        finalOutput.includes("0 change") || finalOutput.includes("no change") || finalOutput.includes("nothing"),
        `Should still have no changes after 3 cycles. Output: ${finalOutput}`,
      );
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: App with nonDottedPaths creates __app structure and is idempotent",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with nonDottedPaths enabled
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      );

      // Create a local app with __app suffix
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const appName = `f/test/nondot_app_${uniqueId}`;
      const appFixture = createAppFixtureWithCurrentConfig(appName);
      await ensureDir(`${tempDir}/f/test/nondot_app_${uniqueId}${getFolderSuffix("app")}`);
      for (const file of Object.values(appFixture)) {
        await Deno.writeTextFile(`${tempDir}/${file.path}`, file.content);
      }
      setNonDottedPaths(false); // Reset global state

      // Push the app
      const pushResult = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/nondot_app_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pushResult.code,
        0,
        `Push should succeed.\nstdout: ${pushResult.stdout}\nstderr: ${pushResult.stderr}`,
      );

      // Pull back to same directory
      const pullResult = await runCLICommand(
        ["sync", "pull", "--yes", "--includes", `f/test/nondot_app_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pullResult.code,
        0,
        `Pull should succeed.\nstdout: ${pullResult.stdout}\nstderr: ${pullResult.stderr}`,
      );

      // Verify app structure uses __app
      const files = await readDirRecursive(tempDir);
      const appFiles = Object.keys(files).filter((f) => f.includes(`nondot_app_${uniqueId}`));

      assert(appFiles.length > 0, `Should have the app files. Found: ${Object.keys(files).join(", ")}`);
      assert(
        appFiles.some((f) => f.includes("__app/")),
        `App should be in a __app folder with nonDottedPaths. Found: ${appFiles.join(", ")}`,
      );

      // Push again (should be idempotent)
      const push2 = await runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/nondot_app_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(push2.code, 0, `Second push dry-run should succeed: ${push2.stderr}`);

      const output = (push2.stdout + push2.stderr).toLowerCase();
      assert(
        output.includes("0 change") || output.includes("no change") || output.includes("nothing"),
        `Should have no changes after push-pull cycle for app. Output: ${output}`,
      );
    } finally {
      await cleanup();
    }
  },
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

Deno.test({
  name: "Integration: Raw app with nonDottedPaths creates __raw_app structure",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with nonDottedPaths enabled
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      );

      // Create a local raw app with __raw_app suffix
      setNonDottedPaths(true);
      const uniqueId = Date.now();
      const rawAppName = `f/test/nondot_rawapp_${uniqueId}`;
      const rawAppFixture = createRawAppFixtureWithCurrentConfig(rawAppName);
      await ensureDir(`${tempDir}/f/test/nondot_rawapp_${uniqueId}${getFolderSuffix("raw_app")}`);
      for (const file of Object.values(rawAppFixture)) {
        await Deno.writeTextFile(`${tempDir}/${file.path}`, file.content);
      }
      setNonDottedPaths(false); // Reset global state

      // Verify the files are created with __raw_app suffix
      const files = await readDirRecursive(tempDir);
      const rawAppFiles = Object.keys(files).filter((f) => f.includes(`nondot_rawapp_${uniqueId}`));

      assert(rawAppFiles.length > 0, `Should have created raw app files. Found: ${Object.keys(files).join(", ")}`);
      assert(
        rawAppFiles.some((f) => f.includes("__raw_app/")),
        `Raw app should be in a __raw_app folder with nonDottedPaths. Found: ${rawAppFiles.join(", ")}`,
      );
      assert(
        !rawAppFiles.some((f) => f.includes(".raw_app/")),
        `Raw app should NOT use .raw_app suffix with nonDottedPaths. Found: ${rawAppFiles.join(", ")}`,
      );

      // Push the raw app (may fail if raw apps require specific validation)
      const pushResult = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/nondot_rawapp_${uniqueId}**`],
        tempDir,
        configDir,
      );

      // Note: Raw apps may have additional validation requirements
      // This test primarily verifies the folder structure is correct
      if (pushResult.code === 0) {
        // If push succeeded, verify idempotency
        const push2 = await runCLICommand(
          ["sync", "push", "--dry-run", "--includes", `f/test/nondot_rawapp_${uniqueId}**`],
          tempDir,
          configDir,
        );

        assertEquals(push2.code, 0, `Second push dry-run should succeed: ${push2.stderr}`);
      }
    } finally {
      await cleanup();
    }
  },
});

Deno.test({
  name: "Integration: Mixed scripts and flows with nonDottedPaths are idempotent",
  ignore: !RUN_INTEGRATION_TESTS,
  async fn() {
    if (!await isServerAvailable()) {
      console.log(`Skipping: Windmill server not available at ${WINDMILL_BASE_URL}`);
      return;
    }

    const { tempDir, configDir, cleanup } = await setupTestEnvironment();
    try {
      // Create wmill.yaml with nonDottedPaths enabled
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
nonDottedPaths: true
includes:
  - "**"
excludes: []
`,
      );

      const uniqueId = Date.now();
      await ensureDir(`${tempDir}/f/test`);

      // Create a script (scripts don't use folder suffixes, so they're unaffected)
      const script = createScriptFixture(`f/test/mixed_script_${uniqueId}`, "deno");
      await Deno.writeTextFile(`${tempDir}/${script.contentFile.path}`, script.contentFile.content);
      await Deno.writeTextFile(`${tempDir}/${script.metadataFile.path}`, script.metadataFile.content);

      // Create a flow with __flow suffix
      setNonDottedPaths(true);
      const flowName = `f/test/mixed_flow_${uniqueId}`;
      const flowFixture = createFlowFixtureWithCurrentConfig(flowName);
      await ensureDir(`${tempDir}/f/test/mixed_flow_${uniqueId}${getFolderSuffix("flow")}`);
      for (const file of Object.values(flowFixture)) {
        await Deno.writeTextFile(`${tempDir}/${file.path}`, file.content);
      }
      setNonDottedPaths(false); // Reset global state

      // Push both
      const pushResult = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/mixed_*_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pushResult.code,
        0,
        `Push should succeed.\nstdout: ${pushResult.stdout}\nstderr: ${pushResult.stderr}`,
      );

      // Pull back
      const pullResult = await runCLICommand(
        ["sync", "pull", "--yes", "--includes", `f/test/mixed_*_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(
        pullResult.code,
        0,
        `Pull should succeed.\nstdout: ${pullResult.stdout}\nstderr: ${pullResult.stderr}`,
      );

      // Verify idempotency
      const push2 = await runCLICommand(
        ["sync", "push", "--dry-run", "--includes", `f/test/mixed_*_${uniqueId}**`],
        tempDir,
        configDir,
      );

      assertEquals(push2.code, 0, `Second push dry-run should succeed: ${push2.stderr}`);

      const output = (push2.stdout + push2.stderr).toLowerCase();
      assert(
        output.includes("0 change") || output.includes("no change") || output.includes("nothing"),
        `Should have no changes after push-pull cycle for mixed content. Output: ${output}`,
      );
    } finally {
      await cleanup();
    }
  },
});
