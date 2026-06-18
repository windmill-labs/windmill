import type { DbType } from '$lib/components/dbTypes'

export function makeDeleteTableQuery(
	tableKey: string,
	resourceType: DbType,
	schema?: string
): string {
	if (schema?.length) tableKey = `${schema}.${tableKey}`
	// same for all sql dbs
	return `DROP TABLE ${tableKey};`
}
