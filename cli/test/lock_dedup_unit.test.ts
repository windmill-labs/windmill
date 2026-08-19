/**
 * Lockfile deduplication (`dedupeLockfiles`) — WIN-1756.
 *
 * A workspace with one `dependencies/requirements.in` resolves the same lock for
 * every Python script, so the repo carries thousands of identical
 * `.script.lock` files: a dependency bump rewrites all of them and every open
 * branch conflicts on all of them. Dedup keeps one file per group under
 * `locks/` and points the metadata at it.
 *
 * What these pin, per the invariants in `lock_dedup.ts`:
 *  - a group keeps the file it already reads, so a bump is a one-file diff
 *  - nothing is rewritten or deleted under scripts the sync map does not cover
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
  collectExistingSharedLocks,
  computeSharedLockPlan,
  isEmptySharedLockPlan,
  scriptsReferencingSharedLock,
  sharedLockRefIn,
  type ExistingSharedLocks,
} from "../src/utils/lock_dedup.ts";
import { yamlOptions } from "../src/commands/sync/sync.ts";

const PY_LOCK = "requests==2.32.0\nurllib3==2.2.1\n";
const PY_LOCK_BUMPED = "requests==2.32.3\nurllib3==2.2.1\n";
const OTHER_LOCK = "requests==2.32.0\nurllib3==2.2.1\npandas==2.2.0\n";
const SHARED_PY = "locks/python3.lock";
const SHARED_PY_2 = "locks/python3-2.lock";
const SHARED_BUN = "locks/bun.lock";
const TRAVERSING_REF = "locks/../../../../tmp/escaped.lock";

function meta(lockRef: string, summary = ""): string {
  return yamlStringify({ summary, lock: lockRef }, yamlOptions);
}

/** A script with its own lock file, as `sync pull` writes it without dedup. */
function ownLock(
  map: Record<string, string>,
  base: string,
  ext: string,
  lock: string,
) {
  map[`${base}${ext}`] = "def main(): ...";
  map[`${base}.script.yaml`] = meta(`!inline ${base}.script.lock`);
  map[`${base}.script.lock`] = lock;
}

/** A script already reading a shared lock. */
function sharedLock(
  map: Record<string, string>,
  base: string,
  ext: string,
  shared: string,
) {
  map[`${base}${ext}`] = "def main(): ...";
  map[`${base}.script.yaml`] = meta(`!inline ${shared}`);
}

