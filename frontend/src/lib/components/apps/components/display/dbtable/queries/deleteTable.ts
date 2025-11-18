import type { DbType } from '$lib/components/dbTypes'

export function makeDeleteTableQuery(tableKey: string, resourceType: DbType): string {
	// same for all sql dbs
	return `DROP TABLE ${tableKey};`
}
