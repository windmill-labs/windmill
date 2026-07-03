import * as fs from "node:fs";
import * as path from "node:path";
import * as log from "../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as wmill from "../../gen/services.gen.ts";
import { readTextFile } from "../utils/utils.ts";
import { Confirm } from "@cliffy/prompt/confirm";

// Migrations live under <cwd>/migrations/datatable/<datatable>/, one folder per
// target data table, as `<timestamp>_<name>.up.sql` (and optional `.down.sql`).
// They are synced as ordinary workspace files (see the workspace tarball export
// and the `datatable_migration` handling in sync.ts); this module only holds the
// `wmill datatable migrate` command helpers and the per-file push primitive.
const MIGRATIONS_DIR = path.join("migrations", "datatable");

// Migration names map directly onto file names and the DB `name` column.
const MIGRATION_NAME_RE = /^[a-zA-Z0-9_-]+$/;

/** Current UTC time as a YYYYMMDDHHMMSS migration version. */
function migrationTimestamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getUTCFullYear()}${p(d.getUTCMonth() + 1)}${p(d.getUTCDate())}` +
    `${p(d.getUTCHours())}${p(d.getUTCMinutes())}${p(d.getUTCSeconds())}`
  );
}

/**
 * Scaffold a new migration under migrations/datatable/<datatable>/ as empty
 * `<timestamp>_<name>.up.sql` and `.down.sql` files. Purely local — no network.
 */
export function createMigration(datatable: string, name: string): void {
  if (!MIGRATION_NAME_RE.test(name)) {
    throw new Error(
      `Invalid migration name '${name}': use only letters, digits, '_' and '-'`,
    );
  }
  const dir = path.join(process.cwd(), MIGRATIONS_DIR, datatable);
  fs.mkdirSync(dir, { recursive: true });

  const timestamp = migrationTimestamp();
  const base = `${timestamp}_${name}`;
  const up = path.join(dir, `${base}.up.sql`);
  const down = path.join(dir, `${base}.down.sql`);
  // Frame the body in an explicit transaction so it applies atomically, matching
  // the template the UI's "New migration" modal seeds.
  const template = (direction: string) =>
    `-- ${direction} migration: ${name}\nBEGIN;\n\n-- Add your migration here\n\nEND;\n`;
  fs.writeFileSync(up, template("up"), "utf-8");
  fs.writeFileSync(down, template("down"), "utf-8");

  log.info(
    colors.green(`Created migration ${base} in ${MIGRATIONS_DIR}/${datatable}/`),
  );
  for (const f of [up, down]) {
    log.info(colors.gray(`  ${path.relative(process.cwd(), f)}`));
  }
}

/**
 * Apply the workspace's pending migrations to a data table (forwards migrations
 * recorded in `_wm_migrations`). Mirrors `wmill datatable migrate up`.
 */
export async function runMigrations(
  workspace: string,
  datatableName: string,
): Promise<void> {
  const result = await wmill.runDatatableMigrations({
    workspace,
    datatableName,
  });
  const applied = result.applied ?? [];
  if (applied.length === 0) {
    log.info(colors.gray(`No pending migrations to run on '${datatableName}'`));
    return;
  }
  log.info(
    colors.green(`Applied ${applied.length} migration(s) to '${datatableName}':`),
  );
  for (const m of applied) {
    log.info(colors.gray(`  ${m.version} ${m.name}`));
  }
}

/**
 * Roll back the most recently applied migration on a data table (one step).
 * Mirrors `wmill datatable migrate down`.
 */
export async function rollbackMigrations(
  workspace: string,
  datatableName: string,
): Promise<void> {
  const result = await wmill.rollbackDatatableMigrations({
    workspace,
    datatableName,
  });
  const rolledBack = result.rolled_back ?? [];
  if (rolledBack.length === 0) {
    log.info(
      colors.gray(`No applied migrations to roll back on '${datatableName}'`),
    );
    return;
  }
  for (const m of rolledBack) {
    log.info(
      colors.green(`Rolled back migration ${m.version} ${m.name} on '${datatableName}'`),
    );
  }
}

/**
 * Validate the on-disk migration files for the given data tables (or all of
 * them when `datatables` is omitted). Returns a list of human-readable problems;
 * an empty list means the migrations are well-formed. Two states are invalid:
 *  - two up (or two down) files sharing the same timestamp, which collide on the
 *    `(datatable, timestamp)` identity used to upsert; and
 *  - a `.down.sql` with no matching `.up.sql` (an up file is mandatory).
 */
export function validateLocalMigrations(datatables?: Set<string>): string[] {
  const errors: string[] = [];
  const root = path.join(process.cwd(), MIGRATIONS_DIR);
  if (!fs.existsSync(root)) return errors;

  for (const datatable of fs.readdirSync(root)) {
    if (datatables && !datatables.has(datatable)) continue;
    const dtDir = path.join(root, datatable);
    if (!fs.statSync(dtDir).isDirectory()) continue;

    const upNamesByTs = new Map<number, string[]>();
    const downNamesByTs = new Map<number, string[]>();
    const upBases = new Set<string>();
    const downBases: { ts: number; name: string }[] = [];

    for (const file of fs.readdirSync(dtDir)) {
      const m = file.match(/^(\d+)_(.*)\.(up|down)\.sql$/);
      if (!m) continue;
      const ts = Number(m[1]);
      const name = m[2];
      if (m[3] === "up") {
        (upNamesByTs.get(ts) ?? upNamesByTs.set(ts, []).get(ts)!).push(name);
        upBases.add(`${ts}_${name}`);
      } else {
        (downNamesByTs.get(ts) ?? downNamesByTs.set(ts, []).get(ts)!).push(name);
        downBases.push({ ts, name });
      }
    }

    for (const [ts, names] of upNamesByTs) {
      if (names.length > 1) {
        errors.push(
          `${datatable}: ${names.length} up migrations share timestamp ${ts} (${names.join(", ")})`,
        );
      }
    }
    for (const [ts, names] of downNamesByTs) {
      if (names.length > 1) {
        errors.push(
          `${datatable}: ${names.length} down migrations share timestamp ${ts} (${names.join(", ")})`,
        );
      }
    }
    for (const d of downBases) {
      if (!upBases.has(`${d.ts}_${d.name}`)) {
        errors.push(
          `${datatable}: ${d.ts}_${d.name}.down.sql has no matching ${d.ts}_${d.name}.up.sql`,
        );
      }
    }
  }

  return errors;
}

/**
 * Sync a single migration to the workspace based on the current on-disk state of
 * its `<datatable>/<timestamp>_<name>.up.sql` file: upsert it when the up file
 * exists, otherwise delete it. Called by `wmill sync push` for each changed
 * `datatable_migration` file.
 */
export async function pushMigrationFromDisk(
  workspace: string,
  m: { datatable: string; timestamp: number; name: string },
): Promise<void> {
  const dir = path.join(process.cwd(), MIGRATIONS_DIR, m.datatable);
  const base = `${m.timestamp}_${m.name}`;
  const upPath = path.join(dir, `${base}.up.sql`);

  if (!fs.existsSync(upPath)) {
    log.info(colors.red(`Deleting datatable_migration ${m.datatable}/${base}`));
    await wmill.deleteDatatableMigration({
      workspace,
      datatableName: m.datatable,
      timestamp: m.timestamp,
    });
    return;
  }

  const code_up = await readTextFile(upPath);
  const downPath = path.join(dir, `${base}.down.sql`);
  const code_down = fs.existsSync(downPath) ? await readTextFile(downPath) : undefined;

  log.info(colors.green(`Pushing datatable_migration ${m.datatable}/${base}`));
  await wmill.upsertDatatableMigration({
    workspace,
    datatableName: m.datatable,
    requestBody: {
      timestamp: m.timestamp,
      name: m.name,
      code_up,
      ...(code_down !== undefined ? { code_down } : {}),
    },
  });
}

/**
 * After a push that introduced new migrations, list them and (interactively)
 * offer to run them, equivalent to `wmill datatable migrate up` on each affected
 * data table.
 */
export async function offerToRunNewMigrations(
  workspace: string,
  newMigrations: { datatable: string; timestamp: number; name: string }[],
  opts?: { yes?: boolean; jsonOutput?: boolean },
): Promise<void> {
  if (newMigrations.length === 0) return;

  log.info(colors.green("New migrations were pushed:"));
  for (const m of newMigrations) {
    log.info(colors.gray(`  ${m.datatable}: ${m.timestamp} ${m.name}`));
  }

  // Running migrations mutates the data tables, so skip the prompt in
  // non-interactive contexts (--yes, --json, no TTY).
  const interactive = !opts?.jsonOutput && !opts?.yes && !!process.stdin.isTTY;
  if (!interactive) {
    return;
  }

  const shouldRun = await Confirm.prompt({
    message: "New migrations were pushed, run them?",
    default: false,
  });
  if (!shouldRun) {
    return;
  }

  for (const datatable of new Set(newMigrations.map((m) => m.datatable))) {
    await runMigrations(workspace, datatable);
  }
}
