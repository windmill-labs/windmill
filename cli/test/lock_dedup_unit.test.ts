/**
 * Lockfile deduplication (`dedupeLockfiles`) — WIN-1756.
 *
 * A workspace with one `dependencies/requirements.in` resolves the same lock for
 * every Python script, so the repo carries thousands of identical
 * `.script.lock` files: a dependency bump rewrites all of them and every open
 * branch conflicts on all of them. Dedup keeps one lockfile per dependency file,
 * named after it, and points the metadata at it.
 *
 * What these pin, per the invariants in `lock_dedup.ts`:
 *  - the name comes from the dependency file, so a bump is a one-file diff and a
 *    sync narrowed to one script reaches the same answer as a full one
 *  - a script whose lock is its own (an `extra_`/inline annotation, or several
 *    dependency files at once) keeps a `.script.lock`
 *  - the pass is idempotent, which is what keeps `sync pull`/`push` from seeing
 *    a diff on every run
 */

import { expect, test, describe } from "bun:test";
import { stringify as yamlStringify } from "yaml";
import * as path from "node:path";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import {
  applySharedLockPlanToMap,
  computeSharedLockPlan,
  isEmptySharedLockPlan,
  scriptsReferencingSharedLock,
  sharedLockRefIn,
} from "../src/utils/lock_dedup.ts";
import { isSharedLockPath } from "../src/utils/script_common.ts";
import { yamlOptions } from "../src/commands/sync/sync.ts";

const PY_LOCK = "requests==2.32.0\nurllib3==2.2.1\n";
const PY_LOCK_BUMPED = "requests==2.32.3\nurllib3==2.2.1\n";
const OTHER_LOCK = "requests==2.32.0\nurllib3==2.2.1\npandas==2.2.0\n";
const PY_DEPS = "dependencies/requirements.in";
const TEAM_DEPS = "dependencies/team_a.requirements.in";
const BUN_DEPS = "dependencies/package.json";
const SHARED_PY = "locks/requirements.in.lock";
const SHARED_TEAM = "locks/team_a.requirements.in.lock";
const SHARED_BUN = "locks/package.json.lock";

function meta(lockRef: string, summary = ""): string {
  return yamlStringify({ summary, lock: lockRef }, yamlOptions);
}

/** A workspace holding the given dependency files. */
function workspace(...depFiles: string[]): Record<string, string> {
  const map: Record<string, string> = {};
  for (const dep of depFiles) map[dep] = "requests\n";
  return map;
}

/** A script with its own lockfile, as `sync pull` writes it without dedup. */
function ownLock(
  map: Record<string, string>,
  base: string,
  ext: string,
  lock: string,
  body = "def main(): ...",
) {
  map[`${base}${ext}`] = body;
  map[`${base}.script.yaml`] = meta(`!inline ${base}.script.lock`);
  map[`${base}.script.lock`] = lock;
}

/** A script already reading a shared lockfile. */
function sharedLock(
  map: Record<string, string>,
  base: string,
  ext: string,
  shared: string,
  body = "def main(): ...",
) {
  map[`${base}${ext}`] = body;
  map[`${base}.script.yaml`] = meta(`!inline ${shared}`);
}

function lockRefOf(metaContent: string): string {
  return metaContent.match(/lock: '(.*)'/)![1];
}

const plan = (map: Record<string, string>) =>
  computeSharedLockPlan(map, { defaultTs: "bun" });

