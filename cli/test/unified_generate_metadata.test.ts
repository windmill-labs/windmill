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

  test("--schema-only only regenerates schemas", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspace(backend, tempDir, "schema_only_test");

      await createScript(tempDir, "f/test", "my_script");

      const result = await backend.runCLICommand(
        ["generate-metadata", "--yes", "--schema-only"],
        tempDir,
        "schema_only_test"
      );

      expect(result.code).toEqual(0);
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
});
