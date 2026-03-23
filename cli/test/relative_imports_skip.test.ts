/**
 * Relative Imports Tests
 *
 * E2E tests for the `generate-metadata` command with relative imports:
 * - Lock files correctly include transitive dependencies
 * - Staleness propagates through import chains
 * - Various import patterns handled correctly
 */

import { expect, test } from "bun:test";
import { writeFile, readFile, mkdir } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";

// TODO: re-enable Python tests on CI if python feature is included by default
const isCI = process.env["CI_MINIMAL_FEATURES"] === "true";

const defaultMetadata = `summary: "Test"
schema:
  type: object
  properties: {}
lock: ""
`;

// =============================================================================
// Test 1: TS basic import with npm dependency propagation
// =============================================================================

test("TS: imported script's npm dep appears in importer's lock", { timeout: 60000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/test`, { recursive: true });

    const scriptA = `import { helper } from "./script_b";
export async function main() { return helper(); }
`;
    const scriptB = `import _ from "lodash";
export function helper() { return _.VERSION; }
`;

    await writeFile(`${tempDir}/f/test/script_a.ts`, scriptA);
    await writeFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_b.ts`, scriptB);
    await writeFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);

    const result = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    expect(result.code, `generate-metadata failed:\nSTDOUT: ${result.stdout}\nSTDERR: ${result.stderr}`).toBe(0);

    const lockA = await readFile(`${tempDir}/f/test/script_a.script.lock`, "utf-8").catch(() => "");
    const lockB = await readFile(`${tempDir}/f/test/script_b.script.lock`, "utf-8").catch(() => "");

    expect(lockB).toContain("lodash");
    expect(lockA).toContain("lodash");
  });
});

// =============================================================================
// Test 2: TS chained imports - dependency propagates through chain
// =============================================================================

test("TS: chained imports propagate npm deps through entire chain", { timeout: 60000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/test`, { recursive: true });

    const scriptA = `import { utilB } from "./script_b";
export async function main() { return utilB(); }
`;
    const scriptB = `import { utilC } from "./script_c";
export function utilB() { return utilC() + " B"; }
`;
    const scriptC = `import _ from "lodash";
export function utilC() { return _.VERSION; }
`;

    await writeFile(`${tempDir}/f/test/script_a.ts`, scriptA);
    await writeFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_b.ts`, scriptB);
    await writeFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_c.ts`, scriptC);
    await writeFile(`${tempDir}/f/test/script_c.script.yaml`, defaultMetadata);

    const result = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    expect(result.code).toBe(0);

    const lockA = await readFile(`${tempDir}/f/test/script_a.script.lock`, "utf-8").catch(() => "");
    const lockB = await readFile(`${tempDir}/f/test/script_b.script.lock`, "utf-8").catch(() => "");
    const lockC = await readFile(`${tempDir}/f/test/script_c.script.lock`, "utf-8").catch(() => "");

    expect(lockC).toContain("lodash");
    expect(lockB).toContain("lodash");
    expect(lockA).toContain("lodash");
  });
});

// =============================================================================
// Test 3: TS circular imports - completes without hanging, locks generated
// =============================================================================

test("TS: circular imports handled gracefully with correct locks", { timeout: 60000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/test`, { recursive: true });

    // Circular: A imports B, B imports A, B has npm dep
    const scriptA = `import { funcB } from "./script_b";
export function funcA() { return "A"; }
export async function main() { return funcA() + funcB(); }
`;
    const scriptB = `import { funcA } from "./script_a";
import _ from "lodash";
export function funcB() { return _.VERSION + funcA(); }
`;

    await writeFile(`${tempDir}/f/test/script_a.ts`, scriptA);
    await writeFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_b.ts`, scriptB);
    await writeFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);

    const result = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    expect(result.code).toBe(0);

    const lockA = await readFile(`${tempDir}/f/test/script_a.script.lock`, "utf-8").catch(() => "");
    const lockB = await readFile(`${tempDir}/f/test/script_b.script.lock`, "utf-8").catch(() => "");

    expect(lockB).toContain("lodash");
    expect(lockA).toContain("lodash");
  });
});

// =============================================================================
// Test 4: Python basic import with pip dependency propagation
// =============================================================================

test.skipIf(isCI)("Python: imported script's pip dep appears in importer's lock", { timeout: 60000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/test`, { recursive: true });

    const mainPy = `from f.test.helper import helper_func

def main():
    return helper_func()
`;
    const helperPy = `import requests

def helper_func():
    return requests.__version__
`;

    await writeFile(`${tempDir}/f/test/main.py`, mainPy);
    await writeFile(`${tempDir}/f/test/main.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/helper.py`, helperPy);
    await writeFile(`${tempDir}/f/test/helper.script.yaml`, defaultMetadata);

    const result = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    if (result.code !== 0) {
      console.log("STDOUT:", result.stdout);
      console.log("STDERR:", result.stderr);
    }
    expect(result.code).toBe(0);

    const lockMain = await readFile(`${tempDir}/f/test/main.script.lock`, "utf-8").catch(() => "");
    const lockHelper = await readFile(`${tempDir}/f/test/helper.script.lock`, "utf-8").catch(() => "");

    expect(lockHelper).toContain("requests");
    expect(lockMain).toContain("requests");
  });
});

