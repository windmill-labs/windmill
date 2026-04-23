/**
 * Integration tests for folder default_permissioned_as — full lifecycle:
 *  1. Full lifecycle: create folder via CLI, set rules, push, pull, modify, push,
 *     create new item under folder — verifies rules are applied at each step
 *  2. Fork/merge: parent workspace has folder with rules, fork inherits them,
 *     fork can update rules, deployed back to parent
 *  3. Backend validation + resolution (minimal API-level checks)
 *  4. Stale rule rejection
 */

import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend, type TestBackend } from "./test_backend.ts";
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

async function api(
  backend: TestBackend,
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  return backend.apiRequest!(path, options);
}

async function createFolderWithRules(
  backend: any,
  name: string,
  rules: Array<{ path_glob: string; permissioned_as: string }>,
  extraPerms?: Record<string, boolean>,
  workspaceId?: string
): Promise<void> {
  const ws = workspaceId ?? backend.workspace;
  const resp = await backend.apiRequest!(`/api/w/${ws}/folders/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      name,
      default_permissioned_as: rules,
      extra_perms: extraPerms ?? {},
    }),
  });
  await resp.text();
  expect(resp.status).toBeLessThan(300);
}

async function updateFolderRules(
  backend: any,
  name: string,
  rules: Array<{ path_glob: string; permissioned_as: string }>,
  workspaceId?: string
): Promise<Response> {
  const ws = workspaceId ?? backend.workspace;
  return backend.apiRequest!(`/api/w/${ws}/folders/update/${name}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ default_permissioned_as: rules }),
  });
}

async function createScript(
  backend: any,
  path: string,
  workspaceId?: string
): Promise<string> {
  const ws = workspaceId ?? backend.workspace;
  const resp = await backend.apiRequest!(`/api/w/${ws}/scripts/create`, {
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
  });
  const body = await resp.text();
  expect(resp.status).toBe(201);
  return body;
}

async function createSchedule(
  backend: any,
  path: string,
  scriptPath: string,
  extra?: Record<string, any>,
  workspaceId?: string
): Promise<Response> {
  const ws = workspaceId ?? backend.workspace;
  return backend.apiRequest!(`/api/w/${ws}/schedules/create`, {
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
  });
}

async function getSchedule(
  backend: any,
  path: string,
  workspaceId?: string
): Promise<any> {
  const ws = workspaceId ?? backend.workspace;
  const resp = await backend.apiRequest!(`/api/w/${ws}/schedules/get/${path}`);
  expect(resp.status).toBe(200);
  return resp.json();
}

async function getFolder(
  backend: any,
  name: string,
  workspaceId?: string
): Promise<any> {
  const ws = workspaceId ?? backend.workspace;
  const resp = await backend.apiRequest!(`/api/w/${ws}/folders/get/${name}`);
  expect(resp.status).toBe(200);
  return resp.json();
}

async function runSQL(query: string): Promise<void> {
  const dbUrl =
    process.env["DATABASE_URL"] ||
    "postgres://postgres:changeme@localhost:5432";
  const proc = Bun.spawn(
    [
      "psql",
      `${dbUrl}/postgres?sslmode=disable`,
      "-t",
      "-c",
      `SELECT datname FROM pg_database WHERE datname LIKE 'windmill_test_%' ORDER BY datname DESC LIMIT 1`,
    ],
    { stdout: "pipe", stderr: "pipe" }
  );
  const dbName = (await new Response(proc.stdout).text()).trim();
  await proc.exited;
  if (!dbName) throw new Error("Could not find test database");
  const sqlProc = Bun.spawn(
    ["psql", `${dbUrl}/${dbName}?sslmode=disable`, "-c", query],
    { stdout: "pipe", stderr: "pipe" }
  );
  await sqlProc.exited;
}

async function removeFromSkipTally(workspaceId: string) {
  await runSQL(
    `DELETE FROM skip_workspace_diff_tally WHERE workspace_id = '${workspaceId}'`
  );
}

