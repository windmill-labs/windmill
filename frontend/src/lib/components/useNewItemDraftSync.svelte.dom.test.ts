import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { flushSync } from 'svelte'

const save = vi.fn()
const remove = vi.fn()
const discard = vi.fn()
const forcePersist = vi.fn(async () => {})
const flush = vi.fn(async () => {})
vi.mock('$lib/userDraft.svelte', () => ({
	UserDraft: {
		save: (...a: unknown[]) => save(...a),
		remove: (...a: unknown[]) => remove(...a),
		discard: (...a: unknown[]) => discard(...a),
		forcePersist: (...a: unknown[]) => forcePersist(...a)
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
		expect(discard.mock.calls[0].slice(0, 2)).toEqual(['resource', 'u/me/auto_name'])
		expect(save).toHaveBeenLastCalledWith('resource', 'u/me/renamed', { n: 2 }, { workspace: 'w' })

		form.pathError = 'path already used'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(discard.mock.calls.at(-1)?.slice(0, 2)).toEqual(['resource', 'u/me/renamed'])
		expect(draftPath).toBe('')

		form.pathError = ''
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		expect(save).toHaveBeenLastCalledWith('resource', 'u/me/renamed', { n: 2 }, { workspace: 'w' })

		cleanup()
		expect(discard).toHaveBeenCalledTimes(2)
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

	/** A draft-only item arrives with a draft already stored under the path the
	 * editor opened. Renaming it has to MOVE that row, so the helper must be
	 * told which key it inherited or it would leave a second one behind. */
	it('moves an adopted key on rename instead of leaving it behind', async () => {
		const form = $state({ path: 'u/me/adopted', n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				contentTouched: () => false,
				value: () => ({ n: form.n })
			})
			sync.adopt('w', 'u/me/adopted', { n: 1 })
		})
		flushSync()

		form.path = 'u/me/moved'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		await sync!.flush()
		expect(discard.mock.calls.at(-1)?.slice(0, 2)).toEqual(['resource', 'u/me/adopted'])
		expect(save).toHaveBeenLastCalledWith('resource', 'u/me/moved', { n: 1 }, { workspace: 'w' })
		cleanup()
	})

	/** The list synthesizes a draft-only row from the path INSIDE the draft while
	 * get and delete address the key, so a stored draft has to describe its own
	 * key. A rename the draft can't follow yet must not smuggle the new path
	 * into the row it is still living in. */
	it('stores a draft describing the key it lives under, not an unusable path', async () => {
		const form = $state({ path: 'u/me/home', pathError: '', n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => form.pathError,
				contentTouched: () => false,
				value: () => ({ path: form.path, n: form.n }),
				keyed: (v, p) => ({ ...v, path: p })
			})
			sync.adopt('w', 'u/me/home', { path: 'u/me/home', n: 1 })
		})
		flushSync()

		// Renamed to a path the draft cannot move to, then edited.
		form.path = 'u/me/taken'
		form.pathError = 'path already used'
		form.n = 2
		flushSync()
		vi.advanceTimersByTime(2000)
		flushSync()
		await sync!.flush()
		expect(save).toHaveBeenLastCalledWith(
			'resource',
			'u/me/home',
			{ path: 'u/me/home', n: 2 },
			{ workspace: 'w' }
		)
	})

	/** Renaming away suspends the handle pinned to the original key; renaming
	 * back has to resume it, or the draft is deleted at both keys and written
	 * to neither. */
	it('resumes a key it returns to after abandoning it', async () => {
		const form = $state({ path: 'u/me/there', n: 1 })
		const events: string[] = []
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				contentTouched: () => false,
				value: () => ({ n: form.n }),
				onAbandonKey: (_ws, p) => events.push(`abandon:${p}`),
				onResumeKey: (_ws, p) => events.push(`resume:${p}`)
			})
			sync.adopt('w', 'u/me/there', { n: 1 })
		})
		flushSync()

		form.path = 'u/me/away'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()

		form.path = 'u/me/there'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		await sync!.flush()
		expect(events).toEqual(['abandon:u/me/there', 'abandon:u/me/away', 'resume:u/me/there'])
		expect(save).toHaveBeenLastCalledWith('resource', 'u/me/there', { n: 1 }, { workspace: 'w' })
		// The resumed handle's own change detection can no-op this write away.
		expect(forcePersist).toHaveBeenCalledWith('resource', 'u/me/there', { workspace: 'w' })
		cleanup()
	})

	/** An editor's own autosave handle stays pinned to the path it opened, so
	 * once the draft moves the helper has to hand that key back for suspension —
	 * otherwise the handle's next write recreates the row just deleted. */
	it('reports the key it abandons so a pinned handle can be suspended', async () => {
		const form = $state({ path: 'u/me/pinned', n: 1 })
		const abandoned: string[] = []
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => '',
				contentTouched: () => false,
				value: () => ({ n: form.n }),
				onAbandonKey: (_ws, p) => abandoned.push(p)
			})
			sync.adopt('w', 'u/me/pinned', { n: 1 })
		})
		flushSync()
		// Untouched: the key is still in use, nothing handed back.
		vi.advanceTimersByTime(2000)
		flushSync()
		expect(abandoned).toEqual([])

		form.path = 'u/me/elsewhere'
		flushSync()
		vi.advanceTimersByTime(1000)
		flushSync()
		await sync!.flush()
		expect(abandoned).toEqual(['u/me/pinned'])
		cleanup()
	})

	/** An adopted draft exists whether or not the user edits it, so the
	 * touched gate that keeps an untouched NEW item from leaving a row must not
	 * apply — deleting here would wipe the item the editor is showing. An
	 * invalid path likewise leaves it where it is rather than dropping it. */
	it('keeps an adopted draft through an untouched open and an invalid path', async () => {
		const form = $state({ path: 'u/me/kept', pathError: '', n: 1 })
		let sync: ReturnType<typeof useNewItemDraftSync> | undefined
		const cleanup = $effect.root(() => {
			sync = useNewItemDraftSync({
				itemKind: 'resource',
				enabled: () => true,
				workspace: () => 'w',
				path: () => form.path,
				pathError: () => form.pathError,
				contentTouched: () => false,
				value: () => ({ n: form.n })
			})
			sync.adopt('w', 'u/me/kept', { n: 1 })
		})
		flushSync()
		vi.advanceTimersByTime(2000)
		flushSync()
		expect(discard).not.toHaveBeenCalled()

		form.pathError = 'path already used'
		flushSync()
		vi.advanceTimersByTime(2000)
		flushSync()
		await sync!.flush()
		expect(discard).not.toHaveBeenCalled()
		expect(sync!.draftPath).toBe('u/me/kept')
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
		expect(discard.mock.calls.at(-1)?.slice(0, 2)).toEqual(['variable', 'u/me/item'])

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
		expect(discard).toHaveBeenCalledTimes(1)

		cleanup()
	})
})
