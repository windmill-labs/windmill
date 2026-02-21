import { expect, test } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import * as path from "@std/path";
import { writeFile, readFile, stat, rm, mkdir } from "node:fs/promises";

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
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

async function readFileContent(filePath: string): Promise<string> {
  return await readFile(filePath, "utf-8");
}

/**
 * Create a raw app directory structure on disk
 * Uses .raw_app folder suffix with raw_app.yaml metadata
 */
async function createRawAppOnDisk(appDir: string): Promise<void> {
  await mkdir(appDir, { recursive: true });
  await mkdir(path.join(appDir, "inline_scripts"), { recursive: true });

  // Create raw_app.yaml metadata file
  await writeFile(path.join(appDir, "raw_app.yaml"), RAW_APP_YAML, "utf-8");

  // Create app source files
  await writeFile(path.join(appDir, "App.tsx"), APP_TSX, "utf-8");
  await writeFile(path.join(appDir, "index.css"), INDEX_CSS, "utf-8");
  await writeFile(path.join(appDir, "index.tsx"), INDEX_TSX, "utf-8");
  await writeFile(path.join(appDir, "package.json"), PACKAGE_JSON, "utf-8");

  // Create inline script in inline_scripts folder
  await writeFile(
    path.join(appDir, "inline_scripts", "a.inline_script.ts"),
    INLINE_SCRIPT_A,
    "utf-8"
  );
  await writeFile(
    path.join(appDir, "inline_scripts", "a.inline_script.lock"),
    INLINE_SCRIPT_A_LOCK,
    "utf-8"
  );
}

test("Raw App: full sync workflow - push, pull, modify, push, clear, pull", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");

      // Create folder structure
      const appDir = path.join(tempDir, "f", "test", "my_raw_app.raw_app");
      await mkdir(path.join(tempDir, "f", "test"), { recursive: true });
      await createRawAppOnDisk(appDir);

      // =========================================================================
      // STEP 1: Initial push - create raw app on backend
      // =========================================================================
      const pushResult1 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_test");

      expect(pushResult1.code).toEqual(0);

      // =========================================================================
      // STEP 2: Clear disk and pull - verify raw app is pulled correctly
      // =========================================================================
      await rm(appDir, { recursive: true });
      expect(!(await fileExists(appDir))).toBeTruthy();

      const pullResult1 = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_test");

      expect(pullResult1.code).toEqual(0);

      // Verify raw app directory structure was created
      expect(await fileExists(appDir)).toBeTruthy();

      // Verify files were pulled
      const appTsxPath = path.join(appDir, "App.tsx");
      const indexCssPath = path.join(appDir, "index.css");
      const indexTsxPath = path.join(appDir, "index.tsx");
      const packageJsonPath = path.join(appDir, "package.json");
      const inlineScriptPath = path.join(appDir, "inline_scripts", "a.inline_script.ts");

      expect(await fileExists(appTsxPath)).toBeTruthy();
      expect(await fileExists(indexCssPath)).toBeTruthy();
      expect(await fileExists(indexTsxPath)).toBeTruthy();
      expect(await fileExists(packageJsonPath)).toBeTruthy();
      expect(await fileExists(inlineScriptPath)).toBeTruthy();

      // Verify file contents
      const appTsxContent = await readFileContent(appTsxPath);
      expect(appTsxContent).toContain("hello world");
      expect(appTsxContent).toContain("backend.a");

      const indexCssContent = await readFileContent(indexCssPath);
      expect(indexCssContent).toContain(".myclass");

      const inlineScriptContent = await readFileContent(inlineScriptPath);
      expect(inlineScriptContent).toContain("export async function main");

      // =========================================================================
      // STEP 3: Modify files locally
      // =========================================================================

      // Modify App.tsx - change the heading
      const modifiedAppTsx = appTsxContent.replace("hello world", "hello modified world");
      await writeFile(appTsxPath, modifiedAppTsx, "utf-8");

      // Modify index.css - change the border color
      const modifiedIndexCss = indexCssContent.replace("gray", "blue");
      await writeFile(indexCssPath, modifiedIndexCss, "utf-8");

      // Modify inline script - change the return value
      const modifiedInlineScript = inlineScriptContent.replace("return x", "return `modified: ${x}`");
      await writeFile(inlineScriptPath, modifiedInlineScript, "utf-8");

      // =========================================================================
      // STEP 4: Push changes
      // =========================================================================
      const pushResult2 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_test");

      expect(pushResult2.code).toEqual(0);

      // =========================================================================
      // STEP 5: Clear disk (delete the app directory)
      // =========================================================================
      await rm(appDir, { recursive: true });
      expect(!(await fileExists(appDir))).toBeTruthy();

      // =========================================================================
      // STEP 6: Pull again and verify modifications persisted
      // =========================================================================
      const pullResult2 = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_test");

      expect(pullResult2.code).toEqual(0);

      // Verify app directory exists again
      expect(await fileExists(appDir)).toBeTruthy();

      // Verify all files were pulled again
      expect(await fileExists(appTsxPath)).toBeTruthy();
      expect(await fileExists(indexCssPath)).toBeTruthy();
      expect(await fileExists(indexTsxPath)).toBeTruthy();
      expect(await fileExists(packageJsonPath)).toBeTruthy();
      expect(await fileExists(inlineScriptPath)).toBeTruthy();

      // Verify modifications were persisted
      const pulledAppTsx = await readFileContent(appTsxPath);
      expect(pulledAppTsx).toContain("hello modified world");

      const pulledIndexCss = await readFileContent(indexCssPath);
      expect(pulledIndexCss).toContain("blue");

      const pulledInlineScript = await readFileContent(inlineScriptPath);
      expect(pulledInlineScript).toContain("modified:");
    });
});

