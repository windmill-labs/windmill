/**
 * `gitRecordedDatatableMigrationPaths` is what a push consults before deleting a data
 * table migration the working tree no longer has: a path in history is a real deletion,
 * a path never recorded is one this clone may simply never have synced. Both failures
 * pinned here made it answer "never recorded" or "git is broken" about a repository
 * that was neither, so a migration the user committed would have been kept and the
 * remedy offered would have been the wrong one.
 *
 * Runs real git in a temp repo — the answer is a property of git's output, not of any
 * logic that could be tested without it.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { gitRecordedDatatableMigrationPaths } from "../src/utils/git.ts";

let dir: string;
let cwd: string;

const git = (...args: string[]) =>
  execFileSync("git", args, { cwd: dir, encoding: "utf8", stdio: "pipe" });

const commitMigration = (name: string) => {
  const rel = `migrations/datatable/dt/${name}`;
  fs.mkdirSync(path.join(dir, path.dirname(rel)), { recursive: true });
  fs.writeFileSync(path.join(dir, rel), "select 1;\n");
  git("add", "-A");
  git("commit", "-m", `add ${name}`);
  return rel;
};

beforeEach(() => {
  cwd = process.cwd();
  dir = fs.mkdtempSync(path.join(os.tmpdir(), "wmill-git-"));
  git("init", "-q", "-b", "main");
  git("config", "user.email", "t@t.dev");
  git("config", "user.name", "t");
  process.chdir(dir);
});

afterEach(() => {
  process.chdir(cwd);
  fs.rmSync(dir, { recursive: true, force: true });
});

describe("gitRecordedDatatableMigrationPaths", () => {
  test("a repository with no commits is not a broken one", () => {
    const r = gitRecordedDatatableMigrationPaths();
    expect(r.kind).toBe("unknown");
    if (r.kind === "unknown") {
      expect(r.reason).toBe("this repository has no commits yet");
    }
  });

  test("records a path with a non-ASCII byte unquoted", () => {
    // core.quotePath would return "migrations/datatable/dt/1_caf\303\251.up.sql",
    // which matches no path the caller holds and reads as never recorded.
    const rel = commitMigration("20260101000000_café.up.sql");
    const r = gitRecordedDatatableMigrationPaths();
    expect(r.kind).toBe("known");
    if (r.kind === "known") expect(r.paths.has(rel)).toBe(true);
  });

  test("a committed migration stays recorded after its deletion is committed", () => {
    const rel = commitMigration("20260101000000_a.up.sql");
    fs.rmSync(path.join(dir, rel));
    git("add", "-A");
    git("commit", "-m", "delete it");
    const r = gitRecordedDatatableMigrationPaths();
    expect(r.kind).toBe("known");
    if (r.kind === "known") expect(r.paths.has(rel)).toBe(true);
  });
});
