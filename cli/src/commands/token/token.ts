import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import type { TruncatedToken } from "../../../gen/types.gen.ts";
import { formatTimestamp } from "../../utils/utils.ts";

function access(t: TruncatedToken): string {
  const level = t.super_admin ? "superadmin" : t.admin ? "admin" : "member";
  return t.read_only ? `${level}, read-only` : level;
}

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  await requireLogin(opts);

  const tokens = await wmill.listTokens({
    excludeEphemeral: true,
  });

  if (opts.json) {
    console.log(JSON.stringify(tokens));
  } else {
    if (tokens.length === 0) {
      log.info("No tokens found.");
      return;
    }
    new Table()
      .header(["Prefix", "Label", "Created At", "Last Used", "Expiration", "Workspace", "Access", "Scopes"])
      .padding(2)
      .border(true)
      .body(
        tokens.map((t) => [
          t.token_prefix,
          t.label ?? "-",
          formatTimestamp(t.created_at),
          formatTimestamp(t.last_used_at),
          t.expiration ? formatTimestamp(t.expiration) : "never",
          t.workspace_id ?? "all",
          access(t),
          t.scopes && t.scopes.length > 0 ? t.scopes.join(" ") : "unscoped",
        ])
      )
      .render();
  }
}

async function create(
  opts: GlobalOptions & {
    label?: string;
    expiration?: string;
    allWorkspaces?: boolean;
    admin?: boolean;
    superadmin?: boolean;
    scope?: string[];
  }
) {
  opts = await mergeConfigWithConfigFile(opts);
  await requireLogin(opts);

  // Bound to the current workspace unless asked otherwise: a bound token is refused on every
  // other workspace and on instance-level endpoints.
  const workspaceId = opts.allWorkspaces
    ? undefined
    : (await resolveWorkspace(opts)).workspaceId;
  if (opts.superadmin && workspaceId) {
    log.warn(
      "The token is bound to workspace " +
        workspaceId +
        ": instance-level endpoints will refuse it. Pass --all-workspaces for instance commands."
    );
  }

  const token = await wmill.createToken({
    requestBody: {
      label: opts.label,
      expiration: opts.expiration,
      workspace_id: workspaceId,
      admin: opts.admin ?? false,
      super_admin: opts.superadmin ?? false,
      scopes: opts.scope && opts.scope.length > 0 ? opts.scope : undefined,
    },
  });

  console.log(token);
}

async function updateScopes(
  opts: GlobalOptions & { scope?: string[]; clear?: boolean },
  tokenPrefix: string
) {
  const scopes = opts.scope ?? [];
  if (!!opts.clear === scopes.length > 0) {
    throw new Error("Pass either one or more --scope, or --clear");
  }
  opts = await mergeConfigWithConfigFile(opts);
  await requireLogin(opts);

  await wmill.updateTokenScopes({
    tokenPrefix,
    requestBody: { scopes: opts.clear ? null : scopes },
  });

  log.info(
    colors.green(
      opts.clear
        ? `Token with prefix '${tokenPrefix}' no longer has scope restrictions.`
        : `Token with prefix '${tokenPrefix}' scoped to: ${scopes.join(" ")}`
    )
  );
}

async function deleteToken(opts: GlobalOptions, tokenPrefix: string) {
  opts = await mergeConfigWithConfigFile(opts);
  await requireLogin(opts);

  await wmill.deleteToken({ tokenPrefix });

  log.info(colors.green(`Token with prefix '${tokenPrefix}' deleted.`));
}

const command = new Command()
  .description("Manage API tokens")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "List API tokens")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("create", "Create a new API token")
  .option("--label <label:string>", "Token label")
  .option("--expiration <expiration:string>", "Token expiration (ISO 8601 timestamp)")
  .option(
    "--all-workspaces",
    "Do not bind the token to the current workspace: it then reaches every workspace you are a member of and instance-level endpoints"
  )
  .option("--admin", "Carry your workspace admin rights (off by default)")
  .option("--superadmin", "Carry your superadmin rights (off by default)")
  .option(
    "--scope <scope:string>",
    "Restrict the token to this scope (e.g. jobs:run, scripts:read:f/folder/*). Repeatable. Omit for no scope restriction.",
    { collect: true }
  )
  .action(create as any)
  .command("update-scopes", "Replace the scopes of one of your tokens, by its prefix")
  .arguments("<token_prefix:string>")
  .option(
    "--scope <scope:string>",
    "Scope the token is restricted to (e.g. jobs:run, scripts:read:f/folder/*). Repeatable; replaces all existing scopes.",
    { collect: true }
  )
  .option(
    "--clear",
    "Remove every scope restriction (the token keeps its workspace binding and admin/superadmin rights)"
  )
  .action(updateScopes as any)
  .command("delete", "Delete a token by its prefix")
  .arguments("<token_prefix:string>")
  .action(deleteToken as any);

export default command;
