/**
 * Script envs sync test
 *
 * Tests that script metadata envs field is correctly preserved during sync push.
 * Reproduces issue: env variables set in metadata of scripts don't work correctly
 * if created by API, then sync pulled, then script modified, then sync push,
 * the env variables aren't there anymore.
 */

import { expect, test } from "bun:test";
import { writeFile, readFile, mkdir } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";

test("Integration: Script envs field is preserved during sync pull/push cycle", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/envs_script_${uniqueId}`;

      // Step 1: Create a script via API with envs set
      await mkdir(`${tempDir}/f/test`, { recursive: true });

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

      expect(createResp.ok).toEqual(true);

      // Verify the script was created with envs
      const getResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const createdScriptText = await getResp.text();
      expect(getResp.ok).toEqual(true);
      const createdScript = JSON.parse(createdScriptText);
      expect(createdScript.envs).toEqual(["MY_ENV_VAR", "ANOTHER_VAR"]);

      // Step 2: Create wmill.yaml and sync pull
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/test/envs_script_${uniqueId}**"
excludes: []
`,
        "utf-8",
      );

      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Verify the pulled metadata contains envs
      const metadataPath = `${tempDir}/f/test/envs_script_${uniqueId}.script.yaml`;
      const metadataContent = await readFile(metadataPath, "utf-8");
      expect(
        metadataContent.includes("envs:") ||
          metadataContent.includes("MY_ENV_VAR") ||
          metadataContent.includes("ANOTHER_VAR"),
      ).toBeTruthy();

      // Step 3: Modify the script locally (change content)
      const scriptFilePath = `${tempDir}/f/test/envs_script_${uniqueId}.ts`;
      const originalContent = await readFile(scriptFilePath, "utf-8");
      await writeFile(
        scriptFilePath,
        originalContent.replace("Hello world", "Hello world modified"),
        "utf-8",
      );

      // Step 4: Sync push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Step 5: Verify envs are still present on the remote
      const getResp2 = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const updatedScriptText = await getResp2.text();
      expect(getResp2.ok).toEqual(true);
      const updatedScript = JSON.parse(updatedScriptText);

      expect(updatedScript.envs).toEqual(["MY_ENV_VAR", "ANOTHER_VAR"]);

      // Also verify the content was updated
      expect(
        updatedScript.content.includes("Hello world modified"),
      ).toBeTruthy();
    });
});

test("Integration: Script envs field changes are detected and pushed", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/envs_change_${uniqueId}`;

      // Create folder
      await mkdir(`${tempDir}/f/test`, { recursive: true });
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
      expect(createResp.ok).toEqual(true);

      // Setup wmill.yaml
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/test/envs_change_${uniqueId}**"
excludes: []
`,
        "utf-8",
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Modify envs in the local metadata file
      const metadataPath = `${tempDir}/f/test/envs_change_${uniqueId}.script.yaml`;
      let metadataContent = await readFile(metadataPath, "utf-8");

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
      await writeFile(metadataPath, metadataContent, "utf-8");

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify envs were updated on remote
      const getResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const scriptText = await getResp.text();
      expect(getResp.ok).toEqual(true);
      const script = JSON.parse(scriptText);

      expect(script.envs).toEqual(["NEW_VAR1", "NEW_VAR2"]);
    });
});

test("Integration: Script with empty envs is handled correctly", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const uniqueId = Date.now();
      const scriptPath = `f/test/empty_envs_${uniqueId}`;

      // Create folder
      await mkdir(`${tempDir}/f/test`, { recursive: true });
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
      expect(createResp.ok).toEqual(true);

      // Setup wmill.yaml
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun
includes:
  - "f/test/empty_envs_${uniqueId}**"
excludes: []
`,
        "utf-8",
      );

      // Pull
      const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(pullResult.code).toEqual(0);

      // Modify content
      const scriptFilePath = `${tempDir}/f/test/empty_envs_${uniqueId}.ts`;
      await writeFile(
        scriptFilePath,
        `export async function main() {\n  return "Modified no envs";\n}`,
        "utf-8",
      );

      // Push
      const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(pushResult.code).toEqual(0);

      // Verify script was updated and envs is still null/empty
      const getResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      const script = await getResp.json();

      expect(
        script.content.includes("Modified no envs"),
      ).toBeTruthy();

      // envs should be null, empty, or undefined
      expect(
        !script.envs || script.envs.length === 0,
      ).toBeTruthy();
    });
});
