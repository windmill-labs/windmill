import {
	getLanguageByResourceType,
	type ColumnDef,
	type TableMetadata
} from './apps/components/display/dbtable/utils'
import { runScriptAndPollResult } from './jobs/utils'
import type { DBSchema, SQLSchema } from '$lib/stores'
import { stringifySchema } from './copilot/lib'
import type { DbInput, DbType } from './dbTypes'
import { assert } from '$lib/utils'
import {
	buildTableEditorValues,
	type TableEditorValues
} from './apps/components/display/dbtable/tableEditor'
import {
	makeAlterTableQueries,
	type AlterTableValues
} from './apps/components/display/dbtable/queries/alterTable'
import { makeCreateTableQuery } from './apps/components/display/dbtable/queries/createTable'
import {
	transformForeignKeys,
	transformSnowflakeForeignKeys,
	type RawForeignKey
} from './apps/components/display/dbtable/queries/relationalKeys'

export type IDbTableOps = {
	dbType: DbType
	tableKey: string
	colDefs: ColumnDef[]

	getRows: (params: {
		offset: number
		limit: number
		quicksearch: string
		order_by: string
		is_desc: boolean
	}) => Promise<unknown[]>
	getCount: (params: { quicksearch: string }) => Promise<number>
	onUpdate?: (
		row: { values: object },
		colDef: { field: string; datatype: string },
		newValue: string
	) => Promise<void>
	onDelete?: (row: { values: object }) => Promise<void>
	onInsert?: (row: { values: object }) => Promise<void>
}

export function dbTableOpsWithPreviewScripts({
	input,
	tableKey,
	colDefs,
	workspace
}: {
	input: DbInput
	tableKey: string
	colDefs: ColumnDef[]
	workspace: string
}): IDbTableOps {
	const dbType = getDbType(input)
	const language = getLanguageByResourceType(dbType)
	const dbArg = getDatabaseArg(input)
	const ducklake = input.type === 'ducklake' ? input.ducklake : undefined

	function makeMarker(op: string, payload: Record<string, unknown>): string {
		if (ducklake) payload.ducklake = ducklake
		return `-- WM_INTERNAL_DB_${op} ${JSON.stringify(payload)}`
	}

	return {
		dbType,
		tableKey,
		colDefs,
		getCount: async ({ quicksearch }) => {
			const content = makeMarker('COUNT', { table: tableKey, columnDefs: colDefs })
			const result = await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, quicksearch }, language, content }
			})
			const count = result?.[0].count as number
			return count
		},
		getRows: async (params) => {
			const content = makeMarker('SELECT', {
				table: tableKey,
				columnDefs: colDefs,
				fixPgIntTypes: true
			})
			let items = (await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, ...params }, language, content }
			})) as unknown[]
			if (!items || !Array.isArray(items)) {
				throw 'items is not an array'
			}
			return items
		},
		onUpdate: async ({ values }, colDef, newValue) => {
			const content = makeMarker('UPDATE', {
				table: tableKey,
				column: colDef,
				columns: colDefs
			})
			await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: { ...dbArg, value_to_update: newValue, ...values },
					language,
					content
				}
			})
		},
		onDelete: async ({ values }) => {
			const content = makeMarker('DELETE', { table: tableKey, columns: colDefs })
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, ...values }, language, content }
			})
		},
		onInsert: async ({ values }) => {
			const content = makeMarker('INSERT', { table: tableKey, columns: colDefs })
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, ...values }, language, content }
			})
		}
	}
}

export type IDbSchemaOps = {
	onDelete: (params: { tableKey: string; schema?: string }) => Promise<void>
	onCreate: (params: { values: TableEditorValues; schema?: string }) => Promise<void>
	previewCreateSql: (params: { values: TableEditorValues; schema?: string }) => string
	onAlter: (params: { values: AlterTableValues; schema?: string }) => Promise<void>
	previewAlterSql: (params: { values: AlterTableValues; schema?: string }) => string[]
	onCreateSchema: (params: { schema: string }) => Promise<void>
	onDeleteSchema: (params: { schema: string }) => Promise<void>
	onFetchTableEditorDefinition: (params: {
		table: string
		schema?: string
		colDefs: TableMetadata
	}) => Promise<TableEditorValues>
}

