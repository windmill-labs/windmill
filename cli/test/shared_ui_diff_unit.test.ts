/**
 * Unit tests for diffSharedUi in shared_ui.ts.
 *
 * Regression guard: the git-sync "Pull from repo" preview (wmill sync push
 * --dry-run --json) folds diffSharedUi into its `changes` list so pending
 * shared-UI (ui/) changes show up in the preview instead of reading as
 * "no changes". The apply already syncs ui/ out-of-band via pushSharedUi;
 * only the dry-run was blind.
 *
 * Asserts diffSharedUi emits ui/<rel> entries when the local ui/ folder
 * differs from the remote shared-UI store, and nothing when they match.
 */

import { expect, test, describe, beforeEach, afterEach, mock } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

let remoteFiles: Record<string, string> = {};

mock.module("../gen/services.gen.ts", () => ({
  getSharedUi: async (_args: { workspace: string }) => ({ files: remoteFiles }),
}));

const { diffSharedUi } = await import("../src/commands/shared_ui.ts");

describe("diffSharedUi", () => {
  const ws = "test-workspace";
  let tmpDir: string;
  let prevCwd: string;

  beforeEach(() => {
    remoteFiles = {};
    prevCwd = process.cwd();
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "wm-shared-ui-"));
    process.chdir(tmpDir);
  });

  afterEach(() => {
    process.chdir(prevCwd);
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  function writeUi(rel: string, content: string) {
    const full = path.join(tmpDir, "ui", rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content, "utf-8");
  }

  test("emits an added ui/<rel> entry when local has a file the remote lacks", async () => {
    writeUi("theme.json", "{}");
    const changes = await diffSharedUi(ws);
    expect(changes).toEqual([{ type: "added", path: "ui/theme.json" }]);
  });

  test("emits an edited ui/<rel> entry when local content differs", async () => {
    remoteFiles = { "theme.json": "{}" };
    writeUi("theme.json", '{"a":1}');
    const changes = await diffSharedUi(ws);
    expect(changes).toEqual([
      { type: "edited", path: "ui/theme.json", before: "{}", after: '{"a":1}' },
    ]);
  });

  test("emits a deleted ui/<rel> entry when remote has a file local lacks", async () => {
    remoteFiles = { "theme.json": "{}", "extra.json": "1" };
    writeUi("theme.json", "{}");
    const changes = await diffSharedUi(ws);
    expect(changes).toEqual([{ type: "deleted", path: "ui/extra.json" }]);
  });

  test("emits nothing when local matches the remote store", async () => {
    remoteFiles = { "theme.json": "{}" };
    writeUi("theme.json", "{}");
    const changes = await diffSharedUi(ws);
    expect(changes).toEqual([]);
  });

  test("emits nothing when there is no local ui/ folder (apply is a no-op)", async () => {
    remoteFiles = { "theme.json": "{}" };
    const changes = await diffSharedUi(ws);
    expect(changes).toEqual([]);
  });
});
