// Sync support for datatable migrations.
//
// Datatable migrations are versioned SQL scripts stored per datatable on the
// backend and synced to git as plain .sql files under:
//
//   _datatable_migrations/{datatable}/{version}_{name}.sql
//
// where {version} is a sortable numeric timestamp and {name} is an optional
// human-readable label. On push, files are upserted by version; after a push
// that touches migrations, pending migrations are applied to the datatable's
// backing database (so merging new migration files to a git-synced branch
// runs them on the target workspace).

import { sep as SEP } from "node:path";
import * as wmill from "../../../gen/services.gen.ts";
import * as log from "../../core/log.ts";

export const DATATABLE_MIGRATIONS_FOLDER = "_datatable_migrations";

export function isDatatableMigrationPath(p: string): boolean {
  return (
    (p.startsWith(DATATABLE_MIGRATIONS_FOLDER + SEP) ||
      p.startsWith(DATATABLE_MIGRATIONS_FOLDER + "/")) &&
    p.endsWith(".sql")
  );
}

export function parseDatatableMigrationPath(
  p: string,
): { datatable: string; version: string; name: string } | undefined {
  const parts = p.replaceAll(SEP, "/").split("/");
  if (parts.length !== 3 || parts[0] !== DATATABLE_MIGRATIONS_FOLDER) {
    return undefined;
  }
  const datatable = parts[1];
  const m = parts[2].match(/^(\d+)(?:_(.+))?\.sql$/);
  if (!datatable || !m) {
    return undefined;
  }
  return { datatable, version: m[1], name: m[2] ?? "" };
}

export async function pushDatatableMigration(
  workspace: string,
  p: string,
  content: string,
): Promise<void> {
  const parsed = parseDatatableMigrationPath(p);
  if (!parsed) {
    throw new Error(
      `Invalid datatable migration path '${p}'. Expected ${DATATABLE_MIGRATIONS_FOLDER}/{datatable}/{version}_{name}.sql`,
    );
  }
  await wmill.upsertDatatableMigration({
    workspace,
    datatable: parsed.datatable,
    requestBody: {
      version: parsed.version,
      name: parsed.name,
      content,
    },
  });
}

export async function deleteDatatableMigration(
  workspace: string,
  p: string,
): Promise<void> {
  const parsed = parseDatatableMigrationPath(p);
  if (!parsed) {
    throw new Error(`Invalid datatable migration path '${p}'`);
  }
  await wmill.deleteDatatableMigration({
    workspace,
    datatable: parsed.datatable,
    requestBody: { version: parsed.version },
  });
}

/// Apply pending migrations for each datatable, logging the outcome.
/// Warns about drifted migrations (edited after having been applied), since
/// the edited SQL will NOT be re-run. Failures are reported per datatable
/// without aborting the sync.
export async function runPendingDatatableMigrations(
  workspace: string,
  datatables: Iterable<string>,
): Promise<void> {
  for (const datatable of datatables) {
    try {
      const { applied } = await wmill.runDatatableMigrations({
        workspace,
        datatable,
      });
      if (applied.length > 0) {
        log.info(
          `Applied ${applied.length} migration${
            applied.length > 1 ? "s" : ""
          } to datatable ${datatable}: ${applied.join(", ")}`,
        );
      } else {
        log.info(`Datatable ${datatable}: no pending migrations`);
      }
    } catch (e) {
      log.error(
        `Failed to run migrations for datatable ${datatable}: ${e}`,
      );
    }
    try {
      const { migrations } = await wmill.listDatatableMigrations({
        workspace,
        datatable,
      });
      for (const m of migrations) {
        if (m.drifted) {
          log.warn(
            `Datatable ${datatable}: migration ${m.version}${
              m.name ? ` (${m.name})` : ""
            } was edited after having been applied — the new content will NOT be re-run. Create a new migration instead.`,
          );
        }
      }
    } catch {
      // drift check is best-effort
    }
  }
}
