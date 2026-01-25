import { assertEquals, assert, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";
import * as path from "https://deno.land/std@0.224.0/path/mod.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";

// =============================================================================
// RAW APP SYNC TESTS
// Tests for raw app sync pull/push workflow
// =============================================================================

// Raw app file contents
const APP_TSX = `import React, { useState } from 'react'
import { backend } from './wmill'
import './index.css'

const App = () => {
    const [value, setValue] = useState(undefined as string | undefined)
    const [loading, setLoading] = useState(false)

    async function runA() {
        setLoading(true)
        try {
            setValue(await backend.a({ x: 42 }))
        } catch (e) {
            console.error()
        }
        setLoading(false)
    }

    return <div style={{ width: "100%" }}>
        <h1>hello world</h1>

        <button style={{ marginTop: "2px" }} onClick={runA}>Run 'a'</button>

        <div style={{ marginTop: "20px", width: '250px' }} className='myclass'>
            {loading ? 'Loading ...' : value ?? 'Click button to see value here'}
        </div>
    </div>;
};

export default App;
`;

const INDEX_CSS = `.myclass {
    border: 1px solid gray;
    padding: 2px;
}`;

const INDEX_TSX = `
import React from 'react'

import { createRoot } from 'react-dom/client'
import App from './App'

const root = createRoot(document.getElementById('root')!);
root.render(<App/>);
`;

const PACKAGE_JSON = `{
    "dependencies": {
        "react": "19.0.0",
        "react-dom": "19.0.0",
        "windmill-client": "^1"
    },
    "devDependencies": {
        "@types/react-dom": "^19.0.0",
        "@types/react": "^19.0.0"
    }
}`;

const INLINE_SCRIPT_A = `// import * as wmill from "windmill-client"

export async function main(x: string) {
  return x
}
`;

const INLINE_SCRIPT_A_LOCK = `{
  "dependencies": {}
}
//bun.lock
<empty>`;

// raw_app.yaml metadata file
const RAW_APP_YAML = `summary: Test Raw App
policy:
  execution_mode: publisher
  triggerables: {}
  triggerables_v2: {}
`;

async function fileExists(filePath: string): Promise<boolean> {
  try {
    await Deno.stat(filePath);
    return true;
  } catch {
    return false;
  }
}

async function readFileContent(filePath: string): Promise<string> {
  return await Deno.readTextFile(filePath);
}

/**
 * Create a raw app directory structure on disk
 * Uses .raw_app folder suffix with raw_app.yaml metadata
 */
async function createRawAppOnDisk(appDir: string): Promise<void> {
  await ensureDir(appDir);
  await ensureDir(path.join(appDir, "inline_scripts"));

  // Create raw_app.yaml metadata file
  await Deno.writeTextFile(path.join(appDir, "raw_app.yaml"), RAW_APP_YAML);

  // Create app source files
  await Deno.writeTextFile(path.join(appDir, "App.tsx"), APP_TSX);
  await Deno.writeTextFile(path.join(appDir, "index.css"), INDEX_CSS);
  await Deno.writeTextFile(path.join(appDir, "index.tsx"), INDEX_TSX);
  await Deno.writeTextFile(path.join(appDir, "package.json"), PACKAGE_JSON);

  // Create inline script in inline_scripts folder
  await Deno.writeTextFile(
    path.join(appDir, "inline_scripts", "a.inline_script.ts"),
    INLINE_SCRIPT_A
  );
  await Deno.writeTextFile(
    path.join(appDir, "inline_scripts", "a.inline_script.lock"),
    INLINE_SCRIPT_A_LOCK
  );
}

Deno.test({
  name: "Raw App: full sync workflow - push, pull, modify, push, clear, pull",
  ignore: shouldSkipOnCI(), // Raw app API requires EE features
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "raw_app_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Create folder structure
      const appDir = path.join(tempDir, "f", "test", "my_raw_app.raw_app");
      await ensureDir(path.join(tempDir, "f", "test"));
      await createRawAppOnDisk(appDir);

      // =========================================================================
      // STEP 1: Initial push - create raw app on backend
      // =========================================================================
      const pushResult1 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_test");

      assertEquals(pushResult1.code, 0, `Initial sync push should succeed: ${pushResult1.stderr}`);

      // =========================================================================
      // STEP 2: Clear disk and pull - verify raw app is pulled correctly
      // =========================================================================
      await Deno.remove(appDir, { recursive: true });
      assert(!(await fileExists(appDir)), "App directory should be deleted before pull");

      const pullResult1 = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_test");

      assertEquals(pullResult1.code, 0, `Sync pull should succeed: ${pullResult1.stderr}`);

      // Verify raw app directory structure was created
      assert(await fileExists(appDir), `Raw app directory should exist at ${appDir}`);

      // Verify files were pulled
      const appTsxPath = path.join(appDir, "App.tsx");
      const indexCssPath = path.join(appDir, "index.css");
      const indexTsxPath = path.join(appDir, "index.tsx");
      const packageJsonPath = path.join(appDir, "package.json");
      const inlineScriptPath = path.join(appDir, "inline_scripts", "a.inline_script.ts");

      assert(await fileExists(appTsxPath), "App.tsx should exist");
      assert(await fileExists(indexCssPath), "index.css should exist");
      assert(await fileExists(indexTsxPath), "index.tsx should exist");
      assert(await fileExists(packageJsonPath), "package.json should exist");
      assert(await fileExists(inlineScriptPath), "Inline script a.inline_script.ts should exist");

      // Verify file contents
      const appTsxContent = await readFileContent(appTsxPath);
      assertStringIncludes(appTsxContent, "hello world", "App.tsx should contain 'hello world'");
      assertStringIncludes(appTsxContent, "backend.a", "App.tsx should reference backend.a");

      const indexCssContent = await readFileContent(indexCssPath);
      assertStringIncludes(indexCssContent, ".myclass", "index.css should contain .myclass");

      const inlineScriptContent = await readFileContent(inlineScriptPath);
      assertStringIncludes(inlineScriptContent, "export async function main", "Inline script should have main function");

      // =========================================================================
      // STEP 3: Modify files locally
      // =========================================================================

      // Modify App.tsx - change the heading
      const modifiedAppTsx = appTsxContent.replace("hello world", "hello modified world");
      await Deno.writeTextFile(appTsxPath, modifiedAppTsx);

      // Modify index.css - change the border color
      const modifiedIndexCss = indexCssContent.replace("gray", "blue");
      await Deno.writeTextFile(indexCssPath, modifiedIndexCss);

      // Modify inline script - change the return value
      const modifiedInlineScript = inlineScriptContent.replace("return x", "return `modified: ${x}`");
      await Deno.writeTextFile(inlineScriptPath, modifiedInlineScript);

      // =========================================================================
      // STEP 4: Push changes
      // =========================================================================
      const pushResult2 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_test");

      assertEquals(pushResult2.code, 0, `Sync push should succeed: ${pushResult2.stderr}`);

      // =========================================================================
      // STEP 5: Clear disk (delete the app directory)
      // =========================================================================
      await Deno.remove(appDir, { recursive: true });
      assert(!(await fileExists(appDir)), "App directory should be deleted");

      // =========================================================================
      // STEP 6: Pull again and verify modifications persisted
      // =========================================================================
      const pullResult2 = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_test");

      assertEquals(pullResult2.code, 0, `Second sync pull should succeed: ${pullResult2.stderr}`);

      // Verify app directory exists again
      assert(await fileExists(appDir), "Raw app directory should exist after second pull");

      // Verify all files were pulled again
      assert(await fileExists(appTsxPath), "App.tsx should exist after second pull");
      assert(await fileExists(indexCssPath), "index.css should exist after second pull");
      assert(await fileExists(indexTsxPath), "index.tsx should exist after second pull");
      assert(await fileExists(packageJsonPath), "package.json should exist after second pull");
      assert(await fileExists(inlineScriptPath), "Inline script should exist after second pull");

      // Verify modifications were persisted
      const pulledAppTsx = await readFileContent(appTsxPath);
      assertStringIncludes(pulledAppTsx, "hello modified world", "Modifications to App.tsx should persist");

      const pulledIndexCss = await readFileContent(indexCssPath);
      assertStringIncludes(pulledIndexCss, "blue", "Modifications to index.css should persist");

      const pulledInlineScript = await readFileContent(inlineScriptPath);
      assertStringIncludes(pulledInlineScript, "modified:", "Modifications to inline script should persist");
    });
  }
});