/** The working tree the map was taken from, as the unfiltered scan reports it. */
function treeOf(
  map: Record<string, string>,
  contents: Record<string, string> = {},
): ExistingSharedLocks {
  const refs = new Map<string, string>();
  const scripts = new Set<string>();
  for (const [key, value] of Object.entries(map)) {
    if (!key.endsWith(".script.yaml")) continue;
    scripts.add(key);
    const match = /!inline (locks\/[^\s'"]+\.lock)/.exec(value);
    if (match) refs.set(key, match[1]);
  }
  return { refs, scripts, contents: new Map(Object.entries(contents)) };
}

function lockRefOf(metaContent: string): string {
  return metaContent.match(/lock: '(.*)'/)![1];
}

describe("computeSharedLockPlan", () => {
  test("collapses identical locks into one file per language", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    ownLock(map, "f/c", ".py", PY_LOCK);
    ownLock(map, "f/ts", ".ts", "bun-lock-contents");
    ownLock(map, "f/ts2", ".ts", "bun-lock-contents");

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) }));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map[SHARED_BUN]).toEqual("bun-lock-contents");
    for (const base of ["f/a", "f/b", "f/c"]) {
      expect(map[`${base}.script.lock`]).toBeUndefined();
      expect(lockRefOf(map[`${base}.script.yaml`])).toEqual(
        `!inline ${SHARED_PY}`,
      );
    }
    expect(lockRefOf(map["f/ts.script.yaml"])).toEqual(`!inline ${SHARED_BUN}`);
  });

  test("is idempotent — a deduplicated tree yields no further changes", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    applySharedLockPlanToMap(map, computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) }));

    const tree = treeOf(map, { [SHARED_PY]: map[SHARED_PY] });
    expect(isEmptySharedLockPlan(computeSharedLockPlan(map, { defaultTs: "bun", existing: tree }))).toBe(
      true,
    );
  });

  test("a dependency bump moves only the shared file", () => {
    const committed: Record<string, string> = {};
    sharedLock(committed, "f/a", ".py", SHARED_PY);
    sharedLock(committed, "f/b", ".py", SHARED_PY);
    committed[SHARED_PY] = PY_LOCK;
    const tree = treeOf(committed, { [SHARED_PY]: PY_LOCK });

    // What the remote sends after the bump: one lock per script, all bumped.
    const remote: Record<string, string> = {};
    ownLock(remote, "f/a", ".py", PY_LOCK_BUMPED);
    ownLock(remote, "f/b", ".py", PY_LOCK_BUMPED);
    applySharedLockPlanToMap(
      remote,
      computeSharedLockPlan(remote, { defaultTs: "bun", existing: tree }),
    );

    const changed = Object.keys(remote).filter((k) => remote[k] !== committed[k]);
    expect(changed).toEqual([SHARED_PY]);
    expect(remote[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
  });

  test("a second group gets a name of its own, and keeps it", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    ownLock(map, "f/c", ".py", OTHER_LOCK);
    ownLock(map, "f/d", ".py", OTHER_LOCK);

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) }));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map[SHARED_PY_2]).toEqual(OTHER_LOCK);
    expect(lockRefOf(map["f/c.script.yaml"])).toEqual(`!inline ${SHARED_PY_2}`);

    // Bumping the second group moves its file, not its name.
    const tree = treeOf(map, {
      [SHARED_PY]: map[SHARED_PY],
      [SHARED_PY_2]: map[SHARED_PY_2],
    });
    const remote: Record<string, string> = {};
    ownLock(remote, "f/a", ".py", PY_LOCK);
    ownLock(remote, "f/b", ".py", PY_LOCK);
    ownLock(remote, "f/c", ".py", OTHER_LOCK + "click==8.1.7\n");
    ownLock(remote, "f/d", ".py", OTHER_LOCK + "click==8.1.7\n");
    applySharedLockPlanToMap(remote, computeSharedLockPlan(remote, { defaultTs: "bun", existing: tree }));

    expect(Object.keys(remote).filter((k) => remote[k] !== map[k])).toEqual([
      SHARED_PY_2,
    ]);
  });

  test("the majority keeps the file when a minority lags behind", () => {
    // The case the feature exists for: a bump that most scripts take. The one
    // that lags must not hold the name hostage and rename the file for the rest.
    const committed: Record<string, string> = {};
    for (const base of ["f/a", "f/b", "f/c", "f/d", "f/lag"]) {
      sharedLock(committed, base, ".py", SHARED_PY);
    }
    const tree = treeOf(committed, { [SHARED_PY]: PY_LOCK });

    const remote: Record<string, string> = {};
    for (const base of ["f/a", "f/b", "f/c", "f/d"]) {
      ownLock(remote, base, ".py", PY_LOCK_BUMPED);
    }
    ownLock(remote, "f/lag", ".py", PY_LOCK);
    applySharedLockPlanToMap(remote, computeSharedLockPlan(remote, { defaultTs: "bun", existing: tree }));

    expect(remote[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
    expect(remote[SHARED_PY_2]).toBeUndefined();
    for (const base of ["f/a", "f/b", "f/c", "f/d"]) {
      expect(lockRefOf(remote[`${base}.script.yaml`])).toEqual(
        `!inline ${SHARED_PY}`,
      );
    }
    expect(remote["f/lag.script.lock"]).toEqual(PY_LOCK);
  });

  test("two groups bumped at once each keep their own file", () => {
    // The shape that exposed a snapshot taken after regeneration: read late, the
    // groups' own references are gone and the two files trade contents while
    // every script's metadata is rewritten.
    const committed: Record<string, string> = {};
    for (const base of ["f/a", "f/b", "f/c"]) {
      sharedLock(committed, base, ".py", SHARED_PY);
    }
    for (const base of ["f/d", "f/e"]) {
      sharedLock(committed, base, ".py", SHARED_PY_2);
    }
    const tree = treeOf(committed, {
      [SHARED_PY]: PY_LOCK,
      [SHARED_PY_2]: OTHER_LOCK,
    });

    const remote: Record<string, string> = {};
    for (const base of ["f/a", "f/b", "f/c"]) {
      ownLock(remote, base, ".py", PY_LOCK_BUMPED);
    }
    for (const base of ["f/d", "f/e"]) {
      ownLock(remote, base, ".py", OTHER_LOCK + "click==8.1.7\n");
    }
    const plan = computeSharedLockPlan(remote, { defaultTs: "bun", existing: tree });
    applySharedLockPlanToMap(remote, plan);

    expect(remote[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
    expect(remote[SHARED_PY_2]).toEqual(OTHER_LOCK + "click==8.1.7\n");
    expect(lockRefOf(remote["f/a.script.yaml"])).toEqual(`!inline ${SHARED_PY}`);
    expect(lockRefOf(remote["f/d.script.yaml"])).toEqual(
      `!inline ${SHARED_PY_2}`,
    );
  });

  test("a lone script keeps its own lock", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);

    expect(
      isEmptySharedLockPlan(computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) })),
    ).toBe(true);
  });

  test("a script that diverges gets its own lock back, the rest keep the file", () => {
    const committed: Record<string, string> = {};
    sharedLock(committed, "f/a", ".py", SHARED_PY);
    sharedLock(committed, "f/b", ".py", SHARED_PY);
    sharedLock(committed, "f/c", ".py", SHARED_PY);
    committed[SHARED_PY] = PY_LOCK;
    const tree = treeOf(committed, { [SHARED_PY]: PY_LOCK });

    const remote: Record<string, string> = {};
    ownLock(remote, "f/a", ".py", PY_LOCK);
    ownLock(remote, "f/b", ".py", PY_LOCK);
    ownLock(remote, "f/c", ".py", OTHER_LOCK);
    applySharedLockPlanToMap(remote, computeSharedLockPlan(remote, { defaultTs: "bun", existing: tree }));

    expect(remote[SHARED_PY]).toEqual(PY_LOCK);
    expect(remote["f/c.script.lock"]).toEqual(OTHER_LOCK);
    expect(remote["f/a.script.lock"]).toBeUndefined();
  });

  test("a shared file nothing reads any more is removed", () => {
    const committed: Record<string, string> = {};
    sharedLock(committed, "f/a", ".py", SHARED_PY);
    sharedLock(committed, "f/b", ".py", SHARED_PY);
    const tree = treeOf(committed, { [SHARED_PY]: PY_LOCK });

    // Both scripts diverge, and to different locks: the file has no reader left.
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", OTHER_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK_BUMPED);

    const plan = computeSharedLockPlan(map, { defaultTs: "bun", existing: tree });
    expect(plan.deletes).toContain(SHARED_PY);
  });

  test("scripts without a lock file are untouched", () => {
    const map: Record<string, string> = {
      "f/a.py": "def main(): ...",
      "f/a.script.yaml": meta(""),
      "f/b.py": "def main(): ...",
      "f/b.script.yaml": meta(""),
    };

    expect(
      isEmptySharedLockPlan(computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) })),
    ).toBe(true);
  });

  test("a language that needs no lock is never deduplicated", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".sh", "some-lock");
    ownLock(map, "f/b", ".sh", "some-lock");

    expect(
      isEmptySharedLockPlan(computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) })),
    ).toBe(true);
  });

  test("module-layout scripts share too, from their folder", () => {
    const map: Record<string, string> = {
      "f/a__mod/script.py": "def main(): ...",
      "f/a__mod/script.yaml": meta("!inline f/a__mod/script.lock"),
      "f/a__mod/script.lock": PY_LOCK,
      "f/b__mod/script.py": "def main(): ...",
      "f/b__mod/script.yaml": meta("!inline f/b__mod/script.lock"),
      "f/b__mod/script.lock": PY_LOCK,
    };

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, { defaultTs: "bun" }));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map["f/a__mod/script.lock"]).toBeUndefined();
  });

  test("a script path containing dots is deduplicated like any other", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a.b", ".py", PY_LOCK);
    ownLock(map, "f/c.d", ".py", PY_LOCK);
    // `f/a` is a Go script that happens to be a prefix of `f/a.b`: its language
    // must come from its own content file, not its neighbour's.
    ownLock(map, "f/a", ".go", "go-lock-contents");
    ownLock(map, "f/e", ".go", "go-lock-contents");

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) }));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(lockRefOf(map["f/a.b.script.yaml"])).toEqual(`!inline ${SHARED_PY}`);
    expect(map["locks/go.lock"]).toEqual("go-lock-contents");
    expect(lockRefOf(map["f/a.script.yaml"])).toEqual(
      "!inline locks/go.lock",
    );
  });

  test("only the lock line of the metadata changes", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    map["f/a.script.yaml"] = meta("!inline f/a.script.lock", "does a thing");
    const before = map["f/a.script.yaml"];

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) }));

    expect(map["f/a.script.yaml"]).toEqual(
      before.replace("f/a.script.lock", SHARED_PY),
    );
  });
});

