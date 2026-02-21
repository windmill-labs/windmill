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

import { expect, test } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { writeFile, mkdir } from "node:fs/promises";
import { stringify as stringifyYaml } from "@std/yaml";

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

test("Workspace deps: Scripts - dry-run shows correct stale scripts when default dep changes", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");

      // Setup dependencies folder (Bun/TypeScript)
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      const defaultDep = `{"dependencies": {"lodash": "4.17.21"}}`;
      const explicitDep = `{"dependencies": {"axios": "1.6.0"}}`;
      await writeFile(`${tempDir}/dependencies/package.json`, defaultDep, "utf-8");
      await writeFile(`${tempDir}/dependencies/explicit.package.json`, explicitDep, "utf-8");

      // Setup script folder
      await mkdir(`${tempDir}/f/test`, { recursive: true });

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
      await writeFile(`${tempDir}/f/test/uses_default.ts`, script1Content, "utf-8");
      await writeFile(`${tempDir}/f/test/uses_default.script.yaml`, script1Metadata, "utf-8");

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
      await writeFile(`${tempDir}/f/test/uses_explicit.ts`, script2Content, "utf-8");
      await writeFile(`${tempDir}/f/test/uses_explicit.script.yaml`, script2Metadata, "utf-8");

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
      await writeFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/uses_default": script1Hash,
        "f/test/uses_explicit": script2Hash,
      }), "utf-8");

      // Verify initial state - both scripts should be up-to-date
      const initialResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/uses_default*,f/test/uses_explicit*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_test"
      );
      expect(initialResult.code).toEqual(0);
      expect(initialResult.stdout).toContain("No metadata to update");

      // Now change package.json (default dep)
      const newDefaultDep = `{"dependencies": {"lodash": "4.17.22"}}`;
      await writeFile(`${tempDir}/dependencies/package.json`, newDefaultDep, "utf-8");

      // Run dry-run again
      const afterDefaultChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/uses_default*,f/test/uses_explicit*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_test"
      );
      expect(afterDefaultChangeResult.code).toEqual(0);

      // uses_default should be stale (uses default dep which changed)
      expect(afterDefaultChangeResult.stdout).toContain("uses_default");

      // uses_explicit should NOT be stale (uses explicit dep, not default)
      expect(!afterDefaultChangeResult.stdout.includes("uses_explicit")).toBeTruthy();

      // Reset and test the reverse: change explicit dep
      await writeFile(`${tempDir}/dependencies/package.json`, defaultDep, "utf-8"); // restore original
      const newExplicitDep = `{"dependencies": {"axios": "1.6.1"}}`;
      await writeFile(`${tempDir}/dependencies/explicit.package.json`, newExplicitDep, "utf-8");

      // Run dry-run again
      const afterExplicitChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/uses_default*,f/test/uses_explicit*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_test"
      );
      expect(afterExplicitChangeResult.code).toEqual(0);

      // uses_explicit should be stale (uses explicit dep which changed)
      expect(afterExplicitChangeResult.stdout).toContain("uses_explicit");

      // uses_default should NOT be stale (uses default dep, not explicit)
      expect(!afterExplicitChangeResult.stdout.includes("uses_default")).toBeTruthy();
    });
  });

// =============================================================================
// Test 2: Flows - filterWorkspaceDependenciesForScripts correctly filters by annotation
// =============================================================================

test("Workspace deps: Flows - filterWorkspaceDependenciesForScripts correctly filters inline scripts", async () => {
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
    expect(Object.keys(defaultFiltered).length).toEqual(1);
    expect("dependencies/package.json" in defaultFiltered).toBeTruthy();
    expect(!("dependencies/explicit.package.json" in defaultFiltered)).toBeTruthy();

    // Filter for explicit script - should only include explicit dep
    const explicitFiltered = filterWorkspaceDependencies(rawWorkspaceDeps, explicitScriptContent, "bun");
    expect(Object.keys(explicitFiltered).length).toEqual(1);
    expect("dependencies/explicit.package.json" in explicitFiltered).toBeTruthy();
    expect(!("dependencies/package.json" in explicitFiltered)).toBeTruthy();

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
    expect(defaultHash1 !== defaultHash2).toBeTruthy();

    // Explicit script hash should NOT change (its dep didn't change)
    expect(explicitHash1).toEqual(explicitHash2);

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
    expect(defaultHash1).toEqual(defaultHash3);

    // Explicit script hash should change (its dep changed)
    expect(explicitHash1 !== explicitHash3).toBeTruthy();
  });

// =============================================================================
// Test 3: Cross-language isolation - Python dep change doesn't affect Bun script
// =============================================================================

