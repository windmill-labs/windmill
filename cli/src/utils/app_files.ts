import {
  APP_BACKEND_FOLDER,
  RECORDINGS_FOLDER,
} from "../commands/app/app_metadata.ts";

/** Directories under a raw app that no push sends. */
const NEVER_DEPLOYED_DIRS = new Set([
  "node_modules",
  "dist",
  ".claude",
  "sql_to_apply",
]);

/** Files under a raw app that no push sends. */
const NEVER_DEPLOYED_FILES = new Set([
  "package-lock.json",
  "DATATABLES.md",
  "AGENTS.md",
  "wmill.d.ts",
]);

/**
 * Whether an app-root-relative path (`/` separators, leading slash optional)
 * reaches the server through any of a push's three channels: `raw_app.yaml` as
 * metadata, the backend folder as runnables, the rest bundled by
 * `collectAppFiles`. A path this rejects deploys nothing, so changing it is not
 * a change to the app however much the sync diff lists it. `collectAppFiles`
 * must not drift from this — it reads the same two sets.
 */
export function deploysWithRawApp(relativePath: string): boolean {
  const segments = relativePath.split("/").filter(Boolean);
  if (segments.length === 0) return false;
  const name = segments[segments.length - 1];
  const dirs = segments.slice(0, -1);
  // The sets below describe the bundle, which never walks into the backend
  // folder — applying them there would strip a runnable whose file shares a
  // name (`backend/wmill.d.ts` is the runnable `wmill.d`). Depth 1 because
  // `loadRunnablesFromBackend` reads that folder's top level only.
  if (dirs[0] === APP_BACKEND_FOLDER) return dirs.length === 1;
  if (NEVER_DEPLOYED_FILES.has(name)) return false;
  if (dirs.some((d) => NEVER_DEPLOYED_DIRS.has(d))) return false;
  // Session recordings are written at the app root only, so an app with a
  // `recordings/` component folder of its own still ships it.
  if (dirs[0] === RECORDINGS_FOLDER) return false;
  return true;
}

export { NEVER_DEPLOYED_DIRS, NEVER_DEPLOYED_FILES };
