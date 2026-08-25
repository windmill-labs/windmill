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
// What the destination already holds. A test sets this to stand in for a workspace that has
// some of the bundle in it — a half-finished run, or an existing workspace.
const present = vi.hoisted(() => ({
	paths: new Set<string>(),
	/** Fires when the probe is entered, so a test can abandon while it is in flight. */
	onProbe: undefined as (() => void) | undefined
}))
vi.mock('./probe', async (orig) => ({
	...(await orig<typeof import('./probe')>()),
	probeWorkspace: vi.fn(async () => ({ exists: false, ours: false })),
	probeImportedPaths: vi.fn(async () => {
		present.onProbe?.()
		return present.paths
	})
}))
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
			// Keyed exactly as the real `installProject` keys it, so this stand-in cannot drift
			// into testing a contract the production code does not have.
			if (args.alreadyPresent?.has(`script:${path}`)) {
				args.onResult({ path, ok: true, skipped: true })
				continue
			}
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
		present.paths = new Set()
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

/**
 * A retry must write only what is missing. Resending the whole bundle turns every item that
 * already landed into an "already exists" failure — a wall of red over work that succeeded.
 * What is already there is skipped, and reported as skipped rather than as imported, because
 * the checklist has to account for every item the project ships.
 */
describe('retrying over what is already there', () => {
	beforeEach(() => {
		present.paths = new Set()
	})

	it('writes nothing for a path the destination already holds', async () => {
		present.paths = new Set(['script:a', 'script:b'])
		const run = new ImportExecution(PLAN, deps)
		await run.run()
		const byPath = new Map(run.itemResults.map((r) => [r.path, r]))
		expect(byPath.get('a')?.skipped).toBe(true)
		expect(byPath.get('b')?.skipped).toBe(true)
		expect(byPath.get('c')?.skipped).toBeUndefined()
	})

	it('still accounts for every item, so the checklist stays complete', async () => {
		present.paths = new Set(['script:a', 'script:b'])
		const run = new ImportExecution(PLAN, deps)
		await run.run()
		expect(run.itemResults.length).toBe(3)
		expect(run.done).toBe(true)
	})

	it('says what it did rather than claiming to have imported all of it', async () => {
		present.paths = new Set(['script:a', 'script:b'])
		const run = new ImportExecution(PLAN, deps)
		await run.run()
		const importRow = run.tasks.find((t) => t.key === 'import')
		expect(importRow?.detail).toMatch(/1 imported/)
		expect(importRow?.detail).toMatch(/2 already there/)
	})

	it('imports everything when the destination is empty', async () => {
		const run = new ImportExecution(PLAN, deps)
		await run.run()
		expect(run.itemResults.every((r) => !r.skipped)).toBe(true)
		expect(run.tasks.find((t) => t.key === 'import')?.detail).toMatch(/3 imported/)
	})
})

describe('abandoning while the presence probe is in flight', () => {
	beforeEach(() => {
		present.paths = new Set()
		present.onProbe = undefined
	})

	/**
	 * `import` goes `running` before the probe is asked, so returning straight out of an
	 * abandon here leaves a spinner on a run that has stopped — next to an enabled Retry and
	 * with no explanation of why it stopped.
	 */
	it('leaves no task running', async () => {
		const run = new ImportExecution(PLAN, deps as any)
		// Abandoned from inside the probe: `import` is already `running` by then, and the
		// executor's next look at the flag is the early return under test.
		present.onProbe = () => run.abandon()
		await run.run()
		expect(run.tasks.find((t) => t.status === 'running')).toBeUndefined()
		expect(run.tasks.find((t) => t.key === 'import')?.status).toBe('failed')
		expect(run.error).toBeTruthy()
		expect(run.done).toBe(false)
	})
})
