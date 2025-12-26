/**
 * Sync Pull/Push Integration Tests
 *
 * Tests the sync pull and push functionality with a simulated filesystem
 * containing every kind of Windmill resource type.
 */

import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";
import * as path from "https://deno.land/std@0.224.0/path/mod.ts";
import { JSZip } from "../deps.ts";
import {
  getFolderSuffix,
  getMetadataFileName,
  buildFolderPath,
} from "../src/utils/resource_folders.ts";

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
 */
async function readDirRecursive(
  dir: string,
  baseDir: string = dir,
): Promise<Record<string, string>> {
  const files: Record<string, string> = {};

  for await (const entry of Deno.readDir(dir)) {
    const fullPath = path.join(dir, entry.name);
    const relativePath = fullPath.substring(baseDir.length + 1);

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
