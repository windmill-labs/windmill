/**
 * Integration tests for standalone CLI commands that previously had zero coverage.
 *
 * Tests:
 * - `wmill folder` (list)
 * - `wmill schedule` (list with data)
 * - `wmill resource-type list` and `wmill resource-type push`
 * - `wmill script show`, `wmill script run`, `wmill script bootstrap`
 * - `wmill user` (list, add, remove)
 */

import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, stat, readFile } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend, type TestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

async function setupWorkspaceProfile(backend: TestBackend): Promise<void> {
  await addWorkspace(
    {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "localhost_test",
      token: backend.token!,
    },
    { force: true, configDir: backend.testConfigDir }
  );
}

/** Create a script on the remote via API and return its path */
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

// =============================================================================
// Folder List
// =============================================================================

describe("folder list command", () => {
  test("lists seeded folders", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["folder"], tempDir);

      expect(result.code).toEqual(0);
      // seedTestData creates a "test" folder
      expect(result.stdout).toContain("test");
      // Table headers should be present
      expect(result.stdout).toContain("Name");
    });
  });
});

// =============================================================================
// Schedule List
// =============================================================================

describe("schedule list command", () => {
  test("lists a schedule created via API", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/sched_list_target_${uniqueId}`;
      const schedulePath = `f/test/sched_list_${uniqueId}`;

      // Create target script
      await createRemoteScript(backend, scriptPath);

      // Create schedule via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: schedulePath,
            schedule: "0 0 12 * * *",
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

      // List schedules via CLI
      const result = await backend.runCLICommand(["schedule"], tempDir);

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain(schedulePath);
      expect(result.stdout).toContain("0 0 12 * * *");
    });
  });
});

// =============================================================================
// Resource Type List & Push
// =============================================================================

describe("resource-type commands", () => {
  test("list returns exit code 0", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["resource-type", "list"],
        tempDir
      );

      expect(result.code).toEqual(0);
      // Table headers should be present
      expect(result.stdout).toContain("Name");
    });
  });

  test("push creates a new resource type", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const rtName = `test_rt_${uniqueId}`;

      // Create a resource type JSON file
      const rtFile = join(tempDir, `${rtName}.resource-type.json`);
      await writeFile(
        rtFile,
        JSON.stringify({
          schema: {
            type: "object",
            properties: {
              host: { type: "string" },
              port: { type: "integer" },
            },
          },
          description: "Test resource type from integration test",
        }),
        "utf-8"
      );

      // Push via CLI — the name argument must include the .resource-type.json suffix
      const pushResult = await backend.runCLICommand(
        ["resource-type", "push", rtFile, `${rtName}.resource-type.json`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/type/get/${rtName}`
      );
      expect(apiResp.status).toEqual(200);
      const rtData = await apiResp.json();
      expect(rtData.name).toBe(rtName);
      expect(rtData.schema).toBeDefined();
      expect(rtData.schema.properties.host.type).toBe("string");
    });
  });

  test("push updates an existing resource type", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const rtName = `test_rt_upd_${uniqueId}`;

      // Create resource type via API first
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/type/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            name: rtName,
            schema: {
              type: "object",
              properties: { old_field: { type: "string" } },
            },
            description: "Original",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create updated resource type file
      const rtFile = join(tempDir, `${rtName}.resource-type.json`);
      await writeFile(
        rtFile,
        JSON.stringify({
          schema: {
            type: "object",
            properties: {
              new_field: { type: "number" },
            },
          },
          description: "Updated description",
        }),
        "utf-8"
      );

      // Push update via CLI — the name argument must include the .resource-type.json suffix
      const pushResult = await backend.runCLICommand(
        ["resource-type", "push", rtFile, `${rtName}.resource-type.json`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify the update via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/resources/type/get/${rtName}`
      );
      expect(apiResp.status).toEqual(200);
      const rtData = await apiResp.json();
      expect(rtData.description).toBe("Updated description");
      expect(rtData.schema.properties.new_field.type).toBe("number");
    });
  });
});

// =============================================================================
// Script Show
// =============================================================================

describe("script show command", () => {
  test("shows script content", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/show_script_${uniqueId}`;
      const scriptContent = `export async function main() { return "show_test_${uniqueId}"; }`;

      await createRemoteScript(backend, scriptPath, scriptContent);

      const result = await backend.runCLICommand(
        ["script", "show", scriptPath],
        tempDir
      );

      expect(result.code).toEqual(0);
      // Should display the script content
      const output = result.stdout + result.stderr;
      expect(output).toContain(`show_test_${uniqueId}`);
      expect(output).toContain(scriptPath);
    });
  });
});

