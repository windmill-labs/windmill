import type { AppInput, RunnableByName } from '$lib/components/apps/inputType'
import type { DbType, DbInput } from '$lib/components/dbTypes'
import { getLanguageByResourceType, type ColumnDef } from '../utils'

export function makeSelectQuery(
	table: string,
	columnDefs: ColumnDef[],
	whereClause: string | undefined,
	dbType: DbType,
	options?: { limit?: number; offset?: number },
	breakingFeatures?: {
		fixPgIntTypes?: boolean
	}
) {
	if (!table) throw new Error('Table name is required')

	const payload: Record<string, unknown> = {
		table,
		columnDefs
	}

	if (whereClause !== undefined) {
		payload.whereClause = whereClause
	}

	if (options?.limit !== undefined) {
		payload.limit = options.limit
	}

	if (options?.offset !== undefined) {
		payload.offset = options.offset
	}

	if (breakingFeatures?.fixPgIntTypes) {
		payload.fixPgIntTypes = true
	}

	return `-- WM_INTERNAL_DB_SELECT ${JSON.stringify(payload)}`
}

export function getSelectInput(
	dbInput: DbInput,
	table: string | undefined,
	columnDefs: ColumnDef[],
	whereClause: string | undefined,
	options?: { limit?: number; offset?: number }
): AppInput | undefined {
	if (
		(dbInput.type == 'ducklake' && !dbInput.ducklake) ||
		(dbInput.type == 'database' && !dbInput.resourcePath) ||
		!table ||
		!columnDefs?.length
	) {
		return undefined
	}

	const dbType = dbInput.type === 'ducklake' ? 'duckdb' : dbInput.resourceType

	const payload: Record<string, unknown> = {
		table,
		columnDefs
	}

	if (whereClause !== undefined) {
		payload.whereClause = whereClause
	}

	if (dbInput.type === 'ducklake') {
		payload.ducklake = dbInput.ducklake
	}

	const content = `-- WM_INTERNAL_DB_SELECT ${JSON.stringify(payload)}`

	const getRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'inline',
		inlineScript: { content, language: getLanguageByResourceType(dbType) }
	}

	const getQuery: AppInput = {
		runnable: getRunnable,
		fields:
			dbInput.type === 'database'
				? {
						database: {
							type: 'static',
							value: dbInput.resourcePath.startsWith('datatable://')
								? dbInput.resourcePath
								: `$res:${dbInput.resourcePath}`,
							fieldType: 'object',
							format: `resource-${dbType}`
						}
					}
				: {},
		type: 'runnable',
		fieldType: 'object',
		hideRefreshButton: true
	}

	return getQuery
}