export function dbSchemaOpsWithPreviewScripts({
	workspace,
	input
}: {
	workspace: string
	input: DbInput
}): IDbSchemaOps {
	const dbType = getDbType(input)
	const dbArg = getDatabaseArg(input)
	const language = getLanguageByResourceType(dbType)
	const ducklake = input.type === 'ducklake' ? input.ducklake : undefined

	function makeMarker(op: string, payload: Record<string, unknown>): string {
		if (ducklake) payload.ducklake = ducklake
		return `-- WM_INTERNAL_DB_${op} ${JSON.stringify(payload)}`
	}

	return {
		onDelete: async ({ tableKey, schema }) => {
			const content = makeMarker('DROP_TABLE', { table: tableKey, schema })
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg }, language, content }
			})
		},
		onCreate: async ({ values, schema }) => {
			const content = makeMarker('CREATE_TABLE', {
				name: values.name,
				columns: values.columns,
				foreignKeys: values.foreignKeys,
				schema
			})
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: dbArg, content, language }
			})
		},
		previewCreateSql: ({ values, schema }) => makeCreateTableQuery(values, dbType, schema),
		onAlter: async ({ values, schema }) => {
			const content = makeMarker('ALTER_TABLE', {
				name: values.name,
				operations: values.operations,
				schema
			})
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: dbArg, content, language }
			})
		},
		previewAlterSql: ({ values, schema }) => makeAlterTableQueries(values, dbType, schema),
		onCreateSchema: async ({ schema }) => {
			const content = makeMarker('CREATE_SCHEMA', { schema })
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg }, language, content }
			})
		},
		onDeleteSchema: async ({ schema }) => {
			const content = makeMarker('DROP_SCHEMA', { schema })
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg }, language, content }
			})
		},
		onFetchTableEditorDefinition: async ({ table, schema, colDefs }) => {
			let foreignKeys: import('./apps/components/display/dbtable/tableEditor').TableEditorForeignKey[] =
				[]
			let pk_constraint_name: string | undefined

			// Fetch foreign keys (not supported for BigQuery)
			if (dbType !== 'bigquery') {
				try {
					const fkContent = makeMarker('FOREIGN_KEYS', { table, schema })
					const fkResult = await runScriptAndPollResult({
						workspace,
						requestBody: { args: dbArg, content: fkContent, language }
					})

					let rawForeignKeys: RawForeignKey[]
					if (dbType === 'snowflake') {
						rawForeignKeys = transformSnowflakeForeignKeys(fkResult as any[])
					} else {
						rawForeignKeys = fkResult as RawForeignKey[]
						if (rawForeignKeys && Array.isArray(rawForeignKeys)) {
							rawForeignKeys = rawForeignKeys.map((fk) => {
								const lowerFk: any = {}
								Object.keys(fk).forEach((key) => {
									lowerFk[key.toLowerCase()] = fk[key]
								})
								return lowerFk
							})
						}
					}

					if (rawForeignKeys && Array.isArray(rawForeignKeys)) {
						foreignKeys = transformForeignKeys(rawForeignKeys)
					}
				} catch (e) {
					console.warn('Failed to fetch foreign keys:', e)
				}
			}

			// Fetch primary key constraint name (not supported for BigQuery/MySQL)
			if (dbType !== 'bigquery' && dbType !== 'mysql') {
				try {
					const pkContent = makeMarker('PRIMARY_KEY_CONSTRAINT', { table, schema })
					const pkResult = (await runScriptAndPollResult({
						workspace,
						requestBody: { args: dbArg, content: pkContent, language }
					})) as { constraint_name?: string; CONSTRAINT_NAME?: string }[]

					if (pkResult && Array.isArray(pkResult) && pkResult.length > 0) {
						const pkRecord: any = pkResult[0]
						pk_constraint_name = pkRecord?.constraint_name || pkRecord?.CONSTRAINT_NAME || ''
					}
				} catch (e) {
					console.warn('Failed to fetch primary key constraint:', e)
				}
			}

			return buildTableEditorValues({
				tableName: table,
				metadata: colDefs,
				foreignKeys,
				pk_constraint_name
			})
		}
	}
}

export async function getDucklakeSchema({
	workspace,
	ducklake
}: {
	workspace: string
	ducklake: string
}): Promise<DBSchema> {
	let result = await runScriptAndPollResult({
		workspace,
		requestBody: {
			language: 'duckdb',
			content: `ATTACH 'ducklake://${ducklake}' AS __ducklake__; ${DUCKLAKE_GET_SCHEMA_QUERY}`,
			args: {}
		}
	})
	let mainSchema = Array.isArray(result) && result.length && (result?.[0]?.['result'] ?? [])
	// Safety for agent workers (duckdb ffi lib used to return JSON as stringified json)
	if (typeof mainSchema === 'string') mainSchema = JSON.parse(mainSchema)

	if (!mainSchema) throw new Error('Failed to get Ducklake schema: ' + JSON.stringify(result))
	assert('mainSchema is an object', typeof mainSchema === 'object')
	let schema: Omit<SQLSchema, 'stringified'> = {
		schema: { main: mainSchema },
		publicOnly: true,
		lang: 'ducklake'
	}
	return { ...schema, stringified: stringifySchema(schema) }
}

const DUCKLAKE_GET_SCHEMA_QUERY = `
SELECT json_group_object(table_name, table_data) AS result FROM (
	SELECT
		table_name,
		json_group_object(
			c.column_name,
			json_object(
				'type', c.data_type,
				'default', c.column_default,
				'required', c.is_nullable == 'NO'
			)
		) AS table_data
	FROM information_schema.columns c
	WHERE table_catalog = '__ducklake__' AND table_schema = current_schema()
	GROUP BY c.table_name
)`

export function getDbType(input: DbInput): DbType {
	switch (input.type) {
		case 'database':
			return input.resourceType
		case 'ducklake':
			return 'duckdb'
	}
}

export function getDatabaseArg(input: DbInput | undefined) {
	if (input?.type === 'database') {
		if (input.resourcePath.startsWith('datatable://')) {
			return { database: input.resourcePath }
		} else {
			return { database: '$res:' + input.resourcePath }
		}
	}
	return {}
}
