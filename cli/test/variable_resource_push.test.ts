/**
 * Integration tests for variable and resource CLI commands.
 * Tests list and push operations via CLI and direct API.
 */

import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";

async function setupWorkspaceProfile(backend: any): Promise<void> {
  await addWorkspace(
    {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "localhost_test",
      token: backend.token,
    },
    { force: true, configDir: backend.testConfigDir }
  );
}

// =============================================================================
// Variable Tests
// =============================================================================

describe("variable", () => {
  test("list returns seeded variables", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["variable"], tempDir);

      expect(result.code).toEqual(0);
      // seedTestData creates f/test/my_variable
      expect(result.stdout).toContain("f/test/my_variable");
    });
  });

  test("push creates a new variable via sync push", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create wmill.yaml
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );

      // Create variable file
      await mkdir(join(tempDir, "f", "test"), { recursive: true });
      const varPath = `f/test/test_var_${uniqueId}.variable.yaml`;
      await writeFile(
        join(tempDir, varPath),
        `value: "hello_from_test_${uniqueId}"\nis_secret: false\ndescription: "Test variable created by integration test"\n`,
        "utf-8"
      );

      // Push with sync push targeting just our variable
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/test_var_${uniqueId}**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify via API that the variable was created
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/variables/get/f/test/test_var_${uniqueId}`
      );
      expect(apiResp.status).toEqual(200);
      const varData = await apiResp.json();
      expect(varData.path).toBe(`f/test/test_var_${uniqueId}`);
      expect(varData.is_secret).toBe(false);
    });
  });

  test("push updates an existing variable", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create variable via API first
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/variables/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/update_var_${uniqueId}`,
            value: "original_value",
            is_secret: false,
            description: "Original description",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml and updated variable file
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );
      await mkdir(join(tempDir, "f", "test"), { recursive: true });
      await writeFile(
        join(tempDir, `f/test/update_var_${uniqueId}.variable.yaml`),
        `value: "updated_value"\nis_secret: false\ndescription: "Updated description"\n`,
        "utf-8"
      );

      // Push the update
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/update_var_${uniqueId}**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify the update via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/variables/get/f/test/update_var_${uniqueId}`
      );
      expect(apiResp.status).toEqual(200);
      const varData = await apiResp.json();
      expect(varData.description).toBe("Updated description");
    });
  });

  test("pull retrieves variables into local files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create a variable via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/variables/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/pull_var_${uniqueId}`,
            value: "pull_test_value",
            is_secret: false,
            description: "Variable for pull test",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "f/test/pull_var_${uniqueId}**"\nexcludes: []\n`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Check the file was created
      const content = await readFile(
        join(tempDir, `f/test/pull_var_${uniqueId}.variable.yaml`), "utf-8"
      );
      expect(content).toContain("pull_test_value");
      expect(content).toContain("is_secret: false");
    });
  });
});

// =============================================================================
// Resource Tests
// =============================================================================

describe("resource", () => {
  test("list returns seeded resources", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["resource"], tempDir);

      expect(result.code).toEqual(0);
      // seedTestData creates f/test/my_resource
      expect(result.stdout).toContain("f/test/my_resource");
    });
  });

  test("push creates a new resource via sync push", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create wmill.yaml
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );

      // Create resource file
      await mkdir(join(tempDir, "f", "test"), { recursive: true });
      const resPath = `f/test/test_res_${uniqueId}.resource.yaml`;
      await writeFile(
        join(tempDir, resPath),
        `resource_type: "any"\nvalue:\n  host: "localhost"\n  port: 3000\ndescription: "Test resource"\n`,
        "utf-8"
      );

      // Push
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/test_res_${uniqueId}**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/get/f/test/test_res_${uniqueId}`
      );
      expect(apiResp.status).toEqual(200);
      const resData = await apiResp.json();
      expect(resData.path).toBe(`f/test/test_res_${uniqueId}`);
      expect(resData.resource_type).toBe("any");
      expect(resData.value.host).toBe("localhost");
    });
  });

  test("push updates an existing resource", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create resource via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/update_res_${uniqueId}`,
            resource_type: "any",
            value: { host: "old_host" },
            description: "Original",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml and updated resource file
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );
      await mkdir(join(tempDir, "f", "test"), { recursive: true });
      await writeFile(
        join(tempDir, `f/test/update_res_${uniqueId}.resource.yaml`),
        `resource_type: "any"\nvalue:\n  host: "new_host"\n  port: 9999\ndescription: "Updated"\n`,
        "utf-8"
      );

      // Push the update
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/update_res_${uniqueId}**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify update
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/get/f/test/update_res_${uniqueId}`
      );
      expect(apiResp.status).toEqual(200);
      const resData = await apiResp.json();
      expect(resData.value.host).toBe("new_host");
      expect(resData.value.port).toBe(9999);
    });
  });

  test("pull retrieves resources into local files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create resource via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/pull_res_${uniqueId}`,
            resource_type: "any",
            value: { key: "pull_test" },
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "f/test/pull_res_${uniqueId}**"\nexcludes: []\nskipVariables: true\n`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Check the resource file was created
      const content = await readFile(
        join(tempDir, `f/test/pull_res_${uniqueId}.resource.yaml`), "utf-8"
      );
      expect(content).toContain("pull_test");
    });
  });
});
