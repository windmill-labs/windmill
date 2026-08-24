import { beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * The list endpoints paginate. A probe that reads one page answers correctly for a small
 * project and under-reports a large one, which puts every item past the first page back
 * through a create call that rejects it.
 */

const calls = vi.hoisted(() => ({ script: [] as { page?: number; perPage?: number }[] }))

/** 150 scripts in the folder — more than one page at any page size the probe might pick. */
const ALL = Array.from({ length: 150 }, (_, i) => ({ path: `f/calendly/s${i}` }))

vi.mock('$lib/gen', () => ({
	ScriptService: {
		listScripts: vi.fn(async (args: any) => {
			calls.script.push({ page: args.page, perPage: args.perPage })
			const per = args.perPage ?? 30
			return ALL.slice((args.page - 1) * per, args.page * per)
		})
	},
	FlowService: { listFlows: vi.fn(async () => []) },
	AppService: { listApps: vi.fn(async () => []) },
	ResourceService: { listResource: vi.fn(async () => []) },
	WorkspaceService: { listWorkspaces: vi.fn(async () => []), getDatatableFullSchema: vi.fn() }
}))
vi.mock('$lib/components/triggers/workspaceTriggersList', () => ({
	listAllWorkspaceTriggers: vi.fn(async () => ({ triggers: [], failedKinds: [] }))
}))

import { probeImportedPaths } from './probe'
import { presenceKey } from '$lib/components/workspaceSettings/projectInstall'

describe('probeImportedPaths paging', () => {
	beforeEach(() => {
		calls.script = []
	})

	it('reads every page, not just the first', async () => {
		const found = await probeImportedPaths('w', 'calendly')
		expect(found.size).toBe(150)
		expect(found.has(presenceKey('script', 'f/calendly/s0'))).toBe(true)
		// The one that a single-page probe misses, and would then try to create again.
		expect(found.has(presenceKey('script', 'f/calendly/s149'))).toBe(true)
	})

	it('stops on the first short page rather than asking forever', async () => {
		await probeImportedPaths('w', 'calendly')
		const pages = calls.script.map((c) => c.page)
		expect(pages).toEqual([1, 2])
		expect(new Set(calls.script.map((c) => c.perPage))).toEqual(new Set([100]))
	})
})
