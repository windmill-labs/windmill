/**
 * Relative Imports Skip Logic Tests
 *
 * Tests that metadata regeneration correctly handles relative imports:
 * When an imported script changes, all scripts that import it should also
 * be marked as stale and have their metadata regenerated.
 *
 * The current bug: CLI only regenerates metadata for the changed script,
 * not for scripts that import it via relative imports.
 */

import { assertEquals, assertNotEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";
import { stringify as stringifyYaml, parse as parseYaml } from "jsr:@std/yaml";

import { generateHash } from "../src/utils/utils.ts";

/**
 * Generate a script hash the same way the CLI does (without workspace deps)
 */
async function generateScriptHash(
  scriptContent: string,
  metadataContent: string
): Promise<string> {
  return await generateHash("{}" + scriptContent + metadataContent);
}

/**
 * Read lock hash for a path from wmill-lock.yaml
 */
async function getLockHash(tempDir: string, path: string): Promise<string | undefined> {
  try {
    const content = await Deno.readTextFile(`${tempDir}/wmill-lock.yaml`);
    const lock = parseYaml(content) as { locks?: Record<string, string> };
    return lock.locks?.[path];
  } catch {
    return undefined;
  }
}

/**
 * Helper to create wmill-lock.yaml content
 */
function createLockfile(locks: Record<string, string>): string {
  return stringifyYaml({ version: "v2", locks });
}

// =============================================================================
// Test 1: TypeScript - Direct relative import, imported script changes
// =============================================================================

Deno.test({
  name: "Relative imports: TypeScript - imported script changes should mark importer as stale",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // Script A imports script B
      const scriptAContent = `import { helper } from "./script_b";
export async function main() { return helper(); }
`;
      const scriptAMetadata = `summary: "Script A"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptBContentOriginal = `export function helper() { return "original"; }
`;
      const scriptBMetadata = `summary: "Script B"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptAContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, scriptAMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptBContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, scriptBMetadata);

      // Compute and store initial hashes
      const scriptAHashInitial = await generateScriptHash(scriptAContent, scriptAMetadata);
      const scriptBHashInitial = await generateScriptHash(scriptBContentOriginal, scriptBMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": scriptAHashInitial,
        "f/test/script_b": scriptBHashInitial,
      }));

      // Verify initial state - both should be up-to-date
      const initialResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_test"
      );
      assertEquals(initialResult.code, 0, `Initial dry-run should succeed: ${initialResult.stderr}`);
      assertStringIncludes(initialResult.stdout, "No metadata to update");

      // Change script B
      const scriptBContentChanged = `export function helper() { return "changed"; }
`;
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptBContentChanged);

      // Compute new hash for B - it should be different
      const scriptBHashNew = await generateScriptHash(scriptBContentChanged, scriptBMetadata);
      assertNotEquals(scriptBHashInitial, scriptBHashNew, "script_b hash should change when content changes");

      // Script A's own content didn't change, but it imports B which changed
      // After the fix, script A should get a new hash that includes B's content hash
      const afterChangeResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_test"
      );
      assertEquals(afterChangeResult.code, 0, `Dry-run should succeed: ${afterChangeResult.stderr}`);

      // Both scripts should be marked as stale
      assertStringIncludes(afterChangeResult.stdout, "script_b",
        `script_b should be stale. Output: ${afterChangeResult.stdout}`);
      assertStringIncludes(afterChangeResult.stdout, "script_a",
        `script_a should be stale because it imports script_b. Output: ${afterChangeResult.stdout}`);
    });
  },
});

// =============================================================================
// Test 2: TypeScript - Chained relative imports (A -> B -> C, C changes)
// =============================================================================

