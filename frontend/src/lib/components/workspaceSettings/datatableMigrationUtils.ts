import { WorkspaceService, type DatatableMigrationWithStatus } from '$lib/gen'
import { migrationScope, scopesOverlap } from '../datatableMigrationSchemas'

/**
 * The pending migrations a new migration declaring `sql`'s scope would run ahead of. A
 * newly-created migration always gets the highest timestamp, so every pending migration is
 * "earlier"; only those whose scope overlaps its own can be ones it depends on.
 */
export function pendingMigrations(
	migrations: DatatableMigrationWithStatus[],
	sql: string
): DatatableMigrationWithStatus[] {
	const scope = migrationScope(sql)
	return migrations.filter((m) => m.status !== 'ran' && scopesOverlap(m.schemas, scope))
}

/** Fetch the data table's migration status and return the pending ones `sql` would run ahead of. */
export async function fetchPendingMigrations(
	workspace: string,
	datatableName: string,
	sql: string
): Promise<DatatableMigrationWithStatus[]> {
	const { migrations } = await WorkspaceService.getDatatableMigrationsStatus({
		workspace,
		datatableName
	})
	return pendingMigrations(migrations, sql)
}

/** Confirmation copy shown before running a just-created migration ahead of
 * `count` still-pending earlier ones (mirrors the row-level Run warning). */
export function outOfOrderRunMessage(count: number): string {
	return `${count} earlier migration(s) touching the same schemas have not been run yet. This migration might depend on them. Run it anyway?`
}
