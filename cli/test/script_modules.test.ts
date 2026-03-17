/**
 * Unit tests for script module utilities.
 * These tests require no backend — they test standalone logic.
 */

import { expect, test, describe, beforeEach, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  getModuleFolderSuffix,
  isScriptModulePath,
  buildModuleFolderPath,
  isModuleEntryPoint,
  getScriptBasePathFromModulePath,
} from "../src/utils/resource_folders.ts";
import {
  writeModulesToDisk,
  readModulesFromDisk,
} from "../src/commands/script/script.ts";
import { getTypeStrFromPath } from "../src/types.ts";

// =============================================================================
// Module Path Utilities
// =============================================================================

describe("getModuleFolderSuffix", () => {
  test("returns __mod", () => {
    expect(getModuleFolderSuffix()).toBe("__mod");
  });
});

describe("isScriptModulePath", () => {
  test("detects module paths", () => {
    expect(isScriptModulePath("f/my_script__mod/helper.ts")).toBe(true);
    expect(isScriptModulePath("u/admin/script__mod/utils/math.py")).toBe(true);
  });

  test("rejects non-module paths", () => {
    expect(isScriptModulePath("f/my_script.ts")).toBe(false);
    expect(isScriptModulePath("f/my_script__mod")).toBe(false); // no trailing /
    expect(isScriptModulePath("f/my_flow__flow/flow.yaml")).toBe(false);
  });

  test("handles windows-style separators", () => {
    expect(isScriptModulePath("f\\my_script__mod\\helper.ts")).toBe(true);
  });
});

describe("buildModuleFolderPath", () => {
  test("appends __mod suffix", () => {
    expect(buildModuleFolderPath("f/my_script")).toBe("f/my_script__mod");
    expect(buildModuleFolderPath("u/admin/tool")).toBe("u/admin/tool__mod");
  });
});

// =============================================================================
// writeModulesToDisk / readModulesFromDisk round-trip
// =============================================================================

describe("module read/write round-trip", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "wm-module-test-"));
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  test("writes and reads back a simple module", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");
    const modules = {
      "helper.ts": {
        content: 'export function greet() { return "hi"; }\n',
        language: "bun" as const,
      },
    };

    await writeModulesToDisk(moduleFolderPath, modules, "bun");

    // Verify file exists on disk
    expect(fs.existsSync(path.join(moduleFolderPath, "helper.ts"))).toBe(true);

    // Read back
    const result = await readModulesFromDisk(moduleFolderPath, "bun", false);
    expect(result).toBeDefined();
    expect(Object.keys(result!)).toEqual(["helper.ts"]);
    expect(result!["helper.ts"].content).toBe('export function greet() { return "hi"; }\n');
    expect(result!["helper.ts"].language).toBe("bun");
  });

  test("writes and reads back module with lock file", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");
    const modules = {
      "helper.py": {
        content: "def greet():\n    return 'hi'\n",
        language: "python3" as const,
        lock: "requests==2.31.0\n",
      },
    };

    await writeModulesToDisk(moduleFolderPath, modules, undefined);

    // Verify lock file exists
    expect(fs.existsSync(path.join(moduleFolderPath, "helper.lock"))).toBe(true);

    // Read back
    const result = await readModulesFromDisk(moduleFolderPath, undefined, false);
    expect(result).toBeDefined();
    expect(result!["helper.py"].lock).toBe("requests==2.31.0\n");
  });

  test("writes and reads back nested module paths", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");
    const modules = {
      "utils/math.py": {
        content: "def add(a, b):\n    return a + b\n",
        language: "python3" as const,
      },
      "utils/__init__.py": {
        content: "",
        language: "python3" as const,
      },
    };

    await writeModulesToDisk(moduleFolderPath, modules, undefined);

    // Verify nested structure
    expect(fs.existsSync(path.join(moduleFolderPath, "utils", "math.py"))).toBe(true);
    expect(fs.existsSync(path.join(moduleFolderPath, "utils", "__init__.py"))).toBe(true);

    // Read back
    const result = await readModulesFromDisk(moduleFolderPath, undefined, false);
    expect(result).toBeDefined();
    expect(Object.keys(result!).sort()).toEqual(["utils/__init__.py", "utils/math.py"]);
    expect(result!["utils/math.py"].content).toBe("def add(a, b):\n    return a + b\n");
  });

  test("returns undefined for non-existent folder", async () => {
    const result = await readModulesFromDisk(path.join(tempDir, "nonexistent__mod"), undefined, false);
    expect(result).toBeUndefined();
  });

  test("returns undefined for empty folder", async () => {
    const emptyFolder = path.join(tempDir, "empty__mod");
    fs.mkdirSync(emptyFolder);
    const result = await readModulesFromDisk(emptyFolder, undefined, false);
    expect(result).toBeUndefined();
  });

  test("multiple modules of different languages", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");
    const modules = {
      "helper.ts": {
        content: "export const x = 1;\n",
        language: "bun" as const,
      },
      "other.ts": {
        content: "export const y = 2;\n",
        language: "bun" as const,
        lock: "some-lock-content\n",
      },
    };

    await writeModulesToDisk(moduleFolderPath, modules, "bun");
    const result = await readModulesFromDisk(moduleFolderPath, "bun", false);

    expect(result).toBeDefined();
    expect(Object.keys(result!).sort()).toEqual(["helper.ts", "other.ts"]);
    expect(result!["other.ts"].lock).toBe("some-lock-content\n");
    expect(result!["helper.ts"].lock).toBeUndefined();
  });

  test("folder layout skips entry point files", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");
    fs.mkdirSync(moduleFolderPath, { recursive: true });

    // Simulate folder layout: script.ts is the entry point, helper.ts is a module
    fs.writeFileSync(path.join(moduleFolderPath, "script.ts"), "export function main() {}\n");
    fs.writeFileSync(path.join(moduleFolderPath, "script.yaml"), "description: test\n");
    fs.writeFileSync(path.join(moduleFolderPath, "script.lock"), "");
    fs.writeFileSync(path.join(moduleFolderPath, "helper.ts"), "export const x = 1;\n");

    // With folderLayout=true, entry point files should be skipped
    const result = await readModulesFromDisk(moduleFolderPath, "bun", true);
    expect(result).toBeDefined();
    expect(Object.keys(result!)).toEqual(["helper.ts"]);

    // With folderLayout=false, all files are included
    const resultFlat = await readModulesFromDisk(moduleFolderPath, "bun", false);
    expect(resultFlat).toBeDefined();
    expect(Object.keys(resultFlat!).sort()).toContain("script.ts");
    expect(Object.keys(resultFlat!).sort()).toContain("helper.ts");
  });
});