Deno.test({
  name: "Relative imports: TypeScript - chained imports, leaf changes should mark all ancestors stale",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_chain_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      const scriptAContent = `import { utilB } from "./script_b";
export async function main() { return utilB(); }
`;
      const scriptAMetadata = `summary: "A"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptBContent = `import { utilC } from "./script_c";
export function utilB() { return utilC() + " B"; }
`;
      const scriptBMetadata = `summary: "B"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptCContentOriginal = `export function utilC() { return "C"; }
`;
      const scriptCMetadata = `summary: "C"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptAContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, scriptAMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptBContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, scriptBMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, scriptCContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.script.yaml`, scriptCMetadata);

      const scriptAHash = await generateScriptHash(scriptAContent, scriptAMetadata);
      const scriptBHash = await generateScriptHash(scriptBContent, scriptBMetadata);
      const scriptCHash = await generateScriptHash(scriptCContentOriginal, scriptCMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": scriptAHash,
        "f/test/script_b": scriptBHash,
        "f/test/script_c": scriptCHash,
      }));

      // Verify initial state
      const initialResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_chain_test"
      );
      assertStringIncludes(initialResult.stdout, "No metadata to update");

      // Change script C (the leaf)
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, `export function utilC() { return "C changed"; }
`);

      const afterChangeResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_chain_test"
      );
      assertEquals(afterChangeResult.code, 0);

      // All three should be stale
      assertStringIncludes(afterChangeResult.stdout, "script_c");
      assertStringIncludes(afterChangeResult.stdout, "script_b");
      assertStringIncludes(afterChangeResult.stdout, "script_a");
    });
  },
});

// =============================================================================
// Test 3: TypeScript - Cross-folder relative imports
// =============================================================================

Deno.test({
  name: "Relative imports: TypeScript - cross-folder imports with ../",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_crossfolder_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/folder_a`);
      await ensureDir(`${tempDir}/f/folder_b`);

      const scriptAContent = `import { helper } from "../folder_b/script_b";
export async function main() { return helper(); }
`;
      const scriptAMetadata = `summary: "A"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptBContentOriginal = `export function helper() { return "original"; }
`;
      const scriptBMetadata = `summary: "B"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/folder_a/script_a.ts`, scriptAContent);
      await Deno.writeTextFile(`${tempDir}/f/folder_a/script_a.script.yaml`, scriptAMetadata);
      await Deno.writeTextFile(`${tempDir}/f/folder_b/script_b.ts`, scriptBContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/folder_b/script_b.script.yaml`, scriptBMetadata);

      const scriptAHash = await generateScriptHash(scriptAContent, scriptAMetadata);
      const scriptBHash = await generateScriptHash(scriptBContentOriginal, scriptBMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/folder_a/script_a": scriptAHash,
        "f/folder_b/script_b": scriptBHash,
      }));

      const initialResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/**/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_crossfolder_test"
      );
      assertStringIncludes(initialResult.stdout, "No metadata to update");

      // Change script B
      await Deno.writeTextFile(`${tempDir}/f/folder_b/script_b.ts`, `export function helper() { return "changed"; }
`);

      const afterChangeResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/**/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_crossfolder_test"
      );

      assertStringIncludes(afterChangeResult.stdout, "script_b");
      assertStringIncludes(afterChangeResult.stdout, "script_a");
    });
  },
});

// =============================================================================
// Test 4: TypeScript - Absolute path imports (/f/folder/script)
// =============================================================================

Deno.test({
  name: "Relative imports: TypeScript - absolute path imports (/f/folder/script)",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_absolute_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);
      await ensureDir(`${tempDir}/f/utils`);

      const scriptAContent = `import { helper } from "/f/utils/helper";
export async function main() { return helper(); }
`;
      const scriptAMetadata = `summary: "A"
schema:
  type: object
  properties: {}
lock: ""
`;

      const helperContentOriginal = `export function helper() { return "original"; }
`;
      const helperMetadata = `summary: "Helper"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptAContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, scriptAMetadata);
      await Deno.writeTextFile(`${tempDir}/f/utils/helper.ts`, helperContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/utils/helper.script.yaml`, helperMetadata);

      const scriptAHash = await generateScriptHash(scriptAContent, scriptAMetadata);
      const helperHash = await generateScriptHash(helperContentOriginal, helperMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": scriptAHash,
        "f/utils/helper": helperHash,
      }));

      const initialResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/**/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_absolute_test"
      );
      assertStringIncludes(initialResult.stdout, "No metadata to update");

      // Change helper
      await Deno.writeTextFile(`${tempDir}/f/utils/helper.ts`, `export function helper() { return "changed"; }
`);

      const afterChangeResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/**/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_absolute_test"
      );

      assertStringIncludes(afterChangeResult.stdout, "helper");
      assertStringIncludes(afterChangeResult.stdout, "script_a");
    });
  },
});

// =============================================================================
// Test 5: Python - Direct relative import
// =============================================================================

Deno.test({
  name: "Relative imports: Python - imported script changes should mark importer as stale",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_python_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      const scriptAContent = `from .script_b import helper

def main():
    return helper()
`;
      const scriptAMetadata = `summary: "A"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptBContentOriginal = `def helper():
    return "original"
`;
      const scriptBMetadata = `summary: "B"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.py`, scriptAContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, scriptAMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.py`, scriptBContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, scriptBMetadata);

      const scriptAHash = await generateScriptHash(scriptAContent, scriptAMetadata);
      const scriptBHash = await generateScriptHash(scriptBContentOriginal, scriptBMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": scriptAHash,
        "f/test/script_b": scriptBHash,
      }));

      const initialResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_python_test"
      );
      assertStringIncludes(initialResult.stdout, "No metadata to update");

      // Change script B
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.py`, `def helper():
    return "changed"
`);

      const afterChangeResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_python_test"
      );

      assertStringIncludes(afterChangeResult.stdout, "script_b");
      assertStringIncludes(afterChangeResult.stdout, "script_a");
    });
  },
});

