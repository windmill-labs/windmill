import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as wmill from "../../../gen/services.gen.ts";

function formatTimestamp(ts: string): string {
  const date = new Date(ts);
  return date.toISOString().replace("T", " ").substring(0, 19);
}

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  await resolveWorkspace(opts);
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
      .header(["Prefix", "Label", "Created At", "Last Used", "Expiration"])
      .padding(2)
      .border(true)
      .body(
        tokens.map((t) => [
          t.token_prefix,
          t.label ?? "-",
          formatTimestamp(t.created_at),
          formatTimestamp(t.last_used_at),
          t.expiration ? formatTimestamp(t.expiration) : "never",
        ])
      )
      .render();
  }
}

async function create(
  opts: GlobalOptions & {
    label?: string;
    expiration?: string;
  }
) {
  opts = await mergeConfigWithConfigFile(opts);
  await resolveWorkspace(opts);
  await requireLogin(opts);

  const token = await wmill.createToken({
    requestBody: {
      label: opts.label,
      expiration: opts.expiration,
    },
  });

  console.log(token);
}

async function deleteToken(opts: GlobalOptions, tokenPrefix: string) {
  opts = await mergeConfigWithConfigFile(opts);
  await resolveWorkspace(opts);
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
  .action(create as any)
  .command("delete", "Delete a token by its prefix")
  .arguments("<token_prefix:string>")
  .action(deleteToken as any);

export default command;
