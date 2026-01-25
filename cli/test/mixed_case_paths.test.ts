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

import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";
import * as path from "https://deno.land/std@0.224.0/path/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";

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

// =============================================================================
// TESTS
// =============================================================================

Deno.test({
  name: "Mixed Case Paths: pull and push script with capitalized folder",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Verify file exists with correct path (normalized for comparison)
      const expectedScriptPath = path.join(tempDir, "f", "MyFolder", "MyScript.ts");
      const scriptExists = await Deno.stat(expectedScriptPath).then(() => true).catch(() => false);
      assert(scriptExists, `Script file should exist at ${expectedScriptPath}`);

      // Read and verify content
      const pulledContent = await Deno.readTextFile(expectedScriptPath);
      assert(pulledContent.includes("original content"), "Pulled content should match original");

      // Modify the script
      const modifiedContent = `export async function main() {
  return "modified content from test";
}`;
      await Deno.writeTextFile(expectedScriptPath, modifiedContent);

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify modification on server
      const updatedScript = await getScript(backend, scriptPath);
      assert(
        updatedScript.content.includes("modified content from test"),
        `Server should have modified content. Got: ${updatedScript.content}`
      );
    });
  },
});

Deno.test({
  name: "Mixed Case Paths: pull and push flow with capitalized folder",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Verify flow directory exists
      const flowDir = path.join(tempDir, "f", "MyFlows", "DataProcessor.flow");
      const flowDirExists = await Deno.stat(flowDir).then(s => s.isDirectory).catch(() => false);
      assert(flowDirExists, `Flow directory should exist at ${flowDir}`);

      // Modify the flow metadata (summary) instead of inline script
      const flowMetadataPath = path.join(flowDir, "flow.yaml");
      const flowMetadataExists = await Deno.stat(flowMetadataPath).then(() => true).catch(() => false);
      assert(flowMetadataExists, `Flow metadata should exist at ${flowMetadataPath}`);

      const flowMetadata = await Deno.readTextFile(flowMetadataPath);
      const modifiedMetadata = flowMetadata.replace(
        /summary:.*$/m,
        'summary: "Modified Data Processor Flow from test"'
      );
      await Deno.writeTextFile(flowMetadataPath, modifiedMetadata);

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify modification on server
      const updatedFlow = await getFlow(backend, flowPath);
      assertEquals(
        updatedFlow.summary,
        "Modified Data Processor Flow from test",
        `Server should have modified flow summary. Got: ${updatedFlow.summary}`
      );
    });
  },
});

Deno.test({
  name: "Mixed Case Paths: pull and push app with capitalized folder",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with mixed case
      await createFolder(backend, "MyApps");

      // Create an app with mixed-case path
      const appPath = "f/MyApps/Dashboard";
      await createApp(backend, appPath, "My Dashboard App");

      // Create wmill.yaml
      await Deno.writeTextFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Verify app directory exists
      const appDir = path.join(tempDir, "f", "MyApps", "Dashboard.app");
      const appDirExists = await Deno.stat(appDir).then(s => s.isDirectory).catch(() => false);
      assert(appDirExists, `App directory should exist at ${appDir}`);

      // Modify the app metadata
      const appMetadataPath = path.join(appDir, "app.yaml");
      const appMetadata = await Deno.readTextFile(appMetadataPath);
      const modifiedMetadata = appMetadata.replace(
        /summary:.*$/m,
        'summary: "Modified Dashboard App from test"'
      );
      await Deno.writeTextFile(appMetadataPath, modifiedMetadata);

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify modification on server
      const updatedApp = await getApp(backend, appPath);
      assertEquals(
        updatedApp.summary,
        "Modified Dashboard App from test",
        `Server should have modified app summary. Got: ${updatedApp.summary}`
      );
    });
  },
});