// =============================================================================
// Test 6: Circular imports - should handle gracefully
// =============================================================================

Deno.test({
  name: "Relative imports: Circular imports - should handle gracefully without infinite loop",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_circular_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // A imports B, B imports A (circular)
      const scriptAContent = `import { helperB } from "./script_b";
export function helperA() { return "A"; }
export async function main() { return helperA() + helperB(); }
`;
      const scriptAMetadata = `summary: "A"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptBContentOriginal = `import { helperA } from "./script_a";
export function helperB() { return "B"; }
`;
      const scriptBMetadata = `summary: "B"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptAContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, scriptAMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptBContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, scriptBMetadata);

      const scriptAHash = await generateScriptHash(scriptAContent, scriptAMetadata);
      const scriptBHash = await generateScriptHash(scriptBContentOriginal, scriptBMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": scriptAHash,
        "f/test/script_b": scriptBHash,
      }));

      const initialResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_circular_test"
      );
      assertStringIncludes(initialResult.stdout, "No metadata to update");

      // Change script B
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, `import { helperA } from "./script_a";
export function helperB() { return "B changed"; }
`);

      // Should complete without hanging
      const afterChangeResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_circular_test"
      );
      assertEquals(afterChangeResult.code, 0);

      // Both should be stale
      assertStringIncludes(afterChangeResult.stdout, "script_b");
      assertStringIncludes(afterChangeResult.stdout, "script_a");
    });
  },
});

// =============================================================================
// Test 6b: Three-way circular imports (A -> B -> C -> A)
// =============================================================================

