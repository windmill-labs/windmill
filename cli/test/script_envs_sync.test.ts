/**
 * Script envs sync test
 *
 * Tests that script metadata envs field is correctly preserved during sync push.
 * Reproduces issue: env variables set in metadata of scripts don't work correctly
 * if created by API, then sync pulled, then script modified, then sync push,
 * the env variables aren't there anymore.
 */

import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";
import { withTestBackend } from "./test_backend.ts";

Deno.test({
  name: "Integration: Script envs field is preserved during sync pull/push cycle",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/envs_script_${uniqueId}`;

      // Step 1: Create a script via API with envs set
      await ensureDir(`${tempDir}/f/test`);

      // Create folder first
      const folderResp = await backend.apiRequest!(`/api/w/${backend.workspace}/folders/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "test" }),
      });
      // Ignore error if folder already exists

      // Create a script with envs via API
      const createResp = await backend.apiRequest!(`/api/w/${backend.workspace}/scripts/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: scriptPath,
          content: `export async function main() {\n  return "Hello world";\n}`,
          summary: "Test script with envs",
          description: "A script to test envs preservation",
          language: "bun",
          envs: ["MY_ENV_VAR", "ANOTHER_VAR"],
          kind: "script",
        }),
      });

      assertEquals(
        createResp.ok,
        true,
        `Failed to create script: ${await createResp.text()}`,
      );

      // Verify the script was created with envs
      const getResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const createdScriptText = await getResp.text();
      assertEquals(getResp.ok, true, `Failed to get script: ${createdScriptText}`);
      const createdScript = JSON.parse(createdScriptText);
      assertEquals(
        createdScript.envs,
        ["MY_ENV_VAR", "ANOTHER_VAR"],
        "Script should have envs after creation",
      );

      // Step 2: Create wmill.yaml and sync pull
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/test/envs_script_${uniqueId}**"
excludes: []
`,
      );

      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(
        pullResult.code,
        0,
        `Pull should succeed.\nstdout: ${pullResult.stdout}\nstderr: ${pullResult.stderr}`,
      );

      // Verify the pulled metadata contains envs
      const metadataPath = `${tempDir}/f/test/envs_script_${uniqueId}.script.yaml`;
      const metadataContent = await Deno.readTextFile(metadataPath);
      assert(
        metadataContent.includes("envs:") ||
          metadataContent.includes("MY_ENV_VAR") ||
          metadataContent.includes("ANOTHER_VAR"),
        `Pulled metadata should contain envs. Content:\n${metadataContent}`,
      );

      // Step 3: Modify the script locally (change content)
      const scriptFilePath = `${tempDir}/f/test/envs_script_${uniqueId}.ts`;
      const originalContent = await Deno.readTextFile(scriptFilePath);
      await Deno.writeTextFile(
        scriptFilePath,
        originalContent.replace("Hello world", "Hello world modified"),
      );

      // Step 4: Sync push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(
        pushResult.code,
        0,
        `Push should succeed.\nstdout: ${pushResult.stdout}\nstderr: ${pushResult.stderr}`,
      );

      // Step 5: Verify envs are still present on the remote
      const getResp2 = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const updatedScriptText = await getResp2.text();
      assertEquals(getResp2.ok, true, `Failed to get script after push: ${updatedScriptText}`);
      const updatedScript = JSON.parse(updatedScriptText);

      assertEquals(
        updatedScript.envs,
        ["MY_ENV_VAR", "ANOTHER_VAR"],
        `Script envs should be preserved after push. Got: ${JSON.stringify(updatedScript.envs)}`,
      );

      // Also verify the content was updated
      assert(
        updatedScript.content.includes("Hello world modified"),
        "Script content should be updated",
      );
    });
  },
});

Deno.test({
  name: "Integration: Script envs field changes are detected and pushed",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/envs_change_${uniqueId}`;

      // Create folder
      await ensureDir(`${tempDir}/f/test`);
      await backend.apiRequest!(`/api/w/${backend.workspace}/folders/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "test" }),
      });

      // Create a script with initial envs
      const createResp = await backend.apiRequest!(`/api/w/${backend.workspace}/scripts/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: scriptPath,
          content: `export async function main() {\n  return "Hello";\n}`,
          summary: "Test envs change",
          description: "",
          language: "bun",
          envs: ["INITIAL_VAR"],
          kind: "script",
        }),
      });
      assertEquals(createResp.ok, true, `Failed to create script: ${await createResp.text()}`);

      // Setup wmill.yaml
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/test/envs_change_${uniqueId}**"
excludes: []
`,
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull failed: ${pullResult.stderr}`);

      // Modify envs in the local metadata file
      const metadataPath = `${tempDir}/f/test/envs_change_${uniqueId}.script.yaml`;
      let metadataContent = await Deno.readTextFile(metadataPath);

      // Replace the envs line(s)
      if (metadataContent.includes("envs:")) {
        // Replace existing envs
        metadataContent = metadataContent.replace(
          /envs:\s*\n(\s+-\s+\S+\n?)*/,
          "envs:\n  - NEW_VAR1\n  - NEW_VAR2\n",
        );
      } else {
        // Add envs if not present
        metadataContent += "\nenvs:\n  - NEW_VAR1\n  - NEW_VAR2\n";
      }
      await Deno.writeTextFile(metadataPath, metadataContent);

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push failed: ${pushResult.stderr}`);

      // Verify envs were updated on remote
      const getResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const scriptText = await getResp.text();
      assertEquals(getResp.ok, true, `Failed to get script: ${scriptText}`);
      const script = JSON.parse(scriptText);

      assertEquals(
        script.envs,
        ["NEW_VAR1", "NEW_VAR2"],
        `Script envs should be updated to new values. Got: ${JSON.stringify(script.envs)}`,
      );
    });
  },
});

Deno.test({
  name: "Integration: Script with empty envs is handled correctly",
  sanitizeResources: false,
  sanitizeOps: false,
  async fn() {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/empty_envs_${uniqueId}`;

      // Create folder
      await ensureDir(`${tempDir}/f/test`);
      await backend.apiRequest!(`/api/w/${backend.workspace}/folders/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "test" }),
      });

      // Create a script WITHOUT envs
      const createResp = await backend.apiRequest!(`/api/w/${backend.workspace}/scripts/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: scriptPath,
          content: `export async function main() {\n  return "No envs";\n}`,
          summary: "Test no envs",
          description: "",
          language: "bun",
          kind: "script",
        }),
      });
      assertEquals(createResp.ok, true, `Failed to create script: ${await createResp.text()}`);

      // Setup wmill.yaml
      await Deno.writeTextFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/test/empty_envs_${uniqueId}**"
excludes: []
`,
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      assertEquals(pullResult.code, 0, `Pull failed: ${pullResult.stderr}`);

      // Modify content
      const scriptFilePath = `${tempDir}/f/test/empty_envs_${uniqueId}.ts`;
      await Deno.writeTextFile(
        scriptFilePath,
        `export async function main() {\n  return "Modified no envs";\n}`,
      );

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      assertEquals(pushResult.code, 0, `Push failed: ${pushResult.stderr}`);

      // Verify script was updated and envs is still null/empty
      const getResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const script = await getResp.json();

      assert(
        script.content.includes("Modified no envs"),
        "Script content should be updated",
      );

      // envs should be null, empty, or undefined
      assert(
        !script.envs || script.envs.length === 0,
        `Script envs should remain empty. Got: ${JSON.stringify(script.envs)}`,
      );
    });
  },
});
