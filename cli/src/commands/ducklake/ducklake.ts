import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";

import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { runCatalogQuery } from "../../utils/catalog.ts";

const DEFAULT_DUCKLAKE_NAME = "main";

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const names = await wmill.listDucklakes({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(names));
  } else {
    new Table()
      .header(["Name"])
      .padding(2)
      .border(true)
      .body(names.map((name) => [name]))
      .render();
  }
}

async function run(
  opts: GlobalOptions & { name?: string; silent?: boolean },
  sql: string,
) {
  const name = opts.name ?? DEFAULT_DUCKLAKE_NAME;
  await runCatalogQuery(opts, "ducklake", name, sql);
}

const command = new Command()
  .description("ducklake related commands")
  .command("list", "list all ducklakes in the workspace")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("run", "run a SQL query on a ducklake")
  .arguments("<sql:string>")
  .option(
    "-n --name <name:string>",
    "Ducklake name (default: main)",
  )
  .option(
    "-s --silent",
    "Output only the final result as JSON. Useful for scripting.",
  )
  .action(run as any);

export default command;
