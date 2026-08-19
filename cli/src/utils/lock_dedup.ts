import { stringify as yamlStringify } from "yaml";
import { mkdir, rm, writeFile } from "node:fs/promises";
import * as path from "node:path";
import { yamlOptions } from "../commands/sync/sync.ts";
import { yamlParseContent } from "./yaml.ts";
import {
  inferContentTypeFromFilePath,
  languageNeedsLock,
} from "./script_common.ts";
import { isSharedLockPath, sharedLockPath } from "./resource_folders.ts";

/**
 * Lockfile deduplication (`dedupeLockfiles` in wmill.yaml).
 *
 * A workspace whose dependencies come from `dependencies/<file>` resolves to the
 * very same lock for every script of that language, so the repo ends up holding
 * thousands of byte-identical `.script.lock` files: one dependency bump rewrites
 * all of them, and every open branch conflicts on all of them.
 *
 * With dedup on, the scripts of a language that share a lock reference ONE file,
 * `dependencies/locks/<language>.lock`, through the `!inline` indirection their
 * metadata already uses — so the bump is a one-file diff. Scripts whose lock
 * differs (extra imports, a named dependency set) keep their own `.script.lock`.
 *
 * The shared file is named after the LANGUAGE, never after the content it holds:
 * a content-addressed name would move on every bump and put the churn back in
 * the metadata of every script.
 */

const INLINE_PREFIX = "!inline ";

/** Below this, a shared file would trade one lock file for two. */
const MIN_SHARED_GROUP = 2;

/** Sync maps are keyed with the platform separator (`path.join`), while an
 *  `!inline` reference is always forward-slash. */
const toMapKey = (refPath: string) => refPath.replaceAll("/", path.sep);
const toRefPath = (mapKey: string) => mapKey.replaceAll("\\", "/");

/** What the sync layer needs to know to dedup: whether to, and how to read a
 *  `.ts` script's language. Both ride on the same `wmill.yaml` options object. */
export type LockDedupOptions = {
  dedupeLockfiles?: boolean | undefined;
  defaultTs?: "bun" | "deno" | undefined;
};

/** The files a dedup pass writes and removes, as paths relative to the sync
 *  root — applicable to an in-memory sync map or to the working tree. */
export type SharedLockPlan = {
  writes: Record<string, string>;
  deletes: string[];
};

export function isEmptySharedLockPlan(plan: SharedLockPlan): boolean {
  return Object.keys(plan.writes).length === 0 && plan.deletes.length === 0;
}

type ScriptEntry = {
  metaKey: string;
  isJson: boolean;
  parsed: Record<string, any>;
  /** The lock file the metadata references today, as a map key. */
  lockKey: string;
  /** The lock file this script owns when it is not sharing one. */
  ownLockKey: string;
  language: string;
  lock: string;
};

/** `f/foo.script.yaml` -> base `f/foo`, lock `f/foo.script.lock`;
 *  `f/foo__mod/script.yaml` -> base `f/foo__mod/script`, lock `…/script.lock`.
 *  The base is what the script's content file is named after. Both are returned
 *  forward-slashed, whatever the map's separator. */
function scriptMetaBase(
  key: string,
): { base: string; ownLockKey: string; isJson: boolean } | undefined {
  const ref = toRefPath(key);
  for (const [suffix, isJson] of [
    [".script.yaml", false],
    [".script.json", true],
    ["/script.yaml", false],
    ["/script.json", true],
  ] as const) {
    if (!ref.endsWith(suffix)) continue;
    const stripped = ref.slice(0, ref.length - suffix.length);
    if (suffix.startsWith("/")) {
      // `/script.yaml` is only script metadata inside a module folder; anywhere
      // else it is an ordinary file that happens to be called `script.yaml`.
      if (!stripped.endsWith("__mod")) return undefined;
      return {
        base: stripped + "/script",
        ownLockKey: stripped + "/script.lock",
        isJson,
      };
    }
    return { base: stripped, ownLockKey: stripped + ".script.lock", isJson };
  }
  return undefined;
}

