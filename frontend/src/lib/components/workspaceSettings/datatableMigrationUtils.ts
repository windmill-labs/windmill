import { WorkspaceService, type DatatableMigrationWithStatus } from '$lib/gen'

/**
 * Migrations that are defined but not yet applied. A newly-created migration
 * always gets the highest timestamp, so every pending migration is "earlier":
 * running the new one on its own would apply it ahead of them (out of order).
 */
export function pendingMigrations(
	migrations: DatatableMigrationWithStatus[]
): DatatableMigrationWithStatus[] {
	return migrations.filter((m) => m.status !== 'ran')
}

/** Fetch the data table's migration status and return the pending ones. */
export async function fetchPendingMigrations(
	workspace: string,
	datatableName: string
): Promise<DatatableMigrationWithStatus[]> {
	const { migrations } = await WorkspaceService.getDatatableMigrationsStatus({
		workspace,
		datatableName
	})
	return pendingMigrations(migrations)
}

/** Confirmation copy shown before running a just-created migration ahead of
 * `count` still-pending earlier ones (mirrors the row-level Run warning). */
export function outOfOrderRunMessage(count: number): string {
	return `${count} earlier migration(s) have not been run yet. This migration might depend on them. Run it anyway?`
}
