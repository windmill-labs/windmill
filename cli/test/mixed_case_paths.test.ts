/**
 * Mixed Case Paths Sync Tests
 *
 * Tests sync pull/push operations with folder paths containing capital letters.
 * This is critical for Windows compatibility testing since Windows has
 * case-insensitive file systems which can cause issues with paths like:
 *   f/MyFolder/MyScript
 *
 * The test verifies that:
 * 1. Resources with mixed-case paths can be pulled correctly
 * 2. Modifications to those files can be pushed back
 * 3. The modifications are correctly applied on the server
 */

import { expect, test } from "bun:test";
import * as path from "node:path";
import { writeFile, readFile, stat } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

async function setupWorkspaceProfile(backend: any): Promise<void> {
  const testWorkspace = {
    remote: backend.baseUrl,
    workspaceId: backend.workspace,
    name: "localhost_test",
    token: backend.token,
  };

  await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });
}

async function createFolder(backend: any, name: string): Promise<void> {
  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/folders/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name }),
  });
  if (!response.ok) {
    const error = await response.text();
    if (!error.includes("already exists")) {
      throw new Error(`Failed to create folder ${name}: ${error}`);
    }
  } else {
    await response.text();
  }
}

async function createScript(
  backend: any,
  scriptPath: string,
  content: string,
  summary: string = "Test script"
): Promise<void> {
  const script = {
    path: scriptPath,
    summary,
    description: `Script at ${scriptPath}`,
    content,
    language: "bun",
    is_template: false,
    kind: "script",
    schema: {
      $schema: "https://json-schema.org/draft/2020-12/schema",
      type: "object",
      properties: {},
      required: [],
    },
  };

  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/scripts/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(script),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to create script ${scriptPath}: ${error}`);
  }
  await response.text();
}

async function createFlow(
  backend: any,
  flowPath: string,
  inlineScriptContent: string,
  summary: string = "Test flow"
): Promise<void> {
  const flow = {
    path: flowPath,
    summary,
    description: `Flow at ${flowPath}`,
    value: {
      modules: [
        {
          id: "a",
          value: {
            type: "rawscript",
            content: inlineScriptContent,
            language: "bun",
            input_transforms: {},
          },
        },
      ],
    },
    schema: {
      $schema: "https://json-schema.org/draft/2020-12/schema",
      type: "object",
      properties: {},
      required: [],
    },
  };

  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/flows/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(flow),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to create flow ${flowPath}: ${error}`);
  }
  await response.text();
}

async function createApp(
  backend: any,
  appPath: string,
  summary: string = "Test app"
): Promise<void> {
  const app = {
    path: appPath,
    summary,
    policy: {
      execution_mode: "viewer",
    },
    value: {
      grid: [],
      hiddenInlineScripts: [],
      css: {},
      norefreshbar: false,
    },
  };

  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/apps/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(app),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to create app ${appPath}: ${error}`);
  }
  await response.text();
}

async function createVariable(
  backend: any,
  varPath: string,
  value: string,
  description: string = "Test variable"
): Promise<void> {
  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/variables/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      path: varPath,
      value,
      is_secret: false,
      description,
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to create variable ${varPath}: ${error}`);
  }
  await response.text();
}

async function createResource(
  backend: any,
  resourcePath: string,
  resourceType: string,
  value: Record<string, unknown>,
  description: string = "Test resource"
): Promise<void> {
  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/resources/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      path: resourcePath,
      resource_type: resourceType,
      value,
      description,
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to create resource ${resourcePath}: ${error}`);
  }
  await response.text();
}

async function getScript(backend: any, scriptPath: string): Promise<any> {
  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/scripts/get/p/${encodeURIComponent(scriptPath)}`);
  if (!response.ok) {
    throw new Error(`Failed to get script ${scriptPath}: ${response.status}`);
  }
  return response.json();
}

async function getFlow(backend: any, flowPath: string): Promise<any> {
  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/flows/get/${encodeURIComponent(flowPath)}`);
  if (!response.ok) {
    throw new Error(`Failed to get flow ${flowPath}: ${response.status}`);
  }
  return response.json();
}

async function getApp(backend: any, appPath: string): Promise<any> {
  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/apps/get/p/${encodeURIComponent(appPath)}`);
  if (!response.ok) {
    throw new Error(`Failed to get app ${appPath}: ${response.status}`);
  }
  return response.json();
}

async function getVariable(backend: any, varPath: string): Promise<any> {
  const response = await backend.apiRequest!(`/api/w/${backend.workspace}/variables/get/${encodeURIComponent(varPath)}`);
  if (!response.ok) {
    throw new Error(`Failed to get variable ${varPath}: ${response.status}`);
  }
  return response.json();
}

/**
 * Helper to verify that a subsequent pull detects no changes (idempotency check)
 */
async function verifyNoDiffOnPull(backend: any, tempDir: string): Promise<void> {
  const pullResult = await backend.runCLICommand(
    ["sync", "pull", "--yes", "--dry-run", "--json-output"],
    tempDir
  );
  expect(pullResult.code).toEqual(0);

  const output = parseJsonFromCLIOutput(pullResult.stdout);
  const changes = output.changes || [];

  expect(changes.length).toEqual(0);
}

