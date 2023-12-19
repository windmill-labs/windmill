import type { AppInput, RunnableByName } from '../../../inputType'
import { Preview } from '$lib/gen'

export function makeQuery(
	table: string,
	columns: string[],
	pageSize: number | undefined,
	page: number
) {
	if (!table) throw new Error('Table name is required')

	if (!pageSize) {
		return `SELECT ${columns ? columns.join(', ') : '*'} FROM ${table}`
	}

	return `SELECT ${columns ? columns.join(', ') : '*'} FROM ${table} LIMIT ${pageSize} OFFSET ${
		pageSize * page
	}`
}

export function makeInsertQuery(table: string, values: string[]) {
	if (!table) throw new Error('Table name is required')

	return `INSERT INTO ${table} VALUES (${values.join(', ')})`
}

export function createPostgresInput(
	resource: string,
	table: string | undefined,
	columns: string[],
	pageSize: number | undefined
): AppInput | undefined {
	if (!resource || !table) {
		// Return undefined if resource or table is not defined
		return undefined
	}

	const getRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: makeQuery(table, columns, pageSize, 1),
			language: Preview.language.POSTGRESQL,
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {
					database: {
						description: 'Database name',
						type: 'object',
						format: 'resource-postgresql'
					}
				},
				required: ['database'],
				type: 'object'
			}
		}
	}

	const getQuery: AppInput = {
		runnable: getRunnable,
		fields: {
			database: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: 'resource-postgresql'
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return getQuery
}
