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
  // Force-clean via SQL to ensure workspace slot is freed (CE limits to 2)
  const esc = forkId.replace(/'/g, "''");
  await runSQL(backend, `
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
      // Sub-test 6: All item types in one merge
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 6: all item types (script, variable, resource, resource_type, flow, app) ---");
      await deleteFork(backend, FORK_ID);

      // Create resource type + resource in parent
      await api(backend, `/api/w/${parentWs}/resources/type/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "merge_test_type", schema: { type: "object" }, description: "Type" }),
      });
      await api(backend, `/api/w/${parentWs}/resources/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/res_t6", resource_type: "merge_test_type", value: { key: "parent" }, description: "R" }),
      });
      // Create flow in parent
      await api(backend, `/api/w/${parentWs}/flows/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: "f/merge_test/flow_t6", summary: "Flow", schema: { type: "object", properties: {}, required: [] },
          value: { modules: [{ id: "a", value: { type: "rawscript", content: "export function main() { return 1; }", language: "bun", input_transforms: {} } }], failure_module: null, same_worker: false },
        }),
      });
      // Create app in parent
      await api(backend, `/api/w/${parentWs}/apps/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: "f/merge_test/app_t6", summary: "App",
          value: { type: "rawscript", content: { v: 1 } },
          policy: { on_behalf_of: "", on_behalf_of_email: "", extra_perms: {}, execution_mode: "publisher" },
        }),
      });

      await createFork(backend, parentWs, FORK_ID);

      // Modify all in fork
      const forkRes = await (await api(backend, `/api/w/${FORK_ID}/resources/get/f/merge_test/res_t6`)).json();
      await api(backend, `/api/w/${FORK_ID}/resources/update/f/merge_test/res_t6`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: forkRes.path, value: { key: "fork" }, description: "Fork" }),
      });

      const forkFlow = await (await api(backend, `/api/w/${FORK_ID}/flows/get/f/merge_test/flow_t6`)).json();
      forkFlow.value.modules[0].value.content = "export function main() { return 2; }";
      await api(backend, `/api/w/${FORK_ID}/flows/update/f/merge_test/flow_t6`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify(forkFlow),
      });

      const forkApp = await (await api(backend, `/api/w/${FORK_ID}/apps/get/p/f/merge_test/app_t6`)).json();
      await api(backend, `/api/w/${FORK_ID}/apps/update/f/merge_test/app_t6`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...forkApp, summary: "Fork App" }),
      });

      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/res_t6", kind: "resource", ahead: 1, behind: 0 },
        { path: "f/merge_test/flow_t6", kind: "flow", ahead: 1, behind: 0 },
        { path: "f/merge_test/app_t6", kind: "app", ahead: 1, behind: 0 },
      ]);

      const comp6 = await (await api(backend, `/api/w/${parentWs}/workspaces/compare/${FORK_ID}`)).json();
      const ahead6 = comp6.diffs.filter((d: any) => d.ahead > 0);
      expect(ahead6.length).toBeGreaterThanOrEqual(3);

      // Deploy all
      for (const d of ahead6) {
        if (d.kind === "resource") {
          const r = await (await api(backend, `/api/w/${FORK_ID}/resources/get/${d.path}`)).json();
          await api(backend, `/api/w/${parentWs}/resources/update/${d.path}`, {
            method: "POST", headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ path: d.path, value: r.value, description: r.description ?? "" }),
          });
        } else if (d.kind === "flow") {
          const f = await (await api(backend, `/api/w/${FORK_ID}/flows/get/${d.path}`)).json();
          for (const m of f.value?.modules ?? []) { if (m.value?.hash) m.value.hash = undefined; }
          await api(backend, `/api/w/${parentWs}/flows/update/${d.path}`, {
            method: "POST", headers: { "Content-Type": "application/json" },
            body: JSON.stringify(f),
          });
        } else if (d.kind === "app") {
          const a = await (await api(backend, `/api/w/${FORK_ID}/apps/get/p/${d.path}`)).json();
          await api(backend, `/api/w/${parentWs}/apps/update/${d.path}`, {
            method: "POST", headers: { "Content-Type": "application/json" },
            body: JSON.stringify(a),
          });
        }
      }

      // Verify
      const pRes = await (await api(backend, `/api/w/${parentWs}/resources/get/f/merge_test/res_t6`)).json();
      expect(JSON.stringify(pRes.value)).toContain("fork");
      const pFlow = await (await api(backend, `/api/w/${parentWs}/flows/get/f/merge_test/flow_t6`)).json();
      expect(pFlow.value.modules[0].value.content).toContain("return 2");
      const pApp = await (await api(backend, `/api/w/${parentWs}/apps/get/p/f/merge_test/app_t6`)).json();
      expect(pApp.summary).toBe("Fork App");
      console.log("  ✓ Sub-test 6 passed: all item types merged");

      // ---------------------------------------------------------------
      // Sub-test 7: Secret variables preserved across fork/merge
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 7: secret variable ---");
      await api(backend, `/api/w/${FORK_ID}/variables/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/secret_fork", value: "s3cret!", is_secret: true, description: "Secret" }),
      });
      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/secret_fork", kind: "variable", ahead: 1, behind: 0 },
      ]);
      const comp7 = await (await api(backend, `/api/w/${parentWs}/workspaces/compare/${FORK_ID}`)).json();
      const secDiff = comp7.diffs.find((d: any) => d.path === "f/merge_test/secret_fork");
      expect(secDiff).toBeDefined();

      const secVar = await (await api(backend, `/api/w/${FORK_ID}/variables/get/f/merge_test/secret_fork?decrypt_secret=true`)).json();
      await api(backend, `/api/w/${parentWs}/variables/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: secVar.path, value: secVar.value ?? "", is_secret: secVar.is_secret, description: secVar.description ?? "" }),
      });
      const pSec = await (await api(backend, `/api/w/${parentWs}/variables/get/f/merge_test/secret_fork?decrypt_secret=true`)).json();
      expect(pSec.value).toBe("s3cret!");
      expect(pSec.is_secret).toBe(true);
      console.log("  ✓ Sub-test 7 passed: secret variable preserved");

      // ---------------------------------------------------------------
      // Sub-test 8: Special characters in variable values
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 8: special characters ---");
      await api(backend, `/api/w/${FORK_ID}/variables/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/special", value: "hello\nworld\t\"quotes\" 'single' \\back 日本語 🎉", is_secret: false, description: "" }),
      });
      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/special", kind: "variable", ahead: 1, behind: 0 },
      ]);
      const forkSpecial = await (await api(backend, `/api/w/${FORK_ID}/variables/get/f/merge_test/special?decrypt_secret=true`)).json();
      await api(backend, `/api/w/${parentWs}/variables/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: forkSpecial.path, value: forkSpecial.value ?? "", is_secret: false, description: "" }),
      });
      const pSpecial = await (await api(backend, `/api/w/${parentWs}/variables/get/f/merge_test/special?decrypt_secret=true`)).json();
      expect(pSpecial.value).toBe(forkSpecial.value);
      console.log("  ✓ Sub-test 8 passed: special characters preserved");

      // ---------------------------------------------------------------
      // Sub-test 9: Partial deploy — only deployed items cleaned by resetDiffTally
      // ---------------------------------------------------------------
      console.log("\n--- Sub-test 9: partial deploy + resetDiffTally ---");
      await api(backend, `/api/w/${FORK_ID}/variables/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/partial_a", value: "a", is_secret: false, description: "A" }),
      });
      await api(backend, `/api/w/${FORK_ID}/variables/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "f/merge_test/partial_b", value: "b", is_secret: false, description: "B" }),
      });
      await populateWorkspaceDiff(backend, parentWs, FORK_ID, [
        { path: "f/merge_test/partial_a", kind: "variable", ahead: 1, behind: 0 },
        { path: "f/merge_test/partial_b", kind: "variable", ahead: 1, behind: 0 },
      ]);

      // Deploy only partial_a
      const va = await (await api(backend, `/api/w/${FORK_ID}/variables/get/f/merge_test/partial_a?decrypt_secret=true`)).json();
      await api(backend, `/api/w/${parentWs}/variables/create`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: va.path, value: va.value ?? "", is_secret: false, description: "" }),
      });

      await api(backend, `/api/w/${parentWs}/workspaces/reset_diff_tally/${FORK_ID}`, { method: "POST" });
      await new Promise(r => setTimeout(r, 500));
      const comp9 = await (await api(backend, `/api/w/${parentWs}/workspaces/compare/${FORK_ID}`)).json();
      const bAfter = comp9.diffs.find((d: any) => d.path === "f/merge_test/partial_b");
      // partial_b was NOT deployed, so it must still appear in diffs
      expect(bAfter).toBeDefined();
      expect(bAfter?.ahead).toBeGreaterThan(0);
      console.log("  ✓ Sub-test 9 passed: partial deploy + resetDiffTally");

      // ---------------------------------------------------------------
      // Cleanup
      // ---------------------------------------------------------------
      await deleteFork(backend, FORK_ID);
      console.log("\n✅ All sub-tests passed!");
    });
  },
  300_000
);
