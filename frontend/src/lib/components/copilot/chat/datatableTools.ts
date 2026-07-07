import { z } from 'zod'
import { WorkspaceService } from '$lib/gen'
import type { DataTableTables } from '$lib/gen/types.gen'
import { runScript } from '$lib/components/jobs/utils'
import { createToolDef, executeTestRun, type Tool } from './shared'

/**
 * Workspace-scoped datatable tools, with no app whitelist and no creation policy.
 *
 * Datatables are workspace-level managed PostgreSQL databases. The backend
 * endpoints used here (`list_datatable_tables`, `get_datatable_table_schema`)
 * and SQL execution (`datatable://<name>`) are gated only by workspace
 * membership, so these tools need no app context and operate directly on the
 * workspace. This is the unrestricted counterpart to the app-mode datatable
 * tools in `app/core.ts`, which additionally filter by the app's whitelist.
 */

// ============= Utility =============

/** Memoize a factory function - the factory is only called once, on first access */
const memo = <T>(factory: () => T): (() => T) => {
	let cached: T | undefined
	return () => (cached ??= factory())
}

// ============= Pure workspace-scoped operations =============

/** List all datatables configured in the workspace, with their schema/table names. */
export async function listDatatables(workspace: string): Promise<DataTableTables[]> {
	return await WorkspaceService.listDataTableTables({ workspace })
}

/** Get the columns (column_name -> compact_type) of one datatable table. */
export async function getDatatableColumns(
	workspace: string,
	datatableName: string,
	schemaName: string,
	tableName: string
): Promise<Record<string, string>> {
	const schema = await WorkspaceService.getDataTableTableSchema({
		workspace,
		datatableName,
		schemaName,
		tableName
	})
	return schema.columns
}

// ============= Error helpers =============

/**
 * The backend returns "datatable <name> not found" when no datatable with that
 * name is configured in the workspace settings. That is a hard, blocking
 * prerequisite (not a transient failure), so we surface an explicit, actionable
 * message instead of the raw internal error.
 */
function isDatatableNotConfiguredError(error: string | null | undefined): boolean {
	return typeof error === 'string' && /datatable\s+\S+\s+not found/i.test(error)
}

function datatableNotConfiguredMessage(datatableName: string): string {
	return (
		`Datatable "${datatableName}" is not configured in this workspace, so this operation cannot run. ` +
		`Datatables are not created by SQL — they must be set up first by the user in the workspace settings ` +
		`(Workspace settings → Data Tables) before any table can be queried or created. ` +
		`This is a required, blocking prerequisite: do not retry on this or another name. ` +
		`Tell the user they need to configure a datatable (e.g. named "${datatableName}") in their workspace settings, then try again.`
	)
}

const NO_DATATABLES_CONFIGURED_MESSAGE =
	'No datatables are configured in this workspace. Datatable operations (querying data, creating or altering tables) are blocked until a datatable exists. ' +
	'Datatables are not created by SQL — the user must set one up in the workspace settings (Workspace settings → Data Tables) first. ' +
	'Do not call exec_datatable_sql or assume a "main" datatable exists; instead tell the user this is a required prerequisite and ask them to configure a datatable in their workspace settings.'

// ============= Tool definitions =============

const getListDatatablesSchema = memo(() => z.object({}))
const getListDatatablesToolDef = memo(() =>
	createToolDef(
		getListDatatablesSchema(),
		'list_datatables',
		'List datatables configured in the workspace with schema and table names only. Does not include column definitions. Use this directly for table-list or available-tables summaries. Only call get_datatable_table_schema when column names/types are required.'
	)
)

const getGetDatatableTableSchemaSchema = memo(() =>
	z.object({
		datatable_name: z.string().describe('The datatable name to inspect, e.g. "main".'),
		schema_name: z.string().describe('The schema name, e.g. "public".'),
		table_name: z.string().describe('The table name to inspect.')
	})
)
const getGetDatatableTableSchemaToolDef = memo(() =>
	createToolDef(
		getGetDatatableTableSchemaSchema(),
		'get_datatable_table_schema',
		'Get column definitions for one datatable table. Do not call this for row counts or table-list summaries; list_datatables is enough for those.'
	)
)

const getExecDatatableSqlSchema = memo(() =>
	z.object({
		datatable_name: z
			.string()
			.describe(
				'The name of the datatable to query (e.g., "main"). Must be one of the datatables configured in the workspace.'
			),
		sql: z
			.string()
			.describe(
				'The SQL query to execute. Supports SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, ALTER TABLE, DROP TABLE, etc. For SELECT queries, results are returned as an array of objects. A newly created table will appear in list_datatables automatically.'
			),
		background: z
			.boolean()
			.optional()
			.describe(
				'Run in the background without waiting. Set true for queries you expect to be slow (large scans, heavy migrations) — you will be notified when it finishes. Leave unset for normal queries, which wait briefly and only background automatically if slow.'
			)
	})
)
const getExecDatatableSqlToolDef = memo(() =>
	createToolDef(
		getExecDatatableSqlSchema(),
		'exec_datatable_sql',
		'Execute a SQL query on a workspace datatable. Use this to explore data, test queries, create/alter tables, or make changes. Creating a table is a normal CREATE TABLE statement — no registration step is needed.'
	)
)

