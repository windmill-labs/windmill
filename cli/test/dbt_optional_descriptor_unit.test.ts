/**
 * An unmodified dbt project is already a complete Windmill script: the
 * descriptor is optional, and a project that never names one must push, diff
 * and pull without ever growing a Windmill file inside it.
 */
import { expect, test, describe, beforeEach, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { FSFSElement, elementsToMap } from "../src/commands/sync/sync.ts";
import { findContentFile } from "../src/commands/script/script.ts";

describe("a dbt project without a descriptor", () => {
  let dir: string;

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "wmill-dbt-nodesc-"));
    fs.mkdirSync(path.join(dir, "f/analytics/analytics__dbt/models"), {
      recursive: true,
    });
    fs.writeFileSync(
      path.join(dir, "f/analytics/analytics__dbt/dbt_project.yml"),
      "name: analytics\n",
    );
    fs.writeFileSync(
      path.join(dir, "f/analytics/analytics__dbt/models/stg_orders.sql"),
      "select 1",
    );
    fs.writeFileSync(path.join(dir, "f/analytics/analytics.script.yaml"), "{}");
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  // Without this the project has no content file, so nothing identifies it as a
  // script and the whole project silently never deploys.
  test("is still discovered, as an empty descriptor", async () => {
    const root = await FSFSElement(dir, [], true);
    const map = await elementsToMap(root, () => false, false, {});
    expect(map["f/analytics/analytics__dbt/wm_dbt.yaml"]).toBe("");
  });

  // The metadata has to resolve to a content path that is not on disk, or every
  // caller that goes metadata -> content aborts the push.
  test("resolves from its metadata to the absent descriptor", async () => {
    const cwd = process.cwd();
    process.chdir(dir);
    try {
      expect(await findContentFile("f/analytics/analytics.script.yaml")).toBe(
        "f/analytics/analytics__dbt/wm_dbt.yaml",
      );
    } finally {
      process.chdir(cwd);
    }
  });
});
