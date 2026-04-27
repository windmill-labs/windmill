/**
 * Unit tests for `wmill app dev` wmill.d.ts generation.
 *
 * Two regressions covered:
 *
 *  1. Path-based runnables (type: script / flow) used to lose their schema and
 *     always render as `args: {}` in wmill.d.ts. Verified by feeding a
 *     fixture-supplied schema through the load -> override -> generate pipeline.
 *
 *  2. Inline runnables stored in the new backend-folder format (a `*.yaml`
 *     declaring `type: inline` plus a sibling code file) used to bail out of
 *     `inferRunnableSchemaFromFile` because the YAML no longer carries an
 *     `inlineScript` block. Verified by inferring a real schema from a TS file
 *     in that layout and confirming the generated wmill.d.ts uses it.
 *
 * These tests run without a backend - they exercise the local pipeline only.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { loadRunnablesFromBackend } from "../src/commands/app/raw_apps.ts";
import {
  APP_BACKEND_FOLDER,
  inferAllInlineSchemas,
  inferRunnableSchemaFromFile,
} from "../src/commands/app/app_metadata.ts";
import { buildWmillTs } from "../src/commands/app/dev.ts";

let tempDir: string;
let backendDir: string;

beforeEach(() => {
  tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "app-dev-wmill-dts-"));
  backendDir = path.join(tempDir, APP_BACKEND_FOLDER);
  fs.mkdirSync(backendDir, { recursive: true });
});

afterEach(() => {
  fs.rmSync(tempDir, { recursive: true, force: true });
});

describe("loadRunnablesFromBackend", () => {
  test("path-based runnable is converted to API format with runType", async () => {
    fs.writeFileSync(
      path.join(backendDir, "myScript.yaml"),
      "type: script\npath: u/admin/my_script\nfields: {}\n",
      "utf-8",
    );

    const runnables = await loadRunnablesFromBackend(backendDir);

    expect(runnables.myScript).toBeDefined();
    expect(runnables.myScript.type).toBe("path");
    expect(runnables.myScript.runType).toBe("script");
    expect(runnables.myScript.path).toBe("u/admin/my_script");
  });

  test("new-format inline runnable reconstructs inlineScript from sibling code", async () => {
    fs.writeFileSync(
      path.join(backendDir, "inlineRunnable.yaml"),
      "type: inline\nfields: {}\n",
      "utf-8",
    );
    fs.writeFileSync(
      path.join(backendDir, "inlineRunnable.ts"),
      "export async function main(name: string, count: number) {\n" +
        "  return { name, count };\n" +
        "}\n",
      "utf-8",
    );

    const runnables = await loadRunnablesFromBackend(backendDir);

    expect(runnables.inlineRunnable).toBeDefined();
    expect(runnables.inlineRunnable.type).toBe("inline");
    expect(runnables.inlineRunnable.inlineScript).toBeDefined();
    expect(runnables.inlineRunnable.inlineScript.language).toBe("bun");
    expect(runnables.inlineRunnable.inlineScript.content).toContain("main(");
  });

  test("CamelCase YAML pairs with lowercased sibling code (legacy repo recovery)", async () => {
    fs.writeFileSync(
      path.join(backendDir, "CamelCaseTSRunnable.yaml"),
      "type: inline\nfields: {}\n",
      "utf-8",
    );
    fs.writeFileSync(
      path.join(backendDir, "camelcasetsrunnable.ts"),
      "export async function main() { return 1; }\n",
      "utf-8",
    );

    const runnables = await loadRunnablesFromBackend(backendDir);

    expect(Object.keys(runnables)).toEqual(["CamelCaseTSRunnable"]);
    expect(runnables.CamelCaseTSRunnable.inlineScript.content).toContain("main(");
  });
});

describe("genWmillTs (regression: path-based runnable schema)", () => {
  test("path-based runnable with schema produces typed args", async () => {
    fs.writeFileSync(
      path.join(backendDir, "fetchUser.yaml"),
      "type: script\npath: u/admin/fetch_user\nfields: {}\n",
      "utf-8",
    );

    const runnables = await loadRunnablesFromBackend(backendDir);

    // Simulate fetchPathRunnableSchemas() - schema fetched from the API.
    const ts = buildWmillTs(runnables, {}, {
      fetchUser: {
        type: "object",
        properties: {
          userId: { type: "string", description: "The user id" },
          includeProfile: { type: "boolean" },
        },
        required: ["userId"],
      },
    });

    // Without the fix, the path-based runnable would render as `args: {}`.
    expect(ts).toContain("fetchUser:");
    expect(ts).toMatch(/fetchUser:\s*\(args:\s*\{[^}]*userId/);
    expect(ts).toContain("userId");
    expect(ts).toContain("includeProfile");
    expect(ts).not.toMatch(/fetchUser:\s*\(args:\s*\{\s*\}\)/);
  });

  test("path-based runnable without schema falls back to {}", async () => {
    fs.writeFileSync(
      path.join(backendDir, "noSchema.yaml"),
      "type: script\npath: u/admin/no_schema\nfields: {}\n",
      "utf-8",
    );

    const runnables = await loadRunnablesFromBackend(backendDir);
    const ts = buildWmillTs(runnables);

    expect(ts).toMatch(/noSchema:\s*\(args:\s*\{\s*\}\)/);
  });
});

describe("inferRunnableSchemaFromFile (regression: new backend-folder format)", () => {
  test("infers schema from inline runnable using sibling .ts file", async () => {
    fs.writeFileSync(
      path.join(backendDir, "compute.yaml"),
      "type: inline\nfields: {}\n",
      "utf-8",
    );
    fs.writeFileSync(
      path.join(backendDir, "compute.ts"),
      "export async function main(x: number, label: string) {\n" +
        "  return `${label}:${x}`;\n" +
        "}\n",
      "utf-8",
    );

    const result = await inferRunnableSchemaFromFile(tempDir, "compute.ts");

    expect(result).toBeDefined();
    expect(result!.runnableId).toBe("compute");
    expect(result!.schema).toBeDefined();

    // The TS parser produces an object schema with one property per arg.
    const props = result!.schema.properties as Record<string, any>;
    expect(Object.keys(props).sort()).toEqual(["label", "x"]);
    expect(props.x.type).toBe("number");
    expect(props.label.type).toBe("string");
  });

  test("end-to-end: inferred schema flows into wmill.d.ts via inline override", async () => {
    fs.writeFileSync(
      path.join(backendDir, "compute.yaml"),
      "type: inline\nfields: {}\n",
      "utf-8",
    );
    fs.writeFileSync(
      path.join(backendDir, "compute.ts"),
      "export async function main(x: number) {\n  return x;\n}\n",
      "utf-8",
    );

    const result = await inferRunnableSchemaFromFile(tempDir, "compute.ts");
    expect(result).toBeDefined();

    const runnables = await loadRunnablesFromBackend(backendDir);
    const ts = buildWmillTs(runnables, { compute: result!.schema });
    // Without the fix, inferRunnableSchemaFromFile would have returned undefined
    // and the runnable would render as `args: {}`.
    expect(ts).toContain("compute:");
    expect(ts).toMatch(/compute:\s*\(args:\s*\{[^}]*x:/);
    expect(ts).not.toMatch(/compute:\s*\(args:\s*\{\s*\}\)/);
  });

  test("returns undefined for path-based runnables", async () => {
    fs.writeFileSync(
      path.join(backendDir, "remote.yaml"),
      "type: script\npath: u/admin/remote\nfields: {}\n",
      "utf-8",
    );
    // No code file - this is a path runnable. Simulate someone touching a file
    // matching the runnableId pattern (shouldn't happen in practice but the
    // function must not crash).
    fs.writeFileSync(path.join(backendDir, "remote.ts"), "// placeholder", "utf-8");

    const result = await inferRunnableSchemaFromFile(tempDir, "remote.ts");
    expect(result).toBeUndefined();
  });

  test("handles compound extensions like bun.ts", async () => {
    fs.writeFileSync(
      path.join(backendDir, "tagged.yaml"),
      "type: inline\nfields: {}\n",
      "utf-8",
    );
    fs.writeFileSync(
      path.join(backendDir, "tagged.bun.ts"),
      "export async function main(value: string) {\n  return value;\n}\n",
      "utf-8",
    );

    const result = await inferRunnableSchemaFromFile(tempDir, "tagged.bun.ts");

    expect(result).toBeDefined();
    expect(result!.runnableId).toBe("tagged");
    const props = result!.schema.properties as Record<string, any>;
    expect(props.value.type).toBe("string");
  });
});

describe("inferAllInlineSchemas (startup seed for wmill.d.ts)", () => {
  test("seeds schemas for every inline code file in the backend folder", async () => {
    fs.writeFileSync(
      path.join(backendDir, "alpha.yaml"),
      "type: inline\nfields: {}\n",
      "utf-8",
    );
    fs.writeFileSync(
      path.join(backendDir, "alpha.ts"),
      "export async function main(a: string, b: number) {\n  return { a, b };\n}\n",
      "utf-8",
    );

    // Code-only (no YAML) - auto-detected as inline by loadRunnablesFromBackend
    fs.writeFileSync(
      path.join(backendDir, "beta.bun.ts"),
      "export async function main(flag: boolean) {\n  return flag;\n}\n",
      "utf-8",
    );

    // Path-based runnable - schema lives remotely, must be skipped here
    fs.writeFileSync(
      path.join(backendDir, "gamma.yaml"),
      "type: script\npath: u/admin/gamma\nfields: {}\n",
      "utf-8",
    );

    const seeded = await inferAllInlineSchemas(tempDir);

    expect(Object.keys(seeded).sort()).toEqual(["alpha", "beta"]);
    expect(seeded.alpha.properties.a.type).toBe("string");
    expect(seeded.alpha.properties.b.type).toBe("number");
    expect(seeded.beta.properties.flag.type).toBe("boolean");
    expect(seeded.gamma).toBeUndefined();
  });

  test("returns empty map when backend folder is missing", async () => {
    fs.rmSync(backendDir, { recursive: true });
    const seeded = await inferAllInlineSchemas(tempDir);
    expect(seeded).toEqual({});
  });

  test("e2e: initial wmill.d.ts has typed args without any file edits", async () => {
    fs.writeFileSync(
      path.join(backendDir, "greet.yaml"),
      "type: inline\nfields: {}\n",
      "utf-8",
    );
    fs.writeFileSync(
      path.join(backendDir, "greet.ts"),
      "export async function main(name: string, times: number) {\n" +
        "  return Array(times).fill(`hi ${name}`);\n" +
        "}\n",
      "utf-8",
    );

    // Mirror the dev-server startup pipeline: seed schemas, then render.
    const seeded = await inferAllInlineSchemas(tempDir);
    const runnables = await loadRunnablesFromBackend(backendDir);
    const ts = buildWmillTs(runnables, seeded);

    // Without the startup seed this would render as `greet: (args: {}) => ...`
    // because inferredSchemas would be empty until the watcher fires.
    expect(ts).toMatch(/greet:\s*\(args:\s*\{[^}]*name/);
    expect(ts).toContain("name");
    expect(ts).toContain("times");
    expect(ts).not.toMatch(/greet:\s*\(args:\s*\{\s*\}\)/);
  });
});