/** Maximum rows returned to the model for a SELECT query. */
const MAX_ROWS = 100

/**
 * The unrestricted workspace datatable tools, for registration in global mode.
 * Helper-free: each tool reads `workspace` directly from the tool call params.
 */
export function getDatatableTools(): Tool<{}>[] {
	return [
		{
			def: getListDatatablesToolDef(),
			fn: async ({ workspace, toolId, toolCallbacks }) => {
				toolCallbacks.setToolStatus(toolId, { content: 'Listing datatables...' })
				try {
					const metadata = await listDatatables(workspace)
					if (metadata.length === 0) {
						toolCallbacks.setToolStatus(toolId, {
							content: 'No datatables configured — set one up in workspace settings'
						})
						return NO_DATATABLES_CONFIGURED_MESSAGE
					}
					const totalTables = metadata.reduce(
						(acc, datatable) =>
							acc +
							Object.values(datatable.schemas).reduce((sum, tables) => sum + tables.length, 0),
						0
					)
					toolCallbacks.setToolStatus(toolId, {
						content: `Listed ${metadata.length} datatable(s) with ${totalTables} table(s)`
					})
					return JSON.stringify(metadata, null, 2)
				} catch (e) {
					const errorMsg = `Error listing datatables: ${e instanceof Error ? e.message : String(e)}`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return errorMsg
				}
			}
		},
		{
			def: getGetDatatableTableSchemaToolDef(),
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsedArgs = getGetDatatableTableSchemaSchema().parse(args)
				toolCallbacks.setToolStatus(toolId, {
					content: `Getting schema for ${parsedArgs.datatable_name}.${parsedArgs.schema_name}.${parsedArgs.table_name}...`
				})
				try {
					const columns = await getDatatableColumns(
						workspace,
						parsedArgs.datatable_name,
						parsedArgs.schema_name,
						parsedArgs.table_name
					)
					toolCallbacks.setToolStatus(toolId, {
						content: `Retrieved schema for ${parsedArgs.schema_name}.${parsedArgs.table_name}`
					})
					return JSON.stringify(
						{
							datatable_name: parsedArgs.datatable_name,
							schema_name: parsedArgs.schema_name,
							table_name: parsedArgs.table_name,
							columns
						},
						null,
						2
					)
				} catch (e) {
					const raw = e instanceof Error ? e.message : String(e)
					const errorMsg = isDatatableNotConfiguredError(raw)
						? datatableNotConfiguredMessage(parsedArgs.datatable_name)
						: `Error getting table schema: ${raw}`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return errorMsg
				}
			}
		},
		{
			def: getExecDatatableSqlToolDef(),
			requiresConfirmation: true,
			confirmationMessage: 'Execute SQL on datatable',
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsedArgs = getExecDatatableSqlSchema().parse(args)
				const name = parsedArgs.datatable_name
				// Route through executeTestRun so a slow query detaches into the jobs
				// tray (and honors `background`) instead of blocking the chat turn,
				// while keeping the datatable-specific result/error shaping.
				return executeTestRun({
					jobStarter: () =>
						runScript({
							workspace,
							requestBody: {
								language: 'postgresql',
								content: parsedArgs.sql,
								args: { database: `datatable://${name}` }
							}
						}),
					workspace,
					toolCallbacks,
					toolId,
					contextName: 'script',
					label: `SQL · ${name}`,
					background: parsedArgs.background,
					startMessage: `Executing SQL on "${name}"...`,
					runningMessage: `SQL running on "${name}"...`,
					formatCompletion: (job) => {
						if (job.success) {
							// Successful runs always carry a `result` array (empty for DDL/DML),
							// so SELECT rows and zero-row statements share one reporting path.
							const rows = Array.isArray(job.result) ? (job.result as Record<string, any>[]) : []
							const rowCount = rows.length
							const payload =
								rowCount > MAX_ROWS
									? {
											success: true,
											rowCount,
											result: rows.slice(0, MAX_ROWS),
											note: `Showing first ${MAX_ROWS} of ${rowCount} rows`
										}
									: { success: true, rowCount, result: rows }
							return {
								llmText: JSON.stringify(payload, null, 2),
								card: {
									content: `Query returned ${rowCount} row(s)`,
									result: JSON.stringify(job.result, null, 2)
								}
							}
						}
						const raw =
							(job.result as any)?.error?.message ??
							(typeof job.result === 'string' ? job.result : JSON.stringify(job.result))
						const errorMsg = isDatatableNotConfiguredError(raw)
							? datatableNotConfiguredMessage(name)
							: raw
						return {
							llmText: JSON.stringify({ success: false, error: errorMsg }),
							card: { content: `Error: ${errorMsg}`, error: errorMsg }
						}
					}
				})
			}
		}
	]
}
