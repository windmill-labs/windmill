/**
 * wmill-lock Tests
 *
 * Tests the wmill-lock.yaml path normalization functionality.
 * Ensures paths are stored with Linux separators and can be correctly
 * looked up on both Windows and Linux systems.
 */

import { expect, test } from "bun:test";
import * as path from "@std/path";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import {
  normalizeLockPath,
  readLockfile,
  updateMetadataGlobalLock,
  checkifMetadataUptodate,
  clearGlobalLock,
} from "../src/utils/metadata.ts";
import { generateHash } from "../src/utils/utils.ts";
import { stringify as yamlStringify } from "@std/yaml";
import { yamlParseFile } from "../src/utils/yaml.ts";

// =============================================================================
// UNIT TESTS - Path Normalization
// =============================================================================

test("normalizeLockPath: converts Windows backslashes to forward slashes", () => {
  expect(normalizeLockPath("f\\test\\script")).toEqual("f/test/script");
  expect(normalizeLockPath("f\\deeply\\nested\\path\\script")).toEqual("f/deeply/nested/path/script");
});

test("normalizeLockPath: preserves already-normalized paths", () => {
  expect(normalizeLockPath("f/test/script")).toEqual("f/test/script");
  expect(normalizeLockPath("f/deeply/nested/path/script")).toEqual("f/deeply/nested/path/script");
});

test("normalizeLockPath: handles paths without separators", () => {
  expect(normalizeLockPath("script")).toEqual("script");
  expect(normalizeLockPath("")).toEqual("");
});

test("normalizeLockPath: handles mixed separators", () => {
  expect(normalizeLockPath("f/test\\nested/script")).toEqual("f/test/nested/script");
  expect(normalizeLockPath("f\\test/nested\\script")).toEqual("f/test/nested/script");
});

// =============================================================================
// INTEGRATION TESTS - Lock File Operations with Windows Paths
// =============================================================================

async function withTempDir(fn: (tempDir: string) => Promise<void>): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_lock_test_"));
  const originalCwd = process.cwd();
  try {
    process.chdir(tempDir);
    await fn(tempDir);
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true });
  }
}

test("wmill-lock: stores paths with Linux separators even when given Windows paths", async () => {
  await withTempDir(async (tempDir) => {
    // Simulate a Windows-style path
    const windowsPath = "f\\flows\\my-flow.flow";
    const hash = "abc123";

    await updateMetadataGlobalLock(windowsPath, hash);

    // Read the lockfile directly to check stored format
    const lockfile = await yamlParseFile("wmill-lock.yaml") as { version: string; locks: Record<string, string> };

    // Path should be stored with forward slashes
    expect(lockfile.locks["f/flows/my-flow.flow"]).toEqual(hash);
    expect(lockfile.locks["f\\flows\\my-flow.flow"]).toEqual(undefined);
  });
});

test("wmill-lock: checkifMetadataUptodate finds paths regardless of separator style", async () => {
  await withTempDir(async (tempDir) => {
    const linuxPath = "f/scripts/my-script";
    const windowsPath = "f\\scripts\\my-script";
    const hash = "def456";

    // Store with Linux-style path
    await updateMetadataGlobalLock(linuxPath, hash);

    // Should find with Linux-style lookup
    const conf = await readLockfile();
    expect(await checkifMetadataUptodate(linuxPath, hash, conf)).toEqual(true);

    // Should also find with Windows-style lookup (simulating Windows usage)
    expect(await checkifMetadataUptodate(windowsPath, hash, conf)).toEqual(true);

    // Should not find with wrong hash
    expect(await checkifMetadataUptodate(linuxPath, "wrong", conf)).toEqual(false);
    expect(await checkifMetadataUptodate(windowsPath, "wrong", conf)).toEqual(false);
  });
});

test("wmill-lock: updateMetadataGlobalLock with subpath normalizes both path and subpath", async () => {
  await withTempDir(async (tempDir) => {
    const windowsPath = "f\\flows\\my-flow.flow";
    const windowsSubpath = "inline\\script.ts";
    const hash = "ghi789";

    await updateMetadataGlobalLock(windowsPath, hash, windowsSubpath);

    // Read the lockfile directly
    const lockfile = await yamlParseFile("wmill-lock.yaml") as { version: string; locks: Record<string, string> };

    // Both path and subpath should use forward slashes
    expect(lockfile.locks["f/flows/my-flow.flow+inline/script.ts"]).toEqual(hash);
  });
});

