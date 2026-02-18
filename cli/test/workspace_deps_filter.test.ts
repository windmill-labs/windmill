/**
 * Workspace Dependencies Filtering Tests
 *
 * Tests that workspace dependencies are correctly filtered based on:
 * 1. Script language (only matching language deps)
 * 2. Annotations (explicit deps vs default)
 *
 * These tests use --dry-run mode to avoid needing workers to generate locks.
 * We pre-compute hashes to simulate an initial state, then verify that
 * changing specific deps only marks the expected scripts as stale.
 */

import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";
import { stringify as stringifyYaml } from "jsr:@std/yaml";

// Import hash generation utilities from CLI
import { generateHash } from "../src/utils/utils.ts";
import { filterWorkspaceDependencies } from "../src/utils/metadata.ts";
import { filterWorkspaceDependenciesForApp } from "../src/commands/app/app_metadata.ts";

/**
 * Generate a script hash the same way the CLI does
 */
async function generateScriptHash(
  filteredWorkspaceDependencies: Record<string, string>,
  scriptContent: string,
  metadataContent: string
): Promise<string> {
  return await generateHash(
    JSON.stringify(filteredWorkspaceDependencies) + scriptContent + metadataContent
  );
}

/**
 * Helper to create wmill-lock.yaml content
 */
function createLockfile(locks: Record<string, string>): string {
  return stringifyYaml({ version: "v2", locks });
}

// =============================================================================
// Test 1: Scripts - changing default dep only marks scripts without annotation as stale
// =============================================================================

Deno.test({
  name: "Workspace deps: Scripts - dry-run shows correct stale scripts when default dep changes",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "workspace_deps_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Setup dependencies folder (Bun/TypeScript)
      await ensureDir(`${tempDir}/dependencies`);
      const defaultDep = `{"dependencies": {"lodash": "4.17.21"}}`;
      const explicitDep = `{"dependencies": {"axios": "1.6.0"}}`;
      await Deno.writeTextFile(`${tempDir}/dependencies/package.json`, defaultDep);
      await Deno.writeTextFile(`${tempDir}/dependencies/explicit.package.json`, explicitDep);

      // Setup script folder
      await ensureDir(`${tempDir}/f/test`);

      // Script 1: No annotation - uses default dep
      const script1Content = `export async function main() {
  return "uses default deps";
}
`;
      const script1Metadata = `summary: "uses default"
schema:
  type: object
  properties: {}
lock: ""
`;
      await Deno.writeTextFile(`${tempDir}/f/test/uses_default.ts`, script1Content);
      await Deno.writeTextFile(`${tempDir}/f/test/uses_default.script.yaml`, script1Metadata);

      // Script 2: Uses explicit dep (TypeScript/Bun with annotation)
      const script2Content = `// package_json: explicit
export async function main() {
  return "uses explicit deps";
}
`;
      const script2Metadata = `summary: "uses explicit"
schema:
  type: object
  properties: {}
lock: ""
`;
      await Deno.writeTextFile(`${tempDir}/f/test/uses_explicit.ts`, script2Content);
      await Deno.writeTextFile(`${tempDir}/f/test/uses_explicit.script.yaml`, script2Metadata);

      // Build raw workspace dependencies map (as the CLI would)
      const rawWorkspaceDeps: Record<string, string> = {
        "dependencies/package.json": defaultDep,
        "dependencies/explicit.package.json": explicitDep,
      };

      // Filter deps for each script and compute hashes
      const script1FilteredDeps = filterWorkspaceDependencies(rawWorkspaceDeps, script1Content, "bun");
      const script2FilteredDeps = filterWorkspaceDependencies(rawWorkspaceDeps, script2Content, "bun");

      const script1Hash = await generateScriptHash(script1FilteredDeps, script1Content, script1Metadata);
      const script2Hash = await generateScriptHash(script2FilteredDeps, script2Content, script2Metadata);

      // Create initial wmill-lock.yaml with these hashes
      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/uses_default": script1Hash,
        "f/test/uses_explicit": script2Hash,
      }));

      // Verify initial state - both scripts should be up-to-date
      const initialResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/uses_default*,f/test/uses_explicit*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_test"
      );
      assertEquals(initialResult.code, 0, `Initial dry-run should succeed: ${initialResult.stderr}`);
      assertStringIncludes(initialResult.stdout, "No metadata to update",
        `Initial state should show no updates needed. Output: ${initialResult.stdout}`);

      // Now change package.json (default dep)
      const newDefaultDep = `{"dependencies": {"lodash": "4.17.22"}}`;
      await Deno.writeTextFile(`${tempDir}/dependencies/package.json`, newDefaultDep);

      // Run dry-run again
      const afterDefaultChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/uses_default*,f/test/uses_explicit*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_test"
      );
      assertEquals(afterDefaultChangeResult.code, 0, `Dry-run should succeed: ${afterDefaultChangeResult.stderr}`);

      // uses_default should be stale (uses default dep which changed)
      assertStringIncludes(afterDefaultChangeResult.stdout, "uses_default",
        `uses_default should be marked stale after default dep change. Output: ${afterDefaultChangeResult.stdout}`);

      // uses_explicit should NOT be stale (uses explicit dep, not default)
      assert(!afterDefaultChangeResult.stdout.includes("uses_explicit"),
        `uses_explicit should NOT be marked stale after default dep change. Output: ${afterDefaultChangeResult.stdout}`);

      // Reset and test the reverse: change explicit dep
      await Deno.writeTextFile(`${tempDir}/dependencies/package.json`, defaultDep); // restore original
      const newExplicitDep = `{"dependencies": {"axios": "1.6.1"}}`;
      await Deno.writeTextFile(`${tempDir}/dependencies/explicit.package.json`, newExplicitDep);

      // Run dry-run again
      const afterExplicitChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/uses_default*,f/test/uses_explicit*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_test"
      );
      assertEquals(afterExplicitChangeResult.code, 0, `Dry-run should succeed: ${afterExplicitChangeResult.stderr}`);

      // uses_explicit should be stale (uses explicit dep which changed)
      assertStringIncludes(afterExplicitChangeResult.stdout, "uses_explicit",
        `uses_explicit should be marked stale after explicit dep change. Output: ${afterExplicitChangeResult.stdout}`);

      // uses_default should NOT be stale (uses default dep, not explicit)
      assert(!afterExplicitChangeResult.stdout.includes("uses_default"),
        `uses_default should NOT be marked stale after explicit dep change. Output: ${afterExplicitChangeResult.stdout}`);
    });
  },
});