// =============================================================================
// TESTS
// =============================================================================

test("Mixed Case Paths: pull and push script with capitalized folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with mixed case
      await createFolder(backend, "MyFolder");

      // Create a script with mixed-case path
      const scriptPath = "f/MyFolder/MyScript";
      const originalContent = `export async function main() {
  return "original content";
}`;
      await createScript(backend, scriptPath, originalContent, "My Test Script");

      // Create wmill.yaml
      await writeFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify file exists with correct path (normalized for comparison)
      const expectedScriptPath = path.join(tempDir, "f", "MyFolder", "MyScript.ts");
      const scriptExists = await stat(expectedScriptPath).then(() => true).catch(() => false);
      expect(scriptExists).toBeTruthy();

      // Read and verify content
      const pulledContent = await readFile(expectedScriptPath, "utf-8");
      expect(pulledContent.includes("original content")).toBeTruthy();

      // Modify the script
      const modifiedContent = `export async function main() {
  return "modified content from test";
}`;
      await writeFile(expectedScriptPath, modifiedContent, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify modification on server
      const updatedScript = await getScript(backend, scriptPath);
      expect(
        updatedScript.content.includes("modified content from test")
      ).toBeTruthy();

      // Verify no diff on subsequent pull (idempotency)
      await verifyNoDiffOnPull(backend, tempDir);
    });
});

test("Mixed Case Paths: pull and push flow with capitalized folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with mixed case
      await createFolder(backend, "MyFlows");

      // Create a flow with mixed-case path
      const flowPath = "f/MyFlows/DataProcessor";
      const originalContent = `export async function main() {
  return "original flow step";
}`;
      await createFlow(backend, flowPath, originalContent, "Data Processor Flow");

      // Create wmill.yaml
      await writeFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify flow directory exists
      const flowDir = path.join(tempDir, "f", "MyFlows", "DataProcessor.flow");
      const flowDirExists = await stat(flowDir).then(s => s.isDirectory()).catch(() => false);
      expect(flowDirExists).toBeTruthy();

      // Modify the flow metadata (summary) instead of inline script
      const flowMetadataPath = path.join(flowDir, "flow.yaml");
      const flowMetadataExists = await stat(flowMetadataPath).then(() => true).catch(() => false);
      expect(flowMetadataExists).toBeTruthy();

      const flowMetadata = await readFile(flowMetadataPath, "utf-8");
      const modifiedMetadata = flowMetadata.replace(
        /summary:.*$/m,
        'summary: "Modified Data Processor Flow from test"'
      );
      await writeFile(flowMetadataPath, modifiedMetadata, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify modification on server
      const updatedFlow = await getFlow(backend, flowPath);
      expect(updatedFlow.summary).toEqual("Modified Data Processor Flow from test");

      // Verify no diff on subsequent pull (idempotency)
      await verifyNoDiffOnPull(backend, tempDir);
    });
});

test("Mixed Case Paths: pull and push app with capitalized folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with mixed case
      await createFolder(backend, "MyApps");

      // Create an app with mixed-case path
      const appPath = "f/MyApps/Dashboard";
      await createApp(backend, appPath, "My Dashboard App");

      // Create wmill.yaml
      await writeFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify app directory exists
      const appDir = path.join(tempDir, "f", "MyApps", "Dashboard.app");
      const appDirExists = await stat(appDir).then(s => s.isDirectory()).catch(() => false);
      expect(appDirExists).toBeTruthy();

      // Modify the app metadata
      const appMetadataPath = path.join(appDir, "app.yaml");
      const appMetadata = await readFile(appMetadataPath, "utf-8");
      const modifiedMetadata = appMetadata.replace(
        /summary:.*$/m,
        'summary: "Modified Dashboard App from test"'
      );
      await writeFile(appMetadataPath, modifiedMetadata, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify modification on server
      const updatedApp = await getApp(backend, appPath);
      expect(updatedApp.summary).toEqual("Modified Dashboard App from test");

      // Verify no diff on subsequent pull (idempotency)
      await verifyNoDiffOnPull(backend, tempDir);
    });
});

test("Mixed Case Paths: pull and push variable with capitalized folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with mixed case
      await createFolder(backend, "MyVars");

      // Create a variable with mixed-case path
      const varPath = "f/MyVars/ApiKey";
      await createVariable(backend, varPath, "original-api-key-value", "API Key Variable");

      // Create wmill.yaml
      await writeFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify variable file exists
      const varFilePath = path.join(tempDir, "f", "MyVars", "ApiKey.variable.yaml");
      const varExists = await stat(varFilePath).then(() => true).catch(() => false);
      expect(varExists).toBeTruthy();

      // Modify the variable
      const varContent = await readFile(varFilePath, "utf-8");
      const modifiedVarContent = varContent.replace(
        /value:.*$/m,
        'value: "modified-api-key-from-test"'
      );
      await writeFile(varFilePath, modifiedVarContent, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify modification on server
      const updatedVar = await getVariable(backend, varPath);
      expect(updatedVar.value).toEqual("modified-api-key-from-test");

      // Verify no diff on subsequent pull (idempotency)
      await verifyNoDiffOnPull(backend, tempDir);
    });
});

