/**
 * A preview job carries no runnable, so the path derived from the file
 * argument is the whole of its identity — it has to come out the same
 * whatever shape the argument had, and be refused rather than guessed when
 * the file has no place in the workspace tree.
 */
import { expect, test, describe, beforeEach, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  assertRemotePath,
  toSyncRootRelativePath,
} from "../src/core/context.ts";

describe("toSyncRootRelativePath", () => {
  let root: string;
  let previousCwd: string;
  let temps: string[];

  beforeEach(() => {
    previousCwd = process.cwd();
    temps = [];
    // realpath: macOS' tmpdir is /var -> /private/var, and the assertions
    // compare against what the process reports as its own directory.
    root = fs.realpathSync(
      fs.mkdtempSync(path.join(os.tmpdir(), "wmill_preview_path_")),
    );
    temps.push(root);
    fs.writeFileSync(path.join(root, "wmill.yaml"), "defaultTs: bun\n");
    fs.mkdirSync(path.join(root, "f", "test"), { recursive: true });
    fs.writeFileSync(path.join(root, "f", "test", "script.ts"), "");
    process.chdir(root);
  });

  afterEach(() => {
    process.chdir(previousCwd);
    for (const dir of temps) fs.rmSync(dir, { recursive: true, force: true });
  });

  const normalized = (p: string) => p.replaceAll("\\", "/");

  /** A dbt project whose descriptor is deliberately not written. */
  function dbtProject(): string {
    const project = path.join(root, "f", "test", "proj__dbt");
    fs.mkdirSync(project, { recursive: true });
    fs.writeFileSync(path.join(project, "dbt_project.yml"), "name: proj\n");
    return project;
  }

  test("every spelling of the same file lands on the same path", () => {
    const fromRoot = ["f/test/script.ts", "./f/test/script.ts"].map((arg) =>
      normalized(toSyncRootRelativePath(arg, root)),
    );
    const absolute = normalized(
      toSyncRootRelativePath(path.join(root, "f", "test", "script.ts"), root),
    );
    // As typed from the directory the file is in: the config read has already
    // moved the process to the root by the time the argument is resolved.
    const fromSubdir = normalized(
      toSyncRootRelativePath("script.ts", path.join(root, "f", "test")),
    );

    expect(fromRoot).toEqual(["f/test/script.ts", "f/test/script.ts"]);
    expect(absolute).toEqual("f/test/script.ts");
    expect(fromSubdir).toEqual("f/test/script.ts");
  });

  test("a file that is deliberately absent keeps the directory it was named in", () => {
    // A dbt project's descriptor is optional; `wmill script preview
    // wm_dbt.yaml` from inside the project must still resolve to the project.
    const project = dbtProject();

    expect(normalized(toSyncRootRelativePath("wm_dbt.yaml", project))).toEqual(
      "f/test/proj__dbt/wm_dbt.yaml",
    );
  });

  // Windows only creates symlinks for a privileged process.
  test.skipIf(process.platform === "win32")(
    "an absent file reached through a symlinked root still lands in the tree",
    () => {
      dbtProject();
      const aliasDir = fs.mkdtempSync(path.join(os.tmpdir(), "wmill_alias_"));
      temps.push(aliasDir);
      const alias = path.join(aliasDir, "link");
      fs.symlinkSync(root, alias, "dir");

      const arg = path.join(alias, "f", "test", "proj__dbt", "wm_dbt.yaml");
      expect(normalized(toSyncRootRelativePath(arg, root))).toEqual(
        "f/test/proj__dbt/wm_dbt.yaml",
      );
    },
  );

  test("a file outside the tree stays outside it", () => {
    const outside = path.join(root, "..", "elsewhere.ts");
    expect(toSyncRootRelativePath(outside, root).startsWith("..")).toBe(true);
  });
});

describe("assertRemotePath", () => {
  test("accepts a workspace path and refuses anything else", () => {
    expect(() => assertRemotePath("f/test/script", "f/test/script.ts")).not.toThrow();
    expect(() => assertRemotePath("u/admin/script", "script.ts")).not.toThrow();
    for (const bad of ["", "script", "f/script", "../elsewhere"]) {
      expect(() => assertRemotePath(bad, "arg.ts")).toThrow(
        /Cannot derive a Windmill path/,
      );
    }
  });
});