describe("computeSharedLockPlan", () => {
  test("collapses the scripts of a dependency file into its lockfile", () => {
    const map = workspace(PY_DEPS, BUN_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    ownLock(map, "f/c", ".py", PY_LOCK);
    ownLock(map, "f/ts", ".ts", "bun-lock", "export async function main() {}");
    ownLock(map, "f/ts2", ".ts", "bun-lock", "export async function main() {}");

    applySharedLockPlanToMap(map, plan(map));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map[SHARED_BUN]).toEqual("bun-lock");
    for (const base of ["f/a", "f/b", "f/c"]) {
      expect(map[`${base}.script.lock`]).toBeUndefined();
      expect(lockRefOf(map[`${base}.script.yaml`])).toEqual(
        `!inline ${SHARED_PY}`,
      );
    }
    expect(lockRefOf(map["f/ts.script.yaml"])).toEqual(`!inline ${SHARED_BUN}`);
  });

  test("is idempotent — a deduplicated tree yields no further changes", () => {
    const map = workspace(PY_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    applySharedLockPlanToMap(map, plan(map));

    expect(isEmptySharedLockPlan(plan(map))).toBe(true);
  });

  test("a dependency bump moves only the shared file", () => {
    const committed = workspace(PY_DEPS);
    sharedLock(committed, "f/a", ".py", SHARED_PY);
    sharedLock(committed, "f/b", ".py", SHARED_PY);
    committed[SHARED_PY] = PY_LOCK;

    // What the remote sends after the bump: one lock per script, all bumped.
    const remote = workspace(PY_DEPS);
    ownLock(remote, "f/a", ".py", PY_LOCK_BUMPED);
    ownLock(remote, "f/b", ".py", PY_LOCK_BUMPED);
    applySharedLockPlanToMap(remote, plan(remote));

    expect(
      Object.keys(remote).filter((k) => remote[k] !== committed[k]),
    ).toEqual([SHARED_PY]);
  });

  test("each named dependency file gets a lockfile of its own", () => {
    const map = workspace(PY_DEPS, TEAM_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    const team = "# requirements: team_a\ndef main(): ...";
    ownLock(map, "f/t1", ".py", OTHER_LOCK, team);
    ownLock(map, "f/t2", ".py", OTHER_LOCK, team);

    applySharedLockPlanToMap(map, plan(map));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map[SHARED_TEAM]).toEqual(OTHER_LOCK);
    expect(lockRefOf(map["f/t1.script.yaml"])).toEqual(`!inline ${SHARED_TEAM}`);
  });

  // The git-sync deploy callback narrows the sync to a single item, so this is
  // the normal case rather than an edge one.
  test("one script in view reaches the same answer as the whole workspace", () => {
    const full = workspace(PY_DEPS);
    for (const base of ["f/a", "f/b", "f/c"]) {
      ownLock(full, base, ".py", PY_LOCK);
    }
    applySharedLockPlanToMap(full, plan(full));

    const narrow = workspace(PY_DEPS);
    ownLock(narrow, "f/a", ".py", PY_LOCK);
    applySharedLockPlanToMap(narrow, plan(narrow));

    expect(narrow[SHARED_PY]).toEqual(full[SHARED_PY]);
    expect(lockRefOf(narrow["f/a.script.yaml"])).toEqual(
      lockRefOf(full["f/a.script.yaml"]),
    );
  });

  test("a script whose lock is its own keeps a private lockfile", () => {
    const map = workspace(PY_DEPS, TEAM_DEPS);
    // `extra_` folds the script's own imports into the lock…
    const extra = "# extra_requirements: default\ndef main(): ...";
    ownLock(map, "f/extra", ".py", OTHER_LOCK, extra);
    // …and naming two files makes it no single file's lock.
    const both = "# requirements: default, team_a\ndef main(): ...";
    ownLock(map, "f/both", ".py", OTHER_LOCK, both);
    ownLock(map, "f/plain", ".py", PY_LOCK);
    ownLock(map, "f/plain2", ".py", PY_LOCK);

    applySharedLockPlanToMap(map, plan(map));

    expect(map["f/extra.script.lock"]).toEqual(OTHER_LOCK);
    expect(map["f/both.script.lock"]).toEqual(OTHER_LOCK);
    expect(lockRefOf(map["f/extra.script.yaml"])).toEqual(
      "!inline f/extra.script.lock",
    );
    expect(map[SHARED_PY]).toEqual(PY_LOCK);
  });

  test("a script with no dependency file behind it is left alone", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);

    expect(isEmptySharedLockPlan(plan(map))).toBe(true);
  });

  test("a stale committed lock is outvoted, and keeps its own", () => {
    const map = workspace(PY_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK_BUMPED);
    ownLock(map, "f/b", ".py", PY_LOCK_BUMPED);
    ownLock(map, "f/stale", ".py", PY_LOCK);

    applySharedLockPlanToMap(map, plan(map));

    expect(map[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
    expect(map["f/stale.script.lock"]).toEqual(PY_LOCK);
  });

  test("a shared lockfile outlives its scripts but not its dependency file", () => {
    // No script in view reads it — a narrowed sync must still leave it be.
    const withDep = workspace(PY_DEPS);
    withDep[SHARED_PY] = PY_LOCK;
    expect(plan(withDep).deletes).not.toContain(SHARED_PY);

    const withoutDep: Record<string, string> = { [SHARED_PY]: PY_LOCK };
    expect(plan(withoutDep).deletes).toContain(SHARED_PY);
  });

  // The git-sync deploy callback narrows to one item, so a dependency file with
  // no script in view is routine — and its lockfile is read by scripts this sync
  // cannot see.
  test("a dependency file with no script in view keeps its lockfile", () => {
    const map = workspace(PY_DEPS, BUN_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    const present = { [SHARED_BUN]: "bun-lock" };

    const result = computeSharedLockPlan(map, { defaultTs: "bun", present });
    applySharedLockPlanToMap(map, result);

    expect(result.deletes).not.toContain(SHARED_BUN);
    // Carried into the map, or the diff reads it as a local-only deletion.
    expect(map[SHARED_BUN]).toEqual("bun-lock");
  });

  test("dependency files absent from the map are not gone", () => {
    // `--skip-workspace-dependencies` keeps them out of both maps; taking that
    // as "deleted" would un-deduplicate the tree and sweep the lockfiles.
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);

    const result = computeSharedLockPlan(map, {
      defaultTs: "bun",
      depFiles: [PY_DEPS],
      present: { [SHARED_PY]: PY_LOCK },
    });
    applySharedLockPlanToMap(map, result);

    expect(result.deletes).not.toContain(SHARED_PY);
    expect(lockRefOf(map["f/a.script.yaml"])).toEqual(`!inline ${SHARED_PY}`);
  });

  test("a script that disagrees moves the lockfile only if the file moved", () => {
    // The dependency file is unchanged, so this script pins something the
    // worker also locks (a Python version, an `//npm` annotation). It must not
    // hand that variant to every other script reading the file.
    const pinned = workspace(PY_DEPS);
    ownLock(pinned, "f/pinned", ".py", OTHER_LOCK);
    applySharedLockPlanToMap(
      pinned,
      computeSharedLockPlan(pinned, {
        defaultTs: "bun",
        present: { [SHARED_PY]: PY_LOCK },
      }),
    );
    expect(pinned[SHARED_PY]).toEqual(PY_LOCK);
    expect(pinned["f/pinned.script.lock"]).toEqual(OTHER_LOCK);

    // The same shape, but the dependency file moved: one script is enough, or a
    // sync narrowed to one item could never carry a bump through.
    const bumped = workspace(PY_DEPS);
    ownLock(bumped, "f/a", ".py", PY_LOCK_BUMPED);
    applySharedLockPlanToMap(
      bumped,
      computeSharedLockPlan(bumped, {
        defaultTs: "bun",
        present: { [SHARED_PY]: PY_LOCK },
        movedDepFiles: [PY_DEPS],
      }),
    );
    expect(bumped[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
    expect(bumped["f/a.script.lock"]).toBeUndefined();
    expect(lockRefOf(bumped["f/a.script.yaml"])).toEqual(`!inline ${SHARED_PY}`);
  });

  test("a lock's own dependency stamp says whether it may move the file", () => {
    const stamp = (dep: string, body: string) =>
      `# workspace-dependencies-mode: manual\n# workspace-dependencies: default:${dep}\n${body}`;

    // Same dependency inputs, different body: this script pinned something, and
    // the file is not its to move — even alone, and even mid-bump.
    const pinned = workspace(PY_DEPS);
    ownLock(pinned, "f/pinned", ".py", stamp("aaa", OTHER_LOCK));
    applySharedLockPlanToMap(
      pinned,
      computeSharedLockPlan(pinned, {
        defaultTs: "bun",
        present: { [SHARED_PY]: stamp("aaa", PY_LOCK) },
        movedDepFiles: [PY_DEPS],
      }),
    );
    expect(pinned[SHARED_PY]).toEqual(stamp("aaa", PY_LOCK));
    expect(pinned["f/pinned.script.lock"]).toEqual(stamp("aaa", OTHER_LOCK));

    // Different dependency inputs: the file moved, and one script is enough.
    const bumped = workspace(PY_DEPS);
    ownLock(bumped, "f/a", ".py", stamp("bbb", PY_LOCK_BUMPED));
    applySharedLockPlanToMap(
      bumped,
      computeSharedLockPlan(bumped, {
        defaultTs: "bun",
        present: { [SHARED_PY]: stamp("aaa", PY_LOCK) },
      }),
    );
    expect(bumped[SHARED_PY]).toEqual(stamp("bbb", PY_LOCK_BUMPED));
    expect(bumped["f/a.script.lock"]).toBeUndefined();
  });

  test("a dependency file that re-resolves without changing still moves", () => {
    // An unpinned `requirements.in` picking up a new patch release: same stamp,
    // new bodies. Read as variants, the whole group would fork to private locks.
    const stamp = (body: string) =>
      `# workspace-dependencies: default:aaa\n${body}`;
    const map = workspace(PY_DEPS);
    ownLock(map, "f/a", ".py", stamp(PY_LOCK_BUMPED));
    ownLock(map, "f/b", ".py", stamp(PY_LOCK_BUMPED));

    applySharedLockPlanToMap(
      map,
      computeSharedLockPlan(map, {
        defaultTs: "bun",
        present: { [SHARED_PY]: stamp(PY_LOCK) },
      }),
    );

    expect(map[SHARED_PY]).toEqual(stamp(PY_LOCK_BUMPED));
    expect(map["f/a.script.lock"]).toBeUndefined();
  });

  test("the many correct a lockfile a lone script planted its variant on", () => {
    const map = workspace(PY_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    applySharedLockPlanToMap(
      map,
      // The file holds a variant, and its dependency file has not moved.
      computeSharedLockPlan(map, {
        defaultTs: "bun",
        present: { [SHARED_PY]: OTHER_LOCK },
      }),
    );

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map["f/a.script.lock"]).toBeUndefined();
  });

  test("a dependency file with a single script can still take a bump", () => {
    const map = workspace(PY_DEPS);
    ownLock(map, "f/only", ".py", PY_LOCK_BUMPED);
    applySharedLockPlanToMap(
      map,
      computeSharedLockPlan(map, {
        defaultTs: "bun",
        present: { [SHARED_PY]: PY_LOCK },
        movedDepFiles: [PY_DEPS],
      }),
    );

    expect(map[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
    expect(map["f/only.script.lock"]).toBeUndefined();
  });

  test("a language that needs no lock is never deduplicated", () => {
    const map = workspace("dependencies/modules.json");
    ownLock(map, "f/a", ".ps1", "some-lock", "echo hi");
    ownLock(map, "f/b", ".ps1", "some-lock", "echo hi");

    expect(isEmptySharedLockPlan(plan(map))).toBe(true);
  });

  test("module-layout scripts share too, from their folder", () => {
    const map = workspace(PY_DEPS);
    for (const base of ["f/a__mod", "f/b__mod"]) {
      map[`${base}/script.py`] = "def main(): ...";
      map[`${base}/script.yaml`] = meta(`!inline ${base}/script.lock`);
      map[`${base}/script.lock`] = PY_LOCK;
    }

    applySharedLockPlanToMap(map, plan(map));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map["f/a__mod/script.lock"]).toBeUndefined();
  });

  test("a script path containing dots reads its own content file", () => {
    const map = workspace(PY_DEPS, BUN_DEPS);
    ownLock(map, "f/a.b", ".py", PY_LOCK);
    ownLock(map, "f/c.d", ".py", PY_LOCK);
    // `f/a` is a bun script whose name is a prefix of `f/a.b`: its language and
    // its annotation must come from its own file, not its neighbour's.
    ownLock(map, "f/a", ".ts", "bun-lock", "export async function main() {}");
    ownLock(map, "f/e", ".ts", "bun-lock", "export async function main() {}");

    applySharedLockPlanToMap(map, plan(map));

    expect(lockRefOf(map["f/a.b.script.yaml"])).toEqual(`!inline ${SHARED_PY}`);
    expect(lockRefOf(map["f/a.script.yaml"])).toEqual(`!inline ${SHARED_BUN}`);
  });

  test("only the lock line of the metadata changes", () => {
    const map = workspace(PY_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    map["f/a.script.yaml"] = meta("!inline f/a.script.lock", "does a thing");
    const before = map["f/a.script.yaml"];

    applySharedLockPlanToMap(map, plan(map));

    expect(map["f/a.script.yaml"]).toEqual(
      before.replace("f/a.script.lock", SHARED_PY),
    );
  });
});

describe("isSharedLockPath", () => {
  test("claims only the names a dependency file gives", () => {
    for (const p of [SHARED_PY, SHARED_TEAM, SHARED_BUN]) {
      expect(isSharedLockPath(p)).toBe(true);
    }
    // Not a dependency file's name, so a repo that already keeps lockfiles here
    // keeps them; `modules.json` is powershell, which takes no lock.
    for (const p of [
      "locks/vendor.lock",
      "locks/Cargo.lock",
      "locks/modules.json.lock",
      "locks/sub/requirements.in.lock",
      "locks/../../escape.lock",
    ]) {
      expect(isSharedLockPath(p)).toBe(false);
    }
  });
});

describe("scriptsReferencingSharedLock", () => {
  test("finds every script on the shared lock and nothing else", () => {
    const map = workspace(PY_DEPS);
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    const extra = "# extra_requirements: default\ndef main(): ...";
    ownLock(map, "f/extra", ".py", OTHER_LOCK, extra);
    applySharedLockPlanToMap(map, plan(map));

    expect(scriptsReferencingSharedLock(map, SHARED_PY).sort()).toEqual([
      "f/a.script.yaml",
      "f/b.script.yaml",
    ]);
  });

  test("prose naming the file is not a reference", () => {
    const map = {
      "f/a.py": "def main(): ...",
      "f/a.script.yaml": yamlStringify(
        { summary: `see !inline ${SHARED_PY}`, lock: "!inline f/a.script.lock" },
        yamlOptions,
      ),
    };
    expect(scriptsReferencingSharedLock(map, SHARED_PY)).toEqual([]);
  });
});

describe("sharedLockRefIn", () => {
  test("returns a reference only when it is one, and it exists", async () => {
    const root = await mkdtemp(path.join(os.tmpdir(), "wmill-dedup-ref-"));
    try {
      await mkdir(path.join(root, "locks"), { recursive: true });
      await writeFile(path.join(root, SHARED_PY), PY_LOCK, "utf-8");

      expect(sharedLockRefIn(meta(`!inline ${SHARED_PY}`), false, root)).toEqual(
        SHARED_PY,
      );
      // Flow-style YAML starts with `{` and is not JSON: deciding the format by
      // the first character would drop the reference and repoint the script.
      expect(
        sharedLockRefIn(
          `{summary: x, lock: '!inline ${SHARED_PY}'}`,
          false,
          root,
        ),
      ).toEqual(SHARED_PY);
      // A regenerated script falls back to its own lock rather than point at a
      // shared file that is not there.
      expect(
        sharedLockRefIn(meta(`!inline ${SHARED_BUN}`), false, root),
      ).toBeUndefined();
      expect(
        sharedLockRefIn(meta("!inline f/a.script.lock"), false, root),
      ).toBeUndefined();
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
