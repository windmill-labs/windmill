import { expect, test } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import * as path from "node:path";
import { writeFile, readFile, readdir, stat, rm, mkdir } from "node:fs/promises";
import { getFolderSuffix, getMetadataFileName } from "../src/utils/resource_folders.ts";

// =============================================================================
// APP INLINE SCRIPT DELETION TESTS
// Regression tests for: deleting inline script files within .app/ folders
// during sync push should re-push the app, not crash with TypeError.
// =============================================================================

async function fileExists(filePath: string): Promise<boolean> {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

test("App: delete inline script file and push does not crash", async () => {
  await withTestBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "app_inline_delete_test",
      token: backend.token,
    };
    await addWorkspace(testWorkspace, {
      force: true,
      configDir: backend.testConfigDir,
    });

    await writeFile(
      `${tempDir}/wmill.yaml`,
      `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []`,
      "utf-8"
    );

    // Create an app with an inline script via the API
    const appPath = "f/test/inline_delete_app";
    const inlineContent = `export async function main() {\n  return "hello";\n}`;

    // Create the folder first
    await backend.apiRequest!(
      `/api/w/${backend.workspace}/folders/create`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "test" }),
      }
    ).then((r) => r.text());

    await backend.createAppWithInlineScript!(appPath, inlineContent, "bun");

    // =========================================================================
    // STEP 1: Pull — get the app folder with inline script files
    // =========================================================================
    const pullResult1 = await backend.runCLICommand(
      ["sync", "pull", "--yes"],
      tempDir,
      "app_inline_delete_test"
    );
    expect(pullResult1.code).toEqual(0);

    // Find the app folder and its inline script files
    const appSuffix = getFolderSuffix("app");
    const appDir = path.join(tempDir, appPath + appSuffix);
    expect(await fileExists(appDir)).toBeTruthy();

    // List files in the app folder to find inline script files
    const appFiles = await readdir(appDir);
    const inlineScriptFiles = appFiles.filter(
      (f) => f.endsWith(".ts") || f.endsWith(".js")
    );
    expect(inlineScriptFiles.length).toBeGreaterThan(0);

    const inlineScriptPath = path.join(appDir, inlineScriptFiles[0]);
    expect(await fileExists(inlineScriptPath)).toBeTruthy();

    // Read the current app.yaml
    const metadataFile = getMetadataFileName("app", "yaml");
    const appYamlPath = path.join(appDir, metadataFile);
    const appYaml = await readFile(appYamlPath, "utf-8");

    // =========================================================================
    // STEP 2: Remove the inline script from app.yaml and delete the .ts file
    // =========================================================================
    // Replace the inline script with a static text component (no inline scripts)
    const updatedAppYaml = `summary: Test app with inline script
value:
  type: app
  grid:
    - id: text1
      data:
        type: textcomponent
        componentInput:
          type: static
          value: hello world
  hiddenInlineScripts: []
  css: {}
  norefreshbar: false
policy:
  on_behalf_of: null
  on_behalf_of_email: null
  triggerables: {}
  execution_mode: viewer
`;
    await writeFile(appYamlPath, updatedAppYaml, "utf-8");

    // Delete the inline script file
    await rm(inlineScriptPath);
    expect(await fileExists(inlineScriptPath)).toBeFalsy();

    // Also delete any lock files for the inline script
    for (const f of appFiles) {
      if (f.endsWith(".lock")) {
        await rm(path.join(appDir, f));
      }
    }

    // =========================================================================
    // STEP 3: Push — should succeed, NOT crash with TypeError
    // =========================================================================
    const pushResult = await backend.runCLICommand(
      ["sync", "push", "--yes"],
      tempDir,
      "app_inline_delete_test"
    );

    // The critical assertion: push should not crash
    expect(pushResult.code).toEqual(0);

    // =========================================================================
    // STEP 4: Verify by pulling again — inline script should be gone
    // =========================================================================
    await rm(appDir, { recursive: true });

    const pullResult2 = await backend.runCLICommand(
      ["sync", "pull", "--yes"],
      tempDir,
      "app_inline_delete_test"
    );
    expect(pullResult2.code).toEqual(0);

    // App should still exist
    expect(await fileExists(appDir)).toBeTruthy();

    // But no inline script files should be present
    const finalFiles = await readdir(appDir);
    const finalInlineScripts = finalFiles.filter(
      (f) =>
        (f.endsWith(".ts") || f.endsWith(".js")) &&
        f.includes("inline_script")
    );
    expect(finalInlineScripts.length).toEqual(0);
  });
});