test("Raw App: add new file and push", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");

      // Create initial raw app
      const appDir = path.join(tempDir, "f", "test", "new_file_app.raw_app");
      await mkdir(path.join(tempDir, "f", "test"), { recursive: true });
      await createRawAppOnDisk(appDir);

      // Initial push
      const pushResult1 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_new_file_test");

      expect(pushResult1.code).toEqual(0);

      // Add a new file
      const newFilePath = path.join(appDir, "utils.ts");
      await writeFile(newFilePath, `export function formatValue(val: string): string {
  return \`Formatted: \${val}\`;
}
`, "utf-8");

      // Push changes
      const pushResult2 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_new_file_test");

      expect(pushResult2.code).toEqual(0);

      // Clear and pull again
      await rm(appDir, { recursive: true });

      const pullResult = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_new_file_test");

      expect(pullResult.code).toEqual(0);

      // Verify new file was persisted
      expect(await fileExists(newFilePath)).toBeTruthy();
      const newFileContent = await readFileContent(newFilePath);
      expect(newFileContent).toContain("formatValue");
    });
});

test("Raw App: delete file and push", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");

      // Create initial raw app
      const appDir = path.join(tempDir, "f", "test", "delete_file_app.raw_app");
      await mkdir(path.join(tempDir, "f", "test"), { recursive: true });
      await createRawAppOnDisk(appDir);

      // Initial push
      const pushResult1 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_delete_file_test");

      expect(pushResult1.code).toEqual(0);

      const indexCssPath = path.join(appDir, "index.css");
      const appTsxPath = path.join(appDir, "App.tsx");
      expect(await fileExists(indexCssPath)).toBeTruthy();

      // First, update App.tsx to remove the CSS import (otherwise bundle will fail)
      const appTsxContent = await readFileContent(appTsxPath);
      const updatedAppTsx = appTsxContent.replace("import './index.css'\n", "");
      await writeFile(appTsxPath, updatedAppTsx, "utf-8");

      // Delete the CSS file
      await rm(indexCssPath);
      expect(!(await fileExists(indexCssPath))).toBeTruthy();

      // Push changes
      const pushResult2 = await backend.runCLICommand([
        'sync', 'push',
        '--yes'
      ], tempDir, "raw_app_delete_file_test");

      expect(pushResult2.code).toEqual(0);

      // Clear and pull again
      await rm(appDir, { recursive: true });

      const pullResult = await backend.runCLICommand([
        'sync', 'pull',
        '--yes'
      ], tempDir, "raw_app_delete_file_test");

      expect(pullResult.code).toEqual(0);

      // Verify the deleted file is NOT pulled (it was deleted from backend)
      expect(!(await fileExists(indexCssPath))).toBeTruthy();

      // But other files should still exist
      expect(await fileExists(appTsxPath)).toBeTruthy();
    });
});

test("Raw App: dry-run push shows expected changes", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");

      // Create raw app
      const appDir = path.join(tempDir, "f", "test", "dry_run_app.raw_app");
      await mkdir(path.join(tempDir, "f", "test"), { recursive: true });
      await createRawAppOnDisk(appDir);

      // Dry-run push
      const dryRunResult = await backend.runCLICommand([
        'sync', 'push',
        '--dry-run',
        '--json-output'
      ], tempDir, "raw_app_dry_run_test");

      expect(dryRunResult.code).toEqual(0);

      // Parse JSON output (may be pretty-printed across multiple lines)
      let jsonOutput = null;
      try {
        // Try parsing the entire stdout as JSON
        jsonOutput = JSON.parse(dryRunResult.stdout.trim());
      } catch {
        // If that fails, try to find JSON object in the output
        const jsonMatch = dryRunResult.stdout.match(/\{[\s\S]*\}/);
        if (jsonMatch) {
          try {
            jsonOutput = JSON.parse(jsonMatch[0]);
          } catch {
            // Ignore parse errors
          }
        }
      }

      expect(jsonOutput !== null).toBeTruthy();
      expect(Array.isArray(jsonOutput.changes)).toBeTruthy();

      // Should include raw app in changes
      const changePaths = jsonOutput.changes.map((c: any) => c.path);
      const hasRawApp = changePaths.some((p: string) => p.includes("dry_run_app"));
      expect(hasRawApp).toBeTruthy();
    });
});
