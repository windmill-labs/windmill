import type { DbType } from '../utils'

export function makeDeleteTableQuery(tableKey: string, resourceType: DbType): string {
	// same for all sql dbs
	return `DROP TABLE ${tableKey};`
}
