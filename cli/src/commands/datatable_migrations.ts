import * as fs from "node:fs";
import * as path from "node:path";
import * as log from "../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as wmill from "../../gen/services.gen.ts";
import type { DatatableMigration } from "../../gen/types.gen.ts";
import { readTextFile, readTextFileSync } from "../utils/utils.ts";
import { Confirm } from "@cliffy/prompt/confirm";

// Migrations live under <cwd>/datatable_migrations/<datatable>/, one folder per
// target data table, as `<timestamp>_<name>.up.sql` (and optional `.down.sql`).
const MIGRATIONS_DIR = "datatable_migrations";

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
 * Scaffold a new migration under datatable_migrations/<datatable>/ as empty
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
  fs.writeFileSync(up, `-- up migration: ${name}\n`, "utf-8");
  fs.writeFileSync(down, `-- down migration: ${name}\n`, "utf-8");

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

function upFileName(m: { timestamp: number; name: string }): string {
  return `${m.timestamp}_${m.name}.up.sql`;
}

function downFileName(m: { timestamp: number; name: string }): string {
  return `${m.timestamp}_${m.name}.down.sql`;
}

/**
 * Parse a `<timestamp>_<name>.up.sql` / `.down.sql` file name into its
 * `{ timestamp, name, kind }` parts. Returns undefined for unrelated files.
 */
function parseMigrationFileName(
  file: string,
): { timestamp: number; name: string; kind: "up" | "down" } | undefined {
  const m = file.match(/^(\d+)_(.*)\.(up|down)\.sql$/);
  if (!m) return undefined;
  return { timestamp: Number(m[1]), name: m[2], kind: m[3] as "up" | "down" };
}

function sortMigrations(migrations: DatatableMigration[]): DatatableMigration[] {
  return migrations.sort(
    (a, b) =>
      a.datatable.localeCompare(b.datatable) || a.timestamp - b.timestamp,
  );
}

/**
 * Push the local <cwd>/datatable_migrations/ folder to the workspace, replacing
 * the remote set. Returns true if a push was performed, false if the folder is
 * missing or already up to date.
 */
export async function pushDatatableMigrations(
  workspace: string,
  opts?: { yes?: boolean; jsonOutput?: boolean },
): Promise<boolean> {
  const localDir = path.join(process.cwd(), MIGRATIONS_DIR);
  if (!fs.existsSync(localDir)) {
    return false;
  }

  // Each immediate subfolder is a target data table; read its migration files.
  const migrations: DatatableMigration[] = [];
  for (const datatable of fs.readdirSync(localDir)) {
    const dtDir = path.join(localDir, datatable);
    if (!fs.statSync(dtDir).isDirectory()) continue;

    // Group .up.sql / .down.sql files by their (timestamp, name) key.
    const byKey = new Map<
      string,
      { timestamp: number; name: string; code_up?: string; code_down?: string }
    >();
    for (const file of fs.readdirSync(dtDir)) {
      const parsed = parseMigrationFileName(file);
      if (!parsed) continue;
      const key = `${parsed.timestamp}_${parsed.name}`;
      const entry =
        byKey.get(key) ?? { timestamp: parsed.timestamp, name: parsed.name };
      const content = await readTextFile(path.join(dtDir, file));
      if (parsed.kind === "up") entry.code_up = content;
      else entry.code_down = content;
      byKey.set(key, entry);
    }

    for (const entry of byKey.values()) {
      if (entry.code_up === undefined) {
        log.warn(
          colors.yellow(
            `Skipping ${datatable}/${entry.timestamp}_${entry.name}: missing .up.sql file`,
          ),
        );
        continue;
      }
      migrations.push({
        datatable,
        timestamp: entry.timestamp,
        name: entry.name,
        code_up: entry.code_up,
        ...(entry.code_down !== undefined ? { code_down: entry.code_down } : {}),
      });
    }
  }
  sortMigrations(migrations);

  // Skip if the remote set is already identical.
  let remote: DatatableMigration[] = [];
  try {
    remote = await wmill.listDatatableMigrations({ workspace });
  } catch {
    // If the endpoint is missing or unauthorized, just attempt the push.
  }
  if (JSON.stringify(remote) === JSON.stringify(migrations)) {
    log.info(colors.gray("Datatable migrations up to date"));
    return false;
  }

  await wmill.updateDatatableMigrations({
    workspace,
    requestBody: { migrations },
  });
  log.info(
    colors.green(`Pushed ${migrations.length} datatable migration(s)`),
  );

  // Migrations newly introduced by this push (absent from the remote set before),
  // keyed by (datatable, timestamp) since timestamps are unique only per table.
  const remoteKeys = new Set(remote.map((m) => `${m.datatable}\0${m.timestamp}`));
  const newMigrations = migrations.filter(
    (m) => !remoteKeys.has(`${m.datatable}\0${m.timestamp}`),
  );
  if (newMigrations.length > 0) {
    await offerToRunNewMigrations(workspace, newMigrations, opts);
  }

  return true;
}

/**
 * After a push that introduced new migrations, list them and (interactively)
 * offer to run them, equivalent to `wmill datatable migrate up` on each affected
 * data table.
 */
async function offerToRunNewMigrations(
  workspace: string,
  newMigrations: DatatableMigration[],
  opts?: { yes?: boolean; jsonOutput?: boolean },
): Promise<void> {
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

/**
 * Pull the workspace's datatable migrations into
 * <cwd>/datatable_migrations/<datatable>/ as `<timestamp>_<name>.up.sql` (and
 * `.down.sql` when a down migration exists). Files no longer present remotely
 * are removed locally.
 */
export async function pullDatatableMigrations(
  workspace: string,
): Promise<boolean> {
  const localDir = path.join(process.cwd(), MIGRATIONS_DIR);
  let migrations: DatatableMigration[];
  try {
    migrations = await wmill.listDatatableMigrations({ workspace });
  } catch (e) {
    log.debug(`Skipping datatable migrations pull: ${e}`);
    return false;
  }

  if (migrations.length === 0 && !fs.existsSync(localDir)) {
    return false;
  }

  // Relative paths (`<datatable>/<file>`) that should exist after the pull.
  const known = new Set<string>();
  for (const m of migrations) {
    const dtDir = path.join(localDir, m.datatable);
    fs.mkdirSync(dtDir, { recursive: true });
    const up = upFileName(m);
    known.add(`${m.datatable}/${up}`);
    writeIfChanged(path.join(dtDir, up), m.code_up);
    if (m.code_down !== undefined && m.code_down !== null) {
      const down = downFileName(m);
      known.add(`${m.datatable}/${down}`);
      writeIfChanged(path.join(dtDir, down), m.code_down);
    }
  }

  // Delete locally-orphaned migration files across all datatable subfolders.
  if (fs.existsSync(localDir)) {
    for (const datatable of fs.readdirSync(localDir)) {
      const dtDir = path.join(localDir, datatable);
      if (!fs.statSync(dtDir).isDirectory()) continue;
      for (const file of fs.readdirSync(dtDir)) {
        if (!parseMigrationFileName(file)) continue;
        if (!known.has(`${datatable}/${file}`)) {
          try {
            fs.unlinkSync(path.join(dtDir, file));
          } catch {
            // ignore
          }
        }
      }
    }
  }

  log.info(colors.green(`Pulled ${migrations.length} datatable migration(s)`));
  return true;
}

function writeIfChanged(full: string, content: string): void {
  let existing: string | undefined;
  try {
    existing = readTextFileSync(full);
  } catch {
    existing = undefined;
  }
  if (existing !== content) {
    fs.writeFileSync(full, content, "utf-8");
  }
}