// =============================================================================
// Test 5: Diamond dependency - A imports B and C, both import D
// =============================================================================

test.skipIf(isCI)("Python: diamond dependency pattern propagates correctly", { timeout: 60000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/test`, { recursive: true });

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

    await writeFile(`${tempDir}/f/test/script_a.py`, scriptA);
    await writeFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_b.py`, scriptB);
    await writeFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_c.py`, scriptC);
    await writeFile(`${tempDir}/f/test/script_c.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_d.py`, scriptD);
    await writeFile(`${tempDir}/f/test/script_d.script.yaml`, defaultMetadata);

    const result = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    expect(result.code).toBe(0);

    const lockA = await readFile(`${tempDir}/f/test/script_a.script.lock`, "utf-8").catch(() => "");
    const lockB = await readFile(`${tempDir}/f/test/script_b.script.lock`, "utf-8").catch(() => "");
    const lockC = await readFile(`${tempDir}/f/test/script_c.script.lock`, "utf-8").catch(() => "");
    const lockD = await readFile(`${tempDir}/f/test/script_d.script.lock`, "utf-8").catch(() => "");

    expect(lockD).toContain("requests");
    expect(lockB).toContain("requests");
    expect(lockC).toContain("requests");
    expect(lockA).toContain("requests");
  });
});

// =============================================================================
// Test 6: Script isolation - unrelated script not marked stale
// =============================================================================

test("Script isolation: unrelated script not affected by changes", { timeout: 120000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/test`, { recursive: true });

    // A imports B, C is isolated
    const scriptA = `import { helper } from "./script_b";
export async function main() { return helper(); }
`;
    const scriptB = `export function helper() { return "B"; }
`;
    const scriptC = `export async function main() { return "isolated"; }
`;

    await writeFile(`${tempDir}/f/test/script_a.ts`, scriptA);
    await writeFile(`${tempDir}/f/test/script_a.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_b.ts`, scriptB);
    await writeFile(`${tempDir}/f/test/script_b.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/script_c.ts`, scriptC);
    await writeFile(`${tempDir}/f/test/script_c.script.yaml`, defaultMetadata);

    // Generate initial metadata
    const initial = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    expect(initial.code).toBe(0);

    // Verify all up to date
    const check1 = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
      tempDir
    );
    expect(check1.stdout).toContain("All metadata up-to-date");

    // Change script_b
    await writeFile(`${tempDir}/f/test/script_b.ts`,
      `export function helper() { return "B changed"; }
`);

    // script_a and script_b should be stale, script_c should NOT be mentioned
    const check2 = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes", "--dry-run"],
      tempDir
    );
    expect(check2.code).toBe(0);
    expect(check2.stdout).toContain("script_b");
    expect(check2.stdout).toContain("script_a");
    expect(check2.stdout).not.toMatch(/script_c/);
  });
});

