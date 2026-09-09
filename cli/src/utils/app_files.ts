import { RECORDINGS_FOLDER } from "../commands/app/app_metadata.ts";

/** Directories under a raw app that no push ever sends — dependencies, build
 * output, editor state, and SQL staged for a datatable migration. */
const NEVER_DEPLOYED_DIRS = new Set([
  "node_modules",
  "dist",
  ".claude",
  "sql_to_apply",
]);

/** Files under a raw app that no push ever sends: generated for the local
 * toolchain, or documentation for whoever edits it. */
const NEVER_DEPLOYED_FILES = new Set([
  "package-lock.json",
  "DATATABLES.md",
  "AGENTS.md",
  "wmill.d.ts",
]);

/**
 * Whether a path under a raw app's folder reaches the server at all. The three
 * channels a push sends are `raw_app.yaml` as metadata, the backend folder as
 * runnables, and everything `collectAppFiles` bundles into `value.files` — so
 * a path this rejects deploys nothing, and changing it is not a change to the
 * app however much the sync diff lists it.
 *
 * `collectAppFiles` is the one that must not drift from this: it applies the
 * same two sets, plus its own two exclusions for the paths that deploy through
 * the other channels rather than the bundle.
 *
 * @param relativePath app-root-relative, with `/` separators (`/index.tsx`,
 *   `sql_to_apply/a.sql`) — either way a leading slash is optional.
 */
export function deploysWithRawApp(relativePath: string): boolean {
  const segments = relativePath.split("/").filter(Boolean);
  if (segments.length === 0) return false;
  const name = segments[segments.length - 1];
  const dirs = segments.slice(0, -1);
  if (NEVER_DEPLOYED_FILES.has(name)) return false;
  if (dirs.some((d) => NEVER_DEPLOYED_DIRS.has(d))) return false;
  // Session recordings are written at the app root only, so an app with a
  // `recordings/` component folder of its own still ships it.
  if (dirs[0] === RECORDINGS_FOLDER) return false;
  return true;
}

export { NEVER_DEPLOYED_DIRS, NEVER_DEPLOYED_FILES };