Deno.test({
  name: "Relative imports: Three-way circular imports - should handle gracefully",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_3way_cycle_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // A imports B, B imports C, C imports A (three-way cycle)
      const scriptAContent = `import { helperB } from "./script_b";
export function helperA() { return "A"; }
export async function main() { return helperA() + helperB(); }
`;
      const scriptAMetadata = `summary: "A"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptBContent = `import { helperC } from "./script_c";
export function helperB() { return "B" + helperC(); }
`;
      const scriptBMetadata = `summary: "B"
schema:
  type: object
  properties: {}
lock: ""
`;

      const scriptCContentOriginal = `import { helperA } from "./script_a";
export function helperC() { return "C"; }
`;
      const scriptCMetadata = `summary: "C"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptAContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, scriptAMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptBContent);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, scriptBMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, scriptCContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.script.yaml`, scriptCMetadata);

      const scriptAHash = await generateScriptHash(scriptAContent, scriptAMetadata);
      const scriptBHash = await generateScriptHash(scriptBContent, scriptBMetadata);
      const scriptCHash = await generateScriptHash(scriptCContentOriginal, scriptCMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": scriptAHash,
        "f/test/script_b": scriptBHash,
        "f/test/script_c": scriptCHash,
      }));

      const initialResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_3way_cycle_test"
      );
      assertStringIncludes(initialResult.stdout, "No metadata to update");

      // Change script C (one node in the cycle)
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, `import { helperA } from "./script_a";
export function helperC() { return "C changed"; }
`);

      // Should complete without hanging (timeout would indicate infinite loop)
      const afterChangeResult = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_3way_cycle_test"
      );
      assertEquals(afterChangeResult.code, 0);

      // All three should be stale since they form a cycle
      assertStringIncludes(afterChangeResult.stdout, "script_c",
        `script_c should be stale (directly changed). Output: ${afterChangeResult.stdout}`);
      assertStringIncludes(afterChangeResult.stdout, "script_b",
        `script_b should be stale (imports script_c). Output: ${afterChangeResult.stdout}`);
      assertStringIncludes(afterChangeResult.stdout, "script_a",
        `script_a should be stale (imports script_b which imports script_c). Output: ${afterChangeResult.stdout}`);
    });
  },
});

// =============================================================================
// Test 7: Flow with inline script that imports another script
// =============================================================================

Deno.test({
  name: "Relative imports: Flow - inline script imports change should mark flow stale",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_flow_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Create helper script first
      await ensureDir(`${tempDir}/f/test`);

      const helperContentOriginal = `export function helper() { return "original"; }
`;
      const helperMetadata = `summary: "Helper"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/helper.ts`, helperContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.script.yaml`, helperMetadata);

      // Create flow folder structure manually (simulating what sync pull would create)
      const flowFolder = `${tempDir}/f/test/my_flow.flow`;
      await ensureDir(flowFolder);

      const inlineScriptContent = `import { helper } from "/f/test/helper";
export async function main() { return helper(); }
`;

      // Create flow.yaml
      const flowYaml = `summary: "Test flow with inline script"
description: "Flow at f/test/my_flow"
value:
  modules:
    - id: a
      value:
        type: rawscript
        content: "!inline a.inline_script.ts"
        language: bun
        input_transforms: {}
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
  required: []
`;
      await Deno.writeTextFile(`${flowFolder}/flow.yaml`, flowYaml);
      await Deno.writeTextFile(`${flowFolder}/a.inline_script.ts`, inlineScriptContent);

      // Create a lock file manually with the initial hash
      // This simulates having already generated locks for the flow
      const { generateHash } = await import("../src/utils/utils.ts");

      // Read inline script content
      const inlineContent = await Deno.readTextFile(`${flowFolder}/a.inline_script.ts`);
      // Read helper content (this is what should be included in hash)
      const helperContent = await Deno.readTextFile(`${tempDir}/f/test/helper.ts`);

      // Generate hash the same way the CLI does (content + deps + imported content)
      const scriptHash = await generateHash(inlineContent + "{}" + helperContent);
      const topHash = await generateHash(JSON.stringify({"a.inline_script.ts": scriptHash}));

      // Write lock file
      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, `version: v2
locks:
  "f/test/my_flow.flow+a.inline_script.ts": "${scriptHash}"
  "f/test/my_flow.flow+__flow_hash": "${topHash}"
`);

      // Verify up-to-date (should show "No locks to update" since hash matches)
      const initialDryRun = await backend.runCLICommand(
        ["flow", "generate-locks", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_flow_test"
      );
      assertStringIncludes(initialDryRun.stdout, "No locks to update",
        `Initial check should show no updates. Output: ${initialDryRun.stdout}, stderr: ${initialDryRun.stderr}`);

      // Change helper
      await Deno.writeTextFile(`${tempDir}/f/test/helper.ts`, `export function helper() { return "changed"; }
`);

      // Flow should be stale
      const afterChangeResult = await backend.runCLICommand(
        ["flow", "generate-locks", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_flow_test"
      );

      assertStringIncludes(afterChangeResult.stdout, "my_flow",
        `Flow should be stale. Output: ${afterChangeResult.stdout}`);
    });
  },
});

// =============================================================================
// Test 8: App with inline script that imports another script
// =============================================================================

