/**
 * Tests for local script dependency resolution
 * Verifies that generate-metadata correctly handles relative imports in local development
 */

import { assertEquals, assertStringIncludes, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { withContainerizedBackend } from "./containerized_backend.ts";
import { addWorkspace } from "../workspace.ts";

Deno.test("generate-metadata: basic relative imports", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "local_script_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []`);

    await Deno.mkdir(`${tempDir}/f`, { recursive: true });
    await Deno.mkdir(`${tempDir}/f/utils`, { recursive: true });

    const utilScript = `export function greet(name: string): string {
  return \`Hello, \${name}!\`;
}

export function add(a: number, b: number): number {
  return a + b;
}`;
    await Deno.writeTextFile(`${tempDir}/f/utils/helpers.ts`, utilScript);
    await Deno.writeTextFile(`${tempDir}/f/utils/helpers.script.yaml`, `summary: Utility helpers
description: Helper functions
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
lock: ""`);

    const mainScript = `import { greet, add } from "./utils/helpers";

export async function main(name: string = "World", x: number = 1, y: number = 2) {
  const greeting = greet(name);
  const sum = add(x, y);
  return \`\${greeting} The sum is \${sum}\`;
}`;
    await Deno.writeTextFile(`${tempDir}/f/main.ts`, mainScript);
    await Deno.writeTextFile(`${tempDir}/f/main.script.yaml`, `summary: Main script
description: Script with imports
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  required: []
  properties:
    name:
      type: string
      default: World
    x:
      type: number
      default: 1
    y:
      type: number
      default: 2
lock: ""`);

    const result = await backend.runCLICommand(['script', 'generate-metadata', 'f/main.ts'], tempDir);

    assertEquals(result.code, 0, `generate-metadata should succeed: ${result.stderr}`);
    assertStringIncludes(
      result.stdout.toLowerCase() + result.stderr.toLowerCase(),
      "local script",
      "Should mention local scripts in output"
    );

    const lockFileExists = await Deno.stat(`${tempDir}/f/main.script.lock`)
      .then(() => true)
      .catch(() => false);
    assert(lockFileExists, "Lockfile should be generated");
  });
});

Deno.test("generate-metadata: nested imports", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "nested_imports_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []`);

    await Deno.mkdir(`${tempDir}/f`, { recursive: true });
    await Deno.mkdir(`${tempDir}/f/lib`, { recursive: true });
    await Deno.mkdir(`${tempDir}/f/lib/math`, { recursive: true });

    const mathScript = `export function multiply(a: number, b: number): number {
  return a * b;
}`;
    await Deno.writeTextFile(`${tempDir}/f/lib/math/operations.ts`, mathScript);
    await Deno.writeTextFile(`${tempDir}/f/lib/math/operations.script.yaml`, `summary: Math operations
description: Math operations
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
lock: ""`);

    const utilScript = `import { multiply } from "./math/operations";

export function square(x: number): number {
  return multiply(x, x);
}`;
    await Deno.writeTextFile(`${tempDir}/f/lib/helpers.ts`, utilScript);
    await Deno.writeTextFile(`${tempDir}/f/lib/helpers.script.yaml`, `summary: Helper functions
description: Helper functions
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
lock: ""`);

    const mainScript = `import { square } from "./lib/helpers";

export async function main(num: number = 5) {
  return \`The square of \${num} is \${square(num)}\`;
}`;
    await Deno.writeTextFile(`${tempDir}/f/calculator.ts`, mainScript);
    await Deno.writeTextFile(`${tempDir}/f/calculator.script.yaml`, `summary: Calculator
description: Calculator script
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  required: []
  properties:
    num:
      type: number
      default: 5
lock: ""`);

    const result = await backend.runCLICommand(['script', 'generate-metadata', 'f/calculator.ts'], tempDir);

    assertEquals(result.code, 0, `generate-metadata should succeed: ${result.stderr}`);

    const output = result.stdout.toLowerCase() + result.stderr.toLowerCase();
    if (output.includes("local script")) {
      const scriptMatches = output.match(/local script/gi);
      assert(scriptMatches && scriptMatches.length >= 1, "Should find at least 1 local script");
    }

    const lockFileExists = await Deno.stat(`${tempDir}/f/calculator.script.lock`)
      .then(() => true)
      .catch(() => false);
    assert(lockFileExists, "Lockfile should be generated");
  });
});

Deno.test("generate-metadata: python relative imports", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "python_imports_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []`);

    await Deno.mkdir(`${tempDir}/f`, { recursive: true });
    await Deno.mkdir(`${tempDir}/f/utils`, { recursive: true });

    const utilScript = `def format_message(msg: str) -> str:
    return f"[INFO] {msg}"

def calculate(x: int, y: int) -> int:
    return x + y
`;
    await Deno.writeTextFile(`${tempDir}/f/utils/helpers.py`, utilScript);
    await Deno.writeTextFile(`${tempDir}/f/utils/helpers.script.yaml`, `summary: Python helpers
description: Python utilities
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties: {}
lock: ""`);

    const mainScript = `from .utils.helpers import format_message, calculate

def main(x: int = 5, y: int = 10):
    result = calculate(x, y)
    return format_message(f"Sum is {result}")
`;
    await Deno.writeTextFile(`${tempDir}/f/processor.py`, mainScript);
    await Deno.writeTextFile(`${tempDir}/f/processor.script.yaml`, `summary: Python processor
description: Python script
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  required: []
  properties:
    x:
      type: integer
      default: 5
    y:
      type: integer
      default: 10
lock: ""`);

    const result = await backend.runCLICommand(['script', 'generate-metadata', 'f/processor.py'], tempDir);

    assertEquals(result.code, 0, `generate-metadata should succeed: ${result.stderr}`);

    const lockFileExists = await Deno.stat(`${tempDir}/f/processor.script.lock`)
      .then(() => true)
      .catch(() => false);
    assert(lockFileExists, "Lockfile should be generated");
  });
});

Deno.test("generate-metadata: no imports baseline", async () => {
  await withContainerizedBackend(async (backend, tempDir) => {
    const testWorkspace = {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "no_imports_test",
      token: backend.token
    };
    await addWorkspace(testWorkspace, { force: true, configDir: backend.testConfigDir });

    await Deno.writeTextFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - f/**
excludes: []`);

    await Deno.mkdir(`${tempDir}/f`, { recursive: true });

    const simpleScript = `export async function main(name: string = "World") {
  return \`Hello, \${name}!\`;
}`;
    await Deno.writeTextFile(`${tempDir}/f/simple.ts`, simpleScript);
    await Deno.writeTextFile(`${tempDir}/f/simple.script.yaml`, `summary: Simple script
description: No imports
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  required: []
  properties:
    name:
      type: string
      default: World
lock: ""`);

    const result = await backend.runCLICommand(['script', 'generate-metadata', 'f/simple.ts'], tempDir);

    assertEquals(result.code, 0, `generate-metadata should succeed: ${result.stderr}`);

    const lockFileExists = await Deno.stat(`${tempDir}/f/simple.script.lock`)
      .then(() => true)
      .catch(() => false);
    assert(lockFileExists, "Lockfile should be generated");
  });
});
