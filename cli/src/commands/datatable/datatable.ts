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
  pushLocalMigrations,
  rollbackMigrations,
  runMigrations,
  validateLocalMigrations,
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

async function migrateUp(opts: GlobalOptions & { datatable?: string }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const dt = opts.datatable ?? DEFAULT_DATATABLE_NAME;
  // Reject malformed local migrations (duplicate timestamps, orphan downs) before
  // pushing — the same check `wmill sync push` runs — so a duplicate timestamp
  // can't silently overwrite one migration on upsert.
  const errors = validateLocalMigrations(new Set([dt]));
  if (errors.length > 0) {
    log.error(
      "Invalid datatable migrations, aborting:\n" +
        errors.map((e) => `  - ${e}`).join("\n"),
    );
    process.exit(1);
  }
  // Push any locally-created/edited migration files first (without running
  // them), so `migrate up` works even before a `wmill sync push`.
  await pushLocalMigrations(workspace.workspaceId, dt);
  await runMigrations(workspace.workspaceId, dt);
}

async function migrateDown(opts: GlobalOptions & { datatable?: string }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const dt = opts.datatable ?? DEFAULT_DATATABLE_NAME;
  await rollbackMigrations(workspace.workspaceId, dt);
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
  .command(
    "up",
    "apply all pending migrations to the main datatable (or one via --datatable)",
  )
  .option(
    "-d --datatable <datatable:string>",
    "Target datatable (default: main)",
  )
  .action(migrateUp as any)
  .command(
    "down",
    "roll back the most recent migration on the main datatable (or one via --datatable)",
  )
  .option(
    "-d --datatable <datatable:string>",
    "Target datatable (default: main)",
  )
  .action(migrateDown as any);

async function create(
  opts: GlobalOptions & { resource?: string; force?: boolean },
  name?: string,
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const dtName = name ?? DEFAULT_DATATABLE_NAME;

  const existing = await wmill.listDataTables({
    workspace: workspace.workspaceId,
  });
  if (existing.some((d) => d.name === dtName)) {
    throw new Error(`Datatable '${dtName}' already exists in this workspace`);
  }
  // edit_datatable_config replaces the whole settings object, and fork
  // metadata on existing datatables can't be read back through the API —
  // so only touch a non-empty config when explicitly asked to.
  if (existing.length > 0 && !opts.force) {
    throw new Error(
      `Workspace already has datatable(s): ${existing
        .map((d) => d.name)
        .join(", ")}. Re-run with --force to add '${dtName}' ` +
        "(note: fork metadata on existing datatables is not preserved)",
    );
  }

  const datatables: Record<
    string,
    { database: { resource_type: "postgresql" | "instance"; resource_path?: string } }
  > = {};
  for (const d of existing) {
    datatables[d.name] = {
      database: {
        resource_type: d.resource_type as "postgresql" | "instance",
        resource_path: d.resource_path ?? undefined,
      },
    };
  }
  datatables[dtName] = opts.resource
    ? { database: { resource_type: "postgresql", resource_path: opts.resource } }
    : { database: { resource_type: "instance", resource_path: "datatable_db" } };

  await wmill.editDataTableConfig({
    workspace: workspace.workspaceId,
    requestBody: { settings: { datatables } },
  });
  log.info(
    `Datatable '${dtName}' created (${
      opts.resource
        ? `postgresql resource ${opts.resource}`
        : "instance-backed"
    }). Scripts can now use datatable://${dtName}.`,
  );
}

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
    "create",
    "register a datatable database in the workspace (default: instance-backed 'main') so scripts can use datatable://<name>",
  )
  .arguments("[name:string]")
  .option(
    "--resource <resource:string>",
    "Back the datatable with an existing postgresql resource path instead of the instance database",
  )
  .option(
    "--force",
    "Allow adding to a workspace that already has datatables (fork metadata on existing ones is not preserved)",
  )
  .action(create as any)
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