Deno.test({
  name: "Relative imports: App - inline script imports change should mark app stale",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "relative_imports_app_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      // Create helper script first
      await ensureDir(`${tempDir}/f/test`);

      const helperContentOriginal = `export function helper() { return "original"; }
`;
      const helperMetadata = `summary: "Helper"
schema:
  type: object
  properties: {}
lock: ""
`;

      await Deno.writeTextFile(`${tempDir}/f/test/helper.ts`, helperContentOriginal);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.script.yaml`, helperMetadata);

      // Create app folder structure manually (simulating what sync pull would create)
      const appFolder = `${tempDir}/f/test/my_app`;
      await ensureDir(appFolder);

      const inlineScriptContent = `import { helper } from "/f/test/helper";
export async function main() { return helper(); }
`;

      // Create app.yaml
      const appYaml = `type: app
grid:
  - id: button1
    data:
      type: buttoncomponent
      componentInput:
        type: runnable
        runnable:
          type: runnableByName
          inlineScript:
            content: "!inline button1.inline_script.ts"
            language: bun
hiddenInlineScripts: []
css: {}
norefreshbar: false
`;
      await Deno.writeTextFile(`${appFolder}/app.yaml`, appYaml);
      await Deno.writeTextFile(`${appFolder}/button1.inline_script.ts`, inlineScriptContent);

      // Create a lock file manually with the initial hash
      const { generateHash } = await import("../src/utils/utils.ts");

      // Read inline script content
      const inlineContent = await Deno.readTextFile(`${appFolder}/button1.inline_script.ts`);
      // Read helper content (this is what should be included in hash)
      const helperContent = await Deno.readTextFile(`${tempDir}/f/test/helper.ts`);

      // Generate hash the same way the CLI does (content + JSON.stringify(deps) + imported content)
      // deps is empty {} so JSON.stringify({}) = "{}"
      const scriptHash = await generateHash(inlineContent + "{}" + helperContent);
      const topHash = await generateHash(JSON.stringify({"button1.inline_script.ts": scriptHash}));

      // Write lock file
      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, `version: v2
locks:
  "f/test/my_app+button1.inline_script.ts": "${scriptHash}"
  "f/test/my_app+__app_hash": "${topHash}"
`);

      // Verify up-to-date (should show "No metadata to update" since hash matches)
      const initialDryRun = await backend.runCLICommand(
        ["app", "generate-locks", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_app_test"
      );
      assertStringIncludes(initialDryRun.stdout, "No metadata to update",
        `Initial check should show no updates. Output: ${initialDryRun.stdout}, stderr: ${initialDryRun.stderr}`);

      // Change helper
      await Deno.writeTextFile(`${tempDir}/f/test/helper.ts`, `export function helper() { return "changed"; }
`);

      // App should be stale
      const afterChangeResult = await backend.runCLICommand(
        ["app", "generate-locks", "--yes", "--dry-run"],
        tempDir,
        "relative_imports_app_test"
      );

      assertStringIncludes(afterChangeResult.stdout, "my_app",
        `App should be stale. Output: ${afterChangeResult.stdout}`);
    });
  },
});

// =============================================================================
// Test 9: Backend should use LOCAL import content, not DEPLOYED
// =============================================================================
// This test catches the core problem:
// - A imports B
// - B is deployed with version 1
// - B is changed locally to version 2 (not pushed yet)
// - When generating A's lock, backend should use LOCAL B (v2), not DEPLOYED B (v1)
//
// Currently FAILS: backend uses deployed B (silently uses wrong version)
// After fix: backend uses local B from temp storage
// =============================================================================

Deno.test({
  name: "Relative imports: Lock generation should use LOCAL import content, not DEPLOYED",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "local_vs_deployed_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // Helper V1: uses wmill (a small package) version 1.0.0
      // Using wmill because it's small and fast to resolve
      const helperContentV1 = `// Helper v1
export function helper(): string {
  return "v1";
}
`;
      const helperMetadata = `summary: "Helper"
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
  required: []
`;

      // Main imports helper - no external deps, just relative import
      const mainContentV1 = `import { helper } from "./helper";

export async function main(): Promise<string> {
  return helper();
}
`;
      const mainMetadata = `summary: "Main"
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
  required: []
`;

      // Write helper first and deploy it standalone (no relative imports)
      await Deno.writeTextFile(`${tempDir}/f/test/helper.ts`, helperContentV1);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.script.yaml`, helperMetadata);

      // Generate and push helper first
      let result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/helper", "--yes"],
        tempDir,
        "local_vs_deployed_test"
      );
      assertEquals(result.code, 0, `Helper generate-metadata failed: ${result.stderr}`);

      result = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
        "local_vs_deployed_test"
      );
      assertEquals(result.code, 0, `Helper push failed: ${result.stderr}`);

      // Now write main and deploy it (helper already exists on server)
      await Deno.writeTextFile(`${tempDir}/f/test/main.ts`, mainContentV1);
      await Deno.writeTextFile(`${tempDir}/f/test/main.script.yaml`, mainMetadata);

      result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/main", "--yes"],
        tempDir,
        "local_vs_deployed_test"
      );
      assertEquals(result.code, 0, `Main generate-metadata failed: ${result.stderr}`);

      result = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
        "local_vs_deployed_test"
      );
      assertEquals(result.code, 0, `Main push failed: ${result.stderr}`);

      // Now change helper locally to V2 with a DIFFERENT external dependency
      // This will cause the lock to be different
      const helperContentV2 = `import leftpad from "leftpad@0.0.1";

// Helper v2 - now uses leftpad
export function helper(): string {
  return leftpad("v2", 5);
}
`;
      await Deno.writeTextFile(`${tempDir}/f/test/helper.ts`, helperContentV2);

      // Generate metadata for BOTH scripts (helper changed locally, main imports it)
      // The bug: when generating main's lock, backend uses DEPLOYED helper (v1, no leftpad)
      // Expected: backend should use LOCAL helper (v2, with leftpad)
      result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir,
        "local_vs_deployed_test"
      );
      assertEquals(result.code, 0, `Second generate-metadata failed: ${result.stderr}`);

      // Read the locally generated lock file for main
      const mainLockContent = await Deno.readTextFile(`${tempDir}/f/test/main.script.lock`);

      // The lock should reflect LOCAL helper (v2 with leftpad)
      // If this fails, it means backend used DEPLOYED helper (v1 without leftpad)
      assertStringIncludes(mainLockContent, "leftpad",
        `Main's lock should include leftpad from LOCAL helper v2, not use DEPLOYED helper v1. ` +
        `This test fails until we implement temp storage for local imports. ` +
        `Lock: ${mainLockContent}`);
    });
  },
});