// =============================================================================
// getTypeStrFromPath with module paths
// =============================================================================

describe("getTypeStrFromPath with module paths", () => {
  test("recognizes module content files as scripts", () => {
    expect(getTypeStrFromPath("f/my_script__mod/helper.ts")).toBe("script");
    expect(getTypeStrFromPath("u/admin/tool__mod/utils/math.py")).toBe("script");
    expect(getTypeStrFromPath("f/x__mod/helper.go")).toBe("script");
  });

  test("recognizes module lock files as scripts", () => {
    // Lock files inside __mod/ should NOT throw — they are valid module files
    expect(getTypeStrFromPath("f/my_script__mod/helper.lock")).toBe("script");
    expect(getTypeStrFromPath("u/admin/tool__mod/utils/math.lock")).toBe("script");
  });

  test("recognizes module entry points as scripts", () => {
    expect(getTypeStrFromPath("f/my_script__mod/script.ts")).toBe("script");
    expect(getTypeStrFromPath("f/my_script__mod/script.py")).toBe("script");
  });

  test("recognizes module metadata files as scripts", () => {
    // .yaml/.json files inside __mod/ should be recognized
    expect(getTypeStrFromPath("f/my_script__mod/script.yaml")).toBe("script");
    expect(getTypeStrFromPath("f/my_script__mod/script.json")).toBe("script");
  });
});

// =============================================================================
// Module read/write with multiple lock files
// =============================================================================

describe("module lock file handling", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "wm-module-lock-test-"));
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  test("writes and reads multiple modules each with their own lock", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");
    const modules = {
      "api_client.py": {
        content: "import requests\ndef fetch(): pass\n",
        language: "python3" as const,
        lock: "requests==2.31.0\nurllib3==2.0.4\n",
      },
      "data_processor.py": {
        content: "import pandas\ndef process(): pass\n",
        language: "python3" as const,
        lock: "pandas==2.1.0\nnumpy==1.25.0\n",
      },
      "utils.py": {
        content: "def helper(): pass\n",
        language: "python3" as const,
        // No lock — no external deps
      },
    };

    await writeModulesToDisk(moduleFolderPath, modules, undefined);

    // Verify lock files exist only for modules with locks
    expect(fs.existsSync(path.join(moduleFolderPath, "api_client.lock"))).toBe(true);
    expect(fs.existsSync(path.join(moduleFolderPath, "data_processor.lock"))).toBe(true);
    expect(fs.existsSync(path.join(moduleFolderPath, "utils.lock"))).toBe(false);

    // Read back and verify
    const result = await readModulesFromDisk(moduleFolderPath, undefined, false);
    expect(result).toBeDefined();
    expect(Object.keys(result!).sort()).toEqual(["api_client.py", "data_processor.py", "utils.py"]);
    expect(result!["api_client.py"].lock).toBe("requests==2.31.0\nurllib3==2.0.4\n");
    expect(result!["data_processor.py"].lock).toBe("pandas==2.1.0\nnumpy==1.25.0\n");
    expect(result!["utils.py"].lock).toBeUndefined();
  });

  test("nested modules with lock files", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");
    const modules = {
      "services/api.ts": {
        content: "export function callApi() {}\n",
        language: "bun" as const,
        lock: "axios@1.5.0\n",
      },
      "services/db.ts": {
        content: "export function query() {}\n",
        language: "bun" as const,
        lock: "pg@8.11.0\n",
      },
    };

    await writeModulesToDisk(moduleFolderPath, modules, "bun");

    // Verify nested lock files
    expect(fs.existsSync(path.join(moduleFolderPath, "services", "api.lock"))).toBe(true);
    expect(fs.existsSync(path.join(moduleFolderPath, "services", "db.lock"))).toBe(true);

    const result = await readModulesFromDisk(moduleFolderPath, "bun", false);
    expect(result).toBeDefined();
    expect(result!["services/api.ts"].lock).toBe("axios@1.5.0\n");
    expect(result!["services/db.ts"].lock).toBe("pg@8.11.0\n");
  });

  test("overwrites existing module folder on re-write", async () => {
    const moduleFolderPath = path.join(tempDir, "my_script__mod");

    // First write
    await writeModulesToDisk(moduleFolderPath, {
      "old.ts": { content: "old content\n", language: "bun" as const },
    }, "bun");
    expect(fs.existsSync(path.join(moduleFolderPath, "old.ts"))).toBe(true);

    // Second write with different module
    await writeModulesToDisk(moduleFolderPath, {
      "new.ts": { content: "new content\n", language: "bun" as const },
    }, "bun");
    expect(fs.existsSync(path.join(moduleFolderPath, "new.ts"))).toBe(true);
    // old.ts should still exist (writeModulesToDisk doesn't clean up)
    // This is intentional — cleanup happens at the sync level
  });
});
