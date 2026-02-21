/**
 * Integration tests for folder and schedule CLI commands.
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
// Folder Tests
// =============================================================================

describe("folder", () => {
  test("list returns seeded folders", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["folder"], tempDir);

      expect(result.code).toEqual(0);
      // seedTestData creates "test" folder
      expect(result.stdout).toContain("test");
    });
  });

  test("push creates a new folder via sync push", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const folderName = `inttest${uniqueId}`;

      // Create wmill.yaml
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );

      // Create folder meta file
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        `display_name: "Integration Test Folder ${uniqueId}"\nowners:\n  - "admin@windmill.dev"\nextra_perms: {}\n`,
        "utf-8"
      );

      // Push
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify folder was created via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/get/${folderName}`
      );
      expect(apiResp.status).toEqual(200);
      const folderData = await apiResp.json();
      expect(folderData.name).toBe(folderName);
    });
  });

  test("push updates an existing folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const folderName = `updfolder${uniqueId}`;

      // Create folder via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: folderName }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml and updated folder meta
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
        "utf-8"
      );
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        `display_name: "Updated Display Name"\nowners:\n  - "u/admin"\nextra_perms:\n  u/admin: true\n`,
        "utf-8"
      );

      // Push the update
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify the display_name was updated
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/get/${folderName}`
      );
      expect(apiResp.status).toEqual(200);
      const folderData = await apiResp.json();
      expect(folderData.display_name).toBe("Updated Display Name");
    });
  });

  test("pull retrieves folder metadata", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const folderName = `pullfolder${uniqueId}`;

      // Create folder via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: folderName }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "f/${folderName}/**"\nexcludes: []\nskipVariables: true\nskipResources: true\n`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Check the folder meta file was created
      const content = await readFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"), "utf-8"
      );
      expect(content).toBeDefined();
    });
  });
});

// =============================================================================
// Schedule Tests
// =============================================================================

describe("schedule", () => {
  test("list returns empty table for fresh workspace", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["schedule"], tempDir);

      expect(result.code).toEqual(0);
      // Table headers should be present
      expect(result.stdout).toContain("Path");
      expect(result.stdout).toContain("Schedule");
    });
  });

  test("push creates a schedule targeting an existing script", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // First create a script that the schedule can target
      const scriptResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/sched_target_${uniqueId}`,
            content: 'export async function main() { return "ok"; }',
            language: "bun",
            summary: "Schedule target script",
            description: "",
            schema: {
              $schema: "https://json-schema.org/draft/2020-12/schema",
              type: "object",
              properties: {},
              required: [],
            },
          }),
        }
      );
      expect(scriptResp.status).toBeLessThan(300);
      await scriptResp.text();

      // Create wmill.yaml with includeSchedules
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\nincludeSchedules: true\n`,
        "utf-8"
      );

      // Create schedule file
      await mkdir(join(tempDir, "f", "test"), { recursive: true });
      await writeFile(
        join(tempDir, `f/test/cron_${uniqueId}.schedule.yaml`),
        `path: "f/test/cron_${uniqueId}"\nschedule: "0 0 */6 * * *"\nscript_path: "f/test/sched_target_${uniqueId}"\nis_flow: false\nargs: {}\nenabled: false\ntimezone: "UTC"\n`,
        "utf-8"
      );

      // Push
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/cron_${uniqueId}**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/get/f/test/cron_${uniqueId}`
      );
      expect(apiResp.status).toEqual(200);
      const schedData = await apiResp.json();
      expect(schedData.schedule).toBe("0 0 */6 * * *");
      expect(schedData.script_path).toBe(`f/test/sched_target_${uniqueId}`);
      expect(schedData.enabled).toBe(false);
    });
  });

  test("push updates a schedule's cron expression", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create target script via API
      const scriptResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/upd_sched_target_${uniqueId}`,
            content: 'export async function main() { return "ok"; }',
            language: "bun",
            summary: "Target",
            description: "",
            schema: {
              $schema: "https://json-schema.org/draft/2020-12/schema",
              type: "object",
              properties: {},
              required: [],
            },
          }),
        }
      );
      expect(scriptResp.status).toBeLessThan(300);
      await scriptResp.text();

      // Create schedule via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/upd_cron_${uniqueId}`,
            schedule: "0 0 * * * *",
            script_path: `f/test/upd_sched_target_${uniqueId}`,
            is_flow: false,
            args: {},
            enabled: false,
            timezone: "UTC",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml with includeSchedules and updated schedule
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\nincludeSchedules: true\n`,
        "utf-8"
      );
      await mkdir(join(tempDir, "f", "test"), { recursive: true });
      await writeFile(
        join(tempDir, `f/test/upd_cron_${uniqueId}.schedule.yaml`),
        `path: "f/test/upd_cron_${uniqueId}"\nschedule: "0 30 2 * * *"\nscript_path: "f/test/upd_sched_target_${uniqueId}"\nis_flow: false\nargs: {}\nenabled: false\ntimezone: "UTC"\n`,
        "utf-8"
      );

      // Push the update
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/test/upd_cron_${uniqueId}**`],
        tempDir
      );

      expect(pushResult.code).toEqual(0);

      // Verify the schedule was updated
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/get/f/test/upd_cron_${uniqueId}`
      );
      expect(apiResp.status).toEqual(200);
      const schedData = await apiResp.json();
      expect(schedData.schedule).toBe("0 30 2 * * *");
    });
  });

  test("pull retrieves schedules into local files", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();

      // Create target script via API
      const scriptResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/pull_sched_target_${uniqueId}`,
            content: 'export async function main() { return "ok"; }',
            language: "bun",
            summary: "Target for pull test",
            description: "",
            schema: {
              $schema: "https://json-schema.org/draft/2020-12/schema",
              type: "object",
              properties: {},
              required: [],
            },
          }),
        }
      );
      expect(scriptResp.status).toBeLessThan(300);
      await scriptResp.text();

      // Create schedule via API
      const createResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/schedules/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/test/pull_cron_${uniqueId}`,
            schedule: "0 15 3 * * 1",
            script_path: `f/test/pull_sched_target_${uniqueId}`,
            is_flow: false,
            args: {},
            enabled: false,
            timezone: "UTC",
          }),
        }
      );
      expect(createResp.status).toBeLessThan(300);
      await createResp.text();

      // Create wmill.yaml
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nincludes:\n  - "f/test/pull_cron_${uniqueId}**"\nexcludes: []\nincludeSchedules: true\nskipVariables: true\nskipResources: true\nskipScripts: true\n`,
        "utf-8"
      );

      // Pull
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Check the schedule file was created
      const content = await readFile(
        join(tempDir, `f/test/pull_cron_${uniqueId}.schedule.yaml`), "utf-8"
      );
      expect(content).toContain("0 15 3 * * 1");
      expect(content).toContain(`f/test/pull_sched_target_${uniqueId}`);
    });
  });
});
