import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { flushSync } from 'svelte'

const save = vi.fn()
const remove = vi.fn()
vi.mock('$lib/userDraft.svelte', () => ({
	UserDraft: {
		save: (...a: unknown[]) => save(...a),
		remove: (...a: unknown[]) => remove(...a)
	}
}))

import { useNewItemDraftSync } from './useNewItemDraftSync.svelte'

beforeEach(() => vi.useFakeTimers())
afterEach(() => {
	vi.useRealTimers()
	vi.clearAllMocks()
})

/** Drives the helper through a new-item drawer session and pins the writes
 * it must and must not make: nothing for an untouched form (the path field
 * auto-fills a name on mount), a move that deletes the key it left, a delete
 * once the path fails validation, and no delete at all on teardown — a draft
 * left behind by closing the drawer is the feature. */
describe('useNewItemDraftSync', () => {
	it('writes only a touched form, follows the path, and leaves the draft on teardown', () => {
		const form = $state({ path: 'u/me/auto_name', pathError: '', touched: false, n: 1 })
		let draftPath = ''
		const cleanup = $effect.root(() => {
			const sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => form.pathError,
				touched: () => form.touched,
				value: () => ({ n: form.n })
			})
			$effect(() => {
				draftPath = sync.draftPath
			})
		})
		flushSync()
		vi.advanceTimersByTime(2000)
		flushSync()
		expect(save).not.toHaveBeenCalled()

		form.touched = true
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenCalledWith('resource', 'u/me/auto_name', { n: 1 }, { workspace: 'w' })
		expect(draftPath).toBe('u/me/auto_name')

		form.n = 2
		flushSync()
		expect(save).toHaveBeenLastCalledWith(
			'resource',
			'u/me/auto_name',
			{ n: 2 },
			{ workspace: 'w' }
		)

		form.path = 'u/me/renamed'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(remove).toHaveBeenCalledWith('resource', 'u/me/auto_name', { workspace: 'w' })
		expect(save).toHaveBeenLastCalledWith('resource', 'u/me/renamed', { n: 2 }, { workspace: 'w' })

		form.pathError = 'path already used'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(remove).toHaveBeenLastCalledWith('resource', 'u/me/renamed', { workspace: 'w' })
		expect(draftPath).toBe('')

		form.pathError = ''
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenLastCalledWith('resource', 'u/me/renamed', { n: 2 }, { workspace: 'w' })

		cleanup()
		expect(remove).toHaveBeenCalledTimes(2)
	})

	it('finish deletes the persisted key and stops mirroring until reset', () => {
		const form = $state({ path: 'u/me/item', touched: true, n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'variable',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				touched: () => form.touched,
				value: () => ({ n: form.n })
			})
		})
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenCalledTimes(1)

		sync!.finish()
		flushSync()
		expect(remove).toHaveBeenCalledWith('variable', 'u/me/item', { workspace: 'w' })

		form.n = 2
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenCalledTimes(1)

		sync!.reset()
		form.n = 3
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenLastCalledWith('variable', 'u/me/item', { n: 3 }, { workspace: 'w' })
		expect(remove).toHaveBeenCalledTimes(1)

		cleanup()
	})
})
