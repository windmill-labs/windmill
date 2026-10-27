import * as fs from "node:fs";
import * as path from "node:path";
import { createHash } from "node:crypto";
import * as log from "../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as wmill from "../../gen/services.gen.ts";
import { readTextFile, readTextFileSync } from "../utils/utils.ts";
import {
  extractFolderPath,
  extractResourceName,
  isRawAppFolderMetadataFile,
} from "../utils/resource_folders.ts";
import { APP_BACKEND_FOLDER } from "./app/app_metadata.ts";

const SHARED_UI_DIR = "ui";

async function readDirRecursive(
  dir: string,
  rel: string = "",
  out: Record<string, string> = {},
): Promise<Record<string, string>> {
  if (!fs.existsSync(dir)) return out;
  const entries = await fs.promises.readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    const r = rel ? rel + "/" + entry.name : entry.name;
    if (entry.isDirectory()) {
      await readDirRecursive(full, r, out);
    } else if (entry.isFile()) {
      out[r] = await readTextFile(full);
    }
  }
  return out;
}

/** Whether an app's own files import from the shared `/ui/` folder. */
export function importsSharedUi(files: Record<string, string>): boolean {
  return Object.values(files).some((c) => /["'`]\/ui\//.test(c));
}

/**
 * A digest of the local <cwd>/ui/ folder, or undefined without one. An app
 * bundles ui/ into itself, so this is what tells a push that the deployed
 * bundle was built against other ui/ content than the local one.
 */
export async function sharedUiHash(): Promise<string | undefined> {
  const localDir = path.join(process.cwd(), SHARED_UI_DIR);
  if (!fs.existsSync(localDir)) return undefined;
  const files = await readDirRecursive(localDir);
  const entries = Object.entries(files).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  return createHash("sha256").update(JSON.stringify(entries)).digest("hex");
}

/**
 * The `raw_app.yaml` keys of the local raw apps that import ui/ and whose
 * deployed bundle was built from other ui/ content: no file of theirs changed,
 * so the file diff never queues them. Apps in `skip` are already being pushed.
 */
export async function rawAppsStaleOnSharedUi(
  workspace: string,
  localMap: Record<string, string>,
  skip: Set<string>,
): Promise<string[]> {
  const hash = await sharedUiHash();
  if (!hash) return [];
  const apps = new Map<
    string,
    { metaPath?: string; files: Record<string, string> }
  >();
  for (const [key, content] of Object.entries(localMap)) {
    const name = extractResourceName(key, "raw_app");
    if (!name || skip.has(name)) continue;
    if (!apps.has(name)) apps.set(name, { files: {} });
    const app = apps.get(name)!;
    const rel = key.replaceAll("\\", "/").slice(extractFolderPath(key, "raw_app")!.length);
    if (isRawAppFolderMetadataFile(key)) {
      app.metaPath = key;
    } else if (!rel.startsWith(APP_BACKEND_FOLDER + "/")) {
      app.files[key] = content;
    }
  }
  const stale: string[] = [];
  for (const [name, { metaPath, files }] of apps) {
    if (!metaPath || !importsSharedUi(files)) continue;
    let deployed: string | undefined;
    try {
      const app = await wmill.getAppByPath({
        workspace,
        path: name.replaceAll(path.sep, "/"),
      });
      deployed = (app.value as any)?.shared_ui_hash;
    } catch {
      // Not deployed yet: the file diff already queues it as added.
      continue;
    }
    if (deployed !== hash) stale.push(metaPath);
  }
  return stale;
}

/** The workspace's shared UI store, or undefined when it cannot be read. */
async function fetchSharedUi(
  workspace: string,
): Promise<Record<string, string> | undefined> {
  try {
    const got = await wmill.getSharedUi({ workspace });
    return got.files ?? {};
  } catch {
    return undefined;
  }
}

export type SharedUiChange =
  | { type: "added"; path: string }
  | { type: "edited"; path: string; before: string; after: string }
  | { type: "deleted"; path: string };

/**
 * The push-direction diff (local -> remote) of two already-read file maps.
 * Split out so the preview and the push itself compare identically off one
 * read of each side.
 */
function sharedUiChanges(
  files: Record<string, string>,
  remote: Record<string, string>,
  keepDeleted?: boolean,
): SharedUiChange[] {
  // Use Object.hasOwn, not `in`: a file named after an Object.prototype member
  // (e.g. ui/toString) would otherwise register as always-present and be
  // misdiffed.
  const changes: SharedUiChange[] = [];
  for (const [rel, content] of Object.entries(files)) {
    const p = `${SHARED_UI_DIR}/${rel}`;
    if (!Object.hasOwn(remote, rel)) {
      changes.push({ type: "added", path: p });
    } else if (remote[rel] !== content) {
      changes.push({ type: "edited", path: p, before: remote[rel], after: content });
    }
  }
  if (!keepDeleted) {
    for (const rel of Object.keys(remote)) {
      if (!Object.hasOwn(files, rel)) {
        changes.push({ type: "deleted", path: `${SHARED_UI_DIR}/${rel}` });
      }
    }
  }
  return changes;
}

/**
 * Diff the local <cwd>/ui/ folder against the workspace's shared UI store in
 * the push direction (local -> remote), returning entries whose `path` is
 * prefixed with `ui/`. This is the same comparison pushSharedUi applies, so the
 * dry-run preview and the real push never diverge.
 *
 * Mirrors pushSharedUi's no-ops: with no local ui/ folder, or under
 * `keepDeleted` with an unreadable store, the apply does nothing, so the
 * preview must be empty (even when the remote store is non-empty) to avoid
 * phantom diffs the apply won't perform.
 */
export async function diffSharedUi(
  workspace: string,
  keepDeleted?: boolean,
): Promise<SharedUiChange[]> {
  const localDir = path.join(process.cwd(), SHARED_UI_DIR);
  if (!fs.existsSync(localDir)) {
    return [];
  }
  const files = await readDirRecursive(localDir);
  const remote = await fetchSharedUi(workspace);
  if (remote === undefined && keepDeleted) {
    return [];
  }
  // If endpoint missing or unauthorized, treat remote as empty (the push
  // would attempt the PUT anyway).
  return sharedUiChanges(files, remote ?? {}, keepDeleted);
}

/**
 * Push the local <cwd>/ui/ folder to the workspace's shared UI store.
 * Returns true if a push was performed, false if the folder is missing or
 * already matches the remote store. Note an empty-but-existing folder still
 * pushes an empty map (clearing the remote store) if the remote is non-empty —
 * unless `keepDeleted`, which folds remote-only files back into the map.
 */
export async function pushSharedUi(
  workspace: string,
  keepDeleted?: boolean,
): Promise<boolean> {
  const localDir = path.join(process.cwd(), SHARED_UI_DIR);
  if (!fs.existsSync(localDir)) {
    return false;
  }

  const files = await readDirRecursive(localDir);
  const remote = await fetchSharedUi(workspace);
  // The store is written whole, so a remote-only file is pruned by omission
  // alone. An unreadable store leaves no way to carry those files over, so
  // the shared UI is left untouched rather than cleared.
  if (remote === undefined && keepDeleted) {
    log.warn(
      colors.yellow(
        "Could not read the shared UI folder from the remote; skipping its push so --keep-deleted does not clear it.",
      ),
    );
    return false;
  }

  // Same comparison as diffSharedUi, off the same two maps, so preview and
  // push never diverge.
  if (sharedUiChanges(files, remote ?? {}, keepDeleted).length === 0) {
    log.info(colors.gray("Shared UI folder up to date"));
    return false;
  }

  if (keepDeleted) {
    for (const [rel, content] of Object.entries(remote!)) {
      // defineProperty, not assignment: `files.__proto__ = "..."` runs the
      // inherited setter and creates no own property, so a remote `ui/__proto__`
      // would be dropped from the whole-map PUT — i.e. deleted despite the flag.
      if (!Object.hasOwn(files, rel)) {
        Object.defineProperty(files, rel, {
          value: content,
          writable: true,
          enumerable: true,
          configurable: true,
        });
      }
    }
  }
  await wmill.updateSharedUi({
    workspace,
    requestBody: { files },
  });
  log.info(
    colors.green(
      `Pushed ${Object.keys(files).length} file(s) to shared UI folder`,
    ),
  );
  return true;
}

/**
 * Pull the workspace's shared UI store into <cwd>/ui/.
 * Files removed remotely are also removed locally, unless `keepDeleted`.
 */
export async function pullSharedUi(
  workspace: string,
  keepDeleted?: boolean,
): Promise<boolean> {
  const localDir = path.join(process.cwd(), SHARED_UI_DIR);
  let got;
  try {
    got = await wmill.getSharedUi({ workspace });
  } catch (e) {
    log.debug(`Skipping shared UI pull: ${e}`);
    return false;
  }
  const files = got?.files ?? {};

  if (Object.keys(files).length === 0 && !fs.existsSync(localDir)) {
    return false;
  }
  fs.mkdirSync(localDir, { recursive: true });

  // Write/refresh files
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(localDir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    let existing: string | undefined;
    try {
      existing = readTextFileSync(full);
    } catch {
      existing = undefined;
    }
    if (existing !== content) {
      fs.writeFileSync(full, content as string, "utf-8");
    }
  }

  // Delete locally-orphaned files
  if (!keepDeleted) {
    const known = new Set(Object.keys(files));
    const local = await readDirRecursive(localDir);
    for (const rel of Object.keys(local)) {
      if (!known.has(rel)) {
        const full = path.join(localDir, rel);
        try {
          fs.unlinkSync(full);
        } catch {
          // ignore
        }
      }
    }
  }

  log.info(
    colors.green(`Pulled ${Object.keys(files).length} file(s) into ui/`),
  );
  return true;
}
