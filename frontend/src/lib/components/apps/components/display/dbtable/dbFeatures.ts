// Separate file to import in both frontend and e2e

import type { DbInput } from '../../../../dbTypes'
import { dbSupportsSchemas } from './utils'

export type DbFeatures = {
	foreignKeys?: boolean
	enforcedForeignKeys?: boolean
	primaryKeys?: boolean
	defaultValues?: boolean
	schemas?: boolean
}

export function getDbFeatures(dbInput: DbInput): Required<DbFeatures> {
	const def: Required<DbFeatures> = {
		foreignKeys: true,
		primaryKeys: true,
		defaultValues: true,
		enforcedForeignKeys: true,
		schemas: dbInput.type !== 'ducklake' && dbSupportsSchemas(dbInput.resourceType)
	}

	if (dbInput.type == 'ducklake')
		return {
			...def,
			foreignKeys: false,
			enforcedForeignKeys: false,
			primaryKeys: false
		}

	if (dbInput.resourceType == 'bigquery')
		return {
			...def,
			foreignKeys: false,
			primaryKeys: true,
			defaultValues: false,
			enforcedForeignKeys: false
		}

	if (dbInput.resourceType == 'snowflake')
		return {
			...def,
			enforcedForeignKeys: false
		}

	return { ...def }
}
