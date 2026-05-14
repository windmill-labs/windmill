import * as wmill from "../../gen/services.gen.ts";
import * as log from "./log.ts";
import { colors } from "@cliffy/ansi/colors";
import type { AddGranularAclsData } from "../../gen/types.gen.ts";

export type ExtraPermsKind = AddGranularAclsData["kind"];

type PermsMap = Record<string, boolean>;

function normalize(
  value: unknown,
  source: string,
): { perms: PermsMap; invalid: string[] } {
  const perms: PermsMap = {};
  const invalid: string[] = [];
  if (!value || typeof value !== "object") return { perms, invalid };
  for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
    if (typeof v === "boolean") {
      perms[k] = v;
    } else {
      invalid.push(k);
    }
  }
  if (invalid.length > 0) {
    log.error(
      colors.red(
        `extra_perms: ignoring ${invalid.length} invalid entry/entries in ${source} (non-boolean value): ${invalid.join(", ")}`,
      ),
    );
  }
  return { perms, invalid };
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

  const { perms: localPerms } = normalize(local, "local yaml");
  const { perms: remotePerms } = normalize(remote, "remote response");

  const toGrant: Array<[string, boolean]> = [];
  for (const [owner, write] of Object.entries(localPerms)) {
    if (!(owner in remotePerms) || remotePerms[owner] !== write) {
      toGrant.push([owner, write]);
    }
  }

  const toRevoke: string[] = Object.keys(remotePerms).filter(
    (owner) => !(owner in localPerms),
  );

  if (toGrant.length === 0 && toRevoke.length === 0) {
    return 0;
  }

  let calls = 0;
  for (const [owner, write] of toGrant) {
    const access = write ? "write" : "read";
    try {
      await wmill.addGranularAcls({
        workspace,
        kind,
        path,
        requestBody: { owner, write },
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