// =============================================================================
// Test 2: Flows - filterWorkspaceDependenciesForScripts correctly filters by annotation
// =============================================================================

Deno.test({
  name: "Workspace deps: Flows - filterWorkspaceDependenciesForScripts correctly filters inline scripts",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    // This test verifies the filtering logic used by flows without needing workers
    // We test filterWorkspaceDependenciesForScripts directly since flow generate-locks
    // doesn't have a --dry-run option

    const defaultDep = `{"dependencies": {"lodash": "4.17.21"}}`;
    const explicitDep = `{"dependencies": {"axios": "1.6.0"}}`;

    const rawWorkspaceDeps: Record<string, string> = {
      "dependencies/package.json": defaultDep,
      "dependencies/explicit.package.json": explicitDep,
    };

    // Script using default dep (no annotation)
    const defaultScriptContent = `export async function main() {
  return "uses default deps";
}
`;

    // Script using explicit dep (with annotation)
    const explicitScriptContent = `// package_json: explicit
export async function main() {
  return "uses explicit deps";
}
`;

    // Filter for default script - should only include default dep
    const defaultFiltered = filterWorkspaceDependencies(rawWorkspaceDeps, defaultScriptContent, "bun");
    assertEquals(Object.keys(defaultFiltered).length, 1,
      `Default script should have 1 filtered dep, got: ${JSON.stringify(defaultFiltered)}`);
    assert("dependencies/package.json" in defaultFiltered,
      `Default script should have package.json`);
    assert(!("dependencies/explicit.package.json" in defaultFiltered),
      `Default script should NOT have explicit.package.json`);

    // Filter for explicit script - should only include explicit dep
    const explicitFiltered = filterWorkspaceDependencies(rawWorkspaceDeps, explicitScriptContent, "bun");
    assertEquals(Object.keys(explicitFiltered).length, 1,
      `Explicit script should have 1 filtered dep, got: ${JSON.stringify(explicitFiltered)}`);
    assert("dependencies/explicit.package.json" in explicitFiltered,
      `Explicit script should have explicit.package.json`);
    assert(!("dependencies/package.json" in explicitFiltered),
      `Explicit script should NOT have package.json`);

    // Verify hashes change correctly when deps change
    const defaultHash1 = await generateScriptHash(defaultFiltered, defaultScriptContent, "metadata");
    const explicitHash1 = await generateScriptHash(explicitFiltered, explicitScriptContent, "metadata");

    // Change default dep
    const newDefaultDep = `{"dependencies": {"lodash": "4.17.22"}}`;
    const newRawWorkspaceDeps1: Record<string, string> = {
      "dependencies/package.json": newDefaultDep,
      "dependencies/explicit.package.json": explicitDep,
    };

    const defaultFiltered2 = filterWorkspaceDependencies(newRawWorkspaceDeps1, defaultScriptContent, "bun");
    const explicitFiltered2 = filterWorkspaceDependencies(newRawWorkspaceDeps1, explicitScriptContent, "bun");

    const defaultHash2 = await generateScriptHash(defaultFiltered2, defaultScriptContent, "metadata");
    const explicitHash2 = await generateScriptHash(explicitFiltered2, explicitScriptContent, "metadata");

    // Default script hash should change (its dep changed)
    assert(defaultHash1 !== defaultHash2,
      `Default script hash should change when default dep changes`);

    // Explicit script hash should NOT change (its dep didn't change)
    assertEquals(explicitHash1, explicitHash2,
      `Explicit script hash should NOT change when default dep changes`);

    // Now change explicit dep
    const newExplicitDep = `{"dependencies": {"axios": "1.6.1"}}`;
    const newRawWorkspaceDeps2: Record<string, string> = {
      "dependencies/package.json": defaultDep, // back to original
      "dependencies/explicit.package.json": newExplicitDep,
    };

    const defaultFiltered3 = filterWorkspaceDependencies(newRawWorkspaceDeps2, defaultScriptContent, "bun");
    const explicitFiltered3 = filterWorkspaceDependencies(newRawWorkspaceDeps2, explicitScriptContent, "bun");

    const defaultHash3 = await generateScriptHash(defaultFiltered3, defaultScriptContent, "metadata");
    const explicitHash3 = await generateScriptHash(explicitFiltered3, explicitScriptContent, "metadata");

    // Default script hash should be back to original (dep is back to original)
    assertEquals(defaultHash1, defaultHash3,
      `Default script hash should be same as original when dep reverts`);

    // Explicit script hash should change (its dep changed)
    assert(explicitHash1 !== explicitHash3,
      `Explicit script hash should change when explicit dep changes`);
  },
});

