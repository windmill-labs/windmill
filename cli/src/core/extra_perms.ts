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
 * it has its own log lines and its own non-fatal failure mode. Each grant /
 * revoke is logged as a separate line on success; failures are logged in red
 * but never throw — a stale yaml referencing a deleted user/group surfaces as
 * a red warning, not a hard error that would block the surrounding deploy.
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
          `  extra_perms: failed to grant ${access} to ${owner} on ${kind}/${path}: ${e?.body ?? e?.message ?? e}`,
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
          `  extra_perms: failed to revoke ${owner} on ${kind}/${path}: ${e?.body ?? e?.message ?? e}`,
        ),
      );
    }
  }

  return calls;
}
