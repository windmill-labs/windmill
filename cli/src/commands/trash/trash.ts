import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { apiErrorMessage, formatTimestamp } from "../../utils/utils.ts";

async function list(
  opts: GlobalOptions & {
    json?: boolean;
    kind?: string;
    page?: number;
    limit?: number;
  }
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (opts.page !== undefined && opts.page < 1) {
    throw new Error("--page starts at 1");
  }

  const items = await wmill.listTrash({
    workspace: workspace.workspaceId,
    itemKind: opts.kind,
    // The trash endpoint counts pages from 0, unlike the API's other list
    // endpoints whose `page` starts at 1; the flag counts from 1 like those.
    page: opts.page === undefined ? undefined : opts.page - 1,
    perPage: opts.limit,
  });

  if (opts.json) {
    console.log(JSON.stringify(items));
    return;
  }
  if (items.length === 0) {
    log.info("No trashed items found.");
    return;
  }
  new Table()
    .header(["ID", "Kind", "Path", "Deleted by", "Deleted at", "Expires at"])
    .padding(2)
    .border(true)
    .body(
      items.map((item) => [
        String(item.id),
        item.item_kind,
        item.item_path,
        item.deleted_by,
        formatTimestamp(item.deleted_at),
        formatTimestamp(item.expires_at),
      ])
    )
    .render();
  log.info(
    colors.gray(
      "`wmill trash get <id>` shows what an item held, `wmill trash restore <id>` puts it back."
    )
  );
}

async function get(opts: GlobalOptions & { json?: boolean }, id: number) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const item = await wmill.getTrashItem({
    workspace: workspace.workspaceId,
    id,
  });

  if (opts.json) {
    console.log(JSON.stringify(item));
    return;
  }
  console.log(colors.bold("ID:") + " " + item.id);
  console.log(colors.bold("Kind:") + " " + item.item_kind);
  console.log(colors.bold("Path:") + " " + item.item_path);
  console.log(colors.bold("Deleted by:") + " " + item.deleted_by);
  console.log(colors.bold("Deleted at:") + " " + formatTimestamp(item.deleted_at));
  console.log(colors.bold("Expires at:") + " " + formatTimestamp(item.expires_at));
  console.log(colors.bold("Data:"));
  console.log(JSON.stringify(item.item_data, null, 2));
}

async function restore(opts: GlobalOptions, ...ids: number[]) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let failed = 0;
  for (const id of ids) {
    try {
      const message = await wmill.restoreTrashItem({
        workspace: workspace.workspaceId,
        id,
      });
      log.info(colors.green(message));
    } catch (e) {
      failed += 1;
      log.error(
        `Could not restore trash item ${id}: ${apiErrorMessage(e) ?? String(e)}`
      );
    }
  }
  if (failed > 0) {
    process.exitCode = 1;
  }
}

const command = new Command()
  .description(
    "List, inspect and restore items deleted in the last three days (requires admin)"
  )
  .option("--json", "Output as JSON (for piping to jq)")
  .option(
    "--kind <kind:string>",
    "Only items of this kind: script, flow, app, schedule, variable, resource or a trigger kind such as http_trigger"
  )
  .option("--limit <limit:integer>", "Number of items to return (default 100, max 1000)")
  .option("--page <page:integer>", "Page to return, starting at 1")
  .action(list as any)
  .command("list", "List trashed items, most recently deleted first")
  .option("--json", "Output as JSON (for piping to jq)")
  .option(
    "--kind <kind:string>",
    "Only items of this kind: script, flow, app, schedule, variable, resource or a trigger kind such as http_trigger"
  )
  .option("--limit <limit:integer>", "Number of items to return (default 100, max 1000)")
  .option("--page <page:integer>", "Page to return, starting at 1")
  .action(list as any)
  .command("get", "Show a trashed item and the data it was deleted with")
  .arguments("<id:integer>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("restore", "Put trashed items back at their paths")
  .arguments("<ids...:integer>")
  .action(restore as any);

export default command;