// The git-sync deploy callback pulls with `extraIncludes` scoped to the item it
// deploys, so a map holding one script out of a thousand is the normal case,
// not an edge one. Anything the plan does to a shared file there lands on
// scripts it cannot see.
describe("computeSharedLockPlan with a partial sync map", () => {
  /** The tree: three scripts on the shared lock. The map: only one of them. */
  function partial(inScopeLock: string) {
    const committed: Record<string, string> = {};
    for (const base of ["f/a", "f/b", "f/c"]) {
      sharedLock(committed, base, ".py", SHARED_PY);
    }
    const tree = treeOf(committed, { [SHARED_PY]: PY_LOCK });
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", inScopeLock);
    return { map, tree };
  }

  test("keeps the shared file the scripts out of scope read", () => {
    const { map, tree } = partial(PY_LOCK);
    const plan = computeSharedLockPlan(map, { defaultTs: "bun", existing: tree });
    applySharedLockPlanToMap(map, plan);

    expect(plan.deletes).not.toContain(SHARED_PY);
    // Carried into the map, or the diff would delete the committed file.
    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(lockRefOf(map["f/a.script.yaml"])).toEqual(`!inline ${SHARED_PY}`);
  });

  test("never rewrites the shared file from the scripts it can see", () => {
    const { map, tree } = partial(OTHER_LOCK);
    const plan = computeSharedLockPlan(map, { defaultTs: "bun", existing: tree });
    applySharedLockPlanToMap(map, plan);

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    // The one script in scope takes a lock of its own instead.
    expect(map["f/a.script.lock"]).toEqual(OTHER_LOCK);
    expect(lockRefOf(map["f/a.script.yaml"])).toEqual(
      "!inline f/a.script.lock",
    );
  });

  test("a script added locally but not yet pushed does not break the group", () => {
    // On a pull the map is the REMOTE, so a script that exists only on disk is
    // absent from it while being fully in scope. Counted as an unseen reader it
    // would veto the bump and explode the group into per-script locks.
    const committed: Record<string, string> = {};
    for (const base of ["f/a", "f/b", "f/local"]) {
      sharedLock(committed, base, ".py", SHARED_PY);
    }
    const tree = treeOf(committed, { [SHARED_PY]: PY_LOCK });

    const remote: Record<string, string> = {};
    ownLock(remote, "f/a", ".py", PY_LOCK_BUMPED);
    ownLock(remote, "f/b", ".py", PY_LOCK_BUMPED);

    applySharedLockPlanToMap(
      remote,
      computeSharedLockPlan(remote, {
        defaultTs: "bun",
        existing: tree,
        inScope: (metaRef) => committed[metaRef] !== undefined,
      }),
    );

    expect(remote[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
    expect(remote["f/a.script.lock"]).toBeUndefined();
  });

  test("forms no new group it cannot see the whole of", () => {
    const committed: Record<string, string> = {};
    for (const base of ["f/a", "f/b", "f/c"]) {
      ownLock(committed, base, ".py", PY_LOCK);
    }
    committed[SHARED_BUN] = "bun-lock-contents";
    const tree = treeOf(committed, { [SHARED_BUN]: "bun-lock-contents" });

    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);

    const plan = computeSharedLockPlan(map, { defaultTs: "bun", existing: tree });
    expect(plan.writes[SHARED_PY]).toBeUndefined();
  });
});

describe("scriptsReferencingSharedLock", () => {
  test("finds every script on the shared lock and nothing else", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    ownLock(map, "f/odd", ".py", OTHER_LOCK);
    applySharedLockPlanToMap(map, computeSharedLockPlan(map, { defaultTs: "bun", existing: treeOf(map) }));

    expect(scriptsReferencingSharedLock(map, SHARED_PY).sort()).toEqual([
      "f/a.script.yaml",
      "f/b.script.yaml",
    ]);
  });

  test("reports none for a shared file nothing reads", () => {
    const map: Record<string, string> = { [SHARED_PY]: PY_LOCK };
    expect(scriptsReferencingSharedLock(map, SHARED_PY)).toEqual([]);
  });
});

