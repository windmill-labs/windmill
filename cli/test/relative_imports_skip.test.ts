/**
 * Relative Imports Tests
 *
 * E2E tests for `generate-metadata` command with relative imports:
 * - Lock files correctly include transitive dependencies
 * - Staleness propagates through import chains
 * - Various import patterns handled correctly
 */

import { assertEquals, assertStringIncludes, assertNotMatch } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/mod.ts";

const defaultMetadata = `summary: "Test"
schema:
  type: object
  properties: {}
lock: ""
`;

// =============================================================================
// Test 1: TS basic import with npm dependency propagation
// =============================================================================

Deno.test({
  name: "TS: imported script's npm dep appears in importer's lock",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "ts_basic",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      const scriptA = `import { helper } from "./script_b";
export async function main() { return helper(); }
`;
      const scriptB = `import _ from "lodash";
export function helper() { return _.VERSION; }
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);

      const result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "ts_basic"
      );
      assertEquals(result.code, 0, `generate-metadata failed: ${result.stderr}\n${result.stdout}`);

      const lockA = await Deno.readTextFile(`${tempDir}/f/test/script_a.script.lock`).catch(() => "");
      const lockB = await Deno.readTextFile(`${tempDir}/f/test/script_b.script.lock`).catch(() => "");

      assertStringIncludes(lockB, "lodash", `script_b should have lodash: ${lockB}`);
      assertStringIncludes(lockA, "lodash", `script_a should have lodash (from script_b): ${lockA}`);
    });
  },
});

// =============================================================================
// Test 2: TS chained imports - dependency propagates through chain
// =============================================================================

Deno.test({
  name: "TS: chained imports propagate npm deps through entire chain",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "ts_chain",
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
      const scriptC = `import _ from "lodash";
export function utilC() { return _.VERSION; }
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, scriptC);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.script.yaml`, defaultMetadata);

      const result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "ts_chain"
      );
      assertEquals(result.code, 0, `generate-metadata failed: ${result.stderr}\n${result.stdout}`);

      const lockA = await Deno.readTextFile(`${tempDir}/f/test/script_a.script.lock`).catch(() => "");
      const lockB = await Deno.readTextFile(`${tempDir}/f/test/script_b.script.lock`).catch(() => "");
      const lockC = await Deno.readTextFile(`${tempDir}/f/test/script_c.script.lock`).catch(() => "");

      assertStringIncludes(lockC, "lodash", `script_c should have lodash: ${lockC}`);
      assertStringIncludes(lockB, "lodash", `script_b should have lodash (from c): ${lockB}`);
      assertStringIncludes(lockA, "lodash", `script_a should have lodash (from b->c): ${lockA}`);
    });
  },
});

// =============================================================================
// Test 3: TS circular imports - completes without hanging, locks generated
// =============================================================================

Deno.test({
  name: "TS: circular imports handled gracefully with correct locks",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "ts_circular",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // Circular: A imports B, B imports A, B has npm dep
      const scriptA = `import { funcB } from "./script_b";
export function funcA() { return "A"; }
export async function main() { return funcA() + funcB(); }
`;
      const scriptB = `import { funcA } from "./script_a";
import _ from "lodash";
export function funcB() { return _.VERSION + funcA(); }
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);

      const result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "ts_circular"
      );
      assertEquals(result.code, 0, `generate-metadata failed: ${result.stderr}\n${result.stdout}`);

      const lockA = await Deno.readTextFile(`${tempDir}/f/test/script_a.script.lock`).catch(() => "");
      const lockB = await Deno.readTextFile(`${tempDir}/f/test/script_b.script.lock`).catch(() => "");

      assertStringIncludes(lockB, "lodash", `script_b should have lodash: ${lockB}`);
      assertStringIncludes(lockA, "lodash", `script_a should have lodash (circular dep): ${lockA}`);
    });
  },
});

// =============================================================================
// Test 4: Python basic import with pip dependency propagation
// =============================================================================

