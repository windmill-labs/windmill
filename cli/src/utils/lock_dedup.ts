import { stringify as yamlStringify } from "yaml";
import { mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import * as path from "node:path";
import { yamlOptions } from "../commands/sync/sync.ts";
import * as log from "../core/log.ts";
import { yamlParseContent } from "./yaml.ts";
import {
  inferContentTypeFromFilePath,
  languageNeedsLock,
} from "./script_common.ts";
import {
  SHARED_LOCK_DIR,
  isSharedLockPath,
  sharedLockPath,
} from "./resource_folders.ts";

/**
 * Lockfile deduplication (`dedupeLockfiles` in wmill.yaml).
 *
 * A workspace whose dependencies come from `dependencies/<file>` resolves to the
 * very same lock for every script of that language, so the repo ends up holding
 * thousands of byte-identical `.script.lock` files: one dependency bump rewrites
 * all of them, and every open branch conflicts on all of them.
 *
 * With dedup on, scripts that share a lock reference ONE file under `locks/`,
 * through the `!inline` indirection their metadata already uses — so the bump is
 * a one-file diff.
 *
 * Three rules make that hold, and each is load-bearing:
 *
 * - **A shared file's name never encodes its content.** A content-addressed name
 *   would move on every bump and put the churn straight back into the metadata
 *   of every script. A group of scripts KEEPS the file it already references
 *   (its incumbent) and the file's CONTENT is what changes. Only a group with no
 *   incumbent is given a name: `<language>.lock`, then `<language>-2.lock`, …
 * - **A shared file's content only moves when every script that reads it agrees.**
 *   Referrers are counted from the whole working tree, never from the sync map,
 *   which wmill.yaml's includes/excludes narrow — and which the git-sync deploy
 *   callback narrows to a single item. A script the current sync does not cover
 *   still reads the file, and rewriting or deleting it underneath would change
 *   that script's lock silently.
 * - **A sync that cannot see the whole tree only conserves.** It keeps existing
 *   groups together but never forms new ones, so a per-item deploy does not
 *   invent shared files that a later full sync has to undo.
 */

const INLINE_PREFIX = "!inline ";

/** Content-file extensions of the languages that carry a lock, longest first.
 *  A language absent here is simply never deduplicated.
 *  for related places search: ADD_NEW_LANG */
const LOCKABLE_EXTS = [
  ".fetch.ts",
  ".deno.ts",
  ".bun.ts",
  ".playbook.yml",
  ".ts",
  ".py",
  ".go",
  ".php",
  ".rs",
];

/** Below this a shared file would trade one lock file for two. It gates only
 *  NEW shared files: a script already on one stays there however few are left,
 *  or a narrowed sync scope would tear the group apart. */
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

/** The working tree as it stands, collected unfiltered (see the header).
 *  Paths are forward-slashed. */
export type ExistingSharedLocks = {
  /** script metadata file -> the shared lockfile it references */
  refs: Map<string, string>;
  /** shared lockfile -> its content */
  contents: Map<string, string>;
  /** every script metadata file in the tree, referencing a shared lock or not */
  scripts: Set<string>;
};

export const NO_EXISTING_SHARED_LOCKS: ExistingSharedLocks = {
  refs: new Map(),
  contents: new Map(),
  scripts: new Set(),
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
  /** `metaKey` forward-slashed, i.e. how the working-tree scan names it. */
  metaRef: string;
  isJson: boolean;
  compactJson: boolean;
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
 *  The base is what the script's content file is named after. All returned
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

/** Map keys grouped by directory. A script's content file is looked up among
 *  its own directory's entries: splitting a name at its first dot would put
 *  `f/a.b.py` under `f/a` and leave every dotted script path undeduplicated. */
function indexByDirectory(map: Record<string, string>): Map<string, string[]> {
  const index = new Map<string, string[]>();
  for (const key of Object.keys(map)) {
    const ref = toRefPath(key);
    const dir = ref.slice(0, ref.lastIndexOf("/") + 1);
    const bucket = index.get(dir);
    if (bucket) {
      bucket.push(ref);
    } else {
      index.set(dir, [ref]);
    }
  }
  return index;
}

function languageOfScript(
  base: string,
  byDirectory: Map<string, string[]>,
  defaultTs: "bun" | "deno" | undefined,
): string | undefined {
  const dir = base.slice(0, base.lastIndexOf("/") + 1);
  const siblings = new Set(byDirectory.get(dir) ?? []);
  // `<base><ext>` and nothing looser: `f/a.b.py` is the content file of the
  // script `f/a.b`, not of `f/a`, and a prefix match would read one script's
  // language off another's file. Longest extension first, so `.bun.ts` is not
  // taken for `.ts` (dbt is absent from the list, and stays out of dedup).
  for (const ext of LOCKABLE_EXTS) {
    if (siblings.has(base + ext)) {
      try {
        return inferContentTypeFromFilePath(base + ext, defaultTs);
      } catch {
        // not a language this CLI knows
      }
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
  const byDirectory = indexByDirectory(map);
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

    const language = languageOfScript(meta.base, byDirectory, defaultTs);
    if (language === undefined || !languageNeedsLock(language)) continue;

    entries.push({
      metaKey,
      metaRef: toRefPath(metaKey),
      isJson: meta.isJson,
      compactJson: !metaContent.includes("\n"),
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
  if (!entry.isJson) return yamlStringify(entry.parsed, yamlOptions);
  // Indented or compact as it was found: `sync` writes JSON metadata indented
  // and `generate-metadata` writes it compact, so imposing either one here
  // reformats files this feature exists to keep quiet.
  return entry.compactJson
    ? JSON.stringify(entry.parsed)
    : JSON.stringify(entry.parsed, null, 2);
}

/**
 * Scripts that read a shared lockfile and that this plan does not speak for,
 * i.e. the ones outside the sync map. Referrers inside it are being reassigned
 * by this same plan, so they cannot be surprised by a content change — counting
 * them would hand a lagging minority a veto over the majority's own file.
 */
function unrepresentedReaders(
  existing: ExistingSharedLocks,
  map: Record<string, string>,
  sharedRef: string,
): number {
  let count = 0;
  for (const [metaRef, ref] of existing.refs) {
    if (ref !== sharedRef) continue;
    if (map[toMapKey(metaRef)] === undefined && map[metaRef] === undefined) {
      count++;
    }
  }
  return count;
}

/**
 * The shared file a group of scripts is entitled to keep: the one most of them
 * already reference. Undefined when the group has no incumbent, or when keeping
 * it would move its content underneath scripts outside the group.
 */
function incumbentFor(
  group: ScriptEntry[],
  content: string,
  existing: ExistingSharedLocks,
  map: Record<string, string>,
  claimed: Set<string>,
): string | undefined {
  const votes = new Map<string, number>();
  for (const entry of group) {
    const ref = existing.refs.get(entry.metaRef);
    if (ref === undefined || claimed.has(ref)) continue;
    votes.set(ref, (votes.get(ref) ?? 0) + 1);
  }
  let best: string | undefined;
  let bestVotes = 0;
  for (const [ref, count] of votes) {
    if (
      count > bestVotes ||
      (count === bestVotes && best !== undefined && ref < best)
    ) {
      best = ref;
      bestVotes = count;
    }
  }
  if (best === undefined) return undefined;
  // Keeping the name costs nothing while the content is unchanged. Moving the
  // content takes a real group speaking for every reader — otherwise one script
  // that drifted would rename the file for all the others, or keep a shared
  // file to itself.
  if (
    existing.contents.get(best) !== content &&
    (group.length < MIN_SHARED_GROUP ||
      unrepresentedReaders(existing, map, best) > 0)
  ) {
    return undefined;
  }
  return best;
}

/** The first `<language>[-N].lock` no other group in this run holds and no
 *  script outside the group still reads. */
function allocateName(
  language: string,
  group: ScriptEntry[],
  existing: ExistingSharedLocks,
  claimed: Set<string>,
): string {
  const inGroup = new Set(group.map((e) => e.metaRef));
  for (let n = 1; ; n++) {
    const candidate =
      n === 1
        ? sharedLockPath(language)
        : `${SHARED_LOCK_DIR}/${language}-${n}.lock`;
    if (claimed.has(candidate)) continue;
    let readByOthers = false;
    for (const [metaRef, ref] of existing.refs) {
      if (ref === candidate && !inGroup.has(metaRef)) {
        readByOthers = true;
        break;
      }
    }
    if (!readByOthers) return candidate;
  }
}

/**
 * What a sync map (path -> content) has to change for duplicated script locks to
 * become one shared file per group. Pure: neither argument is touched.
 */
export function computeSharedLockPlan(
  map: Record<string, string>,
  defaultTs: "bun" | "deno" | undefined,
  existing: ExistingSharedLocks = NO_EXISTING_SHARED_LOCKS,
  json = false,
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

  // A sync that cannot see the whole tree conserves only (see the header). Only
  // the metadata twin this sync reads counts as missing: a leftover
  // `.script.json` in a YAML repo is dropped by the map for reasons that have
  // nothing to do with scope, and would otherwise put the tree in conserve-only
  // mode for good.
  const unseen = [...existing.scripts].filter(
    (script) =>
      script.endsWith(json ? ".json" : ".yaml") &&
      map[toMapKey(script)] === undefined &&
      map[script] === undefined,
  );
  const partialView = unseen.length > 0;
  if (partialView) {
    log.debug(
      `Lockfile dedup: ${unseen.length} script(s) outside this sync's scope (e.g. ${unseen[0]}); keeping existing groups but forming none.`,
    );
  }

  // Where every script ends up, so a shared file with nothing left reading it
  // can be dropped. Seeded with the whole tree, out-of-scope scripts included.
  const finalRefs = new Map(existing.refs);
  const claimed = new Set<string>();

  for (const [language, byLock] of [...byLanguage].sort(([a], [b]) =>
    a.localeCompare(b),
  )) {
    // Biggest group first: it has the strongest claim to a name, and ties break
    // on the content itself so the outcome never depends on map ordering.
    const groups = [...byLock].sort(
      ([contentA, a], [contentB, b]) =>
        b.length - a.length || contentA.localeCompare(contentB),
    );

    for (const [content, group] of groups) {
      let target = incumbentFor(group, content, existing, map, claimed);
      if (
        target === undefined &&
        !partialView &&
        group.length >= MIN_SHARED_GROUP
      ) {
        target = allocateName(language, group, existing, claimed);
      }
      if (target !== undefined) {
        claimed.add(target);
        const sharedKey = toMapKey(target);
        if (map[sharedKey] !== content) plan.writes[sharedKey] = content;
      }

      for (const entry of group) {
        const targetKey =
          target !== undefined ? toMapKey(target) : entry.ownLockKey;
        finalRefs.set(entry.metaRef, toRefPath(targetKey));
        if (entry.lockKey === targetKey) continue;
        if (target === undefined && map[targetKey] !== entry.lock) {
          plan.writes[targetKey] = entry.lock;
        }
        // A shared file is only ever dropped by the sweep below, which knows
        // who else reads it.
        if (!isSharedLockPath(entry.lockKey)) plan.deletes.push(entry.lockKey);
        entry.parsed["lock"] = INLINE_PREFIX + toRefPath(targetKey);
        plan.writes[entry.metaKey] = serializeMetadata(entry);
      }
    }
  }

  const stillRead = new Set(finalRefs.values());
  for (const [sharedRef, content] of existing.contents) {
    const key = toMapKey(sharedRef);
    if (!stillRead.has(sharedRef)) {
      // Nothing reads it any more: its scripts diverged, or the sync deleted them.
      if (plan.writes[key] === undefined) plan.deletes.push(key);
    } else if (plan.writes[key] === undefined && map[key] === undefined) {
      // Read by a script this sync does not cover, and absent from the map (the
      // remote never serializes shared files). Carried over, or the diff would
      // delete the file those scripts read.
      plan.writes[key] = content;
    }
  }

  // Deletes are applied after writes, so a path some script still writes must
  // not also be dropped — two metadata files pointing at one lock file would
  // otherwise cancel each other out and leave the survivor without a lock.
  plan.deletes = plan.deletes.filter((key) => plan.writes[key] === undefined);

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

const SHARED_LOCK_REF_RE = new RegExp(
  `${INLINE_PREFIX}(${SHARED_LOCK_DIR}/[^\\s'"]+\\.lock)`,
);

/**
 * The shared lockfile a metadata FILE reads, when it reads one that is there.
 * `parseMetadataFile` resolves `lock` to the lockfile's content, so the
 * reference itself survives only in the raw text.
 */
export function sharedLockRefIn(
  metadataContent: string,
  root: string = ".",
): string | undefined {
  const ref = SHARED_LOCK_REF_RE.exec(metadataContent)?.[1];
  return ref && isSharedLockPath(ref) && existsSync(path.resolve(root, ref))
    ? ref
    : undefined;
}

// `.wmill` is the stateful-push mirror: its copies of a script's metadata would
// be counted as extra readers of a shared lockfile and freeze its content.
// `.git` and `node_modules` cannot be Windmill paths and are where the walk
// would otherwise spend its time. Nothing else is skipped — a build-output name
// like `dist` is a legal folder, and a script hidden from this scan is exactly
// the script the scan exists to protect.
const SCAN_SKIP_DIRS = new Set([".git", ".wmill", "node_modules"]);

/** Workspace-shared raw-app components, which sync handles on its own. */
const SCAN_SKIP_ROOTS = ["ui/"];

/**
 * The shared lockfiles in the working tree, the scripts that read them, and
 * every script metadata file there is — deliberately NOT filtered by the sync's
 * includes/excludes, which is what lets a plan tell what its map leaves out.
 */
export async function collectExistingSharedLocks(
  root: string,
): Promise<ExistingSharedLocks> {
  const existing: ExistingSharedLocks = {
    refs: new Map(),
    contents: new Map(),
    scripts: new Set(),
  };
  // With no shared lockfiles there is no name to keep stable and nothing to
  // protect — and no reason to read every metadata file in the workspace.
  if (!existsSync(path.join(root, ...SHARED_LOCK_DIR.split("/")))) {
    return existing;
  }

  const walk = async (dir: string): Promise<void> => {
    let entries;
    try {
      entries = await readdir(path.join(root, dir), { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const rel = dir === "" ? entry.name : `${dir}/${entry.name}`;
      if (entry.isDirectory()) {
        if (!SCAN_SKIP_DIRS.has(entry.name)) await walk(rel);
        continue;
      }
      const full = path.join(root, ...rel.split("/"));
      if (isSharedLockPath(rel)) {
        existing.contents.set(rel, await readFile(full, "utf-8"));
        continue;
      }
      if (scriptMetaBase(rel) === undefined) continue;
      if (SCAN_SKIP_ROOTS.some((root) => rel.startsWith(root))) continue;
      existing.scripts.add(rel);
      // A substring match on the raw text rather than a parse: this reads every
      // script metadata file in the workspace, and the reference is a literal.
      const match = SHARED_LOCK_REF_RE.exec(await readFile(full, "utf-8"));
      // Validated, not just matched: a reference is repo content, it becomes a
      // path this pass writes to, and `locks/../../../x.lock`
      // satisfies the pattern while resolving outside the workspace.
      if (match && isSharedLockPath(match[1])) existing.refs.set(rel, match[1]);
    }
  };
  await walk("");
  return existing;
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
