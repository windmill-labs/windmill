import { stringify as yamlStringify } from "yaml";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import * as path from "node:path";
import { yamlOptions } from "../commands/sync/sync.ts";
import { yamlParseContent } from "./yaml.ts";
import {
  depFileOfSharedLock,
  extractWorkspaceDepsAnnotation,
  hasLockAffectingAnnotation,
  inferContentTypeFromFilePath,
  isSharedLockPath,
  languageNeedsLock,
  sharedLockPathFor,
  workspaceDependenciesPathToLanguageAndFilename,
  type ScriptLanguage,
} from "./script_common.ts";

/**
 * Lockfile deduplication (`dedupeLockfiles` in wmill.yaml).
 *
 * A workspace whose dependencies come from `dependencies/<file>` resolves to the
 * very same lock for every script of that language, so the repo ends up holding
 * thousands of byte-identical `.script.lock` files: one dependency bump rewrites
 * all of them, and every open branch conflicts on all of them.
 *
 * With dedup on, the scripts that resolve against a workspace dependency file
 * reference ONE lockfile named after it — `dependencies/requirements.in` ->
 * `locks/requirements.in.lock` — through the `!inline` indirection their
 * metadata already uses. A dependency bump is a one-file diff.
 *
 * Identity comes from the dependency file, never from the content or from which
 * scripts happen to be in view. That is what makes the pass stateless: a sync
 * narrowed to a single script (the git-sync deploy callback) computes the same
 * NAME as a full one.
 *
 * The CONTENT follows, because a group only ever holds scripts whose lock IS
 * that file's lock: one carrying an annotation the worker acts on — a pinned
 * interpreter, `npm`, `nobundling` — never joins, so there is no variant inside
 * a group to tell apart from a bump. What is left is a script whose committed
 * lock is simply behind, and the many outvote it.
 *
 * Two cases are not shared at all and keep a `.script.lock` of their own: a
 * script whose annotation is `extra_` or carries inline dependencies (its lock
 * folds in its own imports), and one naming several dependency files at once
 * (its lock is no single file's).
 */

const INLINE_PREFIX = "!inline ";

/** Sync maps are keyed with the platform separator, while an `!inline`
 *  reference is always forward-slash. */
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
  compactJson: boolean;
  parsed: Record<string, any>;
  /** The lockfile the metadata references today, as a map key. */
  lockKey: string;
  /** The lockfile this script owns when it is not sharing one. */
  ownLockKey: string;
  lock: string;
  /** The workspace dependency file whose lock this is, when it is one. */
  depFile: string | undefined;
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

/** Map keys grouped by directory, so a script's content file is looked up among
 *  its own directory's entries: splitting a name at its first dot would put
 *  `f/a.b.py` under `f/a` and leave every dotted script path undeduplicated. */
function indexByDirectory(
  map: Record<string, string>,
): Map<string, Set<string>> {
  const index = new Map<string, Set<string>>();
  for (const key of Object.keys(map)) {
    const ref = toRefPath(key);
    const dir = ref.slice(0, ref.lastIndexOf("/") + 1);
    const bucket = index.get(dir);
    if (bucket) {
      bucket.add(ref);
    } else {
      index.set(dir, new Set([ref]));
    }
  }
  return index;
}

/** A script's content file and its language: `<base><ext>` and nothing looser,
 *  since `f/a.b.py` is the content file of `f/a.b`, not of `f/a`. */