// =============================================================================
// Test 10: Multiple scripts in dependency chain - all should use LOCAL versions
// =============================================================================

Deno.test({
  name: "Relative imports: Chain A->B->C should all use LOCAL versions for lock generation",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      const testWorkspace = {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "chain_local_test",
        token: backend.token ?? ""
      };
      await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // C: leaf node with external dep
      const scriptCv1 = `import dayjs from "dayjs@1.11.0";
export function utilC(): string { return dayjs().format(); }
`;
      // B: imports C
      const scriptB = `import { utilC } from "./script_c";
export function utilB(): string { return utilC(); }
`;
      // A: imports B
      const scriptA = `import { utilB } from "./script_b";
export async function main(): Promise<string> { return utilB(); }
`;

      const metadata = `summary: "Script"
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
  required: []
`;

      // Write all files
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, scriptCv1);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.script.yaml`, metadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, metadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, metadata);

      // Deploy in order: C first (no deps), then B (imports C), then A (imports B)
      let result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/script_c", "--yes"],
        tempDir,
        "chain_local_test"
      );
      assertEquals(result.code, 0, `C generate failed: ${result.stderr}`);

      result = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
        "chain_local_test"
      );
      assertEquals(result.code, 0, `C push failed: ${result.stderr}`);

      result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/script_b", "--yes"],
        tempDir,
        "chain_local_test"
      );
      assertEquals(result.code, 0, `B generate failed: ${result.stderr}`);

      result = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
        "chain_local_test"
      );
      assertEquals(result.code, 0, `B push failed: ${result.stderr}`);

      result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/script_a", "--yes"],
        tempDir,
        "chain_local_test"
      );
      assertEquals(result.code, 0, `A generate failed: ${result.stderr}`);

      result = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
        "chain_local_test"
      );
      assertEquals(result.code, 0, `A push failed: ${result.stderr}`);

      // Change C locally to use dayjs@1.11.10
      const scriptCv2 = `import dayjs from "dayjs@1.11.10";
export function utilC(): string { return dayjs().format(); }
`;
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, scriptCv2);

      // Generate metadata for all
      result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir,
        "chain_local_test"
      );
      assertEquals(result.code, 0, `Second generate failed: ${result.stderr}`);

      // Read A's local lock - should have dayjs@1.11.10 (from local C)
      const scriptALock = await Deno.readTextFile(`${tempDir}/f/test/script_a.script.lock`);

      assertStringIncludes(scriptALock, "1.11.10",
        `Script A's lock should reflect LOCAL C (dayjs@1.11.10), not DEPLOYED (1.11.0). ` +
        `Lock: ${scriptALock}`);
    });
  },
});