Deno.test({
  name: "Python: imported script's pip dep appears in importer's lock",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "py_basic",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      const mainPy = `from f.test.helper import helper_func

def main():
    return helper_func()
`;
      const helperPy = `import requests

def helper_func():
    return requests.__version__
`;

      await Deno.writeTextFile(`${tempDir}/f/test/main.py`, mainPy);
      await Deno.writeTextFile(`${tempDir}/f/test/main.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.py`, helperPy);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.script.yaml`, defaultMetadata);

      const result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "py_basic"
      );
      assertEquals(result.code, 0, `generate-metadata failed: ${result.stderr}\n${result.stdout}`);

      const lockMain = await Deno.readTextFile(`${tempDir}/f/test/main.script.lock`).catch(() => "");
      const lockHelper = await Deno.readTextFile(`${tempDir}/f/test/helper.script.lock`).catch(() => "");

      assertStringIncludes(lockHelper, "requests", `helper should have requests: ${lockHelper}`);
      assertStringIncludes(lockMain, "requests", `main should have requests (from helper): ${lockMain}`);
    });
  },
});

// =============================================================================
// Test 5: Diamond dependency - A imports B and C, both import D
// =============================================================================

Deno.test({
  name: "Python: diamond dependency pattern propagates correctly",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "py_diamond",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // Diamond: A -> B, A -> C, B -> D, C -> D
      const scriptA = `from f.test.script_b import func_b
from f.test.script_c import func_c

def main():
    return func_b() + func_c()
`;
      const scriptB = `from f.test.script_d import func_d

def func_b():
    return "B" + func_d()
`;
      const scriptC = `from f.test.script_d import func_d

def func_c():
    return "C" + func_d()
`;
      const scriptD = `import requests

def func_d():
    return requests.__version__
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.py`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.py`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.py`, scriptC);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_d.py`, scriptD);
      await Deno.writeTextFile(`${tempDir}/f/test/script_d.script.yaml`, defaultMetadata);

      const result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "py_diamond"
      );
      assertEquals(result.code, 0, `generate-metadata failed: ${result.stderr}\n${result.stdout}`);

      const lockA = await Deno.readTextFile(`${tempDir}/f/test/script_a.script.lock`).catch(() => "");
      const lockB = await Deno.readTextFile(`${tempDir}/f/test/script_b.script.lock`).catch(() => "");
      const lockC = await Deno.readTextFile(`${tempDir}/f/test/script_c.script.lock`).catch(() => "");
      const lockD = await Deno.readTextFile(`${tempDir}/f/test/script_d.script.lock`).catch(() => "");

      assertStringIncludes(lockD, "requests", `script_d should have requests: ${lockD}`);
      assertStringIncludes(lockB, "requests", `script_b should have requests (from d): ${lockB}`);
      assertStringIncludes(lockC, "requests", `script_c should have requests (from d): ${lockC}`);
      assertStringIncludes(lockA, "requests", `script_a should have requests (from b,c->d): ${lockA}`);
    });
  },
});

// =============================================================================
// Test 6: Script isolation - unrelated script not marked stale
// =============================================================================

Deno.test({
  name: "Script isolation: unrelated script not affected by changes",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "isolation",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // A imports B, C is isolated
      const scriptA = `import { helper } from "./script_b";
export async function main() { return helper(); }
`;
      const scriptB = `export function helper() { return "B"; }
`;
      const scriptC = `export async function main() { return "isolated"; }
`;

      await Deno.writeTextFile(`${tempDir}/f/test/script_a.ts`, scriptA);
      await Deno.writeTextFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`, scriptB);
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.ts`, scriptC);
      await Deno.writeTextFile(`${tempDir}/f/test/script_c.script.yaml`, defaultMetadata);

      // Generate initial metadata
      const initial = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "isolation"
      );
      assertEquals(initial.code, 0, `initial failed: ${initial.stderr}\n${initial.stdout}`);

      // Verify all up to date
      const check1 = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "isolation"
      );
      assertStringIncludes(check1.stdout, "No metadata to update");

      // Change script_b
      await Deno.writeTextFile(`${tempDir}/f/test/script_b.ts`,
        `export function helper() { return "B changed"; }
`);

      // script_a and script_b should be stale, script_c should NOT be mentioned
      const check2 = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
        tempDir, "isolation"
      );
      assertEquals(check2.code, 0);
      assertStringIncludes(check2.stdout, "script_b", `script_b should be stale`);
      assertStringIncludes(check2.stdout, "script_a", `script_a should be stale`);
      assertNotMatch(check2.stdout, /script_c/, `script_c should NOT be stale`);
    });
  },
});

