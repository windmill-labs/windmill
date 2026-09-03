import { untrack } from 'svelte'
import { UserDraft, type UserDraftItemKind } from '$lib/userDraft.svelte'

/** Longer than `Path`'s 500ms debounced existence check, so the key never
 * lands on a half-typed path and the check's verdict is in before a commit. */
const COMMIT_DELAY_MS = 1000

export interface NewItemDraftSyncOptions<V> {
	itemKind: UserDraftItemKind
	/** Reactive: false leaves the helper inert (edit mode — the handle syncs). */
	enabled: () => boolean
	/** Reactive workspace the draft is stored in. */
	workspace: () => string | undefined
	/** Reactive path field (`''` while none). */
	path: () => string
	/** Reactive `Path` validation error (`''` when valid). */
	pathError: () => string
	/** Reactive: the user edited the name or the content. `Path` auto-fills a
	 * name on mount, so opening and closing an untouched drawer must not leave
	 * a draft behind. */
	touched: () => boolean
	/** Reactive deep read of the value to persist (`$state.snapshot` of the
	 * form state); `undefined` while there is nothing to persist. */
	value: () => V | undefined
}

export interface NewItemDraftSync {
	/** Storage path of the persisted draft, `''` when none. */
	readonly draftPath: string
	/** Delete the persisted draft and stop mirroring, once the item is created. */
	finish(): void
	/** Re-arm for the next drawer session (an editor instance that outlives
	 * its drawer). Forgets the previous session's key without deleting it: a
	 * draft left behind by closing the drawer is the point. */
	reset(): void
}

/**
 * Server-side autosave for a drawer editor's brand-new item. Those editors
 * key their `useMany` handle on the path they were opened with, which is
 * empty for a new item, so the handle is detached and never POSTs. This
 * mirrors the form into a draft keyed by the typed path instead — the key
 * the list pages' draft-only rows and the get-by-path draft overlay resolve —
 * and moves it (delete the old key, write the new) as the path changes.
 */
export function useNewItemDraftSync<V>(opts: NewItemDraftSyncOptions<V>): NewItemDraftSync {
	let draftPath = $state('')
	let finished = $state(false)
	// Last key actually written: a moved or finished draft deletes exactly the
	// row it left behind, and component teardown deletes nothing.
	let writtenPath = ''

	$effect(() => {
		if (!opts.enabled() || finished) return
		const p = opts.path()
		const target = p !== '' && opts.pathError() === '' && opts.touched() ? p : ''
		if (target === untrack(() => draftPath)) return
		const t = setTimeout(() => (draftPath = target), COMMIT_DELAY_MS)
		return () => clearTimeout(t)
	})

	$effect(() => {
		if (!opts.enabled() || finished) return
		const ws = opts.workspace()
		const p = draftPath
		const v = opts.value()
		untrack(() => {
			if (!ws) return
			if (writtenPath && writtenPath !== p) {
				UserDraft.remove(opts.itemKind, writtenPath, { workspace: ws })
				writtenPath = ''
			}
			if (!p || v === undefined) return
			UserDraft.save(opts.itemKind, p, v, { workspace: ws })
			writtenPath = p
		})
	})

	return {
		get draftPath() {
			return draftPath
		},
		finish() {
			finished = true
			const ws = untrack(() => opts.workspace())
			if (writtenPath && ws) UserDraft.remove(opts.itemKind, writtenPath, { workspace: ws })
			writtenPath = ''
			draftPath = ''
		},
		reset() {
			finished = false
			writtenPath = ''
			draftPath = ''
		}
	}
}