/** Everything sharing a metadata file's base name, i.e. its content file and
 *  its lock — indexed once, because resolving it per script by scanning the map
 *  is quadratic in the number of files. */
function indexByBase(map: Record<string, string>): Map<string, string[]> {
  const index = new Map<string, string[]>();
  for (const key of Object.keys(map)) {
    const ref = toRefPath(key);
    const slash = ref.lastIndexOf("/");
    const dot = ref.indexOf(".", slash + 1);
    if (dot === -1) continue;
    const base = ref.slice(0, dot);
    const bucket = index.get(base);
    if (bucket) {
      bucket.push(ref);
    } else {
      index.set(base, [ref]);
    }
  }
  return index;
}

function languageOfScript(
  base: string,
  byBase: Map<string, string[]>,
  defaultTs: "bun" | "deno" | undefined,
): string | undefined {
  // A script's content file is never `.yaml`/`.json`/`.lock` — those are its
  // metadata and its lock (`.playbook.yml` is `.yml`, and a dbt descriptor
  // belongs to a base of its own, so dbt is left out of dedup entirely).
  const candidates = (byBase.get(base) ?? []).filter(
    (c) => !c.endsWith(".yaml") && !c.endsWith(".json") && !c.endsWith(".lock"),
  );
  // Sorted so a base that somehow carries two content files resolves the same
  // way on every run, rather than following map insertion order.
  for (const candidate of candidates.sort()) {
    try {
      return inferContentTypeFromFilePath(candidate, defaultTs);
    } catch {
      // not a script content file (a resource, a schema, …)
    }
  }
  return undefined;
}

/** A map entry addressed by a forward-slashed path, whatever separator the map
 *  was keyed with. */
function lookup(
  map: Record<string, string>,
  refPath: string,
): { key: string; content: string } | undefined {
  for (const key of [toMapKey(refPath), refPath]) {
    const content = map[key];
    if (content !== undefined) return { key, content };
  }
  return undefined;
}

function collectScripts(
  map: Record<string, string>,
  defaultTs: "bun" | "deno" | undefined,
): ScriptEntry[] {
  const byBase = indexByBase(map);
  const entries: ScriptEntry[] = [];
  for (const [metaKey, metaContent] of Object.entries(map)) {
    const meta = scriptMetaBase(metaKey);
    if (!meta) continue;

    let parsed: any;
    try {
      parsed = meta.isJson
        ? JSON.parse(metaContent)
        : yamlParseContent(metaKey, metaContent);
    } catch {
      continue;
    }
    if (typeof parsed !== "object" || parsed === null) continue;

    const lockRef = parsed["lock"];
    if (typeof lockRef !== "string" || !lockRef.startsWith(INLINE_PREFIX)) {
      continue;
    }
    const lockFile = lookup(map, lockRef.slice(INLINE_PREFIX.length));
    // An absent or empty lock is not a lock to share: a script with no
    // dependencies carries `lock: ''` and no file at all.
    if (lockFile === undefined || lockFile.content === "") continue;

    const language = languageOfScript(meta.base, byBase, defaultTs);
    if (language === undefined || !languageNeedsLock(language)) continue;

    entries.push({
      metaKey,
      isJson: meta.isJson,
      parsed,
      lockKey: lockFile.key,
      ownLockKey: toMapKey(meta.ownLockKey),
      language,
      lock: lockFile.content,
    });
  }
  return entries;
}

function serializeMetadata(entry: ScriptEntry): string {
  return entry.isJson
    ? JSON.stringify(entry.parsed, null, 2)
    : yamlStringify(entry.parsed, yamlOptions);
}

/**
 * What a sync map (path -> content) has to change for every duplicated script
 * lock to become one shared file per language. Pure: the map is not touched.
 */
