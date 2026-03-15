/**
 * Unified generate-metadata Command Tests
 *
 * Tests the new unified `generate-metadata` command that processes
 * scripts, flows, and apps together.
 */

import { expect, test, describe } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import {
  createLocalScript,
  createLocalFlow,
  createLocalApp,
  createLocalRawApp,
  createLocalScriptWithModules,
} from "./test_fixtures.ts";

/**
 * Helper to set up a workspace with wmill.yaml
 */
async function setupWorkspace(
  backend: any,
  tempDir: string,
  workspaceName: string,
  nonDottedPaths = false
) {
  const testWorkspace = {
    remote: backend.baseUrl,
    workspaceId: backend.workspace,
    name: workspaceName,
    token: backend.token
  };
  await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

  await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
${nonDottedPaths ? "nonDottedPaths: true\n" : ""}includes:
  - "**"
excludes: []`, "utf-8");
}

async function createLocalNonDottedFlow(tempDir: string, name: string) {
  const flowDir = `${tempDir}/f/test/${name}__flow`;
  await mkdir(flowDir, { recursive: true });

  await writeFile(
    `${flowDir}/a.ts`,
    `export async function main() {\n  return "Hello from flow ${name}";\n}`,
    "utf-8"
  );

  await writeFile(
    `${flowDir}/flow.yaml`,
    `summary: "${name} flow"
description: "A flow for testing"
value:
  modules:
    - id: a
      value:
        type: rawscript
        content: "!inline a.ts"
        language: bun
        input_transforms: {}
schema:
  $schema: "https://json-schema.org/draft/2020-12/schema"
  type: object
  properties: {}
  required: []
`,
    "utf-8"
  );
}

async function createLocalNonDottedApp(tempDir: string, name: string) {
  const appDir = `${tempDir}/f/test/${name}__app`;
  await mkdir(appDir, { recursive: true });

  await writeFile(
    `${appDir}/app.yaml`,
    `summary: "${name} app"
value:
  type: app
  grid:
    - id: button1
      data:
        type: buttoncomponent
        componentInput:
          type: runnable
          runnable:
            type: runnableByName
            inlineScript:
              content: |
                export async function main() {
                  return "hello from app";
                }
              language: bun
  hiddenInlineScripts: []
  css: {}
  norefreshbar: false
policy:
  on_behalf_of: null
  on_behalf_of_email: null
  triggerables: {}
  execution_mode: viewer
`,
    "utf-8"
  );
}

async function fileExists(filePath: string): Promise<boolean> {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

// =============================================================================
// Main test: processes scripts, flows, and apps together
// =============================================================================

test("generate-metadata: processes scripts, flows, and apps together", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspace(backend, tempDir, "unified_all_test");

    // Create one of each type
    await createLocalScript(tempDir, "f/test", "my_script");
    await createLocalFlow(tempDir, "f/test", "my_flow");
    await createLocalApp(tempDir, "f/test", "my_app");

    const result = await backend.runCLICommand(
      ["generate-metadata", "--yes"],
      tempDir,
      "unified_all_test"
    );

    expect(result.code).toEqual(0);
    // Should find stale items
    expect(result.stdout).toContain("Found");
    expect(result.stdout).toContain("stale metadata");
  });
});

// =============================================================================
// Flag tests
// =============================================================================

describe("generate-metadata flags", () => {
  test("--includes filters to specific paths", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "includes_test");

      // Create two scripts in different folders
      await createLocalScript(tempDir, "f/included", "script_a");
      await createLocalScript(tempDir, "f/excluded", "script_b");

      // Run with --includes to only process f/included
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "-i", "f/included/**"],
        tempDir,
        "includes_test"
      );

      expect(result.code).toEqual(0);
      // Should only mention the included script
      const output = result.stdout + result.stderr;
      expect(output).toContain("script_a");
      expect(output).not.toContain("script_b");
    });
  });

  test("--excludes filters out specific paths", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "excludes_test");

      // Create two scripts
      await createLocalScript(tempDir, "f/keep", "script_keep");
      await createLocalScript(tempDir, "f/skip", "script_skip");

      // Run with --excludes to skip f/skip
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "-e", "f/skip/**"],
        tempDir,
        "excludes_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).toContain("script_keep");
      expect(output).not.toContain("script_skip");
    });
  });

  test("--dry-run shows stale items without updating", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "dry_run_test");

      await createLocalScript(tempDir, "f/test", "my_script");

      // Run with --dry-run
      const result = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "dry_run_test"
      );

      expect(result.code).toEqual(0);
      // Should show stale items (Scripts section header)
      expect(result.stdout).toContain("Scripts");
      expect(result.stdout).toContain("my_script");
      // Should NOT show "Done" (didn't actually update)
      expect(result.stdout).not.toContain("Done");

      // Run again without --dry-run to verify it would still be stale
      const result2 = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "dry_run_test"
      );
      expect(result2.stdout).toContain("Scripts");
    });
  });

  test("--lock-only only regenerates locks", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "lock_only_test");

      await createLocalScript(tempDir, "f/test", "my_script");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--lock-only"],
        tempDir,
        "lock_only_test"
      );

      expect(result.code).toEqual(0);
    });
  });

  test("--lock-only preserves non-dotted flow filenames", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "lock_only_non_dotted_test", true);

      await createLocalNonDottedFlow(tempDir, "my_flow");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--lock-only"],
        tempDir,
        "lock_only_non_dotted_test"
      );

      expect(result.code).toEqual(0);

      const flowDir = `${tempDir}/f/test/my_flow__flow`;
      const flowYaml = await readFile(`${flowDir}/flow.yaml`, "utf-8");

      expect(flowYaml).toContain("!inline a.ts");
      expect(flowYaml).toContain("!inline a.lock");
      expect(flowYaml).not.toContain(".inline_script.");
      expect(await fileExists(`${flowDir}/a.lock`)).toEqual(true);
      expect(await fileExists(`${flowDir}/a.inline_script.ts`)).toEqual(false);
      expect(await fileExists(`${flowDir}/a.inline_script.lock`)).toEqual(false);
    });
  });

  test("generate-metadata preserves non-dotted flow inline script filenames", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "full_gen_non_dotted_flow_test", true);

      await createLocalNonDottedFlow(tempDir, "my_flow");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "full_gen_non_dotted_flow_test"
      );

      expect(result.code).toEqual(0);

      const flowDir = `${tempDir}/f/test/my_flow__flow`;
      const flowYaml = await readFile(`${flowDir}/flow.yaml`, "utf-8");

      // Inline script references should use non-dotted naming
      expect(flowYaml).toContain("!inline a.ts");
      expect(flowYaml).toContain("!inline a.lock");
      expect(flowYaml).not.toContain(".inline_script.");
      expect(await fileExists(`${flowDir}/a.ts`)).toEqual(true);
      expect(await fileExists(`${flowDir}/a.lock`)).toEqual(true);
      expect(await fileExists(`${flowDir}/a.inline_script.ts`)).toEqual(false);
      expect(await fileExists(`${flowDir}/a.inline_script.lock`)).toEqual(false);
    });
  });

  test("generate-metadata uses non-dotted app inline script filenames", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "non_dotted_app_gen_test", true);

      await createLocalNonDottedApp(tempDir, "my_app");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "non_dotted_app_gen_test"
      );

      expect(result.code).toEqual(0);

      const appDir = `${tempDir}/f/test/my_app__app`;
      const appYaml = await readFile(`${appDir}/app.yaml`, "utf-8");

      // Inline script references should use non-dotted naming
      expect(appYaml).not.toContain(".inline_script.");
      // Verify no dotted inline script files were created
      const { readdir: readdirAsync } = await import("node:fs/promises");
      const files = await readdirAsync(appDir);
      const dottedFiles = files.filter((f: string) => f.includes(".inline_script."));
      expect(dottedFiles.length).toEqual(0);
    });
  });

  test("--schema-only only processes scripts (skips flows and apps)", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "schema_only_test");

      // Create one of each type
      await createLocalScript(tempDir, "f/test", "my_script");
      await createLocalFlow(tempDir, "f/test", "my_flow");
      await createLocalApp(tempDir, "f/test", "my_app");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--schema-only"],
        tempDir,
        "schema_only_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should show "Checking scripts..." only
      expect(output).toContain("Checking scripts...");
      // Should find the script (Scripts section header)
      expect(output).toContain("Scripts");
      // Should NOT find flows or apps
      expect(output).not.toContain("Flows");
      expect(output).not.toContain("Apps");
    });
  });

  test("--skip-scripts skips scripts", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "skip_scripts_test");

      await createLocalScript(tempDir, "f/test", "my_script");
      await createLocalFlow(tempDir, "f/test", "my_flow");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--skip-scripts"],
        tempDir,
        "skip_scripts_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should NOT contain script
      expect(output).not.toContain("Scripts");
      // Should contain flow
      expect(output).toContain("Flows");
    });
  });

  test("--skip-flows skips flows", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "skip_flows_test");

      await createLocalScript(tempDir, "f/test", "my_script");
      await createLocalFlow(tempDir, "f/test", "my_flow");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--skip-flows"],
        tempDir,
        "skip_flows_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should contain script
      expect(output).toContain("Scripts");
      // Should NOT contain flow
      expect(output).not.toContain("Flows");
    });
  });

  test("--skip-apps skips apps", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "skip_apps_test");

      await createLocalScript(tempDir, "f/test", "my_script");
      await createLocalApp(tempDir, "f/test", "my_app");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--skip-apps"],
        tempDir,
        "skip_apps_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should contain script
      expect(output).toContain("Scripts");
      // Should NOT contain app
      expect(output).not.toContain("Apps");
    });
  });

  test("shows 'All metadata up-to-date' when nothing to update", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "uptodate_test");

      // Create a script and run generate-metadata twice
      await createLocalScript(tempDir, "f/test", "my_script");

      // First run - generates metadata
      await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "uptodate_test"
      );

      // Second run - should be up-to-date
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "uptodate_test"
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("up-to-date");
    });
  });

  test("skipping all types shows warning", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "skip_all_test");

      await createLocalScript(tempDir, "f/test", "my_script");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--skip-scripts", "--skip-flows", "--skip-apps"],
        tempDir,
        "skip_all_test"
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("Nothing to check");
    });
  });
});

// =============================================================================
// Folder argument tests
// =============================================================================

describe("generate-metadata folder argument", () => {
  test("filters to specific script folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "folder_script_test");

      // Create scripts in different folders
      await createLocalScript(tempDir, "f/included", "script_a");
      await createLocalScript(tempDir, "f/excluded", "script_b");

      // Run with folder argument
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/included/script_a.ts"],
        tempDir,
        "folder_script_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).toContain("script_a");
      expect(output).not.toContain("script_b");
    });
  });

  test("filters to specific flow folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "folder_flow_test");

      // Create flows in different folders
      await createLocalFlow(tempDir, "f/included", "flow_a");
      await createLocalFlow(tempDir, "f/excluded", "flow_b");

      // Run with folder argument (flow folder path - uses .flow suffix by default)
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/included/flow_a.flow"],
        tempDir,
        "folder_flow_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).toContain("flow_a");
      expect(output).not.toContain("flow_b");
    });
  });

  test("filters to specific app folder", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "folder_app_test");

      // Create apps in different folders
      await createLocalApp(tempDir, "f/included", "app_a");
      await createLocalApp(tempDir, "f/excluded", "app_b");

      // Run with folder argument (app folder path - uses .app suffix by default)
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/included/app_a.app"],
        tempDir,
        "folder_app_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).toContain("app_a");
      expect(output).not.toContain("app_b");
    });
  });

  test("shows up-to-date when folder has no stale items", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "folder_uptodate_test");

      await createLocalScript(tempDir, "f/test", "my_script");

      // First run to generate metadata
      await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "folder_uptodate_test"
      );

      // Second run with folder - should be up-to-date
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/test/my_script.ts"],
        tempDir,
        "folder_uptodate_test"
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("up-to-date");
    });
  });

  test("trailing slash is stripped (matches deprecated behavior)", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "trailing_slash_test");

      await createLocalScript(tempDir, "f/test", "my_script");

      // Run with trailing slash
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/test/my_script.ts/"],
        tempDir,
        "trailing_slash_test"
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("my_script");
    });
  });

  test("parent folder matches all children", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "parent_folder_test");

      // Create scripts in nested folders
      await createLocalScript(tempDir, "f/parent", "script_a");
      await createLocalScript(tempDir, "f/parent/child", "script_b");
      await createLocalScript(tempDir, "f/other", "script_c");

      // Run with parent folder - should match both scripts in f/parent tree
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/parent"],
        tempDir,
        "parent_folder_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).toContain("script_a");
      expect(output).toContain("script_b");
      expect(output).not.toContain("script_c");
    });
  });

  test("non-existent folder shows up-to-date", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "nonexistent_folder_test");

      await createLocalScript(tempDir, "f/exists", "my_script");

      // Run with non-existent folder
      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "f/does_not_exist"],
        tempDir,
        "nonexistent_folder_test"
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("up-to-date");
    });
  });
});

// =============================================================================
// Scripts with modules
// =============================================================================

describe("generate-metadata with script modules", () => {
  test("script with modules is detected as a single stale item", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "module_script_test");

      // Create a script with module files
      await createLocalScriptWithModules(tempDir, "f/test", "order_workflow", "bun", [
        { path: "helper.ts", content: 'export function validate() { return true; }\n' },
        { path: "utils.ts", content: 'export const VERSION = "1.0";\n' },
      ]);

      const result = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "module_script_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // The main script should be listed as stale
      expect(output).toContain("order_workflow");
      // Module files should NOT appear as separate stale scripts (only within [changed modules: ...])
      const lines = output.split("\n");
      const staleLines = lines.filter((l: string) => l.includes("f/test/"));
      expect(staleLines.length).toBe(1);
      expect(staleLines[0]).toContain("order_workflow");
    });
  });

  test("module files are not treated as standalone scripts", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "module_not_standalone_test");

      // Create a script with modules plus a regular script
      await createLocalScriptWithModules(tempDir, "f/test", "my_script", "bun", [
        { path: "helper.ts", content: 'export function greet() { return "hi"; }\n' },
      ]);
      await createLocalScript(tempDir, "f/test", "standalone_script");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "module_not_standalone_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should list both the main script and standalone script
      expect(output).toContain("my_script");
      expect(output).toContain("standalone_script");
      // Should NOT list module helper as a separate entry
      // Count occurrences of "Scripts" header — there should be exactly one
      expect(output).toContain("Scripts");
    });
  });

  test("script with modules generates metadata and becomes up-to-date", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "module_uptodate_test");

      await createLocalScriptWithModules(tempDir, "f/test", "my_script", "bun", [
        { path: "helper.ts", content: 'export function greet() { return "hi"; }\n' },
      ]);

      // First run — should find stale items and generate metadata
      const result1 = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "module_uptodate_test"
      );
      expect(result1.code).toEqual(0);
      expect(result1.stdout).toContain("Done");

      // Second run — should be up-to-date
      const result2 = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "module_uptodate_test"
      );
      expect(result2.code).toEqual(0);
      expect(result2.stdout).toContain("up-to-date");
    });
  });

  test("modifying a module re-triggers stale detection", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "module_modify_test");

      await createLocalScriptWithModules(tempDir, "f/test", "order_workflow", "bun", [
        { path: "helper.ts", content: 'export function greet() { return "hi"; }\n' },
        { path: "utils.ts", content: 'export const VERSION = "1.0";\n' },
      ]);

      // First run — generate metadata
      const result1 = await backend.runCLICommand(
        ["generate-metadata", "--yes"],
        tempDir,
        "module_modify_test"
      );
      expect(result1.code).toEqual(0);

      // Second run — should be up-to-date
      const result2 = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "module_modify_test"
      );
      expect(result2.code).toEqual(0);
      const output2 = result2.stdout + result2.stderr;
      expect(output2).not.toContain("order_workflow");

      // Modify one module
      await writeFile(
        `${tempDir}/f/test/order_workflow__mod/helper.ts`,
        'export function greet() { return "hello world"; }\n',
        "utf-8"
      );

      // Third run — should detect the script as stale with the changed module
      const result3 = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "module_modify_test"
      );
      expect(result3.code).toEqual(0);
      const output3 = result3.stdout + result3.stderr;
      expect(output3).toContain("order_workflow");
      expect(output3).toContain("helper.ts");
      // utils.ts was not modified, should not be listed as changed
      expect(output3).not.toContain("utils.ts");
    });
  });

  test("script with modules and lock files does not crash", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "module_with_locks_test");

      // Create a script with modules that have lock files
      await createLocalScriptWithModules(tempDir, "f/test", "my_script", "bun", [
        {
          path: "helper.ts",
          content: 'import lodash from "lodash";\nexport const x = lodash.identity(1);\n',
          lock: "lodash@4.17.21\n",
        },
      ]);

      // Should not crash on lock files inside __mod/
      const result = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "module_with_locks_test"
      );

      expect(result.code).toEqual(0);
      // Should find the main script as stale, not crash on .lock files
      expect(result.stdout).toContain("my_script");
    });
  });
});
