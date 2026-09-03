import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { flushSync } from 'svelte'

const save = vi.fn()
const remove = vi.fn()
const flush = vi.fn(async () => {})
vi.mock('$lib/userDraft.svelte', () => ({
	UserDraft: {
		save: (...a: unknown[]) => save(...a),
		remove: (...a: unknown[]) => remove(...a)
	}
}))
vi.mock('$lib/userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: { flush: (...a: unknown[]) => flush(...(a as [])) }
}))

import { useNewItemDraftSync } from './useNewItemDraftSync.svelte'

beforeEach(() => vi.useFakeTimers())
afterEach(() => {
	vi.useRealTimers()
	vi.clearAllMocks()
})

/** Drives the helper through a new-item drawer session and pins the writes it
 * must and must not make: nothing for an untouched form (the path field
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
				contentTouched: () => form.touched,
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

	/** The commit is delayed so the key can't land on a half-typed path, and the
	 * editor is destroyed the moment its drawer closes. Closing right after the
	 * first edit therefore tears down mid-delay, and the draft must survive it. */
	it('persists a commit still pending when the editor is torn down', async () => {
		const form = $state({ path: 'u/me/quick', touched: false, n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				contentTouched: () => form.touched,
				value: () => ({ n: form.n })
			})
		})
		flushSync()
		form.touched = true
		flushSync()

		// Torn down half-way through the commit delay.
		vi.advanceTimersByTime(500)
		cleanup()
		vi.advanceTimersByTime(500)
		expect(save).toHaveBeenCalledWith('resource', 'u/me/quick', { n: 1 }, { workspace: 'w' })

		// The drawer's close handler awaits this, so the list refetch behind it
		// sees the row rather than racing the syncer's debounce.
		await sync!.flush()
		expect(flush).toHaveBeenCalledWith({
			workspace: 'w',
			itemKind: 'resource',
			path: 'u/me/quick'
		})
	})

	/** A close cuts the commit delay short, so `Path`'s debounced existence
	 * check may not have run. Keying a draft on an occupied path would hand it
	 * to the item already there, and saving from that item would overwrite it. */
	it('refuses to commit a forced flush onto an occupied path', async () => {
		const form = $state({ path: 'u/me/taken', touched: false, n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				// Still clear: the check that would set it has not run yet.
				pathError: () => '',
				contentTouched: () => form.touched,
				value: () => ({ n: form.n }),
				pathIsFree: async () => false
			})
		})
		flushSync()
		form.touched = true
		flushSync()

		await sync!.flush()
		expect(save).not.toHaveBeenCalled()
		cleanup()
	})

	/** A move deletes the key it left on the same debounce as the write, so the
	 * close-time flush has to settle both or the list refetch renders a ghost
	 * row at the old path. */
	it('settles the key a move deleted, not just the one it wrote', async () => {
		const form = $state({ path: 'u/me/first', touched: true, n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				contentTouched: () => form.touched,
				value: () => ({ n: form.n })
			})
		})
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()

		form.path = 'u/me/second'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()

		await sync!.flush()
		const flushed = flush.mock.calls.map((c: any[]) => c[0].path)
		expect(flushed).toContain('u/me/first')
		expect(flushed).toContain('u/me/second')
		cleanup()
	})

	/** `Path` auto-fills a unique name on mount and flips its own `dirty` on any
	 * keyup, tabbing included. Only a departure from that name counts, or an
	 * untouched drawer would leave a phantom row behind. */
	it('treats the auto-filled path as untouched but a typed one as an edit', () => {
		const form = $state({ path: '', n: 1 })
		const cleanup = $effect.root(() => {
			useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				contentTouched: () => false,
				value: () => ({ n: form.n })
			})
		})
		flushSync()
		// `Path` fills its generated name in after mount.
		form.path = 'u/me/lucky_resource'
		flushSync()
		vi.advanceTimersByTime(2000)
		flushSync()
		expect(save).not.toHaveBeenCalled()

		form.path = 'u/me/typed_by_hand'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenCalledWith(
			'resource',
			'u/me/typed_by_hand',
			{ n: 1 },
			{ workspace: 'w' }
		)
		cleanup()
	})

	it('finish deletes the persisted key and stops mirroring until reset', async () => {
		const form = $state({ path: 'u/me/item', touched: true, n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'variable',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				contentTouched: () => form.touched,
				value: () => ({ n: form.n })
			})
		})
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenCalledTimes(1)

		await sync!.finish()
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
