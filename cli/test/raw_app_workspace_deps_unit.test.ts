/**
 * Raw app workspace dependencies
 *
 * A raw app keeps its runnables in `backend/`, not in `raw_app.yaml`. The
 * workspace dependency filtering must resolve those files, otherwise the
 * default `dependencies/package.json` is dropped and locks are regenerated
 * against unpinned versions.
 *
 * Exercised through the legacy (tree-less) path: tree mode sources its deps
 * from `getMismatchedWorkspaceDeps()`, which is only populated by an
 * `uploadScripts` round-trip, so it cannot run offline. Both paths filter the
 * same `appValue`, so resolving it correctly is what this pins.
 */

import { expect, test } from "bun:test";
import * as path from "node:path";
import os from "node:os";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { generateAppLocksInternal } from "../src/commands/app/app_metadata.ts";
import { Workspace } from "../src/commands/workspace/workspace.ts";

const stubWorkspace: Workspace = {
  remote: "http://localhost:0/",
  workspaceId: "test",
  name: "test",
  token: "test",
};

const APP_FOLDER = path.join("f", "example.raw_app");

async function withTempDir(fn: (tempDir: string) => Promise<void>): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_raw_app_deps_"));
  const originalCwd = process.cwd();
  try {
    process.chdir(tempDir);
    await fn(tempDir);
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true, force: true });
  }
}

test("raw app: default workspace deps are picked up from backend runnables", async () => {
  await withTempDir(async () => {
    await mkdir(path.join(APP_FOLDER, "backend"), { recursive: true });
    await writeFile(
      path.join(APP_FOLDER, "raw_app.yaml"),
      `summary: "example raw app"\npolicy:\n  execution_mode: publisher\n  triggerables: {}\n`,
      "utf-8",
    );
    await writeFile(
      path.join(APP_FOLDER, "backend", "test.ts"),
      `import * as wmill from "windmill-client"\n\nexport async function main() {\n  return wmill.getVariable("example")\n}\n`,
      "utf-8",
    );

    await generateAppLocksInternal(
      APP_FOLDER,
      true,
      false,
      stubWorkspace,
      { defaultTs: "bun" },
      true, // justUpdateMetadataLock — no backend round-trip
      true,
    );

    expect(
      await generateAppLocksInternal(APP_FOLDER, true, true, stubWorkspace, { defaultTs: "bun" }, false, true),
    ).toBeUndefined();

    // The runnable has no `package_json` annotation, so it uses the default
    // manifest — adding it must invalidate the app.
    await mkdir("dependencies", { recursive: true });
    await writeFile(
      path.join("dependencies", "package.json"),
      `{"dependencies": {"windmill-client": "1.742.0"}}`,
      "utf-8",
    );

    expect(
      await generateAppLocksInternal(APP_FOLDER, true, true, stubWorkspace, { defaultTs: "bun" }, false, true),
    ).toEqual("f/example.raw_app");
  });
});
