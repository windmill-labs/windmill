import { beforeEach, describe, expect, it, vi } from 'vitest'

// Everything the executor reaches over the network, stubbed. The two behaviours under test
// are decisions it makes around those calls, not the calls themselves.
vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		createWorkspace: vi.fn(),
		listDataTables: vi.fn(async () => []),
		// No workspace of ours at that id: the existing-workspace plan these tests use never
		// reaches the create, and an empty list is the honest answer for a fresh instance.
		listWorkspaces: vi.fn(async () => [])
	},
	UserService: {
		whoami: vi.fn(async () => ({ username: 'u' })),
		globalWhoami: vi.fn(async () => ({ email: 'u@example.com' }))
	}
}))
vi.mock('$lib/storeUtils', () => ({ switchWorkspace: vi.fn() }))
vi.mock('$lib/user', () => ({ getUserExt: vi.fn(async () => ({ username: 'u' })) }))
// Let a test abandon *during* a write loop, which is the only way it happens for real:
// `run()` clears the flag on entry so a retry can proceed. Two hooks, because the item and
// migration phases stop in different places and the second is what pins the migrate row.
const hooks = vi.hoisted(() => ({
	afterFirstItem: undefined as (() => void) | undefined,
	afterMigrationsStart: undefined as (() => void) | undefined
}))

vi.mock('$lib/components/workspaceSettings/projectInstall', () => ({
	installProject: vi.fn(async (args: any) => {
		// Ordered as the real one is: every item loop, then `onMigrationsStart`, then the
		// migrations — and `stopped` checked before each write, returning the same way it
		// returns on success.
		for (const path of ['a', 'b', 'c']) {
			if (args.stopped?.() === true) return
			args.onResult({ path, ok: true })
			hooks.afterFirstItem?.()
			hooks.afterFirstItem = undefined
		}
		if (args.stopped?.() === true) return
		if (args.migrations?.length) {
			args.onMigrationsStart?.()
			hooks.afterMigrationsStart?.()
			hooks.afterMigrationsStart = undefined
			for (const m of args.migrations) {
				if (args.stopped?.() === true) return
				args.onResult({ path: `data table: ${m.datatable_name}`, ok: true })
			}
		}
	})
}))

// The export the run fetches from the hub proxy. Two items so an abandoned run can stop
// partway through, which is the case under test.
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
const deps = { reviewMigrations: async () => [], hasEeLicense: false }

/** A project that ships one, so `#import` appends the `migrate` row at all. */
const MIGRATION = {
	datatable_name: 'main',
	sql: 'CREATE TABLE IF NOT EXISTS "calendly"."config" (id int)',
	sql_down: '',
	enabled: true
}
const depsWithMigration = { reviewMigrations: async () => [MIGRATION], hasEeLicense: false }

describe('planTag', () => {
	// A run handed back after a remount is checked against the plan being rendered. Tagging
	// it with that plan instead of its own would make the check pass by construction.
	it('identifies the plan the run belongs to', () => {
		const a = new ImportExecution(PLAN, deps)
		const b = new ImportExecution(
			{ ...PLAN, destination: { kind: 'existing', workspaceId: 'ws-b' } },
			deps
		)
		expect(a.planTag).not.toBe(b.planTag)
	})

	it('ignores the folder, which stays editable after the run is made', () => {
		const a = new ImportExecution(PLAN, deps)
		const tag = a.planTag
		a.setFolder('somewhere-else')
		expect(a.planTag).toBe(tag)
	})

	it('separates two projects going to the same workspace', () => {
		const a = new ImportExecution(PLAN, deps)
		const b = new ImportExecution({ ...PLAN, slug: 'bitly' }, deps)
		expect(a.planTag).not.toBe(b.planTag)
	})
})

describe('abandoning mid-import', () => {
	beforeEach(() => {
		hooks.afterFirstItem = undefined
		hooks.afterMigrationsStart = undefined
	})

	it('does not report done, so the resumed step offers Retry rather than Continue', async () => {
		const run = new ImportExecution(PLAN, deps)
		hooks.afterFirstItem = () => run.abandon()
		await run.run()
		// `installProject` returned early exactly as it does on success; calling that done
		// would report a clean import over items that never started.
		expect(run.done).toBe(false)
		expect(run.error).toMatch(/stopped/i)
	})

	it('reports done when nothing abandoned it', async () => {
		const run = new ImportExecution(PLAN, deps)
		await run.run()
		expect(run.done).toBe(true)
		expect(run.itemResults.length).toBe(3)
	})

	it('stops the migrate row spinning when it is abandoned mid-migration', async () => {
		const run = new ImportExecution(PLAN, depsWithMigration)
		// After `onMigrationsStart`, which is where the row is actually set to running —
		// the real one fires it at the head of the migration loop, past every item loop.
		hooks.afterMigrationsStart = () => run.abandon()
		await run.run()
		const migrate = run.tasks.find((t) => t.key === 'migrate')
		// Guards the test itself: without a migration in the export there is no row, and the
		// assertion below would pass over a branch that never ran.
		expect(migrate).toBeDefined()
		// A row left on `running` reads as work still in progress on a run that has stopped.
		expect(migrate?.status).not.toBe('running')
		expect(run.done).toBe(false)
	})
})
