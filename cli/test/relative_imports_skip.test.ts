/**
 * Relative Imports Tests
 *
 * Tests that the `generate-metadata` command correctly handles relative imports:
 * - When an imported script changes, importers are marked stale
 * - Transitive dependencies propagate staleness
 * - Circular imports are handled gracefully
 */

import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";
import { stringify as stringifyYaml } from "jsr:@std/yaml";
import { generateHash } from "../src/utils/utils.ts";

async function generateScriptHash(content: string, metadata: string): Promise<string> {
  return await generateHash("{}" + content + metadata);
}

function createLockfile(locks: Record<string, string>): string {
  return stringifyYaml({ version: "v2", locks });
}

const defaultMetadata = `summary: "Test"
schema:
  type: object
  properties: {}
lock: ""
`;

// =============================================================================
// Test 1: Basic staleness propagation (A imports B, B changes -> A is stale)
// =============================================================================

Deno.test({
  name: "Relative imports: imported script change marks importer as stale",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "test_ws",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      const scriptA = `import { helper } from "./script_b";
export async function main() { return helper(); }
`;
      const scriptB = `export function helper() { return "original"; }
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);

      const hashA = await generateScriptHash(scriptA, defaultMetadata);
      const hashB = await generateScriptHash(scriptB, defaultMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": hashA,
        "f/test/script_b": hashB,
      }));

      // Initial check - should be up to date
      const initial = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "test_ws"
      );
      assertEquals(initial.code, 0);
      assertStringIncludes(initial.stdout, "No metadata to update");

      // Change script B
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`,
        `export function helper() { return "changed"; }
`);

      // Both should be stale now
      const after = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "test_ws"
      );
      assertEquals(after.code, 0);
      assertStringIncludes(after.stdout, "script_b", `script_b should be stale: ${after.stdout}`);
      assertStringIncludes(after.stdout, "script_a", `script_a should be stale: ${after.stdout}`);
    });
  },
});

// =============================================================================
// Test 2: Chained imports (A -> B -> C, C changes -> all stale)
// =============================================================================

Deno.test({
  name: "Relative imports: chained imports propagate staleness",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "chain_ws",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      const scriptA = `import { utilB } from "./script_b";
export async function main() { return utilB(); }
`;
      const scriptB = `import { utilC } from "./script_c";
export function utilB() { return utilC() + " B"; }
`;
      const scriptC = `export function utilC() { return "C"; }
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, scriptC);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.script.yaml`, defaultMetadata);

      const hashA = await generateScriptHash(scriptA, defaultMetadata);
      const hashB = await generateScriptHash(scriptB, defaultMetadata);
      const hashC = await generateScriptHash(scriptC, defaultMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": hashA,
        "f/test/script_b": hashB,
        "f/test/script_c": hashC,
      }));

      // Initial check
      const initial = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "chain_ws"
      );
      assertEquals(initial.code, 0);
      assertStringIncludes(initial.stdout, "No metadata to update");

      // Change only script C (the leaf)
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`,
        `export function utilC() { return "C changed"; }
`);

      // All three should be stale
      const after = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "chain_ws"
      );
      assertEquals(after.code, 0);
      assertStringIncludes(after.stdout, "script_c", `script_c should be stale: ${after.stdout}`);
      assertStringIncludes(after.stdout, "script_b", `script_b should be stale: ${after.stdout}`);
      assertStringIncludes(after.stdout, "script_a", `script_a should be stale: ${after.stdout}`);
    });
  },
});

// =============================================================================
// Test 3: Circular imports (A -> B -> A) - should not infinite loop
// =============================================================================

Deno.test({
  name: "Relative imports: circular imports handled gracefully",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "circular_ws",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // Circular: A imports B, B imports A
      const scriptA = `import { funcB } from "./script_b";
export function funcA() { return "A"; }
export async function main() { return funcA() + funcB(); }
`;
      const scriptB = `import { funcA } from "./script_a";
export function funcB() { return "B" + funcA(); }
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);

      const hashA = await generateScriptHash(scriptA, defaultMetadata);
      const hashB = await generateScriptHash(scriptB, defaultMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/script_a": hashA,
        "f/test/script_b": hashB,
      }));

      // Initial check - should complete without hanging
      const initial = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "circular_ws"
      );
      assertEquals(initial.code, 0);
      assertStringIncludes(initial.stdout, "No metadata to update");

      // Change one script in the cycle
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`,
        `import { funcA } from "./script_a";
export function funcB() { return "B changed" + funcA(); }
`);

      // Should complete and both should be stale
      const after = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "circular_ws"
      );
      assertEquals(after.code, 0);
      assertStringIncludes(after.stdout, "script_a", `script_a should be stale: ${after.stdout}`);
      assertStringIncludes(after.stdout, "script_b", `script_b should be stale: ${after.stdout}`);
    });
  },
});

// =============================================================================
// Test 4: Python imports
// =============================================================================

Deno.test({
  name: "Relative imports: Python imports propagate staleness",
  ignore: false,
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "python_ws",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      const mainPy = `from f.test.helper import helper_func

def main():
    return helper_func()
`;
      const helperPy = `def helper_func():
    return "original"
`;

      await Deno.writeTextFile(`${tempDir}/f/test/main.py`, mainPy);
      await Deno.writeTextFile(`${tempDir}/f/test/main.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.py`, helperPy);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.script.yaml`, defaultMetadata);

      const hashMain = await generateScriptHash(mainPy, defaultMetadata);
      const hashHelper = await generateScriptHash(helperPy, defaultMetadata);

      await Deno.writeTextFile(`${tempDir}/wmill-lock.yaml`, createLockfile({
        "f/test/main": hashMain,
        "f/test/helper": hashHelper,
      }));

      // Initial check
      const initial = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "python_ws"
      );
      assertEquals(initial.code, 0);
      assertStringIncludes(initial.stdout, "No metadata to update");

      // Change helper
      await Deno.writeTextFile(`${tempDir}/f/test/helper.py`,
        `def helper_func():
    return "changed"
`);

      // Both should be stale
      const after = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "python_ws"
      );
      assertEquals(after.code, 0);
      assertStringIncludes(after.stdout, "helper", `helper should be stale: ${after.stdout}`);
      assertStringIncludes(after.stdout, "main", `main should be stale: ${after.stdout}`);
    });
  },
});
