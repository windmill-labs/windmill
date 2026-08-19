/**
 * Lockfile deduplication (`dedupeLockfiles`) — WIN-1756.
 *
 * A workspace with one `dependencies/requirements.in` resolves the same lock for
 * every Python script, so the repo carries thousands of identical
 * `.script.lock` files: a dependency bump rewrites all of them and every open
 * branch conflicts on all of them. Dedup keeps ONE
 * `dependencies/locks/<language>.lock` and points the metadata at it.
 *
 * What these pin:
 *  - the shared file is named after the language, so a bump is a one-file diff
 *  - a script whose lock genuinely differs keeps its own, and can get it back
 *  - the pass is idempotent: a deduped tree produces no further changes, which
 *    is what keeps `sync pull`/`push` from seeing a diff on every run
 */

import { expect, test, describe } from "bun:test";
import { stringify as yamlStringify } from "yaml";
import {
  applySharedLockPlanToMap,
  computeSharedLockPlan,
  isEmptySharedLockPlan,
  scriptsReferencingSharedLock,
} from "../src/utils/lock_dedup.ts";
import { yamlOptions } from "../src/commands/sync/sync.ts";

const PY_LOCK = "requests==2.32.0\nurllib3==2.2.1\n";
const PY_LOCK_BUMPED = "requests==2.32.3\nurllib3==2.2.1\n";
const OTHER_LOCK = "requests==2.32.0\nurllib3==2.2.1\npandas==2.2.0\n";
const SHARED_PY = "dependencies/locks/python3.lock";
const SHARED_BUN = "dependencies/locks/bun.lock";

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

/** A script already pointing at a shared lock. */
function sharedLock(
  map: Record<string, string>,
  base: string,
  ext: string,
  shared: string,
) {
  map[`${base}${ext}`] = "def main(): ...";
  map[`${base}.script.yaml`] = meta(`!inline ${shared}`);
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

    const plan = computeSharedLockPlan(map, "bun");
    applySharedLockPlanToMap(map, plan);

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map[SHARED_BUN]).toEqual("bun-lock-contents");
    for (const base of ["f/a", "f/b", "f/c"]) {
      expect(map[`${base}.script.lock`]).toBeUndefined();
      expect(lockRefOf(map[`${base}.script.yaml`])).toEqual(
        `!inline ${SHARED_PY}`,
      );
    }
    expect(map["f/ts.script.lock"]).toBeUndefined();
    expect(lockRefOf(map["f/ts.script.yaml"])).toEqual(`!inline ${SHARED_BUN}`);
  });

  test("is idempotent — a deduped tree yields no further changes", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    applySharedLockPlanToMap(map, computeSharedLockPlan(map, "bun"));

    expect(isEmptySharedLockPlan(computeSharedLockPlan(map, "bun"))).toBe(true);
  });

  test("a dependency bump moves only the shared file", () => {
    const map: Record<string, string> = {};
    sharedLock(map, "f/a", ".py", SHARED_PY);
    sharedLock(map, "f/b", ".py", SHARED_PY);
    map[SHARED_PY] = PY_LOCK;

    // What the remote sends after the bump: one lock per script, all bumped.
    const remote: Record<string, string> = {};
    ownLock(remote, "f/a", ".py", PY_LOCK_BUMPED);
    ownLock(remote, "f/b", ".py", PY_LOCK_BUMPED);
    applySharedLockPlanToMap(remote, computeSharedLockPlan(remote, "bun"));

    const changed = Object.keys(remote).filter((k) => remote[k] !== map[k]);
    expect(changed).toEqual([SHARED_PY]);
    expect(remote[SHARED_PY]).toEqual(PY_LOCK_BUMPED);
  });

  test("the minority keeps its own lock; the majority shares", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    ownLock(map, "f/odd", ".py", OTHER_LOCK);

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, "bun"));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map["f/odd.script.lock"]).toEqual(OTHER_LOCK);
    expect(lockRefOf(map["f/odd.script.yaml"])).toEqual(
      "!inline f/odd.script.lock",
    );
  });

  test("a script that diverges from the shared lock gets its own back", () => {
    const map: Record<string, string> = {};
    sharedLock(map, "f/a", ".py", SHARED_PY);
    sharedLock(map, "f/b", ".py", SHARED_PY);
    sharedLock(map, "f/c", ".py", SHARED_PY);
    map[SHARED_PY] = PY_LOCK;
    // f/c grew an import: the remote now sends it a lock of its own.
    map["f/c.script.yaml"] = meta("!inline f/c.script.lock");
    map["f/c.script.lock"] = OTHER_LOCK;

    const plan = computeSharedLockPlan(map, "bun");
    applySharedLockPlanToMap(map, plan);

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map["f/c.script.lock"]).toEqual(OTHER_LOCK);
    expect(map["f/a.script.lock"]).toBeUndefined();
  });

  test("a lone script is left with its own lock", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);

    expect(isEmptySharedLockPlan(computeSharedLockPlan(map, "bun"))).toBe(true);
  });

  test("a shared file nothing references any more is removed", () => {
    const map: Record<string, string> = {};
    sharedLock(map, "f/a", ".py", SHARED_PY);
    map[SHARED_PY] = PY_LOCK;
    // The one remaining script diverged, so nothing shares that lock.
    map["f/a.script.yaml"] = meta("!inline f/a.script.lock");
    map["f/a.script.lock"] = OTHER_LOCK;

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, "bun"));

    expect(map[SHARED_PY]).toBeUndefined();
  });

  test("scripts without a lock file are untouched", () => {
    const map: Record<string, string> = {
      "f/a.py": "def main(): ...",
      "f/a.script.yaml": meta(""),
      "f/b.py": "def main(): ...",
      "f/b.script.yaml": meta(""),
    };

    expect(isEmptySharedLockPlan(computeSharedLockPlan(map, "bun"))).toBe(true);
  });

  test("a language that needs no lock is never deduped", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".sh", "some-lock");
    ownLock(map, "f/b", ".sh", "some-lock");

    expect(isEmptySharedLockPlan(computeSharedLockPlan(map, "bun"))).toBe(true);
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

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, "bun"));

    expect(map[SHARED_PY]).toEqual(PY_LOCK);
    expect(map["f/a__mod/script.lock"]).toBeUndefined();
    expect(lockRefOf(map["f/a__mod/script.yaml"])).toEqual(
      `!inline ${SHARED_PY}`,
    );
  });

  test("only the lock line of the metadata changes", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    map["f/a.script.yaml"] = meta("!inline f/a.script.lock", "does a thing");
    const before = map["f/a.script.yaml"];

    applySharedLockPlanToMap(map, computeSharedLockPlan(map, "bun"));

    expect(map["f/a.script.yaml"]).toEqual(
      before.replace("f/a.script.lock", SHARED_PY),
    );
  });
});

describe("scriptsReferencingSharedLock", () => {
  test("finds every script on the shared lock and nothing else", () => {
    const map: Record<string, string> = {};
    ownLock(map, "f/a", ".py", PY_LOCK);
    ownLock(map, "f/b", ".py", PY_LOCK);
    ownLock(map, "f/odd", ".py", OTHER_LOCK);
    applySharedLockPlanToMap(map, computeSharedLockPlan(map, "bun"));

    expect(scriptsReferencingSharedLock(map, SHARED_PY).sort()).toEqual([
      "f/a.script.yaml",
      "f/b.script.yaml",
    ]);
  });
});