describe("collectExistingSharedLocks", () => {
  test("counts every reader, and only the ones in the workspace", async () => {
    const root = await mkdtemp(path.join(os.tmpdir(), "wmill-dedup-"));
    try {
      const write = async (rel: string, content: string) => {
        const full = path.join(root, ...rel.split("/"));
        await mkdir(path.dirname(full), { recursive: true });
        await writeFile(full, content, "utf-8");
      };
      const readsShared = meta(`!inline ${SHARED_PY}`);
      await write(SHARED_PY, PY_LOCK);
      await write("f/team/a.script.yaml", readsShared);
      // A folder named after a build directory is still a Windmill folder: a
      // script hidden from this scan is the one the scan exists to protect.
      await write("f/team/dist/b.script.yaml", readsShared);
      // The stateful-push mirror holds copies, not scripts — counting them as
      // readers would freeze the shared file's content.
      await write(".wmill/f/team/a.script.yaml", readsShared);
      await write("f/team/escape.script.yaml", meta(`!inline ${TRAVERSING_REF}`));
      await write(
        "f/team/prose.script.yaml",
        yamlStringify(
          { summary: `see !inline ${SHARED_PY}`, lock: "!inline f/team/prose.script.lock" },
          yamlOptions,
        ),
      );
      await write("docs/example.script.yaml", readsShared);

      const existing = await collectExistingSharedLocks(root);

      expect([...existing.refs.keys()].sort()).toEqual([
        "f/team/a.script.yaml",
        "f/team/dist/b.script.yaml",
      ]);
      expect(existing.contents.get(SHARED_PY)).toEqual(PY_LOCK);
      // A reference is repo content and becomes a path this pass writes to, so
      // one that escapes the workspace is not a reference at all.
      expect([...existing.refs.values()]).not.toContain(TRAVERSING_REF);
      expect(existing.refs.get("f/team/escape.script.yaml")).toBeUndefined();
      // `!inline` in a summary is prose, not a reference: only the `lock` field
      // decides which lockfile a script reads.
      expect(existing.refs.get("f/team/prose.script.yaml")).toBeUndefined();
      // And a file merely named like script metadata, outside the namespaces
      // sync reads, is not a script — counted, it would leave the planner in
      // conserve-only mode for good.
      expect(existing.scripts.has("docs/example.script.yaml")).toBe(false);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
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
      // A regenerated script must fall back to its own lock rather than point
      // at a shared file that is not there.
      expect(
        sharedLockRefIn(meta("!inline locks/go.lock"), false, root),
      ).toBeUndefined();
      expect(
        sharedLockRefIn(meta(`!inline ${TRAVERSING_REF}`), false, root),
      ).toBeUndefined();
      expect(
        sharedLockRefIn(meta("!inline f/a.script.lock"), false, root),
      ).toBeUndefined();
      // Flow-style YAML starts with `{` and is not JSON: deciding the format by
      // the first character would drop the reference and repoint the script.
      expect(
        sharedLockRefIn(`{summary: x, lock: '!inline ${SHARED_PY}'}`, false, root),
      ).toEqual(SHARED_PY);
      // The `lock` field alone decides, whatever prose surrounds it.
      expect(
        sharedLockRefIn(
          yamlStringify(
            { summary: `see !inline ${SHARED_PY}`, lock: "!inline f/a.script.lock" },
            yamlOptions,
          ),
          false,
          root,
        ),
      ).toBeUndefined();
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
