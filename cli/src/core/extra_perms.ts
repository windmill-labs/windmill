import * as wmill from "../../gen/services.gen.ts";
import * as log from "./log.ts";
import { colors } from "@cliffy/ansi/colors";
import type { AddGranularAclsData } from "../../gen/types.gen.ts";

export type ExtraPermsKind = AddGranularAclsData["kind"];

type Level = "read" | "run" | "write";
type PermsMap = Record<string, Level>;

/** Kinds whose read grant does not include running: there `false` is "run" and `"read"` views
 *  only. Elsewhere `false` and `"read"` are the same read grant. */
const RUN_LEVEL_KINDS: string[] = ["script", "flow", "resource", "folder"];

/** A grant as the level it gives: `true` or `"write"`, `false` or `"run"`, or `"read"`. */
function levelOf(value: unknown, runLevel: boolean): Level | undefined {
  if (value === true || value === "write") return "write";
  if (value === false || value === "run") return runLevel ? "run" : "read";
  if (value === "read") return "read";
  return undefined;
}

type Normalized = {
  perms: PermsMap;
  /** Keys with non-boolean values: never revoked, never granted — treat as "no opinion". */
  invalidOwners: Set<string>;
  /** Top-level value is not a plain object (array, primitive, etc.). The
   *  entire map is rejected and the caller must treat it like "no opinion"
   *  to avoid silently revoking every remote ACL. */
  malformedTop: boolean;
};

function normalize(value: unknown, source: string, runLevel: boolean): Normalized {
  const perms: PermsMap = {};
  const invalidOwners = new Set<string>();

  if (
    value === null ||
    value === undefined ||
    typeof value !== "object" ||
    Array.isArray(value)
  ) {
    if (value !== undefined && value !== null) {
      log.error(
        colors.red(
          `extra_perms: ${source} is not a {owner: grant} map — skipping ACL sync to avoid clobbering remote ACLs`,
        ),
      );
      return { perms, invalidOwners, malformedTop: true };
    }
    return { perms, invalidOwners, malformedTop: false };
  }

  const invalidList: string[] = [];
  for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
    const level = levelOf(v, runLevel);
    if (level !== undefined) {
      perms[k] = level;
    } else {
      invalidOwners.add(k);
      invalidList.push(k);
    }
  }
  if (invalidList.length > 0) {
    log.error(
      colors.red(
        `extra_perms: ${invalidList.length} invalid entry/entries in ${source} (not true, false, "read", "run" or "write", treating as "no opinion"): ${invalidList.join(", ")}`,
      ),
    );
  }
  return { perms, invalidOwners, malformedTop: false };
}

function formatError(e: unknown): string {
  if (!e || typeof e !== "object") return String(e);
  const anyE = e as { body?: unknown; message?: unknown };
  if (anyE.body !== undefined) {
    if (typeof anyE.body === "string") return anyE.body;
    try {
      return JSON.stringify(anyE.body);
    } catch {
      // fall through
    }
  }
  if (typeof anyE.message === "string") return anyE.message;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

/**
 * Apply the diff between `local` and `remote` granular ACL maps as a sequence
 * of `/acls/add` and `/acls/remove` calls. Used by every CLI push path so a
 * yaml change that *only* touches `extra_perms` never bumps the script/flow/app
 * version — perm mutations route through the dedicated granular-ACL endpoints
 * exactly as if the user had clicked through the UI.
 *
 * **`local === undefined` means "no opinion".** If the yaml does not carry an
 * `extra_perms` field at all, this function is a no-op — the remote ACLs are
 * left untouched. This is what prevents a stale local checkout from racing
 * a concurrent UI grant: only users who explicitly track perms in source (by
 * writing `extra_perms:` in the yaml, even as `{}`) get destructive sync.
 *
 * The function is intentionally independent of the surrounding push logic:
 * it has its own log lines and its own non-fatal failure mode. Each grant /
 * revoke is logged as a separate line on success; failures are logged in red
 * but never throw — a stale yaml referencing a deleted user/group surfaces as
 * a red error, not a hard error that would block the surrounding deploy.
 *
 * @returns number of /acls/* calls actually issued (0 means perms in sync, or
 *          the local yaml had no `extra_perms` field).
 */
export async function applyExtraPermsDiff(
  workspace: string,
  kind: ExtraPermsKind,
  path: string,
  local: unknown,
  remote: unknown,
): Promise<number> {
  // Absent local field = "no opinion" — never call /acls/* in this case.
  // Crucially, this protects users who don't track ACLs in source from having
  // their UI-managed perms silently revoked by a stale CLI push.
  if (local === undefined || local === null) {
    return 0;
  }

  const runLevel = RUN_LEVEL_KINDS.includes(kind);
  const localN = normalize(local, "local yaml", runLevel);
  // Top-level malformed (array, primitive, etc.) → treat as "no opinion" so a
  // typo in yaml can never silently revoke every remote ACL.
  if (localN.malformedTop) {
    return 0;
  }
  const remoteN = normalize(remote, "remote response", runLevel);

  const localPerms = localN.perms;
  const remotePerms = remoteN.perms;

  const toGrant: Array<[string, Level]> = [];
  for (const [owner, level] of Object.entries(localPerms)) {
    if (remotePerms[owner] !== level) {
      toGrant.push([owner, level]);
    }
  }

  // Owners with a malformed value in local yaml are treated as "no opinion":
  // they are excluded from the revoke set so a typo (`g/devs: "write"`) never
  // silently strips an existing ACL.
  const toRevoke: string[] = Object.keys(remotePerms).filter(
    (owner) =>
      !(owner in localPerms) && !localN.invalidOwners.has(owner),
  );

  if (toGrant.length === 0 && toRevoke.length === 0) {
    return 0;
  }

  let calls = 0;
  for (const [owner, level] of toGrant) {
    const access = level === "read" && runLevel ? "view only" : level;
    try {
      await wmill.addGranularAcls({
        workspace,
        kind,
        path,
        // Run and write as the `write` flag every server reads; `level` only for view-only.
        requestBody:
          level === "read" && runLevel
            ? { owner, write: false, level }
            : { owner, write: level === "write" },
      });
      log.info(
        colors.green(
          `  extra_perms: granted ${access} to ${owner} on ${kind}/${path}`,
        ),
      );
      calls += 1;
    } catch (e: any) {
      log.error(
        colors.red(
          `  extra_perms: failed to grant ${access} to ${owner} on ${kind}/${path}: ${formatError(e)}`,
        ),
      );
    }
  }

  for (const owner of toRevoke) {
    try {
      await wmill.removeGranularAcls({
        workspace,
        kind,
        path,
        requestBody: { owner },
      });
      log.info(
        colors.green(`  extra_perms: revoked ${owner} on ${kind}/${path}`),
      );
      calls += 1;
    } catch (e: any) {
      log.error(
        colors.red(
          `  extra_perms: failed to revoke ${owner} on ${kind}/${path}: ${formatError(e)}`,
        ),
      );
    }
  }

  return calls;
}