test("wmill-lock: checkifMetadataUptodate with subpath handles Windows separators", async () => {
  await withTempDir(async (tempDir) => {
    const linuxPath = "f/apps/my-app.app";
    const linuxSubpath = "scripts/button.ts";
    const windowsPath = "f\\apps\\my-app.app";
    const windowsSubpath = "scripts\\button.ts";
    const hash = "jkl012";

    // Store with Linux-style paths
    await updateMetadataGlobalLock(linuxPath, hash, linuxSubpath);

    const conf = await readLockfile();

    // Should find with Linux-style lookup
    expect(await checkifMetadataUptodate(linuxPath, hash, conf, linuxSubpath)).toEqual(true);

    // Should find with Windows-style lookup
    expect(await checkifMetadataUptodate(windowsPath, hash, conf, windowsSubpath)).toEqual(true);

    // Should find with mixed-style lookup
    expect(await checkifMetadataUptodate(windowsPath, hash, conf, linuxSubpath)).toEqual(true);
    expect(await checkifMetadataUptodate(linuxPath, hash, conf, windowsSubpath)).toEqual(true);
  });
});

test("wmill-lock: clearGlobalLock clears paths regardless of separator style", async () => {
  await withTempDir(async (tempDir) => {
    const basePath = "f/flows/my-flow.flow";
    const subpath1 = "scripts/a.ts";
    const subpath2 = "scripts/b.ts";

    // Store multiple entries for the same flow
    await updateMetadataGlobalLock(basePath, "hash1", subpath1);
    await updateMetadataGlobalLock(basePath, "hash2", subpath2);
    await updateMetadataGlobalLock(basePath, "topHash", "__flow_hash");

    // Verify they exist
    let conf = await readLockfile();
    expect(await checkifMetadataUptodate(basePath, "hash1", conf, subpath1)).toEqual(true);
    expect(await checkifMetadataUptodate(basePath, "hash2", conf, subpath2)).toEqual(true);

    // Clear using Windows-style path
    await clearGlobalLock("f\\flows\\my-flow.flow");

    // All entries should be cleared
    conf = await readLockfile();
    expect(await checkifMetadataUptodate(basePath, "hash1", conf, subpath1)).toEqual(false);
    expect(await checkifMetadataUptodate(basePath, "hash2", conf, subpath2)).toEqual(false);
    expect(await checkifMetadataUptodate(basePath, "topHash", conf, "__flow_hash")).toEqual(false);
  });
});

test("wmill-lock: lock file created on Linux can be used on Windows (simulated)", async () => {
  await withTempDir(async (tempDir) => {
    // Simulate a lock file created on Linux
    const linuxLockContent = {
      version: "v2" as const,
      locks: {
        "f/scripts/utility": "hash1",
        "f/flows/main.flow+scripts/step1.ts": "hash2",
        "f/apps/dashboard.app+components/chart.ts": "hash3",
      },
    };

    await writeFile(
      "wmill-lock.yaml",
      yamlStringify(linuxLockContent as Record<string, unknown>),
      "utf-8"
    );

    const conf = await readLockfile();

    // Simulate Windows lookups (using backslashes)
    expect(await checkifMetadataUptodate("f\\scripts\\utility", "hash1", conf)).toEqual(true);
    expect(await checkifMetadataUptodate("f\\flows\\main.flow", "hash2", conf, "scripts\\step1.ts")).toEqual(true);
    expect(await checkifMetadataUptodate("f\\apps\\dashboard.app", "hash3", conf, "components\\chart.ts")).toEqual(true);
  });
});

test("wmill-lock: multiple updates with different separator styles result in single entry", async () => {
  await withTempDir(async (tempDir) => {
    const linuxPath = "f/scripts/shared";
    const windowsPath = "f\\scripts\\shared";

    // Update with Linux-style path
    await updateMetadataGlobalLock(linuxPath, "hash1");

    // Update same path with Windows-style path (should overwrite, not create new entry)
    await updateMetadataGlobalLock(windowsPath, "hash2");

    const lockfile = await yamlParseFile("wmill-lock.yaml") as { version: string; locks: Record<string, string> };

    // Should only have one entry with the latest hash
    const lockKeys = Object.keys(lockfile.locks);
    expect(lockKeys.length).toEqual(1);
    expect(lockKeys[0]).toEqual("f/scripts/shared");
    expect(lockfile.locks["f/scripts/shared"]).toEqual("hash2");
  });
});

// =============================================================================
// HASH COMPUTATION TESTS - OS-Independent Hash Generation
// =============================================================================

