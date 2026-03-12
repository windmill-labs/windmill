/**
 * Unified generate-metadata Command Tests
 *
 * Tests the new unified `generate-metadata` command that processes
 * scripts, flows, and apps together.
 */

import { expect, test, describe } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { writeFile, mkdir, readFile } from "node:fs/promises";

/**
 * Helper to set up a workspace with wmill.yaml
 */
async function setupWorkspace(backend: any, tempDir: string, workspaceName: string) {
  const testWorkspace = {
    remote: backend.baseUrl,
    workspaceId: backend.workspace,
    name: workspaceName,
    token: backend.token
  };
  await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

  await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");
}

/**
 * Helper to create a test script
 */
async function createScript(tempDir: string, path: string, name: string = "test") {
  await mkdir(`${tempDir}/${path}`, { recursive: true });
  await writeFile(`${tempDir}/${path}/${name}.ts`, `export async function main(x: string) {
  return "hello " + x;
}
`, "utf-8");
  await writeFile(`${tempDir}/${path}/${name}.script.yaml`, `summary: "${name} script"
schema:
  type: object
  properties: {}
lock: ""
`, "utf-8");
}

/**
 * Helper to create a test flow
 */
async function createFlow(tempDir: string, path: string, name: string = "test") {
  await mkdir(`${tempDir}/${path}/${name}.flow`, { recursive: true });
  await writeFile(`${tempDir}/${path}/${name}.flow/flow.yaml`, `summary: "${name} flow"
description: ""
value:
  modules:
    - id: a
      value:
        type: rawscript
        content: |
          export async function main() {
            return "hello from flow";
          }
        language: bun
`, "utf-8");
}

/**
 * Helper to create a test app
 */
async function createApp(tempDir: string, path: string, name: string = "test") {
  await mkdir(`${tempDir}/${path}/${name}.app`, { recursive: true });
  await writeFile(`${tempDir}/${path}/${name}.app/app.yaml`, `summary: "${name} app"
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
`, "utf-8");
}

// =============================================================================
// Main test: processes scripts, flows, and apps together
// =============================================================================

test("generate-metadata: processes scripts, flows, and apps together", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspace(backend, tempDir, "unified_all_test");

    // Create one of each type
    await createScript(tempDir, "f/test", "my_script");
    await createFlow(tempDir, "f/test", "my_flow");
    await createApp(tempDir, "f/test", "my_app");

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
      await createScript(tempDir, "f/included", "script_a");
      await createScript(tempDir, "f/excluded", "script_b");

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
      await createScript(tempDir, "f/keep", "script_keep");
      await createScript(tempDir, "f/skip", "script_skip");

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

      await createScript(tempDir, "f/test", "my_script");

      // Run with --dry-run
      const result = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "dry_run_test"
      );

      expect(result.code).toEqual(0);
      // Should show stale items
      expect(result.stdout).toContain("[S]");
      expect(result.stdout).toContain("my_script");
      // Should NOT show "Done" (didn't actually update)
      expect(result.stdout).not.toContain("Done");

      // Run again without --dry-run to verify it would still be stale
      const result2 = await backend.runCLICommand(
        ["generate-metadata", "--dry-run"],
        tempDir,
        "dry_run_test"
      );
      expect(result2.stdout).toContain("[S]");
    });
  });

  test("--lock-only only regenerates locks", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "lock_only_test");

      await createScript(tempDir, "f/test", "my_script");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--lock-only"],
        tempDir,
        "lock_only_test"
      );

      expect(result.code).toEqual(0);
    });
  });

  test("--schema-only only processes scripts (skips flows and apps)", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "schema_only_test");

      // Create one of each type
      await createScript(tempDir, "f/test", "my_script");
      await createFlow(tempDir, "f/test", "my_flow");
      await createApp(tempDir, "f/test", "my_app");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--schema-only"],
        tempDir,
        "schema_only_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should show "Checking scripts..." only
      expect(output).toContain("Checking scripts...");
      // Should find the script
      expect(output).toContain("[S]");
      // Should NOT find flows or apps
      expect(output).not.toContain("[F]");
      expect(output).not.toContain("[A]");
    });
  });

  test("--skip-scripts skips scripts", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "skip_scripts_test");

      await createScript(tempDir, "f/test", "my_script");
      await createFlow(tempDir, "f/test", "my_flow");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--skip-scripts"],
        tempDir,
        "skip_scripts_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should NOT contain script
      expect(output).not.toContain("[S]");
      // Should contain flow
      expect(output).toContain("[F]");
    });
  });

  test("--skip-flows skips flows", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "skip_flows_test");

      await createScript(tempDir, "f/test", "my_script");
      await createFlow(tempDir, "f/test", "my_flow");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--skip-flows"],
        tempDir,
        "skip_flows_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should contain script
      expect(output).toContain("[S]");
      // Should NOT contain flow
      expect(output).not.toContain("[F]");
    });
  });

  test("--skip-apps skips apps", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "skip_apps_test");

      await createScript(tempDir, "f/test", "my_script");
      await createApp(tempDir, "f/test", "my_app");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--skip-apps"],
        tempDir,
        "skip_apps_test"
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      // Should contain script
      expect(output).toContain("[S]");
      // Should NOT contain app
      expect(output).not.toContain("[A]");
    });
  });

  test("shows 'All metadata up-to-date' when nothing to update", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "uptodate_test");

      // Create a script and run generate-metadata twice
      await createScript(tempDir, "f/test", "my_script");

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

      await createScript(tempDir, "f/test", "my_script");

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
