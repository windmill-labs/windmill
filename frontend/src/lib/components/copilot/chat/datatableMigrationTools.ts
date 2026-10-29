import { z } from 'zod'
import { createToolDef } from './shared'
import { NONE, RUN_PREVIEW, type SessionTool } from './sessionCapabilities'
import { runServerTool } from './global/backendSessionTools'

/**
 * Tools over a data table's versioned SQL migrations (`datatable_migrations.rs`).
 *
 * A data table with migrations enabled keeps its schema as an ordered list of definitions, each
 * applied once per database and recorded there. That list is what git sync and a workspace merge
 * carry, so a schema change made with plain SQL exists in one database only.
 */

const memo = <T>(factory: () => T): (() => T) => {
	let cached: T | undefined
	return () => (cached ??= factory())
}

const datatableNameSchema = () =>
	z.string().describe('The datatable the migrations belong to, e.g. "main".')

// ============= Tool definitions =============

const getListMigrationsSchema = memo(() =>
	z.object({
		datatable_name: datatableNameSchema(),
		version: z
			.number()
			.optional()
			.describe('Return the SQL of this one migration instead of the list.')
	})
)
const getListMigrationsToolDef = memo(() =>
	createToolDef(
		getListMigrationsSchema(),
		'list_datatable_migrations',
		"List a datatable's migrations in order, each with its version, name and whether it is applied or pending. Pass `version` to read one migration's SQL."
	)
)

const getCreateMigrationSchema = memo(() =>
	z.object({
		datatable_name: datatableNameSchema(),
		name: z
			.string()
			.regex(/^[A-Za-z0-9_-]+$/)
			.describe(
				'Short name of the change, e.g. "add_refunds_table". Letters, digits, "_" and "-" only.'
			),
		code_up: z
			.string()
			.describe(
				'The SQL that makes the change. Wrap the statements in `BEGIN; ... END;` so they apply atomically, unless one of them cannot run in a transaction.'
			),
		code_down: z
			.string()
			.optional()
			.describe(
				'The SQL that undoes `code_up`. Without it the migration cannot be rolled back, so provide it whenever the change is reversible.'
			),
		role: z
			.string()
			.regex(/^[A-Za-z0-9_-]{1,63}$/)
			.optional()
			.describe(
				'The datatable role the migration runs as, on a datatable that has roles. Omit to run as admin, which only the datatable admins may do.'
			),
		schemas: z
			.array(z.string().regex(/^[A-Za-z0-9_$-]{1,63}$/))
			.optional()
			.describe(
				'Every schema the migration changes or depends on (including schemas of referenced tables, types and functions), with every name in `code_up` schema-qualified. A migration then waits only on earlier migrations touching those schemas. Omit only for a change that spans the whole database (an extension, a grant across schemas): every later migration then waits for it.'
			),
		version: z
			.number()
			.optional()
			.describe(
				'Rewrite this migration instead of adding one: only for a pending migration, typically the one that just failed to apply.'
			),
		apply: z
			.boolean()
			.optional()
			.describe('Set false to save the migration as pending without running it. Defaults to true.')
	})
)
const getCreateMigrationToolDef = memo(() =>
	createToolDef(
		getCreateMigrationSchema(),
		'create_datatable_migration',
		'Change the schema of a datatable that has migrations enabled: saves the change as a new migration and applies it. One that fails to apply stays saved as pending: call this again with its `version` to fix it.'
	)
)

const getRunMigrationsSchema = memo(() => z.object({ datatable_name: datatableNameSchema() }))
const getRunMigrationsToolDef = memo(() =>
	createToolDef(
		getRunMigrationsSchema(),
		'run_datatable_migrations',
		"Apply a datatable's pending migrations, oldest first. It stops at the first one that fails."
	)
)

const getRollbackMigrationSchema = memo(() =>
	z.object({
		datatable_name: datatableNameSchema(),
		version: z
			.number()
			.describe('The applied migration to undo, normally the most recent one, as listed.')
	})
)
const getRollbackMigrationToolDef = memo(() =>
	createToolDef(
		getRollbackMigrationSchema(),
		'rollback_datatable_migration',
		'Undo one applied migration of a datatable by running its down SQL. The migration stays in the list as pending.'
	)
)

// ============= Tools =============

export function getDatatableMigrationTools(): SessionTool<{}>[] {
	return [
		{
			requires: NONE,
			def: getListMigrationsToolDef(),
			planModeSafe: true,
			fn: async (ctx) => {
				const { datatable_name } = getListMigrationsSchema().parse(ctx.args)
				ctx.toolCallbacks.setToolStatus(ctx.toolId, {
					content: `Listing migrations of "${datatable_name}"...`
				})
				return (await runServerTool('list_datatable_migrations', ctx)).text
			}
		},
		{
			// `ensure_not_operator` in datatable_migrations.rs: the condition `run_preview` stands for.
			requires: RUN_PREVIEW,
			def: getCreateMigrationToolDef(),
			requiresConfirmation: true,
			confirmAction: (args) => (args?.apply === false ? 'create' : 'apply'),
			confirmationMessage: (args) =>
				`${args?.version === undefined ? 'Create' : 'Rewrite'}${args?.apply === false ? '' : ' and apply'} data table migration`,
			queuedLabel: (args) => `Write migration "${args?.name ?? ''}"`,
			showDetails: true,
			fn: async (ctx) => {
				const parsed = getCreateMigrationSchema().parse(ctx.args)
				ctx.toolCallbacks.setToolStatus(ctx.toolId, {
					content: `Saving migration "${parsed.name}" on "${parsed.datatable_name}"...`
				})
				return (await runServerTool('create_datatable_migration', ctx)).text
			}
		},
		{
			requires: RUN_PREVIEW,
			def: getRunMigrationsToolDef(),
			requiresConfirmation: true,
			confirmAction: 'apply',
			confirmationMessage: 'Apply pending data table migrations',
			queuedLabel: (args) => `Apply migrations on "${args?.datatable_name ?? 'datatable'}"`,
			showDetails: true,
			fn: async (ctx) => {
				const { datatable_name } = getRunMigrationsSchema().parse(ctx.args)
				ctx.toolCallbacks.setToolStatus(ctx.toolId, {
					content: `Applying migrations on "${datatable_name}"...`
				})
				return (await runServerTool('run_datatable_migrations', ctx)).text
			}
		},
		{
			requires: RUN_PREVIEW,
			def: getRollbackMigrationToolDef(),
			requiresConfirmation: true,
			confirmAction: 'rollback',
			confirmationMessage: (args) => `Roll back data table migration ${args?.version ?? ''}`,
			queuedLabel: (args) => `Roll back migration ${args?.version ?? ''}`,
			showDetails: true,
			fn: async (ctx) => {
				const { datatable_name, version } = getRollbackMigrationSchema().parse(ctx.args)
				ctx.toolCallbacks.setToolStatus(ctx.toolId, {
					content: `Rolling back migration ${version} on "${datatable_name}"...`
				})
				return (await runServerTool('rollback_datatable_migration', ctx)).text
			}
		}
	]
}