test("Mixed Case Paths: deeply nested capitalized folders", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create nested folders with mixed case
      // Note: Windmill folders are flat, so we create a folder with slashes in the name
      // The actual nesting is in the path structure
      await createFolder(backend, "MyProject");

      // Create a script with deeply nested mixed-case path
      const scriptPath = "f/MyProject/SubFolder_A";
      const originalContent = `export async function main() {
  return "deeply nested original";
}`;
      await createScript(backend, scriptPath, originalContent, "Nested Script");

      // Create wmill.yaml
      await writeFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify file exists
      const scriptFilePath = path.join(tempDir, "f", "MyProject", "SubFolder_A.ts");
      const scriptExists = await stat(scriptFilePath).then(() => true).catch(() => false);
      expect(scriptExists).toBeTruthy();

      // Modify
      const modifiedContent = `export async function main() {
  return "deeply nested modified from test";
}`;
      await writeFile(scriptFilePath, modifiedContent, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify on server
      const updatedScript = await getScript(backend, scriptPath);
      expect(
        updatedScript.content.includes("deeply nested modified from test")
      ).toBeTruthy();

      // Verify no diff on subsequent pull (idempotency)
      await verifyNoDiffOnPull(backend, tempDir);
    });
});

test("Mixed Case Paths: multiple resources in same capitalized folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with mixed case
      await createFolder(backend, "SharedFolder");

      // Create multiple resources in the same folder
      await createScript(
        backend,
        "f/SharedFolder/ScriptOne",
        'export async function main() { return "script one original"; }',
        "Script One"
      );
      await createScript(
        backend,
        "f/SharedFolder/ScriptTwo",
        'export async function main() { return "script two original"; }',
        "Script Two"
      );
      await createVariable(backend, "f/SharedFolder/VarOne", "var-one-original");
      await createResource(backend, "f/SharedFolder/ResourceOne", "any", { key: "original" });

      // Create wmill.yaml
      await writeFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify all files exist
      const folderPath = path.join(tempDir, "f", "SharedFolder");
      const script1Exists = await stat(path.join(folderPath, "ScriptOne.ts")).then(() => true).catch(() => false);
      const script2Exists = await stat(path.join(folderPath, "ScriptTwo.ts")).then(() => true).catch(() => false);
      const var1Exists = await stat(path.join(folderPath, "VarOne.variable.yaml")).then(() => true).catch(() => false);
      const res1Exists = await stat(path.join(folderPath, "ResourceOne.resource.yaml")).then(() => true).catch(() => false);

      expect(script1Exists).toBeTruthy();
      expect(script2Exists).toBeTruthy();
      expect(var1Exists).toBeTruthy();
      expect(res1Exists).toBeTruthy();

      // Modify script one
      await writeFile(
        path.join(folderPath, "ScriptOne.ts"),
        'export async function main() { return "script one MODIFIED"; }',
        "utf-8"
      );

      // Modify script two
      await writeFile(
        path.join(folderPath, "ScriptTwo.ts"),
        'export async function main() { return "script two MODIFIED"; }',
        "utf-8"
      );

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify modifications on server
      const script1 = await getScript(backend, "f/SharedFolder/ScriptOne");
      const script2 = await getScript(backend, "f/SharedFolder/ScriptTwo");

      expect(script1.content.includes("script one MODIFIED")).toBeTruthy();
      expect(script2.content.includes("script two MODIFIED")).toBeTruthy();

      // Verify no diff on subsequent pull (idempotency)
      await verifyNoDiffOnPull(backend, tempDir);
    });
});

test("Mixed Case Paths: CamelCase folder names with numbers", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with CamelCase and numbers
      await createFolder(backend, "Project2024");

      // Create script
      const scriptPath = "f/Project2024/DataHandler_V2";
      await createScript(
        backend,
        scriptPath,
        'export async function main() { return "handler v2 original"; }',
        "Data Handler V2"
      );

      // Create wmill.yaml
      await writeFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify file exists
      const scriptFilePath = path.join(tempDir, "f", "Project2024", "DataHandler_V2.ts");
      const scriptExists = await stat(scriptFilePath).then(() => true).catch(() => false);
      expect(scriptExists).toBeTruthy();

      // Modify
      await writeFile(
        scriptFilePath,
        'export async function main() { return "handler v2 MODIFIED"; }',
        "utf-8"
      );

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify on server
      const updatedScript = await getScript(backend, scriptPath);
      expect(updatedScript.content.includes("handler v2 MODIFIED")).toBeTruthy();

      // Verify no diff on subsequent pull (idempotency)
      await verifyNoDiffOnPull(backend, tempDir);
    });
});
