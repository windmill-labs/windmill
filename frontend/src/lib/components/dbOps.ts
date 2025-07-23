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
	if (input.type !== 'database') throw new Error('Unimplemented')
	return {
		resourcePath: input.resourcePath,
		resourceType: input.resourceType,
		tableKey,
		colDefs,
		getCount: async ({ quicksearch }) => {
			const countQuery = makeCountQuery(input.resourceType, tableKey, undefined, colDefs)
			const result = await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: { database: '$res:' + input.resourcePath, quicksearch },
					language: getLanguageByResourceType(input.resourceType),
					content: countQuery
				}
			})
			const count = result?.[0].count as number
			return count
		},
		getRows: async (params) => {
			const query = makeSelectQuery(tableKey, colDefs, undefined, input.resourceType as DbType)
			let items = (await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: { database: '$res:' + input.resourcePath, ...params },
					language: getLanguageByResourceType(input.resourceType),
					content: query
				}
			})) as unknown[]
			if (input.resourceType === 'ms_sql_server') items = items?.[0] as unknown[]
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
					language: getLanguageByResourceType(input.resourceType),
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
					language: getLanguageByResourceType(input.resourceType),
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
					language: getLanguageByResourceType(input.resourceType),
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
	if (input.type !== 'database') throw new Error('Unimplemented')
	return ({ tableKey, refresh }) => ({
		confirmTitle: `Are you sure you want to delete '${tableKey}' ? This action is irreversible`,
		displayName: 'Delete',
		confirmBtnText: `Delete permanently`,
		icon: Trash2,
		successText: `Table '${tableKey}' deleted successfully`,
		action: async () => {
			const deleteQuery = makeDeleteTableQuery(tableKey, input.resourceType)
			await runScriptAndPollResult({
				workspace,
				requestBody: {
					args: { database: '$res:' + input.resourcePath },
					language: getLanguageByResourceType(input.resourceType),
					content: deleteQuery
				}
			})
			refresh()
		}
	})
}
