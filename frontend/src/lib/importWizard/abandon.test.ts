import { beforeEach, describe, expect, it, vi } from 'vitest'

// Everything the executor reaches over the network, stubbed. The two behaviours under test
// are decisions it makes around those calls, not the calls themselves.
vi.mock('$lib/gen', () => ({
	WorkspaceService: { createWorkspace: vi.fn(), listDataTables: vi.fn(async () => []) },
	UserService: { whoami: vi.fn(async () => ({ username: 'u' })) }
}))
vi.mock('$lib/storeUtils', () => ({ switchWorkspace: vi.fn() }))
vi.mock('$lib/user', () => ({ getUserExt: vi.fn(async () => ({ username: 'u' })) }))
// Lets a test abandon *during* the write loop, which is the only way it happens for real:
// `run()` clears the flag on entry so a retry can proceed.
const hooks = vi.hoisted(() => ({ afterFirstItem: undefined as (() => void) | undefined }))

vi.mock('$lib/components/workspaceSettings/projectInstall', () => ({
	installProject: vi.fn(async (args: any) => {
		// Behaves like the real one: reports what it wrote, and returns early the moment
		// `stopped` goes true — returning the same way it does on success.
		for (const path of ['a', 'b', 'c']) {
			if (args.stopped?.() === true) return
			args.onResult({ path, ok: true })
			hooks.afterFirstItem?.()
			hooks.afterFirstItem = undefined
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
import { clearParkedImport, parkImport, resumableImport } from './parking'

const PLAN = { slug: 'calendly', destination: { kind: 'existing' as const, workspaceId: 'ws-a' } }
const deps = { reviewMigrations: async () => [], hasEeLicense: false }

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
		clearParkedImport()
		hooks.afterFirstItem = undefined
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

	it('keeps the parked workspace so the link the leave dialog promises still resumes', async () => {
		parkImport({ slug: 'calendly', workspaceId: 'ws-a' })
		const run = new ImportExecution(PLAN, deps)
		hooks.afterFirstItem = () => run.abandon()
		await run.run()
		expect(resumableImport('calendly', 'ws-a')).toBe(true)
	})

	it('clears it on a clean finish, so a later import reaches its own create', async () => {
		parkImport({ slug: 'calendly', workspaceId: 'ws-a' })
		const run = new ImportExecution(PLAN, deps)
		await run.run()
		expect(resumableImport('calendly', 'ws-a')).toBe(false)
	})
})
