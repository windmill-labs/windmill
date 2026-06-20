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
import { WorkspaceService } from '$lib/gen'
import { randomUUID } from '$lib/utils/uuid'
import {
	buildTableEditorValues,
	type TableEditorValues
} from './apps/components/display/dbtable/tableEditor'
import { type AlterTableValues } from './apps/components/display/dbtable/queries/alterTable'
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
	previewCreateSql: (params: { values: TableEditorValues; schema?: string }) => Promise<string>
	onAlter: (params: {
		values: AlterTableValues
		/** Reverse diff (new → old), used to generate the down migration. */
		reverse?: AlterTableValues
		schema?: string
	}) => Promise<void>
	previewAlterSql: (params: { values: AlterTableValues; schema?: string }) => Promise<string>
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

	// When managing a data table, schema changes are recorded as migrations
	// instead of being run ad-hoc, so the manager stays the source of truth.
	const datatableName =
		input.type === 'database' && input.resourcePath.startsWith('datatable://')
			? input.resourcePath.slice('datatable://'.length)
			: undefined

	function makeMarker(op: string, payload: Record<string, unknown>): string {
		if (ducklake) payload.ducklake = ducklake
		return `-- WM_INTERNAL_DB_${op} ${JSON.stringify(payload)}`
	}

	// Auto-generated migration name, e.g. `create_customers_2da8`. The slug keeps
	// successive changes to the same object from colliding on the same timestamp.
	function migrationName(op: string, target: string): string {
		const slug = randomUUID().slice(0, 4)
		const safe = target.replace(/[^a-zA-Z0-9_-]+/g, '_').replace(/^_+|_+$/g, '')
		return safe ? `${op}_${safe}_${slug}` : `${op}_${slug}`
	}

	// Frame a single (or multi-) statement body in an explicit, `;`-terminated
	// transaction, matching the data table migration convention.
	function wrapMigration(sql: string): string {
		const t = sql.trimEnd()
		return `BEGIN;\n\n${t.endsWith(';') ? t : `${t};`}\n\nEND;`
	}

	// Apply a DDL marker. For a data table that has migrations enabled this
	// creates a migration and runs it (rolling the record back if the run fails);
	// otherwise it runs ad-hoc via the internal-db job as before. `downContent`,
	// when provided, is expanded into the migration's down SQL (Postgres only).
	async function applyDdl(migName: string, content: string, downContent?: string): Promise<void> {
		let migrationsEnabled = false
		if (datatableName) {
			try {
				const status = await WorkspaceService.getDatatableMigrationsStatus({
					workspace,
					datatableName
				})
				migrationsEnabled = status.enabled
			} catch {
				migrationsEnabled = false
			}
		}
		if (!datatableName || !migrationsEnabled) {
			await runScriptAndPollResult({ workspace, requestBody: { args: dbArg, content, language } })
			return
		}
		const codeUp = wrapMigration(await expandMarker(workspace, language, content))
		// Down migrations are only generated for Postgres for now.
		let codeDown: string | undefined
		if (downContent && dbType === 'postgresql') {
			const downSql = (await expandMarker(workspace, language, downContent)).trim()
			if (downSql) codeDown = wrapMigration(downSql)
		}
		const created = await WorkspaceService.createDatatableMigration({
			workspace,
			datatableName,
			requestBody: { name: migName, code_up: codeUp, ...(codeDown ? { code_down: codeDown } : {}) }
		})
		try {
			await WorkspaceService.runDatatableMigrations({
				workspace,
				datatableName,
				only: created.timestamp
			})
		} catch (e) {
			await WorkspaceService.deleteDatatableMigration({
				workspace,
				datatableName,
				timestamp: created.timestamp
			}).catch(() => {})
			throw e
		}
	}

	return {
		onDelete: async ({ tableKey, schema }) => {
			const content = makeMarker('DROP_TABLE', { table: tableKey, schema })
			await applyDdl(migrationName('drop', tableKey), content)
		},
		onCreate: async ({ values, schema }) => {
			const content = makeMarker('CREATE_TABLE', {
				name: values.name,
				columns: values.columns,
				foreignKeys: values.foreignKeys,
				schema
			})
			const downContent = makeMarker('DROP_TABLE', { table: values.name, schema })
			await applyDdl(migrationName('create', values.name), content, downContent)
		},
		previewCreateSql: async ({ values, schema }) => {
			const content = makeMarker('CREATE_TABLE', {
				name: values.name,
				columns: values.columns,
				foreignKeys: values.foreignKeys,
				schema
			})
			return expandMarker(workspace, language, content)
		},
		onAlter: async ({ values, reverse, schema }) => {
			const content = makeMarker('ALTER_TABLE', {
				name: values.name,
				operations: values.operations,
				schema
			})
			// The down is the same alter run in the opposite direction.
			const downContent = reverse
				? makeMarker('ALTER_TABLE', {
						name: reverse.name,
						operations: reverse.operations,
						schema
					})
				: undefined
			await applyDdl(migrationName('alter', values.name), content, downContent)
		},
		previewAlterSql: async ({ values, schema }) => {
			const content = makeMarker('ALTER_TABLE', {
				name: values.name,
				operations: values.operations,
				schema
			})
			return expandMarker(workspace, language, content)
		},
		onCreateSchema: async ({ schema }) => {
			const content = makeMarker('CREATE_SCHEMA', { schema })
			const downContent = makeMarker('DROP_SCHEMA', { schema })
			await applyDdl(migrationName('create_schema', schema), content, downContent)
		},
		onDeleteSchema: async ({ schema }) => {
			const content = makeMarker('DROP_SCHEMA', { schema })
			const downContent = makeMarker('CREATE_SCHEMA', { schema })
			await applyDdl(migrationName('drop_schema', schema), content, downContent)
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

async function expandMarker(workspace: string, language: string, content: string): Promise<string> {
	const response = await fetch(`/api/w/${workspace}/internal_db/expand_marker`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ language, content })
	})
	if (!response.ok) {
		throw new Error(await response.text())
	}
	const result = (await response.json()) as { code: string }
	return result.code
}
