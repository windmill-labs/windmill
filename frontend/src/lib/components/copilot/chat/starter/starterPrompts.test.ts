import { beforeEach, describe, expect, it, vi } from 'vitest'

const account = vi.hoisted(() => ({ email: 'ada@example.com' as string | undefined }))
const listRunnables = vi.hoisted(() => vi.fn())

vi.mock('$lib/userScopedStorage', () => ({
	scopedKey: (base: string) => (account.email ? `${base}::${account.email}` : undefined)
}))
vi.mock('$lib/onboardingProfile', () => ({ onboardingProfile: async () => undefined }))
vi.mock('$lib/gen', () => ({
	ScriptService: { listRunnables },
	JobService: {
		countCompletedJobs: async () => 0,
		getQueueCount: async () => ({ database_length: 0, suspended: 0 })
	},
	WorkspaceService: { getSettings: async () => ({}) },
	WorkerService: { listWorkers: async () => [] }
}))

import { readComposerSignals } from './starterPrompts.svelte'

const role = { operator: false, workspaceAdmin: false, instanceAdmin: false }
const draftOf = (owner: string) => ({
	items: [{ type: 'flow', path: `u/${owner}/secret_plan`, summary: '', is_draft: true }]
})

beforeEach(() => {
	listRunnables.mockReset()
})

describe('readComposerSignals', () => {
	it("never answers one account's signals for another", async () => {
		listRunnables.mockResolvedValueOnce(draftOf('ada'))
		expect((await readComposerSignals('acme', role)).myDraft?.path).toBe('u/ada/secret_plan')

		// Another account in the same browser, well inside the reuse window.
		account.email = 'bob@example.com'
		listRunnables.mockResolvedValueOnce({ items: [] })
		expect((await readComposerSignals('acme', role)).myDraft).toBeUndefined()
		expect(listRunnables).toHaveBeenCalledTimes(2)
	})
})
