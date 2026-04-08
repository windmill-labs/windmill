import { expect, test } from "bun:test";
import { withTestBackend, type TestBackend } from "./test_backend.ts";

// =============================================================================
// FORK & MERGE INTEGRATION TESTS
//
// Tests the full fork/merge cycle using a single test backend instance.
// All sub-tests share the same backend to avoid workspace-limit issues
// (CE limits to 2 non-admins workspaces).
//
// workspace_diff tracking is an EE feature (populated by git sync).
// We manually insert rows into workspace_diff to simulate what git sync
// does, matching the pattern in backend/.../workspace_comparison.rs.
// =============================================================================

const FORK_ID = "wm-fork-merge-test";

async function api(
  backend: TestBackend,
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  return backend.apiRequest!(path, options);
}

async function runSQL(backend: TestBackend, query: string): Promise<void> {
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

async function populateWorkspaceDiff(
  backend: TestBackend,
  parentWs: string,
  forkWs: string,
  diffs: Array<{ path: string; kind: string; ahead: number; behind: number }>
): Promise<void> {
  const values = diffs
    .map(
      (d) =>
        `('${parentWs}', '${forkWs}', '${d.path}', '${d.kind}', ${d.ahead}, ${d.behind})`
    )
    .join(",\n");
  await runSQL(
    backend,
    `INSERT INTO workspace_diff (source_workspace_id, fork_workspace_id, path, kind, ahead, behind)
     VALUES ${values}
     ON CONFLICT (source_workspace_id, fork_workspace_id, path, kind)
     DO UPDATE SET ahead = EXCLUDED.ahead, behind = EXCLUDED.behind, has_changes = NULL`
  );
}

async function removeFromSkipTally(backend: TestBackend, workspaceId: string) {
  await runSQL(backend, `DELETE FROM skip_workspace_diff_tally WHERE workspace_id = '${workspaceId}'`);
}

async function deleteFork(backend: TestBackend, forkId: string) {
  try {
    await api(backend, `/api/w/${forkId}/workspaces/delete`, { method: "POST" });
  } catch {}
  await runSQL(backend, `DELETE FROM workspace_diff WHERE fork_workspace_id = '${forkId}'`);
  await runSQL(backend, `DELETE FROM skip_workspace_diff_tally WHERE workspace_id = '${forkId}'`);
}

async function createTestItems(backend: TestBackend, workspace: string) {
  await api(backend, `/api/w/${workspace}/folders/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name: "merge_test" }),
  });
  await api(backend, `/api/w/${workspace}/scripts/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      path: "f/merge_test/script_a",
      content: "export function main() { return 'parent v1'; }",
      language: "bun",
      summary: "Script A",
      schema: { type: "object", properties: {}, required: [] },
    }),
  });
  await api(backend, `/api/w/${workspace}/variables/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path: "f/merge_test/var_a", value: "parent_value", is_secret: false, description: "Variable A" }),
  });
  await api(backend, `/api/w/${workspace}/resources/type/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name: "merge_test_type", schema: { type: "object" }, description: "Test type" }),
  });
  await api(backend, `/api/w/${workspace}/resources/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path: "f/merge_test/resource_a", resource_type: "merge_test_type", value: { key: "parent" }, description: "Resource A" }),
  });
}

async function createFork(backend: TestBackend, parentWs: string, forkId: string, color?: string) {
  const r = await api(backend, `/api/w/${parentWs}/workspaces/create_fork`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id: forkId, name: "Test Fork", color, forked_datatables: [] }),
  });
  if (!r.ok) {
    const err = await r.text();
    throw new Error(`Fork creation failed: ${r.status} ${err}`);
  }
  await removeFromSkipTally(backend, forkId);
  await new Promise((resolve) => setTimeout(resolve, 500));
}

// =====================================================================
// All fork/merge sub-tests run inside a single withTestBackend to share
// the backend instance and avoid CE workspace limits.
// =====================================================================
test(
  "Fork/Merge: full cycle integration tests",
  async () => {
    await withTestBackend(async (backend, _tempDir) => {
      const parentWs = backend.workspace;

      // ---------------------------------------------------------------
      // Sub-test 1: Deploy changes from fork to parent
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 1: fork→parent deploy ---");
      await deleteFork(backend, FORK_ID);
      await createTestItems(backend, parentWs);
      await createFork(backend, parentWs, FORK_ID, "#ff5500");

      // Make changes in fork
      const forkScript = await (await api(backend, `/api/w/${FORK_ID}/scripts/get/p/f/merge_test/script_a`)).json();
      await api(backend, `/api/w/${FORK_ID}/scripts/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...forkScript, content: "export function main() { return 'fork v2 - modified!'; }", summary: "Script A (fork)", parent_hash: forkScript.hash }),
      });
      await api(backend, `/api/w/${FORK_ID}/variables/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/new_fork_var", value: "fork_only", is_secret: false, description: "New from fork" }),
      });
      await api(backend, `/api/w/${FORK_ID}/resources/update/f/merge_test/resource_a`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/resource_a", value: { key: "fork_modified" }, description: "Resource A (fork)" }),
      });

      // Populate workspace_diff
      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/script_a", kind: "script", ahead: 1, behind: 0 },
        { path: "f/merge_test/new_fork_var", kind: "variable", ahead: 1, behind: 0 },
        { path: "f/merge_test/resource_a", kind: "resource", ahead: 1, behind: 0 },
      ]);

      // Compare
      const comp1 = await (await api(backend, `/api/w/${parentWs}/workspaces/compare/${FORK_ID}`)).json();
      expect(comp1.skipped_comparison).toBe(false);
      expect(comp1.summary.total_diffs).toBeGreaterThanOrEqual(3);
      expect(comp1.summary.conflicts).toBe(0);

      // Deploy fork→parent
      for (const diff of comp1.diffs.filter((d: any) => d.ahead > 0)) {
        const { kind, path } = diff;
        if (kind === "script") {
          const s = await (await api(backend, `/api/w/${FORK_ID}/scripts/get/p/${path}`)).json();
          let parentHash;
          try { parentHash = (await (await api(backend, `/api/w/${parentWs}/scripts/get/p/${path}`)).json()).hash; } catch {}
          expect((await api(backend, `/api/w/${parentWs}/scripts/create`, {
            method: "POST", headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ ...s, lock: s.lock, parent_hash: parentHash }),
          })).ok).toBe(true);
        } else if (kind === "variable") {
          const v = await (await api(backend, `/api/w/${FORK_ID}/variables/get/${path}?decrypt_secret=true`)).json();
          const exists = await (await api(backend, `/api/w/${parentWs}/variables/exists/${path}`)).json();
          if (exists) {
            await api(backend, `/api/w/${parentWs}/variables/update/${path}?already_encrypted=false`, {
              method: "POST", headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ path, value: v.value ?? "", is_secret: v.is_secret, description: v.description ?? "" }),
            });
          } else {
            await api(backend, `/api/w/${parentWs}/variables/create`, {
              method: "POST", headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ path, value: v.value ?? "", is_secret: v.is_secret, description: v.description ?? "" }),
            });
          }
        } else if (kind === "resource") {
          const res = await (await api(backend, `/api/w/${FORK_ID}/resources/get/${path}`)).json();
          const exists = await (await api(backend, `/api/w/${parentWs}/resources/exists/${path}`)).json();
          if (exists) {
            await api(backend, `/api/w/${parentWs}/resources/update/${path}`, {
              method: "POST", headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ path, value: res.value, description: res.description ?? "" }),
            });
          } else {
            await api(backend, `/api/w/${parentWs}/resources/create`, {
              method: "POST", headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ path, value: res.value, resource_type: res.resource_type, description: res.description ?? "" }),
            });
          }
        }
      }

      // Verify
      expect((await (await api(backend, `/api/w/${parentWs}/scripts/get/p/f/merge_test/script_a`)).json()).content).toContain("fork v2");
      expect((await api(backend, `/api/w/${parentWs}/variables/get/f/merge_test/new_fork_var`)).ok).toBe(true);
      expect(JSON.stringify((await (await api(backend, `/api/w/${parentWs}/resources/get/f/merge_test/resource_a`)).json()).value)).toContain("fork_modified");
      console.log("  ✓ Sub-test 1 passed: fork→parent deploy");

      // ---------------------------------------------------------------
      // Sub-test 2: Deploy parent→fork direction
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 2: parent→fork deploy ---");

      // Create new script in parent
      await api(backend, `/api/w/${parentWs}/scripts/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/script_b", content: "export function main() { return 'parent only'; }", language: "bun", summary: "Script B", schema: { type: "object", properties: {}, required: [] } }),
      });

      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/script_b", kind: "script", ahead: 0, behind: 1 },
      ]);

      const comp2 = await (await api(backend, `/api/w/${parentWs}/workspaces/compare/${FORK_ID}`)).json();
      const behindDiffs = comp2.diffs.filter((d: any) => d.behind > 0);
      expect(behindDiffs.length).toBeGreaterThanOrEqual(1);

      // Deploy parent→fork
      for (const diff of behindDiffs) {
        if (diff.kind === "script") {
          const s = await (await api(backend, `/api/w/${parentWs}/scripts/get/p/${diff.path}`)).json();
          let forkHash;
          try { forkHash = (await (await api(backend, `/api/w/${FORK_ID}/scripts/get/p/${diff.path}`)).json()).hash; } catch {}
          await api(backend, `/api/w/${FORK_ID}/scripts/create`, {
            method: "POST", headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ ...s, lock: s.lock, parent_hash: forkHash }),
          });
        }
      }

      expect((await api(backend, `/api/w/${FORK_ID}/scripts/get/p/f/merge_test/script_b`)).ok).toBe(true);
      console.log("  ✓ Sub-test 2 passed: parent→fork deploy");

      // ---------------------------------------------------------------
      // Sub-test 3: Conflict detection
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 3: conflict detection ---");

      // Create actual divergence: modify script_a differently in both workspaces
      const parentScriptA = await (await api(backend, `/api/w/${parentWs}/scripts/get/p/f/merge_test/script_a`)).json();
      await api(backend, `/api/w/${parentWs}/scripts/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...parentScriptA, content: "export function main() { return 'parent conflict version'; }", parent_hash: parentScriptA.hash }),
      });

      const forkScriptA = await (await api(backend, `/api/w/${FORK_ID}/scripts/get/p/f/merge_test/script_a`)).json();
      await api(backend, `/api/w/${FORK_ID}/scripts/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...forkScriptA, content: "export function main() { return 'fork conflict version'; }", parent_hash: forkScriptA.hash }),
      });

      // Now populate the conflict diff
      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/script_a", kind: "script", ahead: 2, behind: 1 },
      ]);

      const comp3 = await (await api(backend, `/api/w/${parentWs}/workspaces/compare/${FORK_ID}`)).json();
      expect(comp3.summary.conflicts).toBeGreaterThanOrEqual(1);
      const conflict = comp3.diffs.find((d: any) => d.path === "f/merge_test/script_a" && d.ahead > 0 && d.behind > 0);
      expect(conflict).toBeDefined();
      console.log("  ✓ Sub-test 3 passed: conflict detected");

      // ---------------------------------------------------------------
      // Sub-test 4: Fork has correct parent_workspace_id
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 4: parent_workspace_id ---");
      // Check parent_workspace_id via direct SQL (no REST endpoint exposes this directly)
      const dbUrl = process.env["DATABASE_URL"] || "postgres://postgres:changeme@localhost:5432";
      const dbProc = Bun.spawn(
        ["psql", `${dbUrl}/postgres?sslmode=disable`, "-t", "-c",
         `SELECT datname FROM pg_database WHERE datname LIKE 'windmill_test_%' ORDER BY datname DESC LIMIT 1`],
        { stdout: "pipe", stderr: "pipe" }
      );
      const testDb = (await new Response(dbProc.stdout).text()).trim();
      await dbProc.exited;
      const parentProc = Bun.spawn(
        ["psql", `${dbUrl}/${testDb}?sslmode=disable`, "-t", "-c",
         `SELECT parent_workspace_id FROM workspace WHERE id = '${FORK_ID}'`],
        { stdout: "pipe", stderr: "pipe" }
      );
      const parentId = (await new Response(parentProc.stdout).text()).trim();
      await parentProc.exited;
      expect(parentId).toBe(parentWs);
      console.log("  ✓ Sub-test 4 passed: parent_workspace_id correct");

      // ---------------------------------------------------------------
      // Sub-test 5: resetDiffTally cleans unchanged items
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 5: resetDiffTally ---");

      // Add a diff for an item that hasn't actually changed (var_a was deployed already)
      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/var_a", kind: "variable", ahead: 1, behind: 0 },
      ]);

      const resetResp = await api(backend, `/api/w/${parentWs}/workspaces/reset_diff_tally/${FORK_ID}`, { method: "POST" });
      expect(resetResp.ok).toBe(true);

      const comp5 = await (await api(backend, `/api/w/${parentWs}/workspaces/compare/${FORK_ID}`)).json();
      const varDiff = comp5.diffs.find((d: any) => d.path === "f/merge_test/var_a" && d.kind === "variable");
      // var_a was already deployed (same value in both), so it should be cleaned up
      expect(varDiff).toBeUndefined();
      console.log("  ✓ Sub-test 5 passed: resetDiffTally cleaned unchanged items");

      // ---------------------------------------------------------------
      // Cleanup
      // ---------------------------------------------------------------
      await deleteFork(backend, FORK_ID);
      console.log("\n✅ All sub-tests passed!");
    });
  },
  180_000
);