test("hash computation: normalized paths produce same hash on Windows and Linux", async () => {
  // Simulate how generateFlowHash/generateAppHash compute hashes
  // by using paths as keys in an object that gets stringified

  const fileContents = {
    "script1.ts": "export function main() { return 1; }",
    "nested/script2.ts": "export function main() { return 2; }",
  };

  // Simulate Windows paths
  const windowsHashes: Record<string, string> = {};
  for (const [relativePath, content] of Object.entries(fileContents)) {
    const windowsPath = relativePath.replace(/\//g, "\\");
    // Normalize before using as key (this is what the fix does)
    const normalizedPath = normalizeLockPath(windowsPath);
    windowsHashes[normalizedPath] = await generateHash(content);
  }
  const windowsTopHash = await generateHash(JSON.stringify(windowsHashes));

  // Simulate Linux paths
  const linuxHashes: Record<string, string> = {};
  for (const [relativePath, content] of Object.entries(fileContents)) {
    // Linux paths are already normalized
    const normalizedPath = normalizeLockPath(relativePath);
    linuxHashes[normalizedPath] = await generateHash(content);
  }
  const linuxTopHash = await generateHash(JSON.stringify(linuxHashes));

  // Both should produce the same top hash
  expect(windowsTopHash).toEqual(linuxTopHash);

  // And the individual hashes should have the same keys
  expect(Object.keys(windowsHashes).sort()).toEqual(Object.keys(linuxHashes).sort());
});

test("hash computation: without normalization, Windows and Linux would produce different hashes", async () => {
  // This test demonstrates the problem that normalization fixes
  const fileContents = {
    "script1.ts": "export function main() { return 1; }",
    "nested/script2.ts": "export function main() { return 2; }",
  };

  // Simulate Windows paths WITHOUT normalization
  const windowsHashesNoNormalize: Record<string, string> = {};
  for (const [relativePath, content] of Object.entries(fileContents)) {
    const windowsPath = relativePath.replace(/\//g, "\\");
    // NOT normalizing - simulating the old behavior
    windowsHashesNoNormalize[windowsPath] = await generateHash(content);
  }
  const windowsTopHashNoNormalize = await generateHash(JSON.stringify(windowsHashesNoNormalize));

  // Simulate Linux paths WITHOUT normalization
  const linuxHashesNoNormalize: Record<string, string> = {};
  for (const [relativePath, content] of Object.entries(fileContents)) {
    // NOT normalizing - simulating the old behavior
    linuxHashesNoNormalize[relativePath] = await generateHash(content);
  }
  const linuxTopHashNoNormalize = await generateHash(JSON.stringify(linuxHashesNoNormalize));

  // Without normalization, the hashes WOULD be different (this is the bug we fixed)
  // The keys are different: "nested\\script2.ts" vs "nested/script2.ts"
  const windowsKeys = Object.keys(windowsHashesNoNormalize).sort();
  const linuxKeys = Object.keys(linuxHashesNoNormalize).sort();

  // Keys should be different without normalization
  expect(windowsKeys.includes("nested\\script2.ts")).toEqual(true);
  expect(linuxKeys.includes("nested/script2.ts")).toEqual(true);
  expect(windowsKeys.includes("nested/script2.ts")).toEqual(false);
  expect(linuxKeys.includes("nested\\script2.ts")).toEqual(false);
});

test("hash computation: deeply nested paths are normalized correctly", async () => {
  const deepWindowsPath = "f\\flows\\my-flow.flow\\inline\\scripts\\deeply\\nested\\handler.ts";
  const deepLinuxPath = "f/flows/my-flow.flow/inline/scripts/deeply/nested/handler.ts";

  const content = "export function main() { return 'deeply nested'; }";

  // Hash with Windows path (normalized)
  const windowsHashes: Record<string, string> = {};
  windowsHashes[normalizeLockPath(deepWindowsPath)] = await generateHash(content);
  const windowsTopHash = await generateHash(JSON.stringify(windowsHashes));

  // Hash with Linux path (normalized)
  const linuxHashes: Record<string, string> = {};
  linuxHashes[normalizeLockPath(deepLinuxPath)] = await generateHash(content);
  const linuxTopHash = await generateHash(JSON.stringify(linuxHashes));

  expect(windowsTopHash).toEqual(linuxTopHash);
  expect(Object.keys(windowsHashes)[0]).toEqual(Object.keys(linuxHashes)[0]);
  expect(Object.keys(windowsHashes)[0]).toEqual(deepLinuxPath);
});

test("hash computation: changedScripts comparison works with inline module paths", () => {
  // This test simulates the comparison done in replaceInlineScripts
  // where changedScripts (from hashes keys) is compared with paths from flow module content

  // Simulate changedScripts populated from hashes (normalized from Windows paths)
  const changedScripts = [
    normalizeLockPath("step1.ts"),
    normalizeLockPath("nested\\step2.ts"),  // Windows-style from FSFSElement
    normalizeLockPath("deeply\\nested\\step3.ts"),
  ];

  // Simulate paths extracted from flow module content (!inline paths are always forward slashes)
  const inlineModulePaths = [
    "step1.ts",
    "nested/step2.ts",  // Forward slashes as stored in flow.yaml
    "deeply/nested/step3.ts",
  ];

  // All inline module paths should be found in changedScripts
  for (const inlinePath of inlineModulePaths) {
    expect(
      changedScripts.includes(inlinePath)
    ).toEqual(true);
  }
});
