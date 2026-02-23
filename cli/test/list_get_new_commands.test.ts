/**
 * Integration tests for the new list/get/new CLI commands.
 *
 * Tests:
 * - `list --json` for all item types
 * - `get <path>` and `get <path> --json` for all item types
 * - `new` (bootstrap) for script, flow, resource, resource-type, variable, schedule, folder, trigger
 * - `bootstrap` alias for script and flow
 */

import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, stat, readFile } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend, type TestBackend } from "./test_backend.ts";
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
        summary: "Test script summary",
        description: "Test script description",
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
// list --json
// =============================================================================

describe("list --json flag", () => {
  test("script list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/list_json_script_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);

      const result = await backend.runCLICommand(
        ["script", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed.some((s: any) => s.path === scriptPath)).toBe(true);
    });
  });

  test("flow list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["flow", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("resource list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["resource", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      // seedTestData creates f/test/my_resource
      expect(parsed.some((r: any) => r.path === "f/test/my_resource")).toBe(
        true
      );
    });
  });

  test("variable list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["variable", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("folder list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["folder", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed.some((f: any) => f.name === "test")).toBe(true);
    });
  });

  test("schedule list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["schedule", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("resource-type list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["resource-type", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("trigger list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["trigger", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("app list --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["app", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("default action with --json works (e.g. wmill script --json)", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["script", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });
});

// =============================================================================
// get <path> and get <path> --json
// =============================================================================

describe("get command", () => {
  test("script get pretty-prints details", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/get_script_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);

      const result = await backend.runCLICommand(
        ["script", "get", scriptPath],
        tempDir
      );

      expect(result.code).toEqual(0);
      const output = result.stdout;
      expect(output).toContain("Path:");
      expect(output).toContain(scriptPath);
      expect(output).toContain("Summary:");
      expect(output).toContain("Language:");
      expect(output).toContain("bun");
    });
  });

  test("script get --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/get_json_script_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);

      const result = await backend.runCLICommand(
        ["script", "get", scriptPath, "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(parsed.path).toBe(scriptPath);
      expect(parsed.language).toBe("bun");
      expect(parsed.summary).toBe("Test script summary");
    });
  });

  test("resource get --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["resource", "get", "f/test/my_resource", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(parsed.path).toBe("f/test/my_resource");
      expect(parsed.resource_type).toBe("any");
    });
  });

  test("resource get pretty-prints details", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["resource", "get", "f/test/my_resource"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const output = result.stdout;
      expect(output).toContain("Path:");
      expect(output).toContain("f/test/my_resource");
      expect(output).toContain("Resource Type:");
    });
  });

  test("variable get --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["variable", "get", "f/test/my_variable", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(parsed.path).toBe("f/test/my_variable");
    });
  });

  test("folder get --json outputs valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["folder", "get", "test", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(parsed.name).toBe("test");
    });
  });

  test("folder get pretty-prints details", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["folder", "get", "test"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const output = result.stdout;
      expect(output).toContain("Name:");
      expect(output).toContain("test");
    });
  });
});

// =============================================================================
// new command
// =============================================================================

