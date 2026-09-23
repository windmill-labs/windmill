import { expect, test, describe } from "bun:test";
import { mkdtemp, rm, mkdir, writeFile } from "node:fs/promises";
import os from "node:os";
import * as path from "node:path";
import { runLint } from "../src/commands/lint/lint.ts";

const WMILL_YAML = "defaultTs: bun\nincludes:\n  - f/**\nexcludes: []\n";
const METADATA = "summary: test\nlock: ''\nschema:\n  properties: {}\n";

async function write(dir: string, rel: string, content: string) {
  const full = path.join(dir, rel);
  await mkdir(path.dirname(full), { recursive: true });
  await writeFile(full, content, "utf-8");
}

/**
 * Runs `fn` with a sync root at `<temp>/<rootName>`, from which lint resolves
 * every walked path. The name is a parameter because it is load-bearing: the
 * folder suffixes lint classifies by (`.app`, `__mod`, …) are matched anywhere
 * in a path, so a root carrying one must not change what lint reports.
 */
async function withSyncRoot(
  rootName: string,
  fn: (syncRoot: string) => Promise<void>,
  opts: { runFromParent?: boolean } = {},
): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_lint_orphan_"));
  const syncRoot = path.join(tempDir, rootName);
  const originalCwd = process.cwd();
  try {
    await write(syncRoot, "wmill.yaml", WMILL_YAML);
    process.chdir(opts.runFromParent ? tempDir : syncRoot);
    await fn(syncRoot);
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true });
  }
}

describe("orphan script metadata", () => {
  test("reports metadata with no content file, with locks not required", async () => {
    await withSyncRoot("repo", async (syncRoot) => {
      await write(syncRoot, "f/paired.py", "def main():\n    pass\n");
      await write(syncRoot, "f/paired.script.yaml", METADATA);
      await write(syncRoot, "f/orphan.script.yaml", METADATA);
      await write(syncRoot, "f/orphan_json.script.json", "{}\n");
      await write(syncRoot, "f/orphan_lock.script.lock", "some-dep==1.0.0\n");

      const report = await runLint({} as any, syncRoot);

      expect(report.exitCode).toBe(1);
      expect(report.issues.map((i) => i.path).sort()).toEqual([
        "f/orphan.script.yaml",
        "f/orphan_json.script.json",
        "f/orphan_lock.script.lock",
      ]);
      expect(report.issues[0].target).toBe("script");
      expect(report.issues[0].errors[0]).toContain("No script file found next to");
    });
  });

  test("reports a module folder's own metadata with no content file", async () => {
    await withSyncRoot("repo", async (syncRoot) => {
      await write(syncRoot, "f/orphan__mod/script.yaml", METADATA);

      const report = await runLint({} as any, syncRoot);

      expect(report.issues.map((i) => i.path)).toEqual([
        "f/orphan__mod/script.yaml",
      ]);

      // Linting the module folder itself: the walked paths no longer carry the
      // `__mod/` boundary that says this is a module's metadata.
      const inFolder = await runLint(
        {} as any,
        path.join(syncRoot, "f/orphan__mod"),
      );

      expect(inFolder.issues.map((i) => i.path)).toEqual(["script.yaml"]);
    });
  });

  test("reports orphans under a sync root named like a resource folder", async () => {
    await withSyncRoot("acme.app", async (syncRoot) => {
      await write(syncRoot, "f/orphan.script.yaml", METADATA);

      const report = await runLint({} as any, syncRoot);

      expect(report.issues.map((i) => i.path)).toEqual(["f/orphan.script.yaml"]);
    });
  });

  test("does not report a paired module under a sync root named like a module folder", async () => {
    // Run from OUTSIDE the checkout, the one invocation whose paths carry the
    // root's own name: nothing above the sync root may be classified.
    await withSyncRoot(
      "repo__mod",
      async (syncRoot) => {
        await write(syncRoot, "f/example__mod/script.yaml", METADATA);
        await write(
          syncRoot,
          "f/example__mod/script.ts",
          "export function main() {}\n",
        );

        const report = await runLint({} as any, syncRoot);

        expect(report.issues).toEqual([]);
      },
      { runFromParent: true },
    );
  });

  test("does not report a non-dotted folder resource's child", async () => {
    // The dotted/non-dotted setting is read from the invocation directory, so
    // an explicit target configured the other way must still be recognized.
    await withSyncRoot(
      "repo",
      async (syncRoot) => {
        await write(syncRoot, "f/a__raw_app/raw_app.yaml", "value: {}\n");
        await write(syncRoot, "f/a__raw_app/backend/config.script.lock", "x\n");

        const report = await runLint({} as any, syncRoot);

        expect(report.issues).toEqual([]);
      },
      { runFromParent: true },
    );
  });

  test("keeps the reported path whole when the lint target is a path segment", async () => {
    await withSyncRoot("repo", async (syncRoot) => {
      await write(syncRoot, "f/conf/orphan.script.yaml", METADATA);

      const report = await runLint({} as any, path.join(syncRoot, "f"));

      expect(report.issues.map((i) => i.path)).toEqual([
        "conf/orphan.script.yaml",
      ]);
      expect(report.issues[0].errors[0]).toContain("conf/orphan.script.yaml");
    });
  });

  test("does not report a fileset child spelled like script metadata", async () => {
    await withSyncRoot("repo", async (syncRoot) => {
      await write(syncRoot, "f/data.resource.yaml", "value: {}\n");
      await write(syncRoot, "f/data.fileset/config.script.yaml", "a: 1\n");

      const report = await runLint({} as any, syncRoot);

      expect(report.issues).toEqual([]);
    });
  });

  test("does not report a dbt project whose optional descriptor is absent", async () => {
    await withSyncRoot("repo", async (syncRoot) => {
      await write(syncRoot, "f/proj.script.yaml", METADATA);
      await write(syncRoot, "f/proj__dbt/dbt_project.yml", "name: proj\n");
      await write(syncRoot, "f/proj__dbt/models/a.sql", "select 1\n");

      const report = await runLint({} as any, syncRoot);

      expect(report.issues).toEqual([]);
    });
  });
});
