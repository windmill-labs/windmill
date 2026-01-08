// Separate file to import in both frontend and e2e

import type { DbInput } from '../../../../dbTypes'
import { dbSupportsSchemas } from './utils'

export type DbFeatures = {
	foreignKeys?: boolean
	primaryKeys?: boolean
	defaultValues?: boolean
	defaultToNotNull?: boolean
	schemas?: boolean
}

export function getDbFeatures(dbInput: DbInput): Required<DbFeatures> {
	const def: Required<DbFeatures> = {
		foreignKeys: true,
		primaryKeys: true,
		defaultValues: true,
		defaultToNotNull: true,
		schemas: dbInput.type !== 'ducklake' && dbSupportsSchemas(dbInput.resourceType)
	}

	if (dbInput.type == 'ducklake') return { ...def, foreignKeys: false, primaryKeys: false }

	if (dbInput.resourceType == 'bigquery')
		return {
			...def,
			foreignKeys: false,
			primaryKeys: true,
			defaultValues: false,
			defaultToNotNull: false
		}

	return { ...def }
}
