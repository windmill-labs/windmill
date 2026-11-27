import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { formatTimestamp } from "../../utils/utils.ts";

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
      .header(["Prefix", "Label", "Created At", "Last Used", "Expiration", "Scopes"])
      .padding(2)
      .border(true)
      .body(
        tokens.map((t) => [
          t.token_prefix,
          t.label ?? "-",
          formatTimestamp(t.created_at),
          formatTimestamp(t.last_used_at),
          t.expiration ? formatTimestamp(t.expiration) : "never",
          t.scopes && t.scopes.length > 0 ? t.scopes.join(" ") : "full access",
        ])
      )
      .render();
  }
}

async function create(
  opts: GlobalOptions & {
    label?: string;
    expiration?: string;
    scope?: string[];
  }
) {
  opts = await mergeConfigWithConfigFile(opts);
  await requireLogin(opts);

  const token = await wmill.createToken({
    requestBody: {
      label: opts.label,
      expiration: opts.expiration,
      scopes: opts.scope && opts.scope.length > 0 ? opts.scope : undefined,
    },
  });

  console.log(token);
}

async function updateScopes(
  opts: GlobalOptions & { scope?: string[]; fullAccess?: boolean },
  tokenPrefix: string
) {
  const scopes = opts.scope ?? [];
  if (!!opts.fullAccess === scopes.length > 0) {
    throw new Error("Pass either one or more --scope, or --full-access");
  }
  opts = await mergeConfigWithConfigFile(opts);
  await requireLogin(opts);

  await wmill.updateTokenScopes({
    tokenPrefix,
    requestBody: { scopes: opts.fullAccess ? null : scopes },
  });

  log.info(
    colors.green(
      opts.fullAccess
        ? `Token with prefix '${tokenPrefix}' now has full access.`
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
    "--scope <scope:string>",
    "Restrict the token to this scope (e.g. jobs:run, scripts:read:f/folder/*). Repeatable. Omit for full access.",
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
  .option("--full-access", "Remove every scope restriction, giving the token full access")
  .action(updateScopes as any)
  .command("delete", "Delete a token by its prefix")
  .arguments("<token_prefix:string>")
  .action(deleteToken as any);

export default command;
