/**
 * `buildTracker` decides whose top hash `wmill-lock.yaml` refreshes. A dbt
 * project is mostly files that are not Windmill script extensions — the project
 * file, `packages.yml`, schema YAML, seed CSVs — and its folder is spelled
 * `__dbt\` on Windows, so both the extension gate and a raw-path search for
 * `__dbt/` left the descriptor untracked and its hash stale.
 */
import { expect, test, describe, beforeEach, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { buildTracker, elementsToMap } from "../src/commands/sync/sync.ts";

describe("buildTracker with a dbt project", () => {
  let dir: string;
  let cwd: string;

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "wmill-dbt-tracker-"));
    cwd = process.cwd();
    process.chdir(dir);
    fs.mkdirSync(path.join(dir, "f/analytics/analytics__dbt/models"), {
      recursive: true,
    });
    fs.mkdirSync(path.join(dir, "f/analytics/analytics__dbt/seeds"), {
      recursive: true,
    });
    // The descriptor sits BESIDE the project folder, and `findContentFile`
    // resolves the metadata path to it by extension.
    fs.writeFileSync(path.join(dir, "f/analytics/analytics.script.yaml"), "{}");
    fs.writeFileSync(
      path.join(dir, "f/analytics/analytics.dbt.yaml"),
      "profile: {}\n",
    );
  });

  afterEach(() => {
    process.chdir(cwd);
    fs.rmSync(dir, { recursive: true, force: true });
  });

  const tracked = async (p: string) =>
    (await buildTracker([{ name: "edited", path: p, before: "", after: "" }]))
      .scripts;

  test("a model edit selects the descriptor", async () => {
    expect(
      await tracked("f/analytics/analytics__dbt/models/stg_orders.sql"),
    ).toEqual(["f/analytics/analytics.dbt.yaml"]);
  });

  test("so do the files that are not script extensions", async () => {
    for (const p of [
      "f/analytics/analytics__dbt/dbt_project.yml",
      "f/analytics/analytics__dbt/packages.yml",
      "f/analytics/analytics__dbt/models/_models.yml",
      "f/analytics/analytics__dbt/seeds/country_codes.csv",
    ]) {
      expect(await tracked(p)).toEqual(
        ["f/analytics/analytics.dbt.yaml"],
        `${p} left the descriptor untracked`,
      );
    }
  });

  // Regression: hoisting the module check above the extension gate made
  // `<base>__mod/script.yaml` — a folder-layout script's METADATA, which is an
  // entry-point path — look like its own content file. Pushed as one, the
  // metadata pass asks for the language of `.yaml` and aborts the command. Not a
  // dbt shape at all; reached by editing the summary of any modular script.
  test("a modular script's own metadata resolves to its content file", async () => {
    fs.mkdirSync(path.join(dir, "f/helpers/util__mod"), { recursive: true });
    fs.writeFileSync(path.join(dir, "f/helpers/util__mod/script.yaml"), "{}");
    fs.writeFileSync(
      path.join(dir, "f/helpers/util__mod/script.ts"),
      "export function main() {}\n",
    );
    expect(await tracked("f/helpers/util__mod/script.yaml")).toEqual([
      "f/helpers/util__mod/script.ts",
    ]);
  });

  test("and a Windows-separated path", async () => {
    expect(
      await tracked("f\\analytics\\analytics__dbt\\models\\stg_orders.sql"),
    ).toEqual(["f/analytics/analytics.dbt.yaml"]);
  });

  // A dbt descriptor is the script's CONTENT and is `.dbt.yaml` in both metadata
  // formats. `--json` drops every other `.yaml` as the metadata twin it does not
  // read — dropping this one too leaves a workspace whose dbt scripts have
  // metadata, a lock and a project bundle, but nothing to run.
  test("--json keeps the descriptor while dropping metadata yaml", async () => {
    const file = (path: string) => ({
      path,
      isDirectory: false,
      getChildren: async function* () {},
      getContentText: async () => "x",
    });
    const root = {
      path: "",
      isDirectory: true,
      getChildren: async function* () {
        yield file("f/analytics/analytics.dbt.yaml");
        yield file("f/analytics/analytics.script.yaml");
        yield file("f/analytics/analytics.script.json");
      },
      getContentText: async () => "",
    };
    const map = await elementsToMap(root as any, () => false, true, {});
    expect(Object.keys(map).sort()).toEqual([
      "f/analytics/analytics.dbt.yaml",
      "f/analytics/analytics.script.json",
    ]);
  });
});