async function deleteFork(backend: TestBackend, forkId: string) {
  try {
    await api(backend, `/api/w/${forkId}/workspaces/delete`, { method: "POST" });
  } catch {}
  const esc = forkId.replace(/'/g, "''");
  await runSQL(`
    SET session_replication_role = replica;
    DO $$ DECLARE r RECORD; BEGIN
      FOR r IN SELECT c.table_name FROM information_schema.columns c
        JOIN information_schema.tables t ON c.table_name = t.table_name AND c.table_schema = t.table_schema
        WHERE c.column_name = 'workspace_id' AND c.table_schema = 'public' AND t.table_type = 'BASE TABLE'
        GROUP BY c.table_name
      LOOP EXECUTE format('DELETE FROM %I WHERE workspace_id = ''${esc}''', r.table_name);
      END LOOP;
    END $$;
    DELETE FROM workspace WHERE id = '${esc}';
    DELETE FROM workspace_diff WHERE fork_workspace_id = '${esc}';
    DELETE FROM skip_workspace_diff_tally WHERE workspace_id = '${esc}';
    SET session_replication_role = DEFAULT;
  `);
}

describe("folder default_permissioned_as", () => {
  // ==========================================================================
  // Test 1: Full CLI lifecycle — create, push, pull, modify, push, verify apply
  // ==========================================================================
  test("full CLI lifecycle: push rules → pull back → modify → push → apply on new item", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const id = Date.now();
      const folderName = `dpa_life_${id}`;

      // Step 1: Seed an empty folder + helper script via API (needed so schedules can reference it)
      await createFolderWithRules(backend, folderName, []);
      await createScript(backend, `f/${folderName}/helper`);

      // Step 2: Pull workspace locally
      await writeFile(
        join(tempDir, "wmill.yaml"),
        `defaultTs: bun\nsyncBehavior: v1\nincludes:\n  - "f/${folderName}/**"\nincludeSchedules: true\n`,
        "utf-8"
      );
      let r = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(r.code).toEqual(0);

      // Step 3: Locally edit folder.meta.yaml to add default_permissioned_as rules
      const metaPath = join(tempDir, "f", folderName, "folder.meta.yaml");
      const metaContent = await readFile(metaPath, "utf-8");
      // Backend omits the field when the rule list is empty
      expect(metaContent).not.toContain("default_permissioned_as:");

      const newMeta = `display_name: ${folderName}
owners:
  - u/admin
extra_perms:
  u/admin: true
summary: null
default_permissioned_as:
  - path_glob: "jobs/critical/**"
    permissioned_as: "g/all"
  - path_glob: "jobs/**"
    permissioned_as: "u/admin"
`;
      await writeFile(metaPath, newMeta, "utf-8");

      // Step 4: Push the folder update AND a brand-new schedule at the same time.
      // This tests the folder-first ordering: the new schedule should pick up
      // the new folder rule within the same push batch.
      const schedPath = join(
        tempDir,
        "f",
        folderName,
        "new_sched.schedule.yaml"
      );
      await writeFile(
        schedPath,
        `schedule: "0 0 */6 * * *"
timezone: UTC
script_path: f/${folderName}/helper
is_flow: false
enabled: false
`,
        "utf-8"
      );

      r = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(r.code).toEqual(0);

      // Verify: folder has the new rules
      const folder = await getFolder(backend, folderName);
      expect(folder.default_permissioned_as).toHaveLength(2);
      expect(folder.default_permissioned_as[0].path_glob).toBe(
        "jobs/critical/**"
      );
      expect(folder.default_permissioned_as[1].path_glob).toBe("jobs/**");

      // Verify: schedule got u/admin (the jobs/** rule) — only works if folder
      // was pushed FIRST within the same batch
      const sched = await getSchedule(backend, `f/${folderName}/new_sched`);
      expect(sched.permissioned_as).toBe("u/admin");

      // Step 5: Pull again — verify the updated rules round-trip through YAML
      r = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
      expect(r.code).toEqual(0);
      const pulledMeta = await readFile(metaPath, "utf-8");
      expect(pulledMeta).toContain("default_permissioned_as:");
      expect(pulledMeta).toContain("jobs/critical/**");
      expect(pulledMeta).toContain("g/all");

      // Step 6: Reorder rules locally and push again — verify order changes
      const reorderedMeta = `display_name: ${folderName}
owners:
  - u/admin
extra_perms:
  u/admin: true
summary: null
default_permissioned_as:
  - path_glob: "jobs/**"
    permissioned_as: "u/admin"
  - path_glob: "jobs/critical/**"
    permissioned_as: "g/all"
`;
      await writeFile(metaPath, reorderedMeta, "utf-8");
      r = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(r.code).toEqual(0);

      const folder2 = await getFolder(backend, folderName);
      expect(folder2.default_permissioned_as[0].path_glob).toBe("jobs/**");
      expect(folder2.default_permissioned_as[1].path_glob).toBe(
        "jobs/critical/**"
      );

      // Step 7: Now that jobs/** is listed first, a new critical schedule should
      // ALSO resolve to u/admin (first-match-wins, critical/** is shadowed)
      const schedPath2 = join(
        tempDir,
        "f",
        folderName,
        "crit_sched.schedule.yaml"
      );
      await mkdir(join(tempDir, "f", folderName, "jobs", "critical"), {
        recursive: true,
      });
      const critYaml = `schedule: "0 0 */6 * * *"
timezone: UTC
script_path: f/${folderName}/helper
is_flow: false
enabled: false
`;
      await writeFile(
        join(
          tempDir,
          "f",
          folderName,
          "jobs",
          "critical",
          "crit_sched.schedule.yaml"
        ),
        critYaml,
        "utf-8"
      );
      r = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(r.code).toEqual(0);

      const critSched = await getSchedule(
        backend,
        `f/${folderName}/jobs/critical/crit_sched`
      );
      expect(critSched.permissioned_as).toBe("u/admin"); // shadowed by jobs/**

      // Step 8: Clear rules locally and push — verify rules are cleared on backend
      const clearedMeta = `display_name: ${folderName}
owners:
  - u/admin
extra_perms:
  u/admin: true
summary: null
default_permissioned_as: []
`;
      await writeFile(metaPath, clearedMeta, "utf-8");
      r = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(r.code).toEqual(0);

      const folder3 = await getFolder(backend, folderName);
      expect(folder3.default_permissioned_as).toHaveLength(0);

      // Step 9: A new schedule after clearing falls back to the acting user
      await writeFile(
        join(tempDir, "f", folderName, "after_clear.schedule.yaml"),
        critYaml,
        "utf-8"
      );
      r = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
      expect(r.code).toEqual(0);

      const sched3 = await getSchedule(
        backend,
        `f/${folderName}/after_clear`
      );
      expect(sched3.permissioned_as).toBe("u/admin"); // admin is the acting user
    });
  });

  // ==========================================================================
  // Test 2: Fork/merge with default_permissioned_as
  // ==========================================================================
  test("fork/merge: rules propagate to fork and deploy back to parent", async () => {
    await withTestBackend(async (backend, _tempDir) => {
      const parentWs = backend.workspace;
      const FORK_ID = "wm-fork-dpa";
      await deleteFork(backend, FORK_ID);

      const id = Date.now();
      const folderName = `dpa_fork_${id}`;

      // Step 1: Parent workspace: create folder with rules + helper script
      await createFolderWithRules(backend, folderName, [
        { path_glob: "jobs/**", permissioned_as: "u/admin" },
      ]);
      await createScript(backend, `f/${folderName}/helper`);

      // Step 2: Create a fork — fork inherits the parent's folder including rules
      const forkResp = await api(
        backend,
        `/api/w/${parentWs}/workspaces/create_fork`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            id: FORK_ID,
            name: "DPA Fork",
            color: "#ff5500",
            forked_datatables: [],
          }),
        }
      );
      if (!forkResp.ok) {
        const err = await forkResp.text();
        throw new Error(`Fork creation failed: ${forkResp.status} ${err}`);
      }
      await removeFromSkipTally(FORK_ID);
      await new Promise((resolve) => setTimeout(resolve, 500));

      // Step 3: Verify fork inherited the folder rules
      const forkFolder = await getFolder(backend, folderName, FORK_ID);
      expect(forkFolder.default_permissioned_as).toHaveLength(1);
      expect(forkFolder.default_permissioned_as[0].path_glob).toBe("jobs/**");
      expect(forkFolder.default_permissioned_as[0].permissioned_as).toBe(
        "u/admin"
      );

      // Step 4: Create a schedule in the fork at a matching path — rule applies
      const forkSchedResp = await createSchedule(
        backend,
        `f/${folderName}/jobs/fork_sched`,
        `f/${folderName}/helper`,
        undefined,
        FORK_ID
      );
      expect(forkSchedResp.status).toBe(200);
      await forkSchedResp.text();

      const forkSched = await getSchedule(
        backend,
        `f/${folderName}/jobs/fork_sched`,
        FORK_ID
      );
      expect(forkSched.permissioned_as).toBe("u/admin");

      // Step 5: Modify the fork's rules (simulating divergence)
      const updateResp = await updateFolderRules(
        backend,
        folderName,
        [
          { path_glob: "jobs/**", permissioned_as: "u/admin" },
          { path_glob: "reports/**", permissioned_as: "g/all" },
        ],
        FORK_ID
      );
      expect(updateResp.status).toBe(200);
      await updateResp.text();

      // Step 6: Deploy the fork's folder rules back to parent via API
      const forkFolderUpdated = await getFolder(backend, folderName, FORK_ID);
      const deployResp = await updateFolderRules(
        backend,
        folderName,
        forkFolderUpdated.default_permissioned_as,
        parentWs
      );
      expect(deployResp.status).toBe(200);
      await deployResp.text();

      // Step 7: Verify parent now has the updated rules
      const parentFolderAfter = await getFolder(backend, folderName, parentWs);
      expect(parentFolderAfter.default_permissioned_as).toHaveLength(2);
      expect(parentFolderAfter.default_permissioned_as[1].path_glob).toBe(
        "reports/**"
      );
      expect(parentFolderAfter.default_permissioned_as[1].permissioned_as).toBe(
        "g/all"
      );

      // Step 8: A new schedule in the parent at a reports/ path should now get
      // the newly-merged g/all rule
      const parentSchedResp = await createSchedule(
        backend,
        `f/${folderName}/reports/parent_sched`,
        `f/${folderName}/helper`,
        undefined,
        parentWs
      );
      expect(parentSchedResp.status).toBe(200);
      await parentSchedResp.text();

      const parentSched = await getSchedule(
        backend,
        `f/${folderName}/reports/parent_sched`,
        parentWs
      );
      expect(parentSched.permissioned_as).toBe("g/all");

      await deleteFork(backend, FORK_ID);
    });
  });

  // ==========================================================================
  // Test 3: No-op guarantee — feature is inert when rules are not used
  // ==========================================================================
  test("no-op: folder without rules behaves like before for all deploy paths", async () => {
    await withTestBackend(async (backend, _tempDir) => {
      const id = Date.now();
      const folderName = `dpa_noop_${id}`;

      // Create folder with NO rules
      await createFolderWithRules(backend, folderName, []);
      await createScript(backend, `f/${folderName}/helper`);

      // Schedule create: should use acting user (admin), not fail or behave differently
      const schedResp = await createSchedule(
        backend,
        `f/${folderName}/noop_sched`,
        `f/${folderName}/helper`
      );
      expect(schedResp.status).toBe(200);
      await schedResp.text();
      const sched = await getSchedule(backend, `f/${folderName}/noop_sched`);
      expect(sched.permissioned_as).toBe("u/admin");
      expect(sched.email).toBe("admin@windmill.dev");

      // Schedule at a nested path: still acting user
      const schedResp2 = await createSchedule(
        backend,
        `f/${folderName}/jobs/deep/sched`,
        `f/${folderName}/helper`
      );
      expect(schedResp2.status).toBe(200);
      await schedResp2.text();
      const sched2 = await getSchedule(
        backend,
        `f/${folderName}/jobs/deep/sched`
      );
      expect(sched2.permissioned_as).toBe("u/admin");

      // Flow create: on_behalf_of_email stays null (nothing injected)
      const flowResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/flows/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: `f/${folderName}/noop_flow`,
            summary: "",
            description: "",
            value: { modules: [] },
            schema: { type: "object", properties: {}, required: [] },
          }),
        }
      );
      expect(flowResp.status).toBe(201);
      await flowResp.text();
      const flow = await (
        await backend.apiRequest!(
          `/api/w/${backend.workspace}/flows/get/f/${folderName}/noop_flow`
        )
      ).json();
      // no folder rule, no client-sent on_behalf_of_email → should be null/undefined
      expect(flow.on_behalf_of_email == null).toBe(true);

      // Folder response has the field but it's empty
      const folder = await getFolder(backend, folderName);
      expect(folder.default_permissioned_as).toEqual([]);

      // Folder update without touching default_permissioned_as should leave it alone
      const updateResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/update/${folderName}`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ summary: "updated summary" }),
        }
      );
      expect(updateResp.status).toBe(200);
      await updateResp.text();
      const folder2 = await getFolder(backend, folderName);
      expect(folder2.summary).toBe("updated summary");
      expect(folder2.default_permissioned_as).toEqual([]);
    });
  });

  // ==========================================================================
  // Test 4: Backend validation + stale rule rejection (minimal API-level)
  // ==========================================================================
  test("validation and stale rule rejection", async () => {
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

      // Stale rule: create helper, then set rule with ghost user, then deploy
      await createScript(backend, `f/${folderName}/helper`);
      const staleResp = await updateFolderRules(backend, folderName, [
        { path_glob: "**", permissioned_as: "u/ghost" },
      ]);
      expect(staleResp.status).toBe(200);
      await staleResp.text();

      const schedResp = await createSchedule(
        backend,
        `f/${folderName}/should_fail`,
        `f/${folderName}/helper`
      );
      expect(schedResp.status).toBe(400);
      const body = await schedResp.text();
      expect(body).toContain("u/ghost");
      expect(body).toContain("does not exist");
    });
  });
});