Deno.test({
  name: "Raw App: add new file and push",
  ignore: shouldSkipOnCI(), // Raw app API requires EE features
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "raw_app_new_file_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Create initial raw app
      const appDir = path.join(tempDir, "f", "test", "new_file_app.raw_app");
      await ensureDir(path.join(tempDir, "f", "test"));
      await createRawAppOnDisk(appDir);

      // Initial push
      const pushResult1 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_new_file_test");

      assertEquals(pushResult1.code, 0, `Initial sync push should succeed: ${pushResult1.stderr}`);

      // Add a new file
      const newFilePath = path.join(appDir, "utils.ts");
      await Deno.writeTextFile(newFilePath, `export function formatValue(val: string): string {
  return \`Formatted: \${val}\`;
}
`);

      // Push changes
      const pushResult2 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_new_file_test");

      assertEquals(pushResult2.code, 0, `Sync push with new file should succeed: ${pushResult2.stderr}`);

      // Clear and pull again
      await Deno.remove(appDir, { recursive: true });

      const pullResult = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_new_file_test");

      assertEquals(pullResult.code, 0, `Sync pull should succeed: ${pullResult.stderr}`);

      // Verify new file was persisted
      assert(await fileExists(newFilePath), "New file utils.ts should exist after pull");
      const newFileContent = await readFileContent(newFilePath);
      assertStringIncludes(newFileContent, "formatValue", "New file content should persist");
    });
  }
});

