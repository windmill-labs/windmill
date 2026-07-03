import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import * as path from "node:path";
import { DoubleLinkedDependencyTree } from "../src/utils/dependency_tree.ts";

// addNode consults wmill-lock.yaml from cwd for workspace deps; run inside a
// temp dir so the test never reads/writes the repo's own lock file.
async function withTempDir(
  fn: () => Promise<void>,
): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_deptree_test_"));
  const originalCwd = process.cwd();
  try {
    process.chdir(tempDir);
    await fn();
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true });
  }
}

async function buildTree(): Promise<DoubleLinkedDependencyTree> {
  const tree = new DoubleLinkedDependencyTree();
  // main imports lib; other is unrelated
  await tree.addNode(
    "f/test/main",
    `import { x } from "./lib.ts"`,
    "bun",
    "",
    ["f/test/lib"],
    "script",
    "f/test/main",
    "f/test/main.ts",
    true,
  );
  await tree.addNode(
    "f/test/lib",
    "export const x = 1",
    "bun",
    "",
    [],
    "script",
    "f/test/lib",
    "f/test/lib.ts",
    true,
  );
  await tree.addNode(
    "f/other/standalone",
    "export const y = 2",
    "bun",
    "",
    [],
    "script",
    "f/other/standalone",
    "f/other/standalone.ts",
    true,
  );
  return tree;
}

test("getAllTempScriptRefs returns refs for every uploaded node", async () => {
  await withTempDir(async () => {
    const tree = await buildTree();
    // Simulate uploadScripts marking all three as uploaded
    tree.setContentHash("f/test/main", "hash_main");
    tree.setContentHash("f/test/lib", "hash_lib");
    tree.setContentHash("f/other/standalone", "hash_standalone");

    expect(tree.getAllTempScriptRefs()).toEqual({
      "f/test/main": "hash_main",
      "f/test/lib": "hash_lib",
      "f/other/standalone": "hash_standalone",
    });
  });
});

test("getAllTempScriptRefs skips nodes without a content hash", async () => {
  await withTempDir(async () => {
    const tree = await buildTree();
    // Only lib diverged from deployed
    tree.setContentHash("f/test/lib", "hash_lib");

    expect(tree.getAllTempScriptRefs()).toEqual({ "f/test/lib": "hash_lib" });
  });
});

test("getAllTempScriptRefs is a superset of getTempScriptRefs for any node", async () => {
  await withTempDir(async () => {
    const tree = await buildTree();
    tree.setContentHash("f/test/lib", "hash_lib");
    tree.setContentHash("f/other/standalone", "hash_standalone");

    const all = tree.getAllTempScriptRefs();
    for (const node of ["f/test/main", "f/test/lib", "f/other/standalone"]) {
      const perNode = tree.getTempScriptRefs(node);
      for (const [k, v] of Object.entries(perNode)) {
        expect(all[k]).toBe(v);
      }
    }
    // And the per-node view of main only sees its own transitive import
    expect(tree.getTempScriptRefs("f/test/main")).toEqual({
      "f/test/lib": "hash_lib",
    });
  });
});
