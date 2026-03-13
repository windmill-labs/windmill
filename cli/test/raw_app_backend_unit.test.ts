import { expect, test, describe, beforeEach, afterEach } from "bun:test";
import { loadRunnablesFromBackend } from "../src/commands/app/raw_apps.ts";
import * as path from "node:path";
import { writeFile, rm, mkdir } from "node:fs/promises";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";

// =============================================================================
// UNIT TESTS: loadRunnablesFromBackend
// Tests for the raw app backend folder runnable loading
// =============================================================================

describe("loadRunnablesFromBackend", () => {
  let tempDir: string;
  let backendPath: string;

  beforeEach(async () => {
    tempDir = await mkdtemp(path.join(tmpdir(), "raw-app-test-"));
    backendPath = path.join(tempDir, "backend");
    await mkdir(backendPath, { recursive: true });
  });

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true });
  });

  test("loads inline runnable from yaml + ts files", async () => {
    // Create a proper inline runnable
    await writeFile(
      path.join(backendPath, "my_runnable.yaml"),
      "type: inline\n",
      "utf-8"
    );
    await writeFile(
      path.join(backendPath, "my_runnable.ts"),
      "export async function main(x: number) { return x; }",
      "utf-8"
    );

    const runnables = await loadRunnablesFromBackend(backendPath);

    expect(Object.keys(runnables)).toEqual(["my_runnable"]);
    expect(runnables["my_runnable"].type).toEqual("inline");
    expect(runnables["my_runnable"].inlineScript?.content).toContain("export async function main");
    expect(runnables["my_runnable"].inlineScript?.language).toEqual("bun");
  });

  test("loads path-based runnable (script type)", async () => {
    // Create a path-based runnable that references an external script
    await writeFile(
      path.join(backendPath, "external.yaml"),
      `type: script
path: u/admin/my_script
`,
      "utf-8"
    );

    const runnables = await loadRunnablesFromBackend(backendPath);

    expect(Object.keys(runnables)).toEqual(["external"]);
    expect(runnables["external"].type).toEqual("path");
    expect(runnables["external"].runType).toEqual("script");
    expect(runnables["external"].path).toEqual("u/admin/my_script");
  });

  test("ignores .script.yaml standalone metadata files", async () => {
    // Create a proper inline runnable
    await writeFile(
      path.join(backendPath, "a.yaml"),
      "type: inline\n",
      "utf-8"
    );
    await writeFile(
      path.join(backendPath, "a.ts"),
      "export async function main(x: number) { return x; }",
      "utf-8"
    );

    // Create standalone script metadata files that should be IGNORED
    // These have 'kind: script' format, not 'type: inline'
    await writeFile(
      path.join(backendPath, "a.script.yaml"),
      `summary: ''
description: ''
lock: '!inline some/path/a.script.lock'
kind: script
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
  properties:
    x:
      type: number
  required:
    - x
`,
      "utf-8"
    );
    await writeFile(
      path.join(backendPath, "a.script.lock"),
      `{
  "dependencies": {}
}
//bun.lock
<empty>`,
      "utf-8"
    );

    const runnables = await loadRunnablesFromBackend(backendPath);

    // Should only load 'a' (from a.yaml), NOT 'a.script' (from a.script.yaml)
    expect(Object.keys(runnables)).toEqual(["a"]);
    expect(runnables["a"].type).toEqual("inline");
    expect(runnables["a.script"]).toBeUndefined();
  });

  test("ignores .flow.yaml standalone metadata files", async () => {
    // Create a proper inline runnable
    await writeFile(
      path.join(backendPath, "b.yaml"),
      "type: inline\n",
      "utf-8"
    );
    await writeFile(
      path.join(backendPath, "b.ts"),
      "export async function main() { return 'hello'; }",
      "utf-8"
    );

    // Create standalone flow metadata file that should be IGNORED
    await writeFile(
      path.join(backendPath, "myflow.flow.yaml"),
      `summary: My Flow
description: A test flow
schema:
  type: object
`,
      "utf-8"
    );

    const runnables = await loadRunnablesFromBackend(backendPath);

    // Should only load 'b', NOT 'myflow.flow'
    expect(Object.keys(runnables)).toEqual(["b"]);
    expect(runnables["myflow.flow"]).toBeUndefined();
  });

  test("ignores .resource.yaml standalone metadata files", async () => {
    // Create a proper inline runnable
    await writeFile(
      path.join(backendPath, "c.yaml"),
      "type: inline\n",
      "utf-8"
    );
    await writeFile(
      path.join(backendPath, "c.py"),
      "def main(): return 'hello'",
      "utf-8"
    );

    // Create standalone resource metadata file that should be IGNORED
    await writeFile(
      path.join(backendPath, "mydb.resource.yaml"),
      `value:
  host: localhost
  port: 5432
resource_type: postgresql
`,
      "utf-8"
    );

    const runnables = await loadRunnablesFromBackend(backendPath);

    // Should only load 'c', NOT 'mydb.resource'
    expect(Object.keys(runnables)).toEqual(["c"]);
    expect(runnables["mydb.resource"]).toBeUndefined();
  });

  test("handles empty backend folder", async () => {
    const runnables = await loadRunnablesFromBackend(backendPath);
    expect(Object.keys(runnables)).toEqual([]);
  });

  test("handles non-existent backend folder", async () => {
    const nonExistentPath = path.join(tempDir, "does_not_exist");
    const runnables = await loadRunnablesFromBackend(nonExistentPath);
    expect(Object.keys(runnables)).toEqual([]);
  });

  test("auto-detects code-only runnables without yaml config", async () => {
    // Create just a code file without corresponding yaml
    await writeFile(
      path.join(backendPath, "auto_detect.ts"),
      "export async function main(name: string) { return `Hello ${name}`; }",
      "utf-8"
    );

    const runnables = await loadRunnablesFromBackend(backendPath);

    expect(Object.keys(runnables)).toEqual(["auto_detect"]);
    expect(runnables["auto_detect"].type).toEqual("inline");
    expect(runnables["auto_detect"].inlineScript?.content).toContain("Hello");
    expect(runnables["auto_detect"].inlineScript?.language).toEqual("bun");
  });

  test("multiple runnables with mixed formats", async () => {
    // Inline runnable with yaml config
    await writeFile(path.join(backendPath, "with_config.yaml"), "type: inline\n", "utf-8");
    await writeFile(path.join(backendPath, "with_config.ts"), "export async function main() { return 1; }", "utf-8");

    // Auto-detected code-only runnable
    await writeFile(path.join(backendPath, "auto.py"), "def main(): return 2", "utf-8");

    // Path-based runnable
    await writeFile(path.join(backendPath, "external.yaml"), "type: flow\npath: f/myflow\n", "utf-8");

    // Standalone metadata files to ignore
    await writeFile(path.join(backendPath, "standalone.script.yaml"), "kind: script\n", "utf-8");
    await writeFile(path.join(backendPath, "another.flow.yaml"), "summary: flow\n", "utf-8");

    const runnables = await loadRunnablesFromBackend(backendPath);

    const keys = Object.keys(runnables).sort();
    expect(keys).toEqual(["auto", "external", "with_config"]);

    // Verify types
    expect(runnables["with_config"].type).toEqual("inline");
    expect(runnables["auto"].type).toEqual("inline");
    expect(runnables["external"].type).toEqual("path");
    expect(runnables["external"].runType).toEqual("flow");
  });
});