test("Workspace deps: Cross-language - Python dep change doesn't affect Bun script", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");

      // Setup dependencies folder with deps for multiple languages
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      const pythonDep = "requests==2.31.0";
      const bunDep = `{"dependencies": {"lodash": "4.17.21"}}`;
      await writeFile(`${tempDir}/dependencies/requirements.in`, pythonDep, "utf-8");
      await writeFile(`${tempDir}/dependencies/package.json`, bunDep, "utf-8");

      // Setup script folder
      await mkdir(`${tempDir}/f/test`, { recursive: true });

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
      await writeFile(`${tempDir}/f/test/python_script.py`, pythonContent, "utf-8");
      await writeFile(`${tempDir}/f/test/python_script.script.yaml`, pythonMetadata, "utf-8");

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
      await writeFile(`${tempDir}/f/test/bun_script.ts`, bunContent, "utf-8");
      await writeFile(`${tempDir}/f/test/bun_script.script.yaml`, bunMetadata, "utf-8");

      // Build raw workspace dependencies map
      const rawWorkspaceDeps: Record<string, string> = {
        "dependencies/requirements.in": pythonDep,
        "dependencies/package.json": bunDep,
      };

      // Filter deps for each script - this is where language filtering happens
      const pythonFilteredDeps = filterWorkspaceDependencies(rawWorkspaceDeps, pythonContent, "python3");
      const bunFilteredDeps = filterWorkspaceDependencies(rawWorkspaceDeps, bunContent, "bun");

      // Python script should only get requirements.in
      expect(Object.keys(pythonFilteredDeps).length).toEqual(1);
      expect("dependencies/requirements.in" in pythonFilteredDeps).toBeTruthy();

      // Bun script should only get package.json
      expect(Object.keys(bunFilteredDeps).length).toEqual(1);
      expect("dependencies/package.json" in bunFilteredDeps).toBeTruthy();

      // Compute initial hashes
      const pythonHash = await generateScriptHash(pythonFilteredDeps, pythonContent, pythonMetadata);
      const bunHash = await generateScriptHash(bunFilteredDeps, bunContent, bunMetadata);

      // Create initial wmill-lock.yaml
      await writeFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/python_script": pythonHash,
        "f/test/bun_script": bunHash,
      }), "utf-8");

      // Verify initial state - both scripts should be up-to-date
      const initialResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/python_script*,f/test/bun_script*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_cross_lang_test"
      );
      expect(initialResult.code).toEqual(0);
      expect(initialResult.stdout).toContain("No metadata to update");

      // Change Python dep (requirements.in)
      await writeFile(`${tempDir}/dependencies/requirements.in`, "requests==2.32.0", "utf-8");

      // Run dry-run
      const afterPythonChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/python_script*,f/test/bun_script*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_cross_lang_test"
      );
      expect(afterPythonChangeResult.code).toEqual(0);

      // python_script should be stale
      expect(afterPythonChangeResult.stdout).toContain("python_script");

      // bun_script should NOT be stale (different language)
      expect(!afterPythonChangeResult.stdout.includes("bun_script")).toBeTruthy();

      // Reset and test the reverse
      await writeFile(`${tempDir}/dependencies/requirements.in`, pythonDep, "utf-8");
      await writeFile(`${tempDir}/dependencies/package.json`, `{"dependencies": {"lodash": "4.17.22"}}`, "utf-8");

      const afterBunChangeResult = await backend.runCLICommand(
        ["script", "generate-metadata", "-i", "f/test/python_script*,f/test/bun_script*", "--yes", "--dry-run"],
        tempDir,
        "workspace_deps_cross_lang_test"
      );
      expect(afterBunChangeResult.code).toEqual(0);

      // bun_script should be stale
      expect(afterBunChangeResult.stdout).toContain("bun_script");

      // python_script should NOT be stale (different language)
      expect(!afterBunChangeResult.stdout.includes("python_script")).toBeTruthy();
    });
  });

// =============================================================================
// Test 4: Apps - Create app via API and test filterWorkspaceDependenciesForApp
// =============================================================================

test("Workspace deps: Apps - filterWorkspaceDependenciesForApp with real app via API", async () => {
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
      await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`, "utf-8");

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
      expect(createResponse.ok).toEqual(true);

      // Pull the app to disk
      const pullResult = await backend.runCLICommand(
        ["sync", "pull", "--yes"],
        tempDir,
        "workspace_deps_app_test"
      );
      expect(pullResult.code).toEqual(0);

      // Setup workspace dependencies
      await mkdir(`${tempDir}/dependencies`, { recursive: true });
      const defaultBunDep = `{"dependencies": {"lodash": "4.17.21"}}`;
      const explicitBunDep = `{"dependencies": {"axios": "1.6.0"}}`;
      const pythonDep = "requests==2.31.0";
      await writeFile(`${tempDir}/dependencies/package.json`, defaultBunDep, "utf-8");
      await writeFile(`${tempDir}/dependencies/explicit.package.json`, explicitBunDep, "utf-8");
      await writeFile(`${tempDir}/dependencies/requirements.in`, pythonDep, "utf-8");

      // Read the pulled app.yaml
      const { yamlParseFile } = await import("../src/utils/yaml.ts");
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
      expect(Object.keys(filteredDeps).length).toEqual(3);
      expect("dependencies/package.json" in filteredDeps).toBeTruthy();
      expect("dependencies/explicit.package.json" in filteredDeps).toBeTruthy();
      expect("dependencies/requirements.in" in filteredDeps).toBeTruthy();

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
      expect(hash1 !== hash2).toBeTruthy();
    });
  });