// =============================================================================
// Test 3: Cross-language isolation - Python dep change doesn't affect Bun script
// =============================================================================

Deno.test({
  name: "Workspace deps: Cross-language - Python dep change doesn't affect Bun script",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "workspace_deps_cross_lang_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Setup dependencies folder with deps for multiple languages
      await ensureDir(`${tempDir}/dependencies`);
      const pythonDep = "requests==2.31.0";
      const bunDep = `{"dependencies": {"lodash": "4.17.21"}}`;
      await Deno.writeTextFile(`${tempDir}/dependencies/requirements.in`, pythonDep);
      await Deno.writeTextFile(`${tempDir}/dependencies/package.json`, bunDep);

      // Setup script folder
      await ensureDir(`${tempDir}/f/test`);

      // Python script
      const pythonContent = `def main():
    return "python script"
`;
      const pythonMetadata = `summary: "python script"
schema:
  type: object
  properties: {}
lock: ""
`;
      await Deno.writeTextFile(`${tempDir}/f/test/python_script.py`, pythonContent);
      await Deno.writeTextFile(`${tempDir}/f/test/python_script.script.yaml`, pythonMetadata);

      // Bun script
      const bunContent = `export async function main() {
  return "bun script";
}
`;
      const bunMetadata = `summary: "bun script"
schema:
  type: object
  properties: {}
lock: ""
`;
      await Deno.writeTextFile(`${tempDir}/f/test/bun_script.ts`, bunContent);
      await Deno.writeTextFile(`${tempDir}/f/test/bun_script.script.yaml`, bunMetadata);

      // Build raw workspace dependencies map
      const rawWorkspaceDeps: Record<string, string> = {
        "dependencies/requirements.in": pythonDep,
        "dependencies/package.json": bunDep,
      };

      // Filter deps for each script - this is where language filtering happens
      const pythonFilteredDeps = filterWorkspaceDependencies(rawWorkspaceDeps, pythonContent, "python3");
      const bunFilteredDeps = filterWorkspaceDependencies(rawWorkspaceDeps, bunContent, "bun");

      // Python script should only get requirements.in
      assertEquals(Object.keys(pythonFilteredDeps).length, 1,
        `Python script should only have 1 filtered dep, got: ${JSON.stringify(pythonFilteredDeps)}`);
      assert("dependencies/requirements.in" in pythonFilteredDeps,
        `Python script should have requirements.in in filtered deps`);

      // Bun script should only get package.json
      assertEquals(Object.keys(bunFilteredDeps).length, 1,
        `Bun script should only have 1 filtered dep, got: ${JSON.stringify(bunFilteredDeps)}`);
      assert("dependencies/package.json" in bunFilteredDeps,
        `Bun script should have package.json in filtered deps`);

      // Compute initial hashes
      const pythonHash = await generateScriptHash(pythonFilteredDeps, pythonContent, pythonMetadata);
      const bunHash = await generateScriptHash(bunFilteredDeps, bunContent, bunMetadata);

      // Create initial wmill-lock.yaml
      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/python_script": pythonHash,
        "f/test/bun_script": bunHash,
      }));

      // Verify initial state - both scripts should be up-to-date
      const initialResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/python_script*,f/test/bun_script*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_cross_lang_test"
      );
      assertEquals(initialResult.code, 0, `Initial dry-run should succeed: ${initialResult.stderr}`);
      assertStringIncludes(initialResult.stdout, "No metadata to update",
        `Initial state should show no updates needed. Output: ${initialResult.stdout}`);

      // Change Python dep (requirements.in)
      await Deno.writeTextFile(`${tempDir}/dependencies/requirements.in`, "requests==2.32.0");

      // Run dry-run
      const afterPythonChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/python_script*,f/test/bun_script*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_cross_lang_test"
      );
      assertEquals(afterPythonChangeResult.code, 0, `Dry-run should succeed: ${afterPythonChangeResult.stderr}`);

      // python_script should be stale
      assertStringIncludes(afterPythonChangeResult.stdout, "python_script",
        `python_script should be marked stale after Python dep change. Output: ${afterPythonChangeResult.stdout}`);

      // bun_script should NOT be stale (different language)
      assert(!afterPythonChangeResult.stdout.includes("bun_script"),
        `bun_script should NOT be marked stale after Python dep change. Output: ${afterPythonChangeResult.stdout}`);

      // Reset and test the reverse
      await Deno.writeTextFile(`${tempDir}/dependencies/requirements.in`, pythonDep);
      await Deno.writeTextFile(`${tempDir}/dependencies/package.json`, `{"dependencies": {"lodash": "4.17.22"}}`);

      const afterBunChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/python_script*,f/test/bun_script*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_cross_lang_test"
      );
      assertEquals(afterBunChangeResult.code, 0, `Dry-run should succeed: ${afterBunChangeResult.stderr}`);

      // bun_script should be stale
      assertStringIncludes(afterBunChangeResult.stdout, "bun_script",
        `bun_script should be marked stale after Bun dep change. Output: ${afterBunChangeResult.stdout}`);

      // python_script should NOT be stale (different language)
      assert(!afterBunChangeResult.stdout.includes("python_script"),
        `python_script should NOT be marked stale after Bun dep change. Output: ${afterBunChangeResult.stdout}`);
    });
  },
});