function contentOfScript(
  map: Record<string, string>,
  base: string,
  byDirectory: Map<string, Set<string>>,
  defaultTs: "bun" | "deno" | undefined,
): { content: string; language: ScriptLanguage } | undefined {
  const siblings = byDirectory.get(base.slice(0, base.lastIndexOf("/") + 1));
  if (!siblings) return undefined;
  for (const ext of LOCKABLE_EXTS) {
    const candidate = base + ext;
    if (!siblings.has(candidate)) continue;
    const content = map[toMapKey(candidate)] ?? map[candidate];
    if (content === undefined) continue;
    try {
      return {
        content,
        language: inferContentTypeFromFilePath(candidate, defaultTs),
      };
    } catch {
      // not a language this CLI knows
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

/**
 * Both parsers, declared format first. YAML is a superset of JSON, and
 * flow-style YAML (`{summary: x, lock: '!inline …'}`) starts with `{` while
 * failing `JSON.parse` — deciding by the first character loses the reference.
 */
function parseMetadata(
  metaPath: string,
  metaContent: string,
  isJson: boolean,
): Record<string, any> | undefined {
  for (const asJson of isJson ? [true, false] : [false, true]) {
    try {
      const parsed = asJson
        ? JSON.parse(metaContent)
        : yamlParseContent(metaPath, metaContent);
      if (typeof parsed === "object" && parsed !== null) return parsed;
    } catch {
      // try the other one
    }
  }
  return undefined;
}

/**
 * The shared lockfile a metadata file's `lock` field names, if any. The raw text
 * is only a prefilter: a summary or a comment can carry the same words, and
 * `!inline` decides where a lock is written, so it is read from the parsed field
 * and nowhere else.
 */
export function sharedLockRefOf(
  metaPath: string,
  metaContent: string,
  isJson: boolean,
): string | undefined {
  // Without the trailing space: the YAML serializer folds a long `lock:` line
  // at a space, so `!inline locks/…` can reach disk as `!inline\n  locks/…`
  // and a prefilter looking for the space would call it a non-reader.
  if (!metaContent.includes(INLINE_PREFIX.trimEnd())) return undefined;
  const lock = parseMetadata(metaPath, metaContent, isJson)?.["lock"];
  if (typeof lock !== "string" || !lock.startsWith(INLINE_PREFIX)) {
    return undefined;
  }
  const ref = lock.slice(INLINE_PREFIX.length);
  return isSharedLockPath(ref) ? ref : undefined;
}

/**
 * Whether a metadata file may reference a shared lockfile but cannot say which.
 *
 * A `.script.yaml` carrying git conflict markers is a file whose `lock` cannot
 * be read, not one that reads nothing, and deleting a lockfile it may point at
 * is the unrecoverable half of that guess.
 */
export function metadataLockUnreadable(
  metaPath: string,
  metaContent: string,
  isJson: boolean,
): boolean {
  if (!metaContent.includes(INLINE_PREFIX.trimEnd())) return false;
  return parseMetadata(metaPath, metaContent, isJson) === undefined;
}

/**
 * The shared lockfile a metadata FILE reads, when it reads one that is there.
 * `parseMetadataFile` resolves `lock` to the lockfile's content, so the
 * reference itself survives only in the raw text.
 */
export function sharedLockRefIn(
  metadataContent: string,
  isJson: boolean,
  root: string = ".",
): string | undefined {
  const ref = sharedLockRefOf("metadata", metadataContent, isJson);
  return ref && existsSync(path.resolve(root, ref)) ? ref : undefined;
}

/** The key a dependency file answers to, i.e. what a script names it by. */
function depKeyOf(depFilePath: string): string | undefined {
  const info = workspaceDependenciesPathToLanguageAndFilename(depFilePath);
  return info && languageNeedsLock(info.language)
    ? `${info.language} ${info.name ?? "default"}`
    : undefined;
}

/** Workspace dependency files keyed by the language and name a script names. */
function depFilesByKey(paths: Iterable<string>): Map<string, string> {
  const byKey = new Map<string, string>();
  for (const key of paths) {
    const ref = toRefPath(key);
    if (!ref.startsWith("dependencies/")) continue;
    // A set named `team/python` exports as `dependencies/team/python.<file>`,
    // which has no distinct name under `locks/`: flattened it collides with the
    // top-level file, and the sweep would then retire a lockfile whose scripts
    // still read it. Such a set shares nothing and its scripts keep own locks.
    if (ref.slice("dependencies/".length).includes("/")) continue;
    const depKey = depKeyOf(ref);
    if (depKey) byKey.set(depKey, ref);
  }
  return byKey;
}

/**
 * The workspace dependency file whose lock a script's lock IS — undefined when
 * the script's lock is its own (see the header for the two cases).
 */
function shareableDepFile(
  scriptContent: string,
  language: ScriptLanguage,
  depFiles: Map<string, string>,
): string | undefined {
  // A script the worker locks differently for reasons of its own — a pinned
  // interpreter, `//npm`, `//nobundling` — cannot stand for its dependency
  // file's lock, so it never joins a group and its lock stays its own.
  if (hasLockAffectingAnnotation(scriptContent, language)) return undefined;
  const annotation = extractWorkspaceDepsAnnotation(scriptContent, language);
  if (annotation && (annotation.mode === "extra" || annotation.inline)) {
    return undefined;
  }
  const names = annotation ? annotation.external : ["default"];
  if (names.length !== 1) return undefined;
  return depFiles.get(`${language} ${names[0]}`);
}

/**
 * The shared lockfile a script belongs in, given the workspace dependency files
 * available — the one place that decides it, for both the sync planner and the
 * per-script regeneration in `updateScriptLock`.
 */
export function sharedLockTargetFor(
  scriptContent: string,
  language: ScriptLanguage,
  depPaths: Iterable<string>,
): string | undefined {
  const depFile = shareableDepFile(
    scriptContent,
    language,
    depFilesByKey(depPaths),
  );
  return depFile === undefined ? undefined : sharedLockPathFor(depFile);
}

function collectScripts(
  map: Record<string, string>,
  defaultTs: "bun" | "deno" | undefined,
  depFiles: Map<string, string>,
): ScriptEntry[] {
  const byDirectory = indexByDirectory(map);
  const entries: ScriptEntry[] = [];
  for (const [metaKey, metaContent] of Object.entries(map)) {
    const meta = scriptMetaBase(metaKey);
    if (!meta) continue;

    const parsed = parseMetadata(metaKey, metaContent, meta.isJson);
    if (parsed === undefined) continue;

    const lockRef = parsed["lock"];
    if (typeof lockRef !== "string" || !lockRef.startsWith(INLINE_PREFIX)) {
      continue;
    }
    const lockFile = lookup(map, lockRef.slice(INLINE_PREFIX.length));
    // An absent or empty lock is not a lock to share: a script with no
    // dependencies carries `lock: ''` and no file at all.
    if (lockFile === undefined || lockFile.content === "") continue;

    const script = contentOfScript(map, meta.base, byDirectory, defaultTs);
    if (script === undefined || !languageNeedsLock(script.language)) continue;

    entries.push({
      metaKey,
      isJson: meta.isJson,
      compactJson: !metaContent.includes("\n"),
      parsed,
      lockKey: lockFile.key,
      ownLockKey: toMapKey(meta.ownLockKey),
      lock: lockFile.content,
      depFile: shareableDepFile(script.content, script.language, depFiles),
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
 * What a sync map (path -> content) has to change for the scripts of a workspace
 * dependency file to share one lockfile. Pure: the map is not touched.
 */
export type SharedLockPlanContext = {
  defaultTs?: "bun" | "deno" | undefined;
  /**
   * Workspace dependency files to consider beyond the ones in `map`, for the
   * one caller whose map cannot hold them: `--skip-workspace-dependencies`.
   * Pass nothing otherwise — with dependency files in the map, an absence there
   * is a deletion, and adding disk's copy would keep a lockfile alive one sync
   * past the file it is named after.
   */
  depFiles?: Iterable<string>;
  /**
   * Shared lockfiles the working tree already holds. The remote never
   * serializes one, so without this a sync that has no script for a dependency
   * file reads its lockfile as deleted — and every script still pointing at it
   * is left with an `!inline` that resolves to nothing.
   */
  present?: Record<string, string>;
};

export function computeSharedLockPlan(
  map: Record<string, string>,
  ctx: SharedLockPlanContext = {},
): SharedLockPlan {
  const plan: SharedLockPlan = { writes: {}, deletes: [] };
  const depFiles = depFilesByKey([...Object.keys(map), ...(ctx.depFiles ?? [])]);
  // Every shared lockfile this sync can see, from either side.
  const present: Record<string, string> = { ...ctx.present };
  for (const [key, content] of Object.entries(map)) {
    if (isSharedLockPath(toRefPath(key))) present[toRefPath(key)] = content;
  }

  const byDepFile = new Map<string, ScriptEntry[]>();
  const ownLock: ScriptEntry[] = [];
  for (const entry of collectScripts(map, ctx.defaultTs, depFiles)) {
    if (entry.depFile === undefined) {
      ownLock.push(entry);
      continue;
    }
    // Push into the existing array rather than rebuild it: a workspace where
    // every script shares one dependency file is the case this exists for, and
    // copying the group per insert makes that quadratic.
    const group = byDepFile.get(entry.depFile);
    if (group) group.push(entry);
    else byDepFile.set(entry.depFile, [entry]);
  }

  const point = (entry: ScriptEntry, targetKey: string) => {
    if (entry.lockKey === targetKey) return;
    // A shared lockfile is dropped by the sweep below, which knows whether its
    // dependency file is still there; only a private one goes with its script.
    if (!isSharedLockPath(entry.lockKey)) plan.deletes.push(entry.lockKey);
    entry.parsed["lock"] = INLINE_PREFIX + toRefPath(targetKey);
    plan.writes[entry.metaKey] = serializeMetadata(entry);
  };

  const takeOwnLock = (entry: ScriptEntry) => {
    if (map[entry.ownLockKey] !== entry.lock) {
      plan.writes[entry.ownLockKey] = entry.lock;
    }
    point(entry, entry.ownLockKey);
  };

  for (const [depFile, group] of byDepFile) {
    // Every script here resolves against the same file and carries nothing the
    // worker locks separately, so their locks agree — unless one's committed
    // lock is simply behind. The many outvote the one; ties break on the content
    // itself so the outcome never depends on map ordering.
    const byContent = new Map<string, ScriptEntry[]>();
    for (const entry of group) {
      const sameLock = byContent.get(entry.lock);
      if (sameLock) sameLock.push(entry);
      else byContent.set(entry.lock, [entry]);
    }
    let content = "";
    let count = 0;
    for (const [lock, members] of byContent) {
      if (
        members.length > count ||
        (members.length === count && lock < content)
      ) {
        content = lock;
        count = members.length;
      }
    }

    const sharedKey = toMapKey(sharedLockPathFor(depFile));
    if (map[sharedKey] !== content) plan.writes[sharedKey] = content;
    for (const entry of group) {
      if (entry.lock === content) point(entry, sharedKey);
      else takeOwnLock(entry);
    }
  }

  // A script that stopped resolving against a dependency file takes its lock
  // back with it.
  for (const entry of ownLock) {
    if (entry.lockKey !== entry.ownLockKey) takeOwnLock(entry);
  }

  // A shared lockfile lives exactly as long as the dependency file it is named
  // after. Asking that, rather than "does any script still read it", is what
  // lets a sync narrowed to one item leave the rest of the workspace alone: a
  // lockfile with no script in view is carried forward, not deleted.
  for (const [sharedRef, content] of Object.entries(present)) {
    const depFile = depFileOfSharedLock(sharedRef);
    if (depFile === undefined) continue;
    const key = toMapKey(sharedRef);
    if (plan.writes[key] !== undefined) continue;
    if (depFiles.has(depKeyOf(depFile) ?? "")) {
      if (map[key] === undefined) plan.writes[key] = content;
    } else {
      plan.deletes.push(key);
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
    // Per write, so `locks/` comes into existence only when there is a shared
    // lockfile to put in it.
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
  const reference = toRefPath(sharedKey);
  const referrers: string[] = [];
  for (const [metaKey, metaContent] of Object.entries(map)) {
    const meta = scriptMetaBase(metaKey);
    if (!meta) continue;
    if (sharedLockRefOf(metaKey, metaContent, meta.isJson) === reference) {
      referrers.push(metaKey);
    }
  }
  return referrers;
}
