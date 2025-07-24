import {
	getLanguageByResourceType,
	type ColumnDef,
	type DbType
} from './apps/components/display/dbtable/utils'
import { makeSelectQuery } from './apps/components/display/dbtable/queries/select'
import { runScriptAndPollResult } from './jobs/utils'
import { makeCountQuery } from './apps/components/display/dbtable/queries/count'
import { makeUpdateQuery } from './apps/components/display/dbtable/queries/update'
import { makeDeleteQuery } from './apps/components/display/dbtable/queries/delete'
import { makeInsertQuery } from './apps/components/display/dbtable/queries/insert'
import { Trash2 } from 'lucide-svelte'
import { makeDeleteTableQuery } from './apps/components/display/dbtable/queries/deleteTable'
import type { DBSchema, SQLSchema } from '$lib/stores'
import { stringifySchema } from './copilot/lib'

export type DbInput =
	| {
			type: 'database'
			resourceType: DbType
			resourcePath: string
	  }
	| { type: 'ducklake'; ducklake: string }

export type IDbTableOps = {
	resourcePath: string
	resourceType: DbType
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
	return {
		resourcePath: input.resourcePath,
		resourceType: input.resourceType,
		tableKey,
		colDefs,
		getCount: async ({ quicksearch }) => {
			let countQuery = makeCountQuery(dbType, tableKey, undefined, colDefs)
			if (input.type === 'ducklake') countQuery = wrapDucklakeQuery(countQuery, input.ducklake)
			console.log('countQuery', countQuery)
			const result = await runScriptAndPollResult({
				workspace,
				requestBody: {
					args:
						input.type === 'database'
							? { database: '$res:' + input.resourcePath, quicksearch }
							: { quicksearch },
					language,
					content: countQuery
				}
			})
			const count = result?.[0].count as number
			return count
		},
		getRows: async (params) => {
			let query = makeSelectQuery(tableKey, colDefs, undefined, dbType)
			if (input.type === 'ducklake') query = wrapDucklakeQuery(query, input.ducklake)
			let items = (await runScriptAndPollResult({
				workspace,
				requestBody: {
					args:
						input.type === 'database'
							? { database: '$res:' + input.resourcePath, ...params }
							: params,
					language,
					content: query
				}
			})) as unknown[]
			if (input.type === 'database' && input.resourceType === 'ms_sql_server')
				items = items?.[0] as unknown[]
			if (!items || !Array.isArray(items)) {
				throw 'items is not an array'
			}
			return items
		},
		onUpdate: async ({ values }, colDef, newValue) => {
			const updateQuery = makeUpdateQuery(tableKey, colDef, colDefs, input.resourceType)

			await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: {
						database: '$res:' + input.resourcePath,
						value_to_update: newValue,
						...values
					},
					language,
					content: updateQuery
				}
			})
		},
		onDelete: async ({ values }) => {
			const deleteQuery = makeDeleteQuery(tableKey, colDefs, input.resourceType)

			await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: { database: '$res:' + input.resourcePath, ...values },
					language,
					content: deleteQuery
				}
			})
		},
		onInsert: async ({ values }) => {
			const insertQuery = makeInsertQuery(tableKey, colDefs, input.resourceType)
			runScriptAndPollResult({
				workspace,
				requestBody: {
					args: { database: '$res:' + input.resourcePath, ...values },
					language,
					content: insertQuery
				}
			})
		}
	}
}

export type DbTableAction = {
	action: () => void | Promise<void>
	displayName: string
	confirmTitle?: string
	confirmBtnText?: string
	icon?: any
	successText?: string
}

export type DbTableActionFactory = (params: {
	tableKey: string
	refresh: () => void
}) => DbTableAction

export function dbDeleteTableActionWithPreviewScript({
	workspace,
	input
}: {
	workspace: string
	input: DbInput
}): DbTableActionFactory {
	return ({ tableKey, refresh }) => ({
		confirmTitle: `Are you sure you want to delete '${tableKey}' ? This action is irreversible`,
		displayName: 'Delete',
		confirmBtnText: `Delete permanently`,
		icon: Trash2,
		successText: `Table '${tableKey}' deleted successfully`,
		action: async () => {
			const dbType = getDbType(input)
			const language = getLanguageByResourceType(dbType)
			let deleteQuery = makeDeleteTableQuery(tableKey, dbType)
			if (input.type === 'ducklake') deleteQuery = wrapDucklakeQuery(deleteQuery, input.ducklake)
			console.log('deleteQuery', deleteQuery)
			await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: input.type === 'database' ? { database: '$res:' + input.resourcePath } : {},
					language,
					content: deleteQuery
				}
			})
			refresh()
		}
	})
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
	const stringified = Array.isArray(result) && result.length && result?.[0]?.['result']
	if (!stringified) throw new Error('Failed to get Ducklake schema: ' + JSON.stringify(result))
	let schema: Omit<SQLSchema, 'stringified'> = {
		schema: { main: JSON.parse(stringified) },
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

function getDbType(input: DbInput): DbType {
	switch (input.type) {
		case 'database':
			return input.resourceType
		case 'ducklake':
			return 'duckdb'
	}
}

function wrapDucklakeQuery(query: string, ducklake: string): string {
	let attach = `ATTACH 'ducklake://${ducklake}' AS dl;USE dl;\n`
	return query.replace(/^(--.*\n)*/, (match) => match + attach)
}