// =============================================================================
// Script Run
// =============================================================================

describe("script run command", () => {
  test("runs a script and returns result", { timeout: 60000 }, async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/run_script_${uniqueId}`;
      const scriptContent = `export async function main() { return { value: "run_result_${uniqueId}" }; }`;

      await createRemoteScript(backend, scriptPath, scriptContent);

      const result = await backend.runCLICommand(
        ["script", "run", scriptPath, "--silent"],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain(`run_result_${uniqueId}`);
    });
  });
});

// =============================================================================
// Script Bootstrap
// =============================================================================

describe("script bootstrap command", () => {
  test("creates TypeScript script files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create a wmill.yaml so bootstrap can read config
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        [
          "script",
          "bootstrap",
          "f/test/new_script",
          "bun",
          "--summary",
          "My new script",
        ],
        tempDir
      );

      expect(result.code).toEqual(0);

      // Verify the code file was created
      const codeStat = await stat(join(tempDir, "f/test/new_script.ts"));
      expect(codeStat.isFile()).toBe(true);

      // Verify the metadata file was created
      const metaStat = await stat(
        join(tempDir, "f/test/new_script.script.yaml")
      );
      expect(metaStat.isFile()).toBe(true);

      // Verify metadata content
      const metaContent = await readFile(
        join(tempDir, "f/test/new_script.script.yaml"),
        "utf-8"
      );
      expect(metaContent).toContain("My new script");
    });
  });

  test("creates Python script files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["script", "bootstrap", "f/test/py_script", "python3"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const codeStat = await stat(join(tempDir, "f/test/py_script.py"));
      expect(codeStat.isFile()).toBe(true);

      const metaStat = await stat(
        join(tempDir, "f/test/py_script.script.yaml")
      );
      expect(metaStat.isFile()).toBe(true);
    });
  });

  test("creates Bash script files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["script", "bootstrap", "f/test/bash_script", "bash"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const codeStat = await stat(join(tempDir, "f/test/bash_script.sh"));
      expect(codeStat.isFile()).toBe(true);
    });
  });

  test("creates Go script files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["script", "bootstrap", "f/test/go_script", "go"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const codeStat = await stat(join(tempDir, "f/test/go_script.go"));
      expect(codeStat.isFile()).toBe(true);
    });
  });
});

// =============================================================================
// User List, Add, Remove
// =============================================================================

describe("user commands", () => {
  test("list shows existing admin user", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["user"], tempDir);

      expect(result.code).toEqual(0);
      // The admin user is always created by the test backend
      expect(result.stdout).toContain("admin@windmill.dev");
      // Table headers
      expect(result.stdout).toContain("email");
    });
  });

  test.skipIf(shouldSkipOnCI())("add creates a new user and remove deletes it", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const email = `testuser_${uniqueId}@example.com`;
      const password = "testpass123";

      // Add user
      const addResult = await backend.runCLICommand(
        ["user", "add", email, password],
        tempDir
      );
      expect(addResult.code).toEqual(0);

      // Verify the user appears in the list
      const listResult = await backend.runCLICommand(["user"], tempDir);
      expect(listResult.code).toEqual(0);
      expect(listResult.stdout).toContain(email);

      // Remove user
      const removeResult = await backend.runCLICommand(
        ["user", "remove", email],
        tempDir
      );
      expect(removeResult.code).toEqual(0);

      // Verify the user no longer appears
      const listAfterResult = await backend.runCLICommand(["user"], tempDir);
      expect(listAfterResult.code).toEqual(0);
      expect(listAfterResult.stdout).not.toContain(email);
    });
  });

  test.skipIf(shouldSkipOnCI())("add with --superadmin flag creates superadmin user", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const email = `superuser_${uniqueId}@example.com`;
      const password = "superpass123";

      // Add superadmin user
      const addResult = await backend.runCLICommand(
        ["user", "add", email, password, "--superadmin"],
        tempDir
      );
      expect(addResult.code).toEqual(0);

      // Verify user exists and is superadmin
      const listResult = await backend.runCLICommand(["user"], tempDir);
      expect(listResult.code).toEqual(0);
      expect(listResult.stdout).toContain(email);

      // Clean up
      await backend.runCLICommand(["user", "remove", email], tempDir);
    });
  });
});
