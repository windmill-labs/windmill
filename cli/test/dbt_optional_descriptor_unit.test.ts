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
import { listWorkspacePaths } from "../src/commands/dev/dev.ts";
import {
  findContentFile,
  hasScriptExt,
  removeExtensionToPath,
} from "../src/commands/script/script.ts";

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

  // The descriptor is the one "extension" that contains a separator, so on
  // Windows it is spelled `__dbt\wm_dbt.yaml` and a forward-slash suffix test
  // matches nothing — every dbt project skipped, with no error.
  test("is recognized when the path is spelled with backslashes", () => {
    const win = "f\\analytics\\analytics__dbt\\wm_dbt.yaml";
    expect(hasScriptExt(win)).toBe(true);
    expect(removeExtensionToPath(win)).toBe("f\\analytics\\analytics");
  });

  // Both `<base>.py` and `<base>__dbt/` deploy to the SAME remote path, and the
  // descriptor is optional — so the project is invisible to the candidate list
  // while being perfectly real. Resolved to the ordinary file, a model edit
  // deploys the Python script over the dbt one.
  test("refuses to resolve when an ordinary script shares its path", async () => {
    const cwd = process.cwd();
    process.chdir(dir);
    try {
      fs.writeFileSync(path.join(dir, "f/analytics/analytics.py"), "def main(): ...");
      const err = await findContentFile("f/analytics/analytics.script.yaml").then(
        () => undefined,
        (e) => e as Error,
      );
      expect(err?.message).toContain("f/analytics/analytics__dbt/dbt_project.yml");
      expect(err?.message).toContain("f/analytics/analytics.py");
    } finally {
      process.chdir(cwd);
    }
  });

  // The two sides spell "absent" differently — nothing on disk, nothing in the
  // export — so without one normalization a descriptor-less project reads as an
  // addition on every push and a deletion on every pull, forever.
  test("compares equal to a remote that carries no descriptor either", async () => {
    const remote = {
      path: "",
      isDirectory: true,
      getChildren: async function* () {
        for (const p of [
          "f/analytics/analytics__dbt/dbt_project.yml",
          "f/analytics/analytics__dbt/models/stg_orders.sql",
          "f/analytics/analytics.script.yaml",
        ]) {
          yield {
            path: p,
            isDirectory: false,
            getChildren: async function* () {},
            getContentText: async () => "x",
          };
        }
      },
      getContentText: async () => "",
    };
    const local = await elementsToMap(
      await FSFSElement(dir, [], true),
      () => false,
      false,
      {},
    );
    const remoteMap = await elementsToMap(remote as any, () => false, false, {});
    const key = "f/analytics/analytics__dbt/wm_dbt.yaml";
    expect(local[key]).toBe("");
    expect(remoteMap[key]).toBe("");
  });
});

// `wmill dev` walks basenames, and a dbt script is a DIRECTORY whose descriptor
// may not exist — so it is recognized by the project folder or not at all. The
// walk must also stop there: the project's own `.sql` models match the script
// extensions and would each be listed as a script of their own.
describe("dev-mode discovery of a dbt project", () => {
	let dir: string
	let cwd: string

	beforeEach(() => {
		dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wmill-dbt-dev-'))
		fs.mkdirSync(path.join(dir, 'f/analytics/analytics__dbt/models'), { recursive: true })
		fs.writeFileSync(path.join(dir, 'f/analytics/analytics__dbt/dbt_project.yml'), 'name: a\n')
		fs.writeFileSync(path.join(dir, 'f/analytics/analytics__dbt/models/stg.sql'), 'select 1')
		cwd = process.cwd()
		process.chdir(dir)
	})

	afterEach(() => {
		process.chdir(cwd)
		fs.rmSync(dir, { recursive: true, force: true })
	})

	test('lists the project itself and nothing inside it', async () => {
		const items = await listWorkspacePaths()
		const paths = items.map((i) => i.path).sort()
		expect(paths).toEqual(['f/analytics/analytics'])
	})
})
