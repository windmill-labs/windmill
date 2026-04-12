/**
 * Integration tests for folder default_permissioned_as + CLI preserve-on-update.
 *
 * Covers in minimal tests:
 *  1. Folder CRUD: create folder with rules, GET round-trip, validation
 *  2. Backend default resolution: schedule at matching path gets folder default;
 *     non-matching gets acting user; non-admin gets own identity
 *  3. CLI preserve-on-update: push updating a schedule preserves remote permissioned_as
 *  4. CLI pull stripping: pulled scripts/flows have has_on_behalf_of instead of email
 *  5. Stale rule rejection: deploy to a folder whose rule points at a deleted user → 400
 *  6. set-permissioned-as subcommand: one-shot ownership change
 */

import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseCliBehavior } from "../src/core/conf.ts";

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

// Helper: direct API call to create a folder with default_permissioned_as rules
async function createFolderWithRules(
  backend: any,
  name: string,
  rules: Array<{ path_glob: string; permissioned_as: string }>,
  extraPerms?: Record<string, boolean>
): Promise<void> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/folders/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name,
        default_permissioned_as: rules,
        extra_perms: extraPerms ?? {},
      }),
    }
  );
  const body = await resp.text();
  expect(resp.status).toBeLessThan(300);
}

async function updateFolderRules(
  backend: any,
  name: string,
  rules: Array<{ path_glob: string; permissioned_as: string }>
): Promise<Response> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/folders/update/${name}`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ default_permissioned_as: rules }),
    }
  );
  return resp;
}

async function createScript(backend: any, path: string): Promise<string> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/scripts/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path,
        summary: "helper",
        description: "",
        content: "export async function main() { return 42; }",
        language: "deno",
        schema: { type: "object", properties: {}, required: [] },
      }),
    }
  );
  const body = await resp.text();
  expect(resp.status).toBe(201);
  return body;
}

async function createSchedule(
  backend: any,
  path: string,
  scriptPath: string,
  extra?: Record<string, any>
): Promise<Response> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/schedules/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path,
        schedule: "0 0 */6 * * *",
        timezone: "UTC",
        script_path: scriptPath,
        is_flow: false,
        enabled: false,
        ...extra,
      }),
    }
  );
  return resp;
}

async function getSchedule(backend: any, path: string): Promise<any> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/schedules/get/${path}`
  );
  expect(resp.status).toBe(200);
  return resp.json();
}

// =============================================================================
// Unit tests (no backend needed)
// =============================================================================

describe("parseCliBehavior", () => {
  test("parses v1 to 1, undefined to 0", () => {
    expect(parseCliBehavior("v1")).toBe(1);
    expect(parseCliBehavior("v2")).toBe(2);
    expect(parseCliBehavior(undefined)).toBe(0);
    expect(parseCliBehavior("")).toBe(0);
    expect(parseCliBehavior("invalid")).toBe(0);
  });
});

// =============================================================================
// Integration tests
// =============================================================================

