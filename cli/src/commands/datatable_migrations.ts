import * as fs from "node:fs";
import * as path from "node:path";
import * as log from "../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as wmill from "../../gen/services.gen.ts";
import type { DatatableMigration } from "../../gen/types.gen.ts";
import { readTextFile, readTextFileSync } from "../utils/utils.ts";

const MIGRATIONS_DIR = "datatable_migrations";

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

/**
 * Push the local <cwd>/datatable_migrations/ folder to the workspace, replacing
 * the remote set. Returns true if a push was performed, false if the folder is
 * missing or already up to date.
 */
export async function pushDatatableMigrations(
  workspace: string,
): Promise<boolean> {
  const localDir = path.join(process.cwd(), MIGRATIONS_DIR);
  if (!fs.existsSync(localDir)) {
    return false;
  }

  // Group .up.sql / .down.sql files by their (timestamp, name) key.
  const byKey = new Map<
    string,
    { timestamp: number; name: string; code_up?: string; code_down?: string }
  >();
  for (const file of fs.readdirSync(localDir)) {
    const parsed = parseMigrationFileName(file);
    if (!parsed) continue;
    const key = `${parsed.timestamp}_${parsed.name}`;
    const entry =
      byKey.get(key) ?? { timestamp: parsed.timestamp, name: parsed.name };
    const content = await readTextFile(path.join(localDir, file));
    if (parsed.kind === "up") entry.code_up = content;
    else entry.code_down = content;
    byKey.set(key, entry);
  }

  const migrations: DatatableMigration[] = [];
  for (const entry of byKey.values()) {
    if (entry.code_up === undefined) {
      log.warn(
        colors.yellow(
          `Skipping migration ${entry.timestamp}_${entry.name}: missing .up.sql file`,
        ),
      );
      continue;
    }
    migrations.push({
      timestamp: entry.timestamp,
      name: entry.name,
      code_up: entry.code_up,
      ...(entry.code_down !== undefined ? { code_down: entry.code_down } : {}),
    });
  }
  migrations.sort((a, b) => a.timestamp - b.timestamp);

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
    colors.green(
      `Pushed ${migrations.length} datatable migration(s)`,
    ),
  );
  return true;
}

/**
 * Pull the workspace's datatable migrations into <cwd>/datatable_migrations/ as
 * `<timestamp>_<name>.up.sql` (and `.down.sql` when a down migration exists).
 * Files no longer present remotely are removed locally.
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
  fs.mkdirSync(localDir, { recursive: true });

  const known = new Set<string>();
  for (const m of migrations) {
    const up = upFileName(m);
    known.add(up);
    writeIfChanged(path.join(localDir, up), m.code_up);
    if (m.code_down !== undefined && m.code_down !== null) {
      const down = downFileName(m);
      known.add(down);
      writeIfChanged(path.join(localDir, down), m.code_down);
    }
  }

  // Delete locally-orphaned migration files.
  for (const file of fs.readdirSync(localDir)) {
    if (!parseMigrationFileName(file)) continue;
    if (!known.has(file)) {
      try {
        fs.unlinkSync(path.join(localDir, file));
      } catch {
        // ignore
      }
    }
  }

  log.info(
    colors.green(`Pulled ${migrations.length} datatable migration(s)`),
  );
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