export function computeSharedLockPlan(
  map: Record<string, string>,
  defaultTs: "bun" | "deno" | undefined,
): SharedLockPlan {
  const plan: SharedLockPlan = { writes: {}, deletes: [] };
  const byLanguage = new Map<string, Map<string, ScriptEntry[]>>();
  for (const entry of collectScripts(map, defaultTs)) {
    let byLock = byLanguage.get(entry.language);
    if (!byLock) {
      byLock = new Map();
      byLanguage.set(entry.language, byLock);
    }
    const group = byLock.get(entry.lock);
    if (group) {
      group.push(entry);
    } else {
      byLock.set(entry.lock, [entry]);
    }
  }

  for (const [language, byLock] of byLanguage) {
    // The most widely shared lock wins the language's shared file, so the odd
    // script out is the one that keeps a lock of its own. Ties break on the
    // content itself: any ordering derived from the map would make the winner
    // depend on how the workspace was walked.
    let winner: string | undefined;
    let winnerCount = 0;
    for (const [lock, group] of byLock) {
      if (
        group.length > winnerCount ||
        (group.length === winnerCount && winner !== undefined && lock < winner)
      ) {
        winner = lock;
        winnerCount = group.length;
      }
    }
    if (winnerCount < MIN_SHARED_GROUP) winner = undefined;

    const sharedKey = toMapKey(sharedLockPath(language));
    if (winner !== undefined && map[sharedKey] !== winner) {
      plan.writes[sharedKey] = winner;
    }

    for (const [lock, group] of byLock) {
      const shared = winner !== undefined && lock === winner;
      for (const entry of group) {
        const target = shared ? sharedKey : entry.ownLockKey;
        if (entry.lockKey === target) continue;
        if (!shared && map[target] !== entry.lock) {
          plan.writes[target] = entry.lock;
        }
        // Never drop the shared file itself: the scripts still on it need it.
        if (!isSharedLockPath(entry.lockKey)) {
          plan.deletes.push(entry.lockKey);
        }
        entry.parsed["lock"] = INLINE_PREFIX + toRefPath(target);
        plan.writes[entry.metaKey] = serializeMetadata(entry);
      }
    }

    // A shared file nothing references any more (the language lost its scripts,
    // or they all diverged) is left behind by the loop above.
    if (
      winner === undefined &&
      map[sharedKey] !== undefined &&
      !plan.deletes.includes(sharedKey)
    ) {
      plan.deletes.push(sharedKey);
    }
  }

  return plan;
}

export function applySharedLockPlanToMap(
  map: Record<string, string>,
  plan: SharedLockPlan,
): void {
  for (const [key, content] of Object.entries(plan.writes)) {
    map[key] = content;
  }
  for (const key of plan.deletes) {
    delete map[key];
  }
}

export async function applySharedLockPlanToDisk(
  plan: SharedLockPlan,
): Promise<void> {
  for (const [key, content] of Object.entries(plan.writes)) {
    await mkdir(path.dirname(key), { recursive: true });
    await writeFile(key, content, "utf-8");
  }
  for (const key of plan.deletes) {
    await rm(key, { force: true });
  }
}

/**
 * The script metadata files that read a given shared lockfile. A change to that
 * file is a change to their lock, and they are what carries it to the remote.
 */
export function scriptsReferencingSharedLock(
  map: Record<string, string>,
  sharedKey: string,
): string[] {
  const reference = INLINE_PREFIX + toRefPath(sharedKey);
  const referrers: string[] = [];
  for (const [metaKey, metaContent] of Object.entries(map)) {
    const meta = scriptMetaBase(metaKey);
    if (!meta) continue;
    // A string test before parsing: the metadata of every script in the
    // workspace would otherwise be parsed to answer this.
    if (!metaContent.includes(reference)) continue;
    let parsed: any;
    try {
      parsed = meta.isJson
        ? JSON.parse(metaContent)
        : yamlParseContent(metaKey, metaContent);
    } catch {
      continue;
    }
    if (parsed?.["lock"] === reference) referrers.push(metaKey);
  }
  return referrers;
}
