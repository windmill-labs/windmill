import { test, expect } from "bun:test";
import { writeFile, readdir } from "node:fs/promises";
import path from "node:path";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../src/commands/workspace/workspace.ts";

/**
 * Regression test for duplicate backend scripts in raw apps.
 *
 * Root cause: updateRawAppRunnables (app_metadata.ts) used runnable.name for
 * file naming while extractInlineScriptsForApps (sync.ts) used the runnable KEY.
 * When name != key (common: keys are "a","b", names are "Fetch Users","Update Record"),
 * generate-metadata created duplicate content files (e.g., fetch_users.ts alongside a.ts).
 * On next push, loadRunnablesFromBackend auto-detected the orphans as new runnables.
 */
test("Raw app: generate-metadata must not create duplicate files when runnable key != name", async () => {
  await withTestBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "diffname_test",
      token: backend.token,
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    await writeFile(
      `${tempDir}/wmill.yaml`,
      `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
      "utf-8",
    );

    // Create raw app where runnable KEY differs from runnable NAME
    const appPath = "f/test/diffname_app";
    const rawAppJson = {
      value: {
        runnables: {
          a: {
            type: "runnableByName",
            name: "Fetch Users",
            inlineScript: {
              content: 'export async function main(x: string) { return x; }',
              language: "bun", lock: "",
            },
            fields: { x: { type: "static", value: "hello" } },
          },
          b: {
            type: "runnableByName",
            name: "Update Record",
            inlineScript: {
              content: 'export async function main(y: number) { return y * 2; }',
              language: "bun", lock: "",
            },
            fields: { y: { type: "static", value: 42 } },
          },
        },
        files: { "/index.tsx": "console.log('hello');" },
      },
      path: appPath,
      summary: "Test Diffname App",
      policy: { execution_mode: "publisher", triggerables: {} },
    };

    const formData = new FormData();
    formData.append("app", new Blob([JSON.stringify(rawAppJson)], { type: "application/json" }));
    formData.append("js", new Blob(["console.log('b');"], { type: "application/javascript" }));
    formData.append("css", new Blob([".t{}"], { type: "text/css" }));

    const createResp = await fetch(
      `${backend.baseUrl}/api/w/${backend.workspace}/apps/create_raw`,
      { method: "POST", headers: { Authorization: `Bearer ${backend.token}` }, body: formData },
    );
    expect(createResp.ok).toBeTruthy();

    // Pull the app
    const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
    expect(pullResult.code).toEqual(0);

    const backendDir = path.join(tempDir, "f/test/diffname_app.raw_app", "backend");
    let files = await readdir(backendDir);

    // After pull: files named by KEY (a, b).
    // Lock files are generated asynchronously by the backend worker and may not
    // have landed yet — exclude them from this precondition to avoid a Windows race.
    const nonLockFiles = files.filter((f) => !f.endsWith(".lock")).sort();
    expect(nonLockFiles).toEqual(["a.ts", "a.yaml", "b.ts", "b.yaml"]);

    // Run generate-metadata — this previously created duplicate files named by NAME
    const metaResult = await backend.runCLICommand(["generate-metadata", "--yes"], tempDir);
    expect(metaResult.code).toEqual(0);

    // After generate-metadata: must still only have KEY-based files, no NAME-based duplicates
    files = await readdir(backendDir);
    const tsFiles = files.filter(f => f.endsWith(".ts")).sort();

    // Critical assertion: should be ["a.ts", "b.ts"], NOT ["a.ts", "b.ts", "fetch_users.ts", "update_record.ts"]
    expect(tsFiles).toEqual(["a.ts", "b.ts"]);

    // No orphaned name-based files should exist
    expect(files).not.toContain("fetch_users.ts");
    expect(files).not.toContain("update_record.ts");
    expect(files).not.toContain("fetch_users.lock");
    expect(files).not.toContain("update_record.lock");

    // Push should be a no-op (no phantom changes)
    const pushResult = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
    expect(pushResult.code).toEqual(0);

    // Backend should still have exactly 2 runnables
    const getResp = await fetch(
      `${backend.baseUrl}/api/w/${backend.workspace}/apps/get/p/${appPath}`,
      { headers: { Authorization: `Bearer ${backend.token}` } },
    );
    const appAfterPush = await getResp.json() as any;
    const runnables = appAfterPush.value?.runnables ?? {};
    expect(Object.keys(runnables).length).toEqual(2);
  });
}, 180000);
