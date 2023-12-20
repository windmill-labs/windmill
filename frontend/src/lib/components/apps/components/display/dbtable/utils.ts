import type { AppInput, RunnableByName } from '../../../inputType'
import { JobService, Preview } from '$lib/gen'

export function makeQuery(
	table: string,
	columns: string[],
	pageSize: number | undefined,
	page: number
) {
	if (!table) throw new Error('Table name is required')

	if (!pageSize) {
		return `SELECT ${columns && columns.length > 0 ? columns.join(', ') : '*'} FROM ${table}`
	}

	return `SELECT ${
		columns && columns.length > 0 ? columns.join(', ') : '*'
	} FROM ${table} LIMIT ${pageSize} OFFSET ${pageSize * page}`
}

export function makeInsertQuery(table: string, values: Record<string, any>) {
	if (!table) throw new Error('Table name is required')
	if (!values || typeof values !== 'object' || Object.keys(values).length === 0) {
		throw new Error('Values must be a non-empty object')
	}

	// Extracting column names and their corresponding values
	const columns = Object.keys(values)
		.map((key) => `\"${key}\"`)
		.join(', ')
	const valuesList = Object.values(values)
		.map((value) => `'${value}'`)
		.join(', ')

	// Constructing the query
	const query = `INSERT INTO ${table} (${columns}) VALUES (${valuesList})`

	return query
}

export function createPostgresInput(
	resource: string,
	table: string | undefined,
	columns: string[],
	pageSize: number | undefined,
	page: number
): AppInput | undefined {
	if (!resource || !table) {
		// Return undefined if resource or table is not defined
		return undefined
	}

	const getRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: makeQuery(table, columns, pageSize, page),
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

export function createUpdatePostgresInput(
	resource: string,
	table: string | undefined,
	column: string,
	value: string,
	primaryKey: string | undefined,
	primaryValue: string | undefined
): AppInput | undefined {
	if (!resource || !table) {
		// Return undefined if resource or table is not defined
		return undefined
	}

	if (!primaryKey || !primaryValue) {
		throw new Error('Primary key and value are required')
	}

	const updateRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: `UPDATE ${table} SET ${column} = '${value}' WHERE ${primaryKey} = ${primaryValue}`,
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

	const updateQuery: AppInput = {
		runnable: updateRunnable,
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

	return updateQuery
}

export type TableMetadata = Array<{
	columnname: string
	datatype: string
	defaultvalue: string
	isprimarykey: boolean
}>

export async function loadTableMetaData(
	resource: string,
	workspace: string | undefined,
	table: string | undefined
): Promise<TableMetadata | undefined> {
	if (!resource || !table || !workspace) {
		return undefined
	}

	const code = `
	SELECT 
	a.attname as ColumnName,
	pg_catalog.format_type(a.atttypid, a.atttypmod) as DataType,
	(SELECT substring(pg_catalog.pg_get_expr(d.adbin, d.adrelid, true) for 128)
	 FROM pg_catalog.pg_attrdef d
	 WHERE d.adrelid = a.attrelid AND d.adnum = a.attnum AND a.atthasdef) as DefaultValue,
	(SELECT CASE WHEN i.indisprimary THEN true ELSE 'NO' END
	 FROM pg_catalog.pg_class tbl, pg_catalog.pg_class idx, pg_catalog.pg_index i, pg_catalog.pg_attribute att
	 WHERE tbl.oid = a.attrelid AND idx.oid = i.indexrelid AND att.attrelid = tbl.oid
				 AND i.indrelid = tbl.oid AND att.attnum = any(i.indkey) AND att.attname = a.attname) as IsPrimaryKey
FROM pg_catalog.pg_attribute a
WHERE a.attrelid = (SELECT oid FROM pg_catalog.pg_class WHERE relname = '${table}') 
		AND a.attnum > 0 AND NOT a.attisdropped
ORDER BY a.attnum;
`

	const maxRetries = 3
	let attempts = 0

	while (attempts < maxRetries) {
		try {
			const job = await JobService.runScriptPreview({
				workspace: workspace,
				requestBody: {
					language: Preview.language.POSTGRESQL,
					content: code,
					args: {
						database: resource
					}
				}
			})

			await new Promise((resolve) => setTimeout(resolve, 1000))

			const testResult = await JobService.getCompletedJob({
				workspace: workspace,
				id: job
			})

			if (testResult.success) {
				attempts = maxRetries

				return testResult.result
			}
		} catch (error) {
			attempts++
		}
		// Exponential back-off
		await new Promise((resolve) => setTimeout(resolve, 2000 * attempts))
	}

	console.error('Failed to load table metadata after maximum retries.')
	return undefined
}

export async function insertRow(
	resource: string,
	workspace: string | undefined,
	table: string | undefined,
	values: Record<string, any>
): Promise<boolean> {
	if (!resource || !table || !workspace) {
		return false
	}

	const code = makeInsertQuery(table, values)

	await JobService.runScriptPreview({
		workspace: workspace,
		requestBody: {
			language: Preview.language.POSTGRESQL,
			content: code,
			args: {
				database: resource
			}
		}
	})

	return false
}
