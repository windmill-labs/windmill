/**
 * An unmodified dbt project is already a complete Windmill script: the
 * descriptor is optional, and a project that never names one must push, diff
 * and pull without ever growing a Windmill file inside it.
 */
import { expect, test, describe, beforeEach, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  FSFSElement,
  dropDeletions,
  elementsToMap,
} from "../src/commands/sync/sync.ts";
import { listWorkspacePaths } from "../src/commands/dev/dev.ts";
import {
  DbtPathCollisionError,
  findContentFile,
  handleFile,
  hasScriptExt,
  removeExtensionToPath,
} from "../src/commands/script/script.ts";
import { pushParentScriptForModule } from "../src/commands/sync/sync.ts";

/** The local map's keys are the walk's own — `path.join`, so `__dbt\\` on
 *  Windows — while a remote's are the API's. The synthesized descriptor follows
 *  the spelling of the `dbt_project.yml` it was derived from, like every other
 *  key in that map, so an assertion on one platform's separator tests the
 *  platform and not the synthesis. */
const normalized = (m: Record<string, string>) =>
  Object.fromEntries(
    Object.entries(m).map(([k, v]) => [k.replaceAll("\\", "/"), v]),
  );

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
    expect(normalized(map)["f/analytics/analytics__dbt/wm_dbt.yaml"]).toBe("");
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

  // The guard has to sit on the push paths themselves, not only on the
  // metadata->content resolution: an ordinary file goes straight to
  // `handleFile`, and a module edit reaches its parent through a call whose
  // errors were swallowed — so each path could still overwrite the other's
  // script while reporting success.
  test("both push paths refuse the collision", async () => {
    const cwd = process.cwd();
    process.chdir(dir);
    try {
      fs.writeFileSync(path.join(dir, "f/analytics/analytics.py"), "def main(): ...");
      const ordinary = await handleFile(
        "f/analytics/analytics.py",
        { workspaceId: "w", remote: "http://localhost", name: "w", token: "t" } as any,
        [],
        undefined,
        undefined,
        {},
        [],
      ).then(
        () => undefined,
        (e) => e as Error,
      );
      expect(ordinary).toBeInstanceOf(DbtPathCollisionError);
      expect(ordinary?.message).toContain("f/analytics/analytics.py");

      const model = await pushParentScriptForModule(
        "f/analytics/analytics__dbt/models/stg_orders.sql",
        { workspaceId: "w", remote: "http://localhost", name: "w", token: "t" } as any,
        [],
        undefined,
        undefined,
        {},
        [],
      ).then(
        () => undefined,
        (e) => e as Error,
      );
      expect(model).toBeInstanceOf(DbtPathCollisionError);
    } finally {
      process.chdir(cwd);
    }
  });

  // The guard above must not fire on the project's OWN descriptor: that file is
  // the dbt script's content, and its base resolves to the same
  // `<base>__dbt/dbt_project.yml` — so a naive check finds the project
  // colliding with itself and every dbt push fails before deploying anything.
  test("a project does not collide with itself", async () => {
    const cwd = process.cwd();
    process.chdir(dir);
    const remote = { workspaceId: "w", remote: "http://127.0.0.1:1", name: "w", token: "t" };
    const push = (p: string) =>
      handleFile(p, remote as any, [], undefined, undefined, {}, []).then(
        () => undefined,
        (e) => e as Error,
      );
    try {
      // Descriptor-less: the fixture's project, pushed through the module path.
      const nodesc = await pushParentScriptForModule(
        "f/analytics/analytics__dbt/models/stg_orders.sql",
        remote as any,
        [],
        undefined,
        undefined,
        {},
        [],
      ).then(
        () => undefined,
        (e) => e as Error,
      );
      expect(nodesc).not.toBeInstanceOf(DbtPathCollisionError);

      // Descriptor present, pushed directly. Both reach the network — which is
      // unreachable here on purpose — so anything BUT the collision is a pass.
      fs.writeFileSync(
        path.join(dir, "f/analytics/analytics__dbt/wm_dbt.yaml"),
        "profile: {}\n",
      );
      expect(await push("f/analytics/analytics__dbt/wm_dbt.yaml")).not.toBeInstanceOf(
        DbtPathCollisionError,
      );
    } finally {
      process.chdir(cwd);
    }
  });

  // The exemption above is only for the project's OWN marker. A descriptor
  // pushed DIRECTLY never passes through the metadata resolution that catches
  // the collision, so without this it would deploy over the ordinary script
  // sitting at the same remote path.
  test("but a descriptor still refuses an ordinary script at its path", async () => {
    const cwd = process.cwd();
    process.chdir(dir);
    try {
      fs.writeFileSync(
        path.join(dir, "f/analytics/analytics__dbt/wm_dbt.yaml"),
        "profile: {}\n",
      );
      fs.writeFileSync(path.join(dir, "f/analytics/analytics.py"), "def main(): ...");
      const err = await handleFile(
        "f/analytics/analytics__dbt/wm_dbt.yaml",
        { workspaceId: "w", remote: "http://127.0.0.1:1", name: "w", token: "t" } as any,
        [],
        undefined,
        undefined,
        {},
        [],
      ).then(
        () => undefined,
        (e) => e as Error,
      );
      expect(err).toBeInstanceOf(DbtPathCollisionError);
      expect(err?.message).toContain("f/analytics/analytics.py");
    } finally {
      process.chdir(cwd);
    }
  });

  // Both layouts deploy to the same remote path, and the folder layout is the
  // one whose base is NOT its filename: `<base>__mod/script.ts` deploys to
  // `<base>`, exactly where the dbt project goes.
  test("the collision holds for a folder-layout script too", async () => {
    const cwd = process.cwd();
    process.chdir(dir);
    const remote = { workspaceId: "w", remote: "http://127.0.0.1:1", name: "w", token: "t" };
    const push = (p: string) =>
      handleFile(p, remote as any, [], undefined, undefined, {}, []).then(
        () => undefined,
        (e) => e as Error,
      );
    try {
      fs.mkdirSync(path.join(dir, "f/analytics/analytics__mod"), { recursive: true });
      fs.writeFileSync(
        path.join(dir, "f/analytics/analytics__mod/script.ts"),
        "export async function main() {}",
      );
      fs.writeFileSync(
        path.join(dir, "f/analytics/analytics__dbt/wm_dbt.yaml"),
        "profile: {}\n",
      );

      // From the dbt side: the descriptor must find the `__mod` entry point.
      const fromDbt = await push("f/analytics/analytics__dbt/wm_dbt.yaml");
      expect(fromDbt).toBeInstanceOf(DbtPathCollisionError);
      expect(fromDbt?.message).toContain("analytics__mod/script.ts");

      // And from the ordinary side, whose base is not its filename.
      const fromMod = await push("f/analytics/analytics__mod/script.ts");
      expect(fromMod).toBeInstanceOf(DbtPathCollisionError);
      expect(fromMod?.message).toContain("analytics__dbt/dbt_project.yml");
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
    expect(normalized(local)[key]).toBe("");
    expect(normalized(remoteMap)[key]).toBe("");
  });
});