Deno.test({
  name: "Raw App: delete file and push",
  ignore: shouldSkipOnCI(), // Raw app API requires EE features
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "raw_app_delete_file_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Create initial raw app
      const appDir = path.join(tempDir, "f", "test", "delete_file_app.raw_app");
      await ensureDir(path.join(tempDir, "f", "test"));
      await createRawAppOnDisk(appDir);

      // Initial push
      const pushResult1 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_delete_file_test");

      assertEquals(pushResult1.code, 0, `Initial sync push should succeed: ${pushResult1.stderr}`);

      const indexCssPath = path.join(appDir, "index.css");
      assert(await fileExists(indexCssPath), "index.css should exist after initial push");

      // Delete the CSS file
      await Deno.remove(indexCssPath);
      assert(!(await fileExists(indexCssPath)), "index.css should be deleted locally");

      // Push changes
      const pushResult2 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_delete_file_test");

      assertEquals(pushResult2.code, 0, `Sync push after delete should succeed: ${pushResult2.stderr}`);

      // Clear and pull again
      await Deno.remove(appDir, { recursive: true });

      const pullResult = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_delete_file_test");

      assertEquals(pullResult.code, 0, `Sync pull should succeed: ${pullResult.stderr}`);

      // Verify the deleted file is NOT pulled (it was deleted from backend)
      assert(!(await fileExists(indexCssPath)), "Deleted index.css should not exist after pull");

      // But other files should still exist
      const appTsxPath = path.join(appDir, "App.tsx");
      assert(await fileExists(appTsxPath), "App.tsx should still exist after pull");
    });
  }
});

Deno.test({
  name: "Raw App: dry-run push shows expected changes",
  ignore: shouldSkipOnCI(), // Raw app API requires EE features
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "raw_app_dry_run_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Create raw app
      const appDir = path.join(tempDir, "f", "test", "dry_run_app.raw_app");
      await ensureDir(path.join(tempDir, "f", "test"));
      await createRawAppOnDisk(appDir);

      // Dry-run push
      const dryRunResult = await backend.runCLICommand([
        'sync', 'push',
        '--dry-run',
        '--json-output'
      ], tempDir, "raw_app_dry_run_test");

      assertEquals(dryRunResult.code, 0, `Dry-run push should succeed: ${dryRunResult.stderr}`);

      // Parse JSON output
      const lines = dryRunResult.stdout.trim().split('\n');
      let jsonOutput = null;
      for (const line of lines) {
        try {
          jsonOutput = JSON.parse(line);
          break;
        } catch {
          continue;
        }
      }

      assert(jsonOutput !== null, "Should have JSON output");
      assert(Array.isArray(jsonOutput.changes), "Should have changes array");

      // Should include raw app in changes
      const changePaths = jsonOutput.changes.map((c: any) => c.path);
      const hasRawApp = changePaths.some((p: string) => p.includes("dry_run_app"));
      assert(hasRawApp, `Dry-run should show raw app. Found: ${changePaths.join(', ')}`);
    });
  }
});
