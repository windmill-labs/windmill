import { z } from 'zod'
import { WorkspaceService } from '$lib/gen'
import { withMigrationRole } from '$lib/components/datatableMigrationRole'
import {
	migrationScope,
	parseMigrationSchemas,
	scopesOverlap,
	suggestMigrationSchemas,
	withMigrationSchemas
} from '$lib/components/datatableMigrationSchemas'
import { createToolDef, type ToolCallbacks } from './shared'
import { NONE, RUN_PREVIEW, type SessionTool } from './sessionCapabilities'

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

function apiErrorMessage(e: unknown): string {
	const err = e as { body?: unknown; message?: unknown } | undefined
	const body = err?.body
	if (typeof body === 'string' && body !== '') return body
	if (typeof err?.message === 'string') return err.message
	return String(e)
}

function fail(toolCallbacks: ToolCallbacks, toolId: string, message: string): string {
	toolCallbacks.setToolStatus(toolId, { content: message, error: message })
	return message
}

const migrationLabel = (m: { version: number; name: string }) => `${m.version} (${m.name})`

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
				'The schemas the migration changes. A migration then waits only on earlier migrations touching those schemas. Omit to have them read off the schema-qualified names in `code_up`; pass [] only for a change that spans the whole database, which every later migration then waits for.'
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
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const { datatable_name, version } = getListMigrationsSchema().parse(args)
				toolCallbacks.setToolStatus(toolId, {
					content: `Listing migrations of "${datatable_name}"...`
				})
				try {
					const status = await WorkspaceService.getDatatableMigrationsStatus({
						workspace,
						datatableName: datatable_name
					})
					if (!status.enabled) {
						toolCallbacks.setToolStatus(toolId, {
							content: `Migrations are not enabled on "${datatable_name}"`
						})
						return `Migrations are not enabled on datatable "${datatable_name}": its schema is changed with plain SQL and there is no history to list. Enabling them is a workspace admin's decision, made in Workspace settings → Data Tables.`
					}
					if (version !== undefined) {
						const migration = status.migrations.find((m) => m.timestamp === version)
						if (!migration) {
							return fail(
								toolCallbacks,
								toolId,
								`No migration ${version} on datatable "${datatable_name}"`
							)
						}
						toolCallbacks.setToolStatus(toolId, {
							content: `Read migration ${migrationLabel({ version, name: migration.name })}`
						})
						return JSON.stringify(
							{
								version,
								name: migration.name,
								status: migrationStatus(migration.status),
								code_up: migration.code_up,
								...(migration.code_down != null && { code_down: migration.code_down })
							},
							null,
							2
						)
					}
					toolCallbacks.setToolStatus(toolId, {
						content: `Listed ${status.migrations.length} migration(s) of "${datatable_name}"`
					})
					return JSON.stringify(
						{
							migrations: status.migrations.map((m) => ({
								version: m.timestamp,
								name: m.name,
								status: migrationStatus(m.status),
								schemas: m.schemas ?? 'whole database'
							})),
							// Set when the database could not be read, which is why every status is unknown.
							...(status.error && { error: status.error })
						},
						null,
						2
					)
				} catch (e) {
					return fail(toolCallbacks, toolId, `Error listing migrations: ${apiErrorMessage(e)}`)
				}
			}
		},
		{
			// `ensure_not_operator` in datatable_migrations.rs: the condition `run_preview` stands for.
			requires: RUN_PREVIEW,
			def: getCreateMigrationToolDef(),
			requiresConfirmation: true,
			confirmationMessage: (args) =>
				`${args?.version === undefined ? 'Create' : 'Rewrite'}${args?.apply === false ? '' : ' and apply'} data table migration`,
			queuedLabel: (args) => `Write migration "${args?.name ?? ''}"`,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsed = getCreateMigrationSchema().parse(args)
				const datatableName = parsed.datatable_name
				toolCallbacks.setToolStatus(toolId, {
					content: `Saving migration "${parsed.name}" on "${datatableName}"...`
				})
				// The role is an annotation in the SQL, read from above everything else, and a rollback
				// has to run as the role that created what it drops.
				const declared = parseMigrationSchemas(parsed.code_up)
				const scope =
					parsed.schemas ??
					(declared.kind === 'schemas' ? declared.schemas : suggestMigrationSchemas(parsed.code_up))
				const definition = {
					name: parsed.name,
					code_up: withMigrationSchemas(
						withRole(parsed.code_up, parsed.role),
						scope.map((s) => s.toLowerCase())
					),
					...(parsed.code_down !== undefined && {
						code_down: withRole(parsed.code_down, parsed.role)
					})
				}
				let version: number
				try {
					if (parsed.version === undefined) {
						const created = await WorkspaceService.createDatatableMigration({
							workspace,
							datatableName,
							requestBody: definition
						})
						version = created.timestamp
					} else {
						version = parsed.version
						// The server inserts at a version it does not know, which would wedge a new
						// migration into the middle of the history, and stores a down left out as none.
						const { migrations } = await WorkspaceService.getDatatableMigrationsStatus({
							workspace,
							datatableName
						})
						const existing = migrations.find((m) => m.timestamp === version)
						if (existing?.status !== 'not_run') {
							return fail(
								toolCallbacks,
								toolId,
								`Migration ${version} is not a pending migration of datatable "${datatableName}", so it cannot be rewritten. Omit \`version\` to add a new migration.`
							)
						}
						await WorkspaceService.upsertDatatableMigration({
							workspace,
							datatableName,
							requestBody: {
								timestamp: version,
								...(existing.code_down != null && { code_down: existing.code_down }),
								...definition
							}
						})
					}
				} catch (e) {
					return fail(toolCallbacks, toolId, `Error saving migration: ${apiErrorMessage(e)}`)
				}
				const label = migrationLabel({ version, name: parsed.name })
				if (parsed.apply === false) {
					toolCallbacks.setToolStatus(toolId, { content: `Saved migration ${label} as pending` })
					return `Saved migration ${label} as pending. It is not applied yet.`
				}

				// Applying it alone would run it ahead of migrations it may depend on, and applying
				// those with it would run SQL the user was never shown.
				let earlierPending: string[]
				try {
					const status = await WorkspaceService.getDatatableMigrationsStatus({
						workspace,
						datatableName
					})
					const ownScope = migrationScope(definition.code_up)
					const earlier = status.migrations.filter(
						(m) => m.timestamp < version && scopesOverlap(m.schemas, ownScope)
					)
					// Unreadable is not pending: reporting it so would send the user to apply
					// migrations that already ran.
					if (status.error || earlier.some((m) => m.status === 'unknown')) {
						throw new Error(status.error ?? 'the applied migrations could not be read')
					}
					earlierPending = earlier
						.filter((m) => m.status === 'not_run')
						.map((m) => migrationLabel({ version: m.timestamp, name: m.name }))
				} catch (e) {
					return fail(
						toolCallbacks,
						toolId,
						`Saved migration ${label} as pending, but could not check what else is pending, so it was not applied: ${apiErrorMessage(e)}`
					)
				}
				if (earlierPending.length > 0) {
					toolCallbacks.setToolStatus(toolId, {
						content: `Saved migration ${label}; ${earlierPending.length} earlier migration(s) still pending`
					})
					return `Saved migration ${label} as pending without applying it: earlier migrations are not applied yet (${earlierPending.join(', ')}) and it may depend on them. Tell the user, then apply them all in order with run_datatable_migrations.`
				}

				toolCallbacks.setToolStatus(toolId, { content: `Applying migration ${label}...` })
				try {
					await WorkspaceService.runDatatableMigrations({
						workspace,
						datatableName,
						only: version
					})
				} catch (e) {
					// Kept whatever failed: part of it may be applied, or all of it with only the record
					// missing, and the definition with its down is then the one trace of that.
					return fail(
						toolCallbacks,
						toolId,
						`Migration ${label} is saved but failed to apply: ${apiErrorMessage(e)}. It stays pending and blocks later migrations. If nothing of it was applied, fix it by calling this tool again with version ${version}; otherwise, or to drop it, tell the user, who can edit or delete it in Workspace settings → Data Tables.`
					)
				}
				toolCallbacks.setToolStatus(toolId, { content: `Applied migration ${label}` })
				return `Applied migration ${label} on datatable "${datatableName}".`
			}
		},
		{
			requires: RUN_PREVIEW,
			def: getRunMigrationsToolDef(),
			requiresConfirmation: true,
			confirmationMessage: 'Apply pending data table migrations',
			queuedLabel: (args) => `Apply migrations on "${args?.datatable_name ?? 'datatable'}"`,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const { datatable_name } = getRunMigrationsSchema().parse(args)
				toolCallbacks.setToolStatus(toolId, {
					content: `Applying migrations on "${datatable_name}"...`
				})
				try {
					const { applied } = await WorkspaceService.runDatatableMigrations({
						workspace,
						datatableName: datatable_name
					})
					if (applied.length === 0) {
						toolCallbacks.setToolStatus(toolId, { content: 'No pending migration to apply' })
						return `No pending migration to apply on datatable "${datatable_name}".`
					}
					toolCallbacks.setToolStatus(toolId, {
						content: `Applied ${applied.length} migration(s) on "${datatable_name}"`
					})
					return `Applied ${applied.length} migration(s) on datatable "${datatable_name}": ${applied.map(migrationLabel).join(', ')}.`
				} catch (e) {
					// Those before the failing one are applied: the list is what says where it stopped.
					return fail(
						toolCallbacks,
						toolId,
						`Error applying migrations: ${apiErrorMessage(e)}. Migrations before the failing one may have been applied: list them to see.`
					)
				}
			}
		},
		{
			requires: RUN_PREVIEW,
			def: getRollbackMigrationToolDef(),
			requiresConfirmation: true,
			confirmationMessage: (args) => `Roll back data table migration ${args?.version ?? ''}`,
			queuedLabel: (args) => `Roll back migration ${args?.version ?? ''}`,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const { datatable_name, version } = getRollbackMigrationSchema().parse(args)
				toolCallbacks.setToolStatus(toolId, {
					content: `Rolling back migration ${version} on "${datatable_name}"...`
				})
				try {
					const { rolled_back } = await WorkspaceService.rollbackDatatableMigrations({
						workspace,
						datatableName: datatable_name,
						only: version
					})
					if (rolled_back.length === 0) {
						toolCallbacks.setToolStatus(toolId, { content: `Migration ${version} is not applied` })
						return `Migration ${version} is not applied on datatable "${datatable_name}", so there is nothing to roll back.`
					}
					const label = rolled_back.map(migrationLabel).join(', ')
					toolCallbacks.setToolStatus(toolId, { content: `Rolled back migration ${label}` })
					return `Rolled back migration ${label} on datatable "${datatable_name}". It is pending again.`
				} catch (e) {
					return fail(toolCallbacks, toolId, `Error rolling back: ${apiErrorMessage(e)}`)
				}
			}
		}
	]
}

function migrationStatus(status: 'ran' | 'not_run' | 'unknown'): 'applied' | 'pending' | 'unknown' {
	return status === 'ran' ? 'applied' : status === 'not_run' ? 'pending' : 'unknown'
}

function withRole(sql: string, role: string | undefined): string {
	return role === undefined ? sql : withMigrationRole(sql, role)
}