describe("new command", () => {
  test("script new creates files (same as bootstrap)", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );
      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["script", "new", "f/test/new_cmd_script", "bun", "--summary", "Test new"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const codeStat = await stat(join(tempDir, "f/test/new_cmd_script.ts"));
      expect(codeStat.isFile()).toBe(true);

      const metaStat = await stat(
        join(tempDir, "f/test/new_cmd_script.script.yaml")
      );
      expect(metaStat.isFile()).toBe(true);

      const metaContent = await readFile(
        join(tempDir, "f/test/new_cmd_script.script.yaml"),
        "utf-8"
      );
      expect(metaContent).toContain("Test new");
    });
  });

  test("script bootstrap still works as alias", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );
      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["script", "bootstrap", "f/test/alias_script", "bun"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const codeStat = await stat(join(tempDir, "f/test/alias_script.ts"));
      expect(codeStat.isFile()).toBe(true);
    });
  });

  test("flow new creates flow directory and flow.yaml", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["flow", "new", "f/test/new_flow", "--summary", "My flow"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const flowYamlStat = await stat(
        join(tempDir, "f/test/new_flow.flow/flow.yaml")
      );
      expect(flowYamlStat.isFile()).toBe(true);

      const flowContent = await readFile(
        join(tempDir, "f/test/new_flow.flow/flow.yaml"),
        "utf-8"
      );
      expect(flowContent).toContain("My flow");
    });
  });

  test("flow bootstrap still works as alias", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["flow", "bootstrap", "f/test/alias_flow"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const flowYamlStat = await stat(
        join(tempDir, "f/test/alias_flow.flow/flow.yaml")
      );
      expect(flowYamlStat.isFile()).toBe(true);
    });
  });

  test("resource new creates resource yaml template", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["resource", "new", "f/test/new_resource"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const filePath = join(tempDir, "f/test/new_resource.resource.yaml");
      const fileStat = await stat(filePath);
      expect(fileStat.isFile()).toBe(true);

      const content = await readFile(filePath, "utf-8");
      expect(content).toContain("resource_type");
      expect(content).toContain("value");
    });
  });

  test("resource-type new creates resource-type yaml template", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["resource-type", "new", "my_custom_type"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const filePath = join(tempDir, "my_custom_type.resource-type.yaml");
      const fileStat = await stat(filePath);
      expect(fileStat.isFile()).toBe(true);

      const content = await readFile(filePath, "utf-8");
      expect(content).toContain("schema");
      expect(content).toContain("description");
    });
  });

  test("variable new creates variable yaml template", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["variable", "new", "f/test/new_var"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const filePath = join(tempDir, "f/test/new_var.variable.yaml");
      const fileStat = await stat(filePath);
      expect(fileStat.isFile()).toBe(true);

      const content = await readFile(filePath, "utf-8");
      expect(content).toContain("is_secret");
      expect(content).toContain("value");
    });
  });

  test("schedule new creates schedule yaml template", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["schedule", "new", "f/test/new_sched"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const filePath = join(tempDir, "f/test/new_sched.schedule.yaml");
      const fileStat = await stat(filePath);
      expect(fileStat.isFile()).toBe(true);

      const content = await readFile(filePath, "utf-8");
      expect(content).toContain("schedule");
      expect(content).toContain("script_path");
      expect(content).toContain("timezone");
    });
  });

  test("folder new creates folder.meta.yaml in f/<name>/", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["folder", "new", "new_folder"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const filePath = join(tempDir, "f/new_folder/folder.meta.yaml");
      const fileStat = await stat(filePath);
      expect(fileStat.isFile()).toBe(true);

      const content = await readFile(filePath, "utf-8");
      expect(content).toContain("owners");
      expect(content).toContain("extra_perms");
    });
  });

  test("trigger new --kind http creates http trigger yaml template", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["trigger", "new", "f/test/new_trigger", "--kind", "http"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const filePath = join(
        tempDir,
        "f/test/new_trigger.http_trigger.yaml"
      );
      const fileStat = await stat(filePath);
      expect(fileStat.isFile()).toBe(true);

      const content = await readFile(filePath, "utf-8");
      expect(content).toContain("script_path");
      expect(content).toContain("route_path");
    });
  });

  test("trigger new without --kind fails with error", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["trigger", "new", "f/test/fail_trigger"],
        tempDir
      );

      expect(result.code).not.toEqual(0);
    });
  });

  test("trigger new --kind kafka creates kafka trigger yaml template", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await mkdir(join(tempDir, "f", "test"), { recursive: true });

      const result = await backend.runCLICommand(
        ["trigger", "new", "f/test/kafka_trigger", "--kind", "kafka"],
        tempDir
      );

      expect(result.code).toEqual(0);

      const filePath = join(
        tempDir,
        "f/test/kafka_trigger.kafka_trigger.yaml"
      );
      const fileStat = await stat(filePath);
      expect(fileStat.isFile()).toBe(true);

      const content = await readFile(filePath, "utf-8");
      expect(content).toContain("kafka_resource_path");
      expect(content).toContain("topics");
    });
  });
});