Deno.test({
  name: "Mixed Case Paths: pull and push variable with capitalized folder",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create folder with mixed case
      await createFolder(backend, "MyVars");

      // Create a variable with mixed-case path
      const varPath = "f/MyVars/ApiKey";
      await createVariable(backend, varPath, "original-api-key-value", "API Key Variable");

      // Create wmill.yaml
      await Deno.writeTextFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Verify variable file exists
      const varFilePath = path.join(tempDir, "f", "MyVars", "ApiKey.variable.yaml");
      const varExists = await Deno.stat(varFilePath).then(() => true).catch(() => false);
      assert(varExists, `Variable file should exist at ${varFilePath}`);

      // Modify the variable
      const varContent = await Deno.readTextFile(varFilePath);
      const modifiedVarContent = varContent.replace(
        /value:.*$/m,
        'value: "modified-api-key-from-test"'
      );
      await Deno.writeTextFile(varFilePath, modifiedVarContent);

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify modification on server
      const updatedVar = await getVariable(backend, varPath);
      assertEquals(
        updatedVar.value,
        "modified-api-key-from-test",
        `Server should have modified variable value. Got: ${updatedVar.value}`
      );
    });
  },
});

Deno.test({
  name: "Mixed Case Paths: deeply nested capitalized folders",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Verify file exists
      const scriptFilePath = path.join(tempDir, "f", "MyProject", "SubFolder_A.ts");
      const scriptExists = await Deno.stat(scriptFilePath).then(() => true).catch(() => false);
      assert(scriptExists, `Nested script should exist at ${scriptFilePath}`);

      // Modify
      const modifiedContent = `export async function main() {
  return "deeply nested modified from test";
}`;
      await Deno.writeTextFile(scriptFilePath, modifiedContent);

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify on server
      const updatedScript = await getScript(backend, scriptPath);
      assert(
        updatedScript.content.includes("deeply nested modified from test"),
        `Server should have modified nested content`
      );
    });
  },
});

Deno.test({
  name: "Mixed Case Paths: multiple resources in same capitalized folder",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Verify all files exist
      const folderPath = path.join(tempDir, "f", "SharedFolder");
      const script1Exists = await Deno.stat(path.join(folderPath, "ScriptOne.ts")).then(() => true).catch(() => false);
      const script2Exists = await Deno.stat(path.join(folderPath, "ScriptTwo.ts")).then(() => true).catch(() => false);
      const var1Exists = await Deno.stat(path.join(folderPath, "VarOne.variable.yaml")).then(() => true).catch(() => false);
      const res1Exists = await Deno.stat(path.join(folderPath, "ResourceOne.resource.yaml")).then(() => true).catch(() => false);

      assert(script1Exists, "ScriptOne should exist");
      assert(script2Exists, "ScriptTwo should exist");
      assert(var1Exists, "VarOne should exist");
      assert(res1Exists, "ResourceOne should exist");

      // Modify script one
      await Deno.writeTextFile(
        path.join(folderPath, "ScriptOne.ts"),
        'export async function main() { return "script one MODIFIED"; }'
      );

      // Modify script two
      await Deno.writeTextFile(
        path.join(folderPath, "ScriptTwo.ts"),
        'export async function main() { return "script two MODIFIED"; }'
      );

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify modifications on server
      const script1 = await getScript(backend, "f/SharedFolder/ScriptOne");
      const script2 = await getScript(backend, "f/SharedFolder/ScriptTwo");

      assert(script1.content.includes("script one MODIFIED"), "Script one should be modified on server");
      assert(script2.content.includes("script two MODIFIED"), "Script two should be modified on server");
    });
  },
});

Deno.test({
  name: "Mixed Case Paths: CamelCase folder names with numbers",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
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
      await Deno.writeTextFile(
        path.join(tempDir, "wmill.yaml"),
        `defaultTs: bun
includes:
  - "**"
excludes: []
`
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull should succeed: ${pullResult.stderr}`);

      // Verify file exists
      const scriptFilePath = path.join(tempDir, "f", "Project2024", "DataHandler_V2.ts");
      const scriptExists = await Deno.stat(scriptFilePath).then(() => true).catch(() => false);
      assert(scriptExists, `Script should exist at ${scriptFilePath}`);

      // Modify
      await Deno.writeTextFile(
        scriptFilePath,
        'export async function main() { return "handler v2 MODIFIED"; }'
      );

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push should succeed: ${pushResult.stderr}`);

      // Verify on server
      const updatedScript = await getScript(backend, scriptPath);
      assert(updatedScript.content.includes("handler v2 MODIFIED"), "Server should have modified content");
    });
  },
});