// =============================================================================
// Test 4: Apps - Create app via API and test filterWorkspaceDependenciesForApp
// =============================================================================

Deno.test({
  name: "Workspace deps: Apps - filterWorkspaceDependenciesForApp with real app via API",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Set up workspace
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "workspace_deps_app_test",
        token: backend.token
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      // Create wmill.yaml
      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Create app with multiple inline scripts via backend API
      const appPath = "f/test/multi_script_app";
      const appValue = {
        type: "app",
        grid: [
          {
            id: "default_bun",
            data: {
              type: "buttoncomponent",
              componentInput: {
                type: "runnable",
                runnable: {
                  type: "runnableByName",
                  inlineScript: {
                    content: `export async function main() { return "uses default bun deps"; }`,
                    language: "bun",
                  },
                },
              },
            },
          },
          {
            id: "explicit_bun",
            data: {
              type: "buttoncomponent",
              componentInput: {
                type: "runnable",
                runnable: {
                  type: "runnableByName",
                  inlineScript: {
                    content: `// package_json: explicit\nexport async function main() { return "uses explicit bun deps"; }`,
                    language: "bun",
                  },
                },
              },
            },
          },
          {
            id: "python_script",
            data: {
              type: "buttoncomponent",
              componentInput: {
                type: "runnable",
                runnable: {
                  type: "runnableByName",
                  inlineScript: {
                    content: `def main():\n    return "python script"`,
                    language: "python3",
                  },
                },
              },
            },
          },
        ],
        hiddenInlineScripts: [],
        css: {},
        norefreshbar: false,
      };

      // Create app via API
      const createResponse = await backend.apiRequest!(
        `/api/w/${backend.workspace}/apps/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            path: appPath,
            value: appValue,
            summary: "Multi-script app for workspace deps testing",
            policy: {
              on_behalf_of: null,
              on_behalf_of_email: null,
              triggerables: {},
              execution_mode: "viewer",
            },
          }),
        }
      );
      assertEquals(createResponse.ok, true, `Failed to create app: ${await createResponse.text()}`);

      // Pull the app to disk
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
        "workspace_deps_app_test"
      );
      assertEquals(pullResult.code, 0, `Sync pull should succeed: ${pullResult.stderr}`);

      // Setup workspace dependencies
      await ensureDir(`${tempDir}/dependencies`);
      const defaultBunDep = `{"dependencies": {"lodash": "4.17.21"}}`;
      const explicitBunDep = `{"dependencies": {"axios": "1.6.0"}}`;
      const pythonDep = "requests==2.31.0";
      await Deno.writeTextFile(`${tempDir}/dependencies/package.json`, defaultBunDep);
      await Deno.writeTextFile(`${tempDir}/dependencies/explicit.package.json`, explicitBunDep);
      await Deno.writeTextFile(`${tempDir}/dependencies/requirements.in`, pythonDep);

      // Read the pulled app.yaml
      const { yamlParseFile } = await import("../deps.ts");
      const appFilePath = `${tempDir}/${appPath}.app/app.yaml`;
      const appFile = await yamlParseFile(appFilePath);

      const rawWorkspaceDeps: Record<string, string> = {
        "dependencies/package.json": defaultBunDep,
        "dependencies/explicit.package.json": explicitBunDep,
        "dependencies/requirements.in": pythonDep,
      };

      // Call the app-specific filtering API
      const filteredDeps = await filterWorkspaceDependenciesForApp(
        appFile.value,
        rawWorkspaceDeps,
        `${tempDir}/${appPath}.app`
      );

      // Verify all 3 dep types are included
      assertEquals(Object.keys(filteredDeps).length, 3,
        `App with bun (default), bun (explicit), and python should have 3 filtered deps, got: ${JSON.stringify(filteredDeps)}`);
      assert("dependencies/package.json" in filteredDeps,
        `Should include default package.json for default bun script`);
      assert("dependencies/explicit.package.json" in filteredDeps,
        `Should include explicit.package.json for annotated bun script`);
      assert("dependencies/requirements.in" in filteredDeps,
        `Should include requirements.in for python script`);

      // Verify hash changes when deps change
      const hash1 = await generateHash(JSON.stringify(filteredDeps));

      const newDefaultBunDep = `{"dependencies": {"lodash": "4.17.22"}}`;
      const newRawDeps: Record<string, string> = {
        "dependencies/package.json": newDefaultBunDep,
        "dependencies/explicit.package.json": explicitBunDep,
        "dependencies/requirements.in": pythonDep,
      };

      const filteredDeps2 = await filterWorkspaceDependenciesForApp(
        appFile.value,
        newRawDeps,
        `${tempDir}/${appPath}.app`
      );

      const hash2 = await generateHash(JSON.stringify(filteredDeps2));
      assert(hash1 !== hash2, `Hash should change when filtered deps change`);
    });
  },
});