// =============================================================================
// Test 7: Python relative imports with dot syntax
// =============================================================================

Deno.test({
  name: "Python: relative imports with dot syntax work correctly",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "py_relative",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/mymodule`);

      // Using relative import syntax
      const mainPy = `from .helper import helper_func

def main():
    return helper_func()
`;
      const helperPy = `import requests

def helper_func():
    return requests.__version__
`;

      await Deno.writeTextFile(`${tempDir}/f/mymodule/main.py`, mainPy);
      await Deno.writeTextFile(`${tempDir}/f/mymodule/main.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/mymodule/helper.py`, helperPy);
      await Deno.writeTextFile(`${tempDir}/f/mymodule/helper.script.yaml`, defaultMetadata);

      const result = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/mymodule/*", "--yes"],
        tempDir, "py_relative"
      );
      assertEquals(result.code, 0, `generate-metadata failed: ${result.stderr}\n${result.stdout}`);

      const lockMain = await Deno.readTextFile(`${tempDir}/f/mymodule/main.script.lock`).catch(() => "");
      const lockHelper = await Deno.readTextFile(`${tempDir}/f/mymodule/helper.script.lock`).catch(() => "");

      assertStringIncludes(lockHelper, "requests", `helper should have requests: ${lockHelper}`);
      assertStringIncludes(lockMain, "requests", `main should have requests (from .helper): ${lockMain}`);
    });
  },
});

// =============================================================================
// Test 8: Adding new import updates importer's lock
// =============================================================================

Deno.test({
  name: "Python: adding new import updates importer's lock correctly",
  sanitizeResources: false,
  sanitizeOps: false,
  fn: async () => {
    await withTestBackend(async (backend, tempDir) => {
      await addWorkspace({
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "py_new_import",
        token: backend.token ?? ""
      }, { force: true, configDir: backend.testConfigDir });

      await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

      await ensureDir(`${tempDir}/f/test`);

      // Initial: main imports helper, helper has no external deps
      const mainPy = `from f.test.helper import helper_func

def main():
    return helper_func()
`;
      const helperPyInitial = `def helper_func():
    return "no deps"
`;

      await Deno.writeTextFile(`${tempDir}/f/test/main.py`, mainPy);
      await Deno.writeTextFile(`${tempDir}/f/test/main.script.yaml`, defaultMetadata);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.py`, helperPyInitial);
      await Deno.writeTextFile(`${tempDir}/f/test/helper.script.yaml`, defaultMetadata);

      // Generate initial locks
      const initial = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "py_new_import"
      );
      assertEquals(initial.code, 0, `initial failed: ${initial.stderr}\n${initial.stdout}`);

      // Add new script with pip dep
      const utilsPy = `import requests

def get_version():
    return requests.__version__
`;
      await Deno.writeTextFile(`${tempDir}/f/test/utils.py`, utilsPy);
      await Deno.writeTextFile(`${tempDir}/f/test/utils.script.yaml`, defaultMetadata);

      // Modify helper to import utils
      const helperPyWithImport = `from f.test.utils import get_version

def helper_func():
    return get_version()
`;
      await Deno.writeTextFile(`${tempDir}/f/test/helper.py`, helperPyWithImport);

      // Regenerate - main should now have requests
      const afterAdd = await backend.runCLICommand(
        ["generate-metadata", "-i", "f/test/*", "--yes"],
        tempDir, "py_new_import"
      );
      assertEquals(afterAdd.code, 0, `after add failed: ${afterAdd.stderr}\n${afterAdd.stdout}`);

      const lockUtils = await Deno.readTextFile(`${tempDir}/f/test/utils.script.lock`).catch(() => "");
      const lockHelper = await Deno.readTextFile(`${tempDir}/f/test/helper.script.lock`).catch(() => "");
      const lockMain = await Deno.readTextFile(`${tempDir}/f/test/main.script.lock`).catch(() => "");

      assertStringIncludes(lockUtils, "requests", `utils should have requests: ${lockUtils}`);
      assertStringIncludes(lockHelper, "requests", `helper should have requests: ${lockHelper}`);
      assertStringIncludes(lockMain, "requests", `main should have requests: ${lockMain}`);
    });
  },
});
