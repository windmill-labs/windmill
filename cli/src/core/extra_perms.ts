import * as wmill from "../../gen/services.gen.ts";
import * as log from "./log.ts";
import { colors } from "@cliffy/ansi/colors";
import type { AddGranularAclsData } from "../../gen/types.gen.ts";

export type ExtraPermsKind = AddGranularAclsData["kind"];

type PermsMap = Record<string, boolean>;

function normalize(value: unknown): PermsMap {
  if (!value || typeof value !== "object") return {};
  const out: PermsMap = {};
  for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
    if (typeof v === "boolean") out[k] = v;
  }
  return out;
}

/**
 * Apply the diff between `local` and `remote` granular ACL maps as a sequence
 * of `/acls/add` and `/acls/remove` calls. Used by every CLI push path so a
 * yaml change that *only* touches `extra_perms` never bumps the script/flow/app
 * version — perm mutations route through the dedicated granular-ACL endpoints
 * exactly as if the user had clicked through the UI.
 *
 * The function is intentionally independent of the surrounding push logic:
 * it has its own log line, its own try/catch, and its own non-fatal failure
 * mode. A stale yaml referencing a deleted user/group surfaces as a yellow
 * warning, not a hard error that would block the surrounding deploy.
 *
 * @returns number of /acls/* calls actually issued (0 means perms in sync).
 */
export async function applyExtraPermsDiff(
  workspace: string,
  kind: ExtraPermsKind,
  path: string,
  local: unknown,
  remote: unknown,
): Promise<number> {
  const localPerms = normalize(local);
  const remotePerms = normalize(remote);

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

  log.info(
    colors.gray(
      `Syncing extra_perms for ${kind} ${path} (${toGrant.length} grant, ${toRevoke.length} revoke)`,
    ),
  );

  let calls = 0;
  for (const [owner, write] of toGrant) {
    try {
      await wmill.addGranularAcls({
        workspace,
        kind,
        path,
        requestBody: { owner, write },
      });
      calls += 1;
    } catch (e: any) {
      log.warn(
        colors.yellow(
          `  extra_perms: failed to grant ${owner}${write ? " (write)" : ""} on ${kind}/${path}: ${e?.body ?? e?.message ?? e}`,
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
      calls += 1;
    } catch (e: any) {
      log.warn(
        colors.yellow(
          `  extra_perms: failed to revoke ${owner} on ${kind}/${path}: ${e?.body ?? e?.message ?? e}`,
        ),
      );
    }
  }

  return calls;
}
