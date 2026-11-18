import { beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * A run that imported every item but failed a data table migration is not a clean finish.
 * `error` is what offers Retry, so this is about whether the user can act on the failure —
 * the `migrate` row already says it happened.
 */

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		createWorkspace: vi.fn(),
		listDataTables: vi.fn(async () => []),
		listWorkspaces: vi.fn(async () => [])
	},
	UserService: {
		whoami: vi.fn(async () => ({ username: 'u' })),
		globalWhoami: vi.fn(async () => ({ email: 'u@example.com' }))
	}
}))
vi.mock('$lib/storeUtils', () => ({ switchWorkspace: vi.fn() }))
vi.mock('./probe', async (orig) => ({
	...(await orig<typeof import('./probe')>()),
	probeWorkspace: vi.fn(async () => ({ exists: false, ours: false })),
	probeImportedPaths: vi.fn(async () => new Set<string>())
}))
vi.mock('$lib/user', () => ({ getUserExt: vi.fn(async () => ({ username: 'u' })) }))

/** Whether the migration this run applies succeeds. The item writes always do. */
const outcome = vi.hoisted(() => ({ migrationOk: true }))

vi.mock('$lib/components/workspaceSettings/projectInstall', () => ({
	installProject: vi.fn(async (args: any) => {
		args.onResult({ path: 'f/calendly/one', ok: true })
		if (args.migrations?.length) {
			args.onMigrationsStart?.()
			for (const m of args.migrations) {
				args.onResult({
					path: `data table: ${m.datatable_name}`,
					ok: outcome.migrationOk,
					error: outcome.migrationOk ? undefined : 'relation already exists'
				})
			}
		}
	})
}))

const EXPORT = {
	project: { slug: 'calendly', name: 'Calendly', summary: '', readme: null },
	scripts: [],
	flows: [],
	apps: [],
	resources: [],
	triggers: [],
	migrations: []
}
vi.stubGlobal(
	'fetch',
	vi.fn(async () => ({ ok: true, status: 200, text: async () => JSON.stringify(EXPORT) }))
)

import { ImportExecution } from './execution.svelte'

const PLAN = { slug: 'calendly', destination: { kind: 'existing' as const, workspaceId: 'ws-a' } }
const MIGRATION = {
	datatable_name: 'main',
	sql: 'CREATE TABLE IF NOT EXISTS "calendly"."config" (id int)',
	sql_down: '',
	enabled: true
}
const deps = { reviewMigrations: async () => [MIGRATION], hasEeLicense: false }

describe('a failed migration', () => {
	beforeEach(() => {
		outcome.migrationOk = true
	})

	it('marks the migrate row failed and leaves the run retryable', async () => {
		outcome.migrationOk = false
		const run = new ImportExecution(PLAN, deps as any)
		await run.run()
		expect(run.tasks.find((t) => t.key === 'migrate')?.status).toBe('failed')
		// `error` is what the page reads to offer Retry: a `migrate` row saying failed with
		// `error` unset sends it down the finished-run path with no way to run it again.
		expect(run.error).toBeTruthy()
		expect(run.error).toContain('migration')
	})

	it('says nothing went wrong when the migration succeeds', async () => {
		const run = new ImportExecution(PLAN, deps as any)
		await run.run()
		expect(run.tasks.find((t) => t.key === 'migrate')?.status).toBe('done')
		expect(run.error).toBeFalsy()
	})
})