// =============================================================================
// Test 7: Python relative imports with dot syntax
// =============================================================================

test.skipIf(isCI)("Python: relative imports with dot syntax work correctly", { timeout: 60000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/mymodule`, { recursive: true });

    // Using relative import syntax
    const mainPy = `from .helper import helper_func

def main():
    return helper_func()
`;
    const helperPy = `import requests

def helper_func():
    return requests.__version__
`;

    await writeFile(`${tempDir}/f/mymodule/main.py`, mainPy);
    await writeFile(`${tempDir}/f/mymodule/main.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/mymodule/helper.py`, helperPy);
    await writeFile(`${tempDir}/f/mymodule/helper.script.yaml`, defaultMetadata);

    const result = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/mymodule/*", "--yes"],
      tempDir
    );
    expect(result.code).toBe(0);

    const lockMain = await readFile(`${tempDir}/f/mymodule/main.script.lock`, "utf-8").catch(() => "");
    const lockHelper = await readFile(`${tempDir}/f/mymodule/helper.script.lock`, "utf-8").catch(() => "");

    expect(lockHelper).toContain("requests");
    expect(lockMain).toContain("requests");
  });
});

// =============================================================================
// Test 8: Adding new import updates importer's lock
// =============================================================================

test.skipIf(isCI)("Python: adding new import updates importer's lock correctly", { timeout: 120000 }, async () => {
  await withTestBackend(async (backend, tempDir) => {
    await writeFile(`${tempDir}/wmill.yaml`, `includes: ["**"]
excludes: []`);

    await mkdir(`${tempDir}/f/test`, { recursive: true });

    // Initial: main imports helper, helper has no external deps
    const mainPy = `from f.test.helper import helper_func

def main():
    return helper_func()
`;
    const helperPyInitial = `def helper_func():
    return "no deps"
`;

    await writeFile(`${tempDir}/f/test/main.py`, mainPy);
    await writeFile(`${tempDir}/f/test/main.script.yaml`, defaultMetadata);
    await writeFile(`${tempDir}/f/test/helper.py`, helperPyInitial);
    await writeFile(`${tempDir}/f/test/helper.script.yaml`, defaultMetadata);

    // Generate initial locks
    const initial = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    expect(initial.code).toBe(0);

    // Add new script with pip dep
    const utilsPy = `import requests

def get_version():
    return requests.__version__
`;
    await writeFile(`${tempDir}/f/test/utils.py`, utilsPy);
    await writeFile(`${tempDir}/f/test/utils.script.yaml`, defaultMetadata);

    // Modify helper to import utils
    const helperPyWithImport = `from f.test.utils import get_version

def helper_func():
    return get_version()
`;
    await writeFile(`${tempDir}/f/test/helper.py`, helperPyWithImport);

    // Regenerate - main should now have requests
    const afterAdd = await backend.runCLICommand(
      ["generate-metadata", "-i", "f/test/*", "--yes"],
      tempDir
    );
    expect(afterAdd.code).toBe(0);

    const lockUtils = await readFile(`${tempDir}/f/test/utils.script.lock`, "utf-8").catch(() => "");
    const lockHelper = await readFile(`${tempDir}/f/test/helper.script.lock`, "utf-8").catch(() => "");
    const lockMain = await readFile(`${tempDir}/f/test/main.script.lock`, "utf-8").catch(() => "");

    expect(lockUtils).toContain("requests");
    expect(lockHelper).toContain("requests");
    expect(lockMain).toContain("requests");
  });
});
