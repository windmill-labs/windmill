import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";

import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { runCatalogQuery } from "./catalog.ts";

const DEFAULT_DATATABLE_NAME = "main";

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const items = await wmill.listDataTables({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(items));
  } else {
    new Table()
      .header(["Name", "Resource Type", "Resource Path"])
      .padding(2)
      .border(true)
      .body(items.map((x) => [x.name, x.resource_type, x.resource_path]))
      .render();
  }
}

async function run(
  opts: GlobalOptions & { name?: string; silent?: boolean },
  sql: string,
) {
  const name = opts.name ?? DEFAULT_DATATABLE_NAME;
  await runCatalogQuery(opts, "datatable", name, sql);
}

const command = new Command()
  .description("datatable related commands")
  .command("list", "list all datatables in the workspace")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("run", "run a SQL query on a datatable")
  .arguments("<sql:string>")
  .option(
    "-n --name <name:string>",
    "Datatable name (default: main)",
  )
  .option(
    "-s --silent",
    "Output only the final result as JSON. Useful for scripting.",
  )
  .action(run as any);

export default command;
