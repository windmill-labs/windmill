import {
	getLanguageByResourceType,
	ColumnIdentity,
	type ColumnDef,
	type TableMetadata
} from './apps/components/display/dbtable/utils'
import { runScriptAndPollResult } from './jobs/utils'
import type { DBSchema, SQLSchema } from '$lib/stores'
import { stringifySchema } from './copilot/lib'
import type { DbInput, DbType } from './dbTypes'
import { assert } from '$lib/utils'
import { WorkspaceService } from '$lib/gen'
import { pendingMigrations } from './workspaceSettings/datatableMigrationUtils'
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
	workspace,
	whereClause,
	version
}: {
	input: DbInput
	tableKey: string
	colDefs: ColumnDef[]
	workspace: string
	// Optional raw SQL predicate AND-ed into the read queries (count + rows).
	// Caller-trusted — build it with escaped values.
	whereClause?: string
	// DuckLake time-travel: when set, reads are pinned to this catalog snapshot
	// via `AT (VERSION => n)` (DuckDB/ducklake only). Read-only by nature.
	version?: number
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
			const content = makeMarker('COUNT', {
				table: tableKey,
				columnDefs: colDefs,
				...(whereClause ? { whereClause } : {}),
				...(version != undefined ? { version } : {})
			})
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
				fixPgIntTypes: true,
				...(whereClause ? { whereClause } : {}),
				...(version != undefined ? { version } : {})
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

export type DucklakeSnapshot = {
	snapshot_id: number
	// DuckLake returns this as microseconds-since-epoch serialized as a string
	// (TIMESTAMP); callers must convert before formatting.
	snapshot_time: string | number
}

/**
 * Column metadata of a ducklake table *at a specific snapshot*. The catalog's
 * `information_schema` only reflects the current schema, so a time-travel read
 * pinned to an older version must enumerate the columns that existed *then* —
 * otherwise a column added in a later snapshot would break the `SELECT … AT
 * (VERSION => n)`. `DESCRIBE SELECT * FROM … AT (VERSION => n)` gives exactly
 * that. Returns minimal `ColumnDef`s (field + datatype) — enough for the
 * read-only preview's SELECT/COUNT and grid headers.
 */
export async function fetchDucklakeColumnsAtVersion({
	workspace,
	ducklake,
	tableKey,
	version
}: {
	workspace: string
	ducklake: string
	tableKey: string
	version: number
}): Promise<ColumnDef[]> {
	// Quote each identifier part (schema.table) so a dotted/odd table name can't
	// break the statement, and double single-quotes in the catalog name so it
	// can't break out of the ATTACH string literal (mirrors the backend's
	// `escape_sql_literal`). `version` is a number — injection-safe.
	const quoted = tableKey
		.split('.')
		.map((p) => `"${p.replace(/"/g, '""')}"`)
		.join('.')
	const ducklakeLit = ducklake.replace(/'/g, "''")
	const content =
		`ATTACH 'ducklake://${ducklakeLit}' AS __dlv__; USE __dlv__; ` +
		`DESCRIBE SELECT * FROM ${quoted} AT (VERSION => ${version});`
	const rows = (await runScriptAndPollResult({
		workspace,
		requestBody: { args: {}, language: 'duckdb', content }
	})) as { column_name: string; column_type: string }[]
	if (!Array.isArray(rows)) return []
	return rows.map((r) => ({
		field: r.column_name,
		datatype: r.column_type,
		defaultvalue: '',
		isprimarykey: false,
		isidentity: ColumnIdentity.No,
		isnullable: 'YES' as const,
		isenum: false
	}))
}

/**
 * List a ducklake table's time-travel history, newest first. DuckLake snapshots
 * are catalog-wide commits; passing `table` (schema-qualified, e.g.
 * `main.events_daily`) scopes the list to snapshots where the table exists —
 * otherwise an `AT (VERSION => n)` read could target a version predating the
 * table's creation and error. Runs the `DUCKLAKE_SNAPSHOTS` marker as a duckdb
 * preview job (server-side SQL build + ATTACH), so no raw SQL is constructed in
 * the client.
 */
export async function fetchDucklakeSnapshots({
	workspace,
	ducklake,
	table
}: {
	workspace: string
	ducklake: string
	table?: string
}): Promise<DucklakeSnapshot[]> {
	const content = `-- WM_INTERNAL_DB_DUCKLAKE_SNAPSHOTS ${JSON.stringify({
		ducklake,
		...(table ? { table } : {})
	})}`
	const rows = await runScriptAndPollResult({
		workspace,
		requestBody: { args: {}, language: 'duckdb', content }
	})
	return Array.isArray(rows) ? (rows as DucklakeSnapshot[]) : []
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

/** Thrown by a schema op when the user declines the out-of-order run warning.
 * Callers should treat it as a silent cancel (no error toast). */
export class MigrationRunCancelled extends Error {
	constructor() {
		super('Migration run cancelled')
		this.name = 'MigrationRunCancelled'
	}
}

export function dbSchemaOpsWithPreviewScripts({
	workspace,
	input,
	confirmRunOutOfOrder
}: {
	workspace: string
	input: DbInput
	/** Asked before running a just-created migration ahead of `pendingCount`
	 * still-pending earlier ones. Return false to abort (throws MigrationRunCancelled). */
	confirmRunOutOfOrder?: (pendingCount: number) => Promise<boolean>
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

	// Auto-generated migration name, e.g. `create_customers`. The server allocates
	// a unique timestamp (bumping on collision), so the name itself need not be unique.
	function migrationName(op: string, target: string): string {
		const safe = target.replace(/[^a-zA-Z0-9_-]+/g, '_').replace(/^_+|_+$/g, '')
		return safe ? `${op}_${safe}` : op
	}

	// A dropped SERIAL column reports its default as `nextval()` of an owned
	// sequence that is dropped along with the column. When the down re-adds such a
	// column, recreate it as its SERIAL type so a fresh sequence is created
	// instead of referencing the gone one.
	const SERIAL_FOR: Record<string, string> = {
		BIGINT: 'BIGSERIAL',
		INT8: 'BIGSERIAL',
		INTEGER: 'SERIAL',
		INT: 'SERIAL',
		INT4: 'SERIAL',
		SMALLINT: 'SMALLSERIAL',
		INT2: 'SMALLSERIAL'
	}
	function reverseSerialFix(reverse: AlterTableValues): AlterTableValues {
		return {
			...reverse,
			operations: reverse.operations.map((op) => {
				if (op.kind !== 'addColumn' || !/nextval\s*\(/i.test(op.column.defaultValue ?? '')) {
					return op
				}
				const serial = SERIAL_FOR[(op.column.datatype ?? '').toUpperCase()]
				return serial
					? { ...op, column: { ...op.column, datatype: serial, defaultValue: undefined } }
					: op
			})
		}
	}

	// Frame a single (or multi-) statement body in an explicit, `;`-terminated
	// transaction, matching the data table migration convention. Some expanded
	// markers (e.g. ALTER TABLE) already come wrapped in their own transaction, so
	// avoid nesting BEGIN/COMMIT in that case.
	function wrapMigration(sql: string): string {
		const t = sql.trim()
		if (/^BEGIN\b/i.test(t)) return t
		return `BEGIN;\n\n${t.endsWith(';') ? t : `${t};`}\n\nEND;`
	}

	// Apply a DDL marker. For a data table that has migrations enabled this
	// creates a migration and runs it (rolling the record back if the run fails);
	// otherwise it runs ad-hoc via the internal-db job as before. `downContent`,
	// when provided, is expanded into the migration's down SQL (Postgres only).
	async function applyDdl(migName: string, content: string, downContent?: string): Promise<void> {
		// A DDL edit on a migrations-enabled data table must be captured as a
		// migration. Don't swallow a status-check failure by defaulting to ad-hoc:
		// that would run the change untracked (schema drift) — exactly what this
		// feature prevents. Let the error propagate (fail closed); only fall back to
		// ad-hoc when there's no data table, or `enabled === false` is returned.
		const status = datatableName
			? await WorkspaceService.getDatatableMigrationsStatus({ workspace, datatableName })
			: undefined
		if (!datatableName || !status?.enabled) {
			await runScriptAndPollResult({ workspace, requestBody: { args: dbArg, content, language } })
			return
		}
		// The new migration gets the highest timestamp, so any still-pending
		// migration is earlier: running only this one applies it out of order.
		// Warn like the row-level Run action does (skipped if no confirm hook).
		if (confirmRunOutOfOrder) {
			const pending = pendingMigrations(status.migrations).length
			if (pending > 0 && !(await confirmRunOutOfOrder(pending))) {
				throw new MigrationRunCancelled()
			}
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
						operations: reverseSerialFix(reverse).operations,
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
	let schemas = Array.isArray(result) && result.length && (result?.[0]?.['result'] ?? {})
	// Safety for agent workers (duckdb ffi lib used to return JSON as stringified json)
	if (typeof schemas === 'string') schemas = JSON.parse(schemas)

	if (!schemas) throw new Error('Failed to get Ducklake schema: ' + JSON.stringify(result))
	assert('schemas is an object', typeof schemas === 'object')
	let schema: Omit<SQLSchema, 'stringified'> = {
		schema: schemas,
		publicOnly: false,
		lang: 'ducklake'
	}
	return { ...schema, stringified: stringifySchema(schema) }
}

// Returns every schema in the ducklake (including empty ones, e.g. freshly created)
// as a nested map { schema: { table: { column: {...} } } }.
const DUCKLAKE_GET_SCHEMA_QUERY = `
SELECT json_group_object(schema_name, COALESCE(schema_data, json_object())) AS result FROM (
	SELECT
		s.schema_name,
		(
			SELECT json_group_object(table_name, table_data) FROM (
				SELECT
					c.table_name,
					json_group_object(
						c.column_name,
						json_object(
							'type', c.data_type,
							'default', c.column_default,
							'required', c.is_nullable == 'NO'
						)
					) AS table_data
				FROM information_schema.columns c
				WHERE c.table_catalog = '__ducklake__' AND c.table_schema = s.schema_name
				GROUP BY c.table_name
			)
		) AS schema_data
	FROM information_schema.schemata s
	WHERE s.catalog_name = '__ducklake__'
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