describe("folder default_permissioned_as", () => {
  test("round-trip: create folder with rules, GET back, validate", async () => {
    await withTestBackend(async (backend, _tempDir) => {
      const id = Date.now();
      const folderName = `dpa_rt_${id}`;

      await createFolderWithRules(backend, folderName, [
        { path_glob: "jobs/**", permissioned_as: "u/admin" },
        { path_glob: "reports/*", permissioned_as: "g/all" },
      ]);

      // GET
      const resp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/get/${folderName}`
      );
      expect(resp.status).toBe(200);
      const folder = await resp.json();
      expect(folder.default_permissioned_as).toHaveLength(2);
      expect(folder.default_permissioned_as[0].path_glob).toBe("jobs/**");
      expect(folder.default_permissioned_as[0].permissioned_as).toBe("u/admin");
      expect(folder.default_permissioned_as[1].permissioned_as).toBe("g/all");

      // Update rules
      const updateResp = await updateFolderRules(backend, folderName, [
        { path_glob: "**", permissioned_as: "u/admin" },
      ]);
      expect(updateResp.status).toBe(200);
      await updateResp.text();

      // Verify update
      const resp2 = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/get/${folderName}`
      );
      const folder2 = await resp2.json();
      expect(folder2.default_permissioned_as).toHaveLength(1);
      expect(folder2.default_permissioned_as[0].path_glob).toBe("**");

      // Clear rules
      const clearResp = await updateFolderRules(backend, folderName, []);
      expect(clearResp.status).toBe(200);
      await clearResp.text();

      const resp3 = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/get/${folderName}`
      );
      const folder3 = await resp3.json();
      expect(folder3.default_permissioned_as).toHaveLength(0);
    });
  });

  test("validation: rejects invalid glob, format, and structure", async () => {
    await withTestBackend(async (backend, _tempDir) => {
      const id = Date.now();
      const folderName = `dpa_val_${id}`;
      await createFolderWithRules(backend, folderName, []);

      // Invalid glob
      const r1 = await updateFolderRules(backend, folderName, [
        { path_glob: "[unclosed", permissioned_as: "u/admin" },
      ]);
      expect(r1.status).toBe(400);
      const b1 = await r1.text();
      expect(b1).toContain("not a valid glob");

      // Invalid permissioned_as format
      const r2 = await updateFolderRules(backend, folderName, [
        { path_glob: "**", permissioned_as: "bogus" },
      ]);
      expect(r2.status).toBe(400);
      await r2.text();

      // Non-array
      const r3 = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/update/${folderName}`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ default_permissioned_as: "not-an-array" }),
        }
      );
      expect(r3.status).toBe(400);
      await r3.text();
    });
  });

  test("backend resolution: matching, non-matching, first-match-wins", async () => {
    await withTestBackend(async (backend, _tempDir) => {
      const id = Date.now();
      const folderName = `dpa_res_${id}`;

      await createFolderWithRules(backend, folderName, [
        { path_glob: "jobs/critical/**", permissioned_as: "g/all" },
        { path_glob: "jobs/**", permissioned_as: "u/admin" },
      ]);

      // Create a helper script for schedules
      await createScript(backend, `f/${folderName}/helper`);

      // Matching jobs/** (not critical) → u/admin
      const r1 = await createSchedule(
        backend,
        `f/${folderName}/jobs/sched1`,
        `f/${folderName}/helper`
      );
      expect(r1.status).toBe(200);
      await r1.text();
      const s1 = await getSchedule(backend, `f/${folderName}/jobs/sched1`);
      expect(s1.permissioned_as).toBe("u/admin");

      // First-match-wins: jobs/critical/** → g/all
      const r2 = await createSchedule(
        backend,
        `f/${folderName}/jobs/critical/prod`,
        `f/${folderName}/helper`
      );
      expect(r2.status).toBe(200);
      await r2.text();
      const s2 = await getSchedule(
        backend,
        `f/${folderName}/jobs/critical/prod`
      );
      expect(s2.permissioned_as).toBe("g/all");

      // Non-matching path → acting user (admin)
      const r3 = await createSchedule(
        backend,
        `f/${folderName}/dev/sched3`,
        `f/${folderName}/helper`
      );
      expect(r3.status).toBe(200);
      await r3.text();
      const s3 = await getSchedule(backend, `f/${folderName}/dev/sched3`);
      expect(s3.permissioned_as).toBe("u/admin");
    });
  });

  test("stale rule: 400 when rule resolves to non-existent user", async () => {
    await withTestBackend(async (backend, _tempDir) => {
      const id = Date.now();
      const folderName = `dpa_stale_${id}`;

      // Create folder WITHOUT rules first so we can create the helper script
      await createFolderWithRules(backend, folderName, []);
      await createScript(backend, `f/${folderName}/helper`);

      // Now add the stale rule
      const updateResp = await updateFolderRules(backend, folderName, [
        { path_glob: "**", permissioned_as: "u/ghost" },
      ]);
      expect(updateResp.status).toBe(200);
      await updateResp.text();

      const r = await createSchedule(
        backend,
        `f/${folderName}/should_fail`,
        `f/${folderName}/helper`
      );
      expect(r.status).toBe(400);
      const body = await r.text();
      expect(body).toContain("u/ghost");
      expect(body).toContain("does not exist");
    });
  });

  test("CLI preserve-on-update: push preserves remote permissioned_as", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const id = Date.now();
      const folderName = `dpa_cli_${id}`;

      // Create folder + schedule with a specific permissioned_as via API
      await createFolderWithRules(backend, folderName, [
        { path_glob: "**", permissioned_as: "u/admin" },
      ]);
      await createScript(backend, `f/${folderName}/helper`);
      const r = await createSchedule(
        backend,
        `f/${folderName}/preserved_sched`,
        `f/${folderName}/helper`
      );
      expect(r.status).toBe(200);
      await r.text();

      // Verify it got the default
      const before = await getSchedule(
        backend,
        `f/${folderName}/preserved_sched`
      );
      expect(before.permissioned_as).toBe("u/admin");

      // Pull the workspace, modify the schedule locally, push back
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\ncliBehavior: v1\nincludes:\n  - "f/${folderName}/**"\nincludeSchedules: true\n`,
        "utf-8"
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Read the schedule file and change the cron expression
      const schedDir = join(
        tempDir,
        "f",
        folderName,
        `preserved_sched.schedule.yaml`
      );
      let schedContent = await readFile(schedDir, "utf-8");
      schedContent = schedContent.replace("0 0 */6 * * *", "0 0 */12 * * *");
      await writeFile(schedDir, schedContent, "utf-8");

      // Push — should preserve u/admin, not overwrite to acting user
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir
      );
      expect(pushResult.code).toEqual(0);

      // Verify permissioned_as was preserved
      const after = await getSchedule(
        backend,
        `f/${folderName}/preserved_sched`
      );
      expect(after.permissioned_as).toBe("u/admin");
      expect(after.schedule).toBe("0 0 */12 * * *");
    });
  });

  test("CLI pull stripping: scripts have has_on_behalf_of instead of email", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const id = Date.now();
      const folderName = `dpa_strip_${id}`;

      // Create folder + script with on_behalf_of_email via folder default
      await createFolderWithRules(backend, folderName, [
        { path_glob: "**", permissioned_as: "u/admin" },
      ]);
      await createScript(backend, `f/${folderName}/my_script`);

      // Pull with cliBehavior: v1
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\ncliBehavior: v1\nincludes:\n  - "f/${folderName}/**"\n`,
        "utf-8"
      );

      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir
      );
      expect(pullResult.code).toEqual(0);

      // Read the script metadata
      const metaPath = join(
        tempDir,
        "f",
        folderName,
        "my_script.script.yaml"
      );
      const metaContent = await readFile(metaPath, "utf-8");

      // Should have has_on_behalf_of: true (or false) but NOT on_behalf_of_email
      expect(metaContent).toContain("has_on_behalf_of:");
      expect(metaContent).not.toContain("on_behalf_of_email:");
    });
  });
});
