import { getLanguageByResourceType, type ColumnDef } from './apps/components/display/dbtable/utils'
import { makeSelectQuery } from './apps/components/display/dbtable/queries/select'
import { runScriptAndPollResult } from './jobs/utils'
import { makeCountQuery } from './apps/components/display/dbtable/queries/count'
import { makeUpdateQuery } from './apps/components/display/dbtable/queries/update'
import { makeDeleteQuery } from './apps/components/display/dbtable/queries/delete'
import { makeInsertQuery } from './apps/components/display/dbtable/queries/insert'
import { makeDeleteTableQuery } from './apps/components/display/dbtable/queries/deleteTable'
import type { DBSchema, SQLSchema } from '$lib/stores'
import { stringifySchema } from './copilot/lib'
import type { DbInput, DbType } from './dbTypes'
import { wrapDucklakeQuery } from './ducklake'
import { assert } from '$lib/utils'
import {
	makeCreateTableQuery,
	type CreateTableValues
} from './apps/components/display/dbtable/queries/createTable'

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
	return {
		dbType,
		tableKey,
		colDefs,
		getCount: async ({ quicksearch }) => {
			let countQuery = makeCountQuery(dbType, tableKey, undefined, colDefs)
			if (input.type === 'ducklake') countQuery = wrapDucklakeQuery(countQuery, input.ducklake)
			const result = await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, quicksearch }, language, content: countQuery }
			})
			const count = result?.[0].count as number
			return count
		},
		getRows: async (params) => {
			let query = makeSelectQuery(tableKey, colDefs, undefined, dbType)
			if (input.type === 'ducklake') query = wrapDucklakeQuery(query, input.ducklake)
			let items = (await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, ...params }, language, content: query }
			})) as unknown[]
			if (input.type === 'database' && input.resourceType === 'ms_sql_server')
				items = items?.[0] as unknown[]
			if (!items || !Array.isArray(items)) {
				throw 'items is not an array'
			}
			return items
		},
		onUpdate: async ({ values }, colDef, newValue) => {
			let updateQuery = makeUpdateQuery(tableKey, colDef, colDefs, dbType)
			if (input.type === 'ducklake') updateQuery = wrapDucklakeQuery(updateQuery, input.ducklake)
			await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: { ...dbArg, value_to_update: newValue, ...values },
					language,
					content: updateQuery
				}
			})
		},
		onDelete: async ({ values }) => {
			let deleteQuery = makeDeleteQuery(tableKey, colDefs, dbType)
			if (input.type === 'ducklake') deleteQuery = wrapDucklakeQuery(deleteQuery, input.ducklake)
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, ...values }, language, content: deleteQuery }
			})
		},
		onInsert: async ({ values }) => {
			let insertQuery = makeInsertQuery(tableKey, colDefs, dbType)
			if (input.type === 'ducklake') insertQuery = wrapDucklakeQuery(insertQuery, input.ducklake)
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg, ...values }, language, content: insertQuery }
			})
		}
	}
}

export type IDbSchemaOps = {
	onDelete: (params: { tableKey: string; schema?: string }) => Promise<void>
	onCreate: (params: { values: CreateTableValues; schema?: string }) => Promise<void>
	previewCreateSql: (params: { values: CreateTableValues; schema?: string }) => string
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
	return {
		onDelete: async ({ tableKey, schema }) => {
			let deleteQuery = makeDeleteTableQuery(tableKey, dbType, schema)
			if (input.type === 'ducklake') deleteQuery = wrapDucklakeQuery(deleteQuery, input.ducklake)
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: { ...dbArg }, language, content: deleteQuery }
			})
		},
		onCreate: async ({ values, schema }) => {
			let query = makeCreateTableQuery(values, dbType, schema)
			if (input?.type === 'ducklake') query = wrapDucklakeQuery(query, input.ducklake)
			await runScriptAndPollResult({
				workspace,
				requestBody: { args: dbArg, content: query, language }
			})
		},
		previewCreateSql: ({ values, schema }) => makeCreateTableQuery(values, dbType, schema)
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
			return { database: 'datatable://' + input.resourcePath }
		} else {
			return { database: '$res:' + input.resourcePath }
		}
	}
	return {}
}
