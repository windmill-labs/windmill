import { beforeEach, describe, expect, it, vi } from 'vitest'

const account = vi.hoisted(() => ({ email: 'ada@example.com' as string | undefined }))
const listRunnables = vi.hoisted(() => vi.fn())
const listResource = vi.hoisted(() => vi.fn())

vi.mock('$lib/userScopedStorage', () => ({
	scopedKey: (base: string) => (account.email ? `${base}::${account.email}` : undefined)
}))
vi.mock('$lib/onboardingProfile', () => ({ onboardingProfile: async () => undefined }))
vi.mock('$lib/components/sidebar/FavoriteMenu.svelte', () => ({ favoriteManager: { current: [] } }))
vi.mock('$lib/gen', () => ({
	ScriptService: { listRunnables },
	ResourceService: { listResource }
}))

import { readComposerSignals } from './starterPrompts.svelte'

const draftOf = (owner: string) => ({
	items: [{ type: 'flow', path: `u/${owner}/secret_plan`, summary: '', is_draft: true }]
})

beforeEach(() => {
	listRunnables.mockReset()
	listResource.mockReset()
	listResource.mockResolvedValue([])
})

describe('readComposerSignals', () => {
	it("never answers one account's signals for another", async () => {
		listRunnables.mockResolvedValueOnce(draftOf('ada'))
		expect((await readComposerSignals('acme')).myDraft?.path).toBe('u/ada/secret_plan')

		// Another account in the same browser, well inside the reuse window.
		account.email = 'bob@example.com'
		listRunnables.mockResolvedValueOnce({ items: [] })
		expect((await readComposerSignals('acme')).myDraft).toBeUndefined()
		expect(listRunnables).toHaveBeenCalledTimes(2)
	})

	it('picks the newest database and the preferred channel', async () => {
		listRunnables.mockResolvedValueOnce({ items: [] })
		listResource.mockResolvedValueOnce([
			{ path: 'f/mail/smtp', resource_type: 'smtp', edited_at: '2026-10-03' },
			{ path: 'f/data/old_db', resource_type: 'postgresql', edited_at: '2026-01-01' },
			{ path: 'f/team/slack', resource_type: 'slack', edited_at: '2026-02-01' },
			{ path: 'f/data/main_db', resource_type: 'mysql', edited_at: '2026-09-01' }
		])
		const signals = await readComposerSignals('globex')
		expect(signals.database).toEqual({ type: 'mysql', path: 'f/data/main_db', name: 'main_db' })
		expect(signals.channel?.path).toBe('f/team/slack')
	})
})