// Unguarded, `--keep-deleted` lets a pull remove the very file it promises to
// keep — the descriptor's warehouse and run arguments with it.
describe("--keep-deleted and a descriptor the remote no longer names", () => {
  const DESCRIPTOR = "f/analytics/analytics__dbt/wm_dbt.yaml";
  const emptied = {
    name: "edited",
    path: DESCRIPTOR,
    before: "warehouse: wh\n",
    after: "",
  };
  const unknownToTheMap = { name: "added", path: DESCRIPTOR, content: "" };
  const changeset = (descriptor: unknown) =>
    [
      descriptor,
      { name: "edited", path: "f/other/script.ts", before: "a", after: "b" },
    ] as Parameters<typeof dropDeletions>[0];

  test("survives a pull", () => {
    const changes = changeset(emptied);
    dropDeletions(changes, "local");
    expect(changes.map((c) => c.path)).toEqual(["f/other/script.ts"]);
  });

  // A stateful pull compares `.wmill`, so a descriptor missing from the compared
  // map can still be a working-tree file holding real settings.
  test("survives a pull that compared a map without it", () => {
    const changes = changeset(unknownToTheMap);
    dropDeletions(changes, "local");
    expect(changes.map((c) => c.path)).toEqual(["f/other/script.ts"]);
  });

  test("is still pushed, deleting no remote item", () => {
    const changes = changeset(emptied);
    dropDeletions(changes, "remote");
    expect(changes.map((c) => c.path)).toEqual([
      DESCRIPTOR,
      "f/other/script.ts",
    ]);
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
