import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";

import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { runCatalogQuery } from "../../utils/catalog.ts";
import { psql as psqlDatatable } from "./psql.ts";
import { serve as serveDatatable } from "./serve.ts";
import {
  createMigration,
  rollbackMigrations,
  runMigrations,
} from "../datatable_migrations.ts";

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

function migrateNew(
  opts: GlobalOptions & { datatable?: string },
  name: string,
) {
  createMigration(opts.datatable ?? DEFAULT_DATATABLE_NAME, name);
}

async function migrateUp(opts: GlobalOptions, name?: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  await runMigrations(workspace.workspaceId, name ?? DEFAULT_DATATABLE_NAME);
}

async function migrateDown(opts: GlobalOptions, name?: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  await rollbackMigrations(workspace.workspaceId, name ?? DEFAULT_DATATABLE_NAME);
}

const migrateCommand = new Command()
  .description("manage datatable migrations")
  .command("new", "scaffold a new migration (.up.sql / .down.sql files)")
  .arguments("<name:string>")
  .option(
    "-d --datatable <datatable:string>",
    "Target datatable (default: main)",
  )
  .action(migrateNew as any)
  .command("up", "apply all pending migrations to a datatable")
  .arguments("[name:string]")
  .action(migrateUp as any)
  .command("down", "roll back the most recent migration on a datatable")
  .arguments("[name:string]")
  .action(migrateDown as any);

async function serve(
  opts: GlobalOptions & { port?: number; host?: string; password?: string },
) {
  await serveDatatable(opts);
}

async function psql(
  opts: GlobalOptions & { name?: string; port?: number; host?: string; password?: string },
) {
  await psqlDatatable(opts);
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
  .action(run as any)
  .command("migrate", migrateCommand)
  .command(
    "serve",
    "Serve all datatables as a Postgres-wire endpoint (psql, DBeaver, pgAdmin); the client picks the datatable via the database name in its connection string",
  )
  .option(
    "--port <port:number>",
    "Port to listen on (default: first free port in 5433-5500)",
  )
  .option(
    "--host <host:string>",
    "Bind address (default: 127.0.0.1)",
  )
  .option(
    "--password <password:string>",
    "Password for Postgres clients (default: generate a random password at startup)",
  )
  .action(serve as any)
  .command(
    "psql",
    "Start a serve listener and launch psql connected to it",
  )
  .option(
    "-n --name <name:string>",
    "Datatable to connect psql to (default: main)",
  )
  .option(
    "--port <port:number>",
    "Port the proxy listens on (default: first free port in 5433-5500)",
  )
  .option(
    "--host <host:string>",
    "Bind address for the proxy (default: 127.0.0.1)",
  )
  .option(
    "--password <password:string>",
    "Password for the temporary Postgres proxy (default: generate a random password at startup)",
  )
  .action(psql as any);

export default command;
