import * as fs from "node:fs";
import * as path from "node:path";
import * as log from "../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as wmill from "../../gen/services.gen.ts";
import { readTextFile, readTextFileSync } from "../utils/utils.ts";

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

export type SharedUiChange =
  | { type: "added"; path: string }
  | { type: "edited"; path: string; before: string; after: string }
  | { type: "deleted"; path: string };

/**
 * Diff the local <cwd>/ui/ folder against the workspace's shared UI store in
 * the push direction (local -> remote), returning entries whose `path` is
 * prefixed with `ui/`. This is the same comparison pushSharedUi applies, so the
 * dry-run preview and the real push never diverge.
 *
 * Mirrors pushSharedUi's no-op: with no local ui/ folder there is nothing to
 * push, so the apply is a no-op and the preview must be empty (even when the
 * remote store is non-empty) to avoid phantom diffs the apply won't perform.
 */
export async function diffSharedUi(workspace: string): Promise<SharedUiChange[]> {
  const localDir = path.join(process.cwd(), SHARED_UI_DIR);
  if (!fs.existsSync(localDir)) {
    return [];
  }
  const files = await readDirRecursive(localDir);

  let remote: Record<string, string> = {};
  try {
    const got = await wmill.getSharedUi({ workspace });
    remote = got.files ?? {};
  } catch {
    // If endpoint missing or unauthorized, treat remote as empty (the push
    // would attempt the PUT anyway).
  }

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
  for (const rel of Object.keys(remote)) {
    if (!Object.hasOwn(files, rel)) {
      changes.push({ type: "deleted", path: `${SHARED_UI_DIR}/${rel}` });
    }
  }
  return changes;
}

/**
 * Push the local <cwd>/ui/ folder to the workspace's shared UI store.
 * Returns true if a push was performed, false if the folder is missing or
 * already matches the remote store. Note an empty-but-existing folder still
 * pushes an empty map (clearing the remote store) if the remote is non-empty.
 */
export async function pushSharedUi(workspace: string): Promise<boolean> {
  const localDir = path.join(process.cwd(), SHARED_UI_DIR);
  if (!fs.existsSync(localDir)) {
    return false;
  }

  // Skip if no change — reuse diffSharedUi so preview and push never diverge.
  const diff = await diffSharedUi(workspace);
  if (diff.length === 0) {
    log.info(colors.gray("Shared UI folder up to date"));
    return false;
  }

  const files = await readDirRecursive(localDir);
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
 * Files removed remotely are also removed locally.
 */
export async function pullSharedUi(workspace: string): Promise<boolean> {
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

  log.info(
    colors.green(`Pulled ${Object.keys(files).length} file(s) into ui/`),
  );
  return true;
}
