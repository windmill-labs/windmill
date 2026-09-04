import { untrack } from 'svelte'
import { UserDraft, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'

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
	/** Reactive: the user edited the form's content. The path is not part of
	 * this — `Path` auto-fills a name on mount, and this helper tracks a
	 * departure from that name itself. */
	contentTouched: () => boolean
	/** Reactive deep read of the value to persist (`$state.snapshot` of the
	 * form state); `undefined` while there is nothing to persist. */
	value: () => V | undefined
	/** Whether nothing is deployed at `path` yet. Consulted before every commit:
	 * `Path`'s own check is debounced and may not have answered. */
	pathIsFree?: (path: string) => Promise<boolean>
	/** Called for a key this helper has stopped writing to, after its row is
	 * deleted. An editor whose own autosave handle is pinned to that key MUST
	 * suspend it here, or the handle's next write would recreate the row. */
	onAbandonKey?: (workspace: string, path: string) => void
	/** Called before writing to a key previously passed to `onAbandonKey`, so
	 * the editor can resume the handle it suspended there. */
	onResumeKey?: (workspace: string, path: string) => void
	/** Return `value` with its own path set to `path`. A stored draft has to
	 * describe the key it lives under: the list synthesizes a draft-only row
	 * from the path INSIDE the draft, while get and delete address the key, so
	 * letting the two diverge makes the row unreachable. Divergence is normal
	 * while the form holds a path the draft cannot move to yet. */
	keyed?: (value: V, path: string) => V
}

export interface NewItemDraftSync<V> {
	/** Storage path of the persisted draft, `''` when none. */
	readonly draftPath: string
	/** Take ownership of a draft that already exists at `path` — a draft-only
	 * item the editor loaded. Without this the helper believes it has written
	 * nothing, and a rename would add a second row instead of moving this one. */
	adopt(workspace: string, path: string, value: V): void
	/** Commit anything still pending and settle it server-side. Callers MUST
	 * await this before a list refetch (both the commit delay and the syncer's
	 * own debounce outlive a closing drawer, so a refetch would miss the row). */
	flush(): Promise<void>
	/** Delete the persisted draft and stop mirroring, once the item is created. */
	finish(): Promise<void>
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
export function useNewItemDraftSync<V>(opts: NewItemDraftSyncOptions<V>): NewItemDraftSync<V> {
	let draftPath = $state('')
	let finished = $state(false)
	// The key last written, workspace included: a move or a delete has to target
	// the row actually left behind, not wherever the form points now.
	let written: { workspace: string; path: string } | undefined
	let writtenValue: string | undefined
	// Every key this session has touched and not yet settled — the deletes a
	// move leaves behind included, since those POST on the same debounce as the
	// write and would otherwise still be pending when the list refetches.
	let unsettled: { workspace: string; path: string }[] = []
	// The commit the timer will make, snapshotted at schedule time so it still
	// lands once the editor is gone: closing a drawer a keystroke after the
	// first edit must keep the draft, so a pending commit is never cancelled by
	// teardown — only superseded by a newer one, or consumed by `flush`.
	let pending:
		| {
				timer: ReturnType<typeof setTimeout>
				workspace: string | undefined
				key: string
				value: V | undefined
		  }
		| undefined
	// `Path` auto-fills a unique name on mount, so a non-empty path is no
	// evidence the user did anything. Only a departure from the name it settled
	// on counts (`Path.dirty` can't: it flips on any keyup, tabbing included).
	let autoPath: string | undefined
	// An adopted draft already exists: it is kept regardless of whether the user
	// edits anything, and an invalid path leaves it where it is rather than
	// deleting it. Only a brand-new item's draft is gated on being touched.
	let adopted = false
	// Keys handed to `onAbandonKey`, so a return to one can resume it.
	const abandoned = new Set<string>()

	function markUnsettled(workspace: string, path: string): void {
		if (!unsettled.some((k) => k.workspace === workspace && k.path === path)) {
			unsettled.push({ workspace, path })
		}
	}

	function touched(): boolean {
		const p = opts.path()
		if (autoPath === undefined && p !== '') autoPath = p
		return opts.contentTouched() || (p !== '' && p !== autoPath)
	}

	function write(workspace: string | undefined, path: string, value: V | undefined): void {
		if (written && (written.path !== path || written.workspace !== workspace)) {
			// `discard`, not `remove`: an adopted key is the editor's own handle key,
			// and `remove` blanks that live cell — the form would lose its state
			// mid-rename. The fallback leaves the cell holding what the form holds.
			UserDraft.discard(opts.itemKind, written.path, value, { workspace: written.workspace })
			opts.onAbandonKey?.(written.workspace, written.path)
			abandoned.add(`${written.workspace}/${written.path}`)
			markUnsettled(written.workspace, written.path)
			written = undefined
			writtenValue = undefined
		}
		if (!workspace || !path || value === undefined) return
		// Stored describing its own key: while the form holds a path the draft
		// cannot move to yet, the row must still name where it actually lives.
		const stored = opts.keyed ? opts.keyed(value, path) : value
		const serialized = JSON.stringify(stored)
		if (written && serialized === writtenValue) return
		const resumed = abandoned.delete(`${workspace}/${path}`)
		if (resumed) opts.onResumeKey?.(workspace, path)
		UserDraft.save(opts.itemKind, path, stored, { workspace })
		if (resumed) {
			// A live handle at this key mirrors CHANGES, and its baseline advanced
			// while it was suspended — the value we just restored can equal it, so
			// nothing would be sent and the row we deleted on the way out would
			// never come back. Safe here: only a never-deployed draft is keyed this
			// way, so there is no baseline this could overwrite.
			void UserDraft.forcePersist(opts.itemKind, path, { workspace })
		}
		markUnsettled(workspace, path)
		written = { workspace, path }
		writtenValue = serialized
	}

	function dropPending(): void {
		if (!pending) return
		clearTimeout(pending.timer)
		pending = undefined
	}

	/** Validate the pending key, then take it. The timer is cleared first and
	 * the payload kept, so the validation below can't race a second commit of
	 * the same transition; a newer transition scheduled meanwhile wins. */
	async function commitPending(): Promise<void> {
		const p = pending
		if (!p) return
		clearTimeout(p.timer)
		if (p.key) {
			// `Path` debounces its own existence check and may not have answered
			// yet, so the key is verified here rather than trusted: keying a draft
			// on a path that already holds an item would take it as an edit of
			// that item, and saving from there would overwrite its value.
			const free =
				opts.pathError() === '' && (opts.pathIsFree ? await opts.pathIsFree(p.key) : true)
			if (pending !== p) return
			if (!free) {
				pending = undefined
				return
			}
		}
		pending = undefined
		draftPath = p.key
		write(p.workspace, p.key, p.value)
	}

	$effect(() => {
		if (!opts.enabled() || finished) return
		const p = opts.path()
		const usable = p !== '' && opts.pathError() === ''
		const key = usable && (adopted || touched()) ? p : adopted ? untrack(() => draftPath) : ''
		const workspace = opts.workspace()
		const value = opts.value()
		if (key === untrack(() => draftPath)) {
			// Back on the committed key: a transition away from it is stale.
			untrack(dropPending)
			return
		}
		untrack(() => {
			dropPending()
			pending = {
				timer: setTimeout(() => void commitPending(), COMMIT_DELAY_MS),
				workspace,
				key,
				value
			}
		})
	})

	$effect(() => {
		if (!opts.enabled() || finished) return
		const workspace = opts.workspace()
		const key = draftPath
		const value = opts.value()
		untrack(() => {
			if (key) write(workspace, key, value)
		})
	})

	async function settle(): Promise<void> {
		const keys = unsettled
		unsettled = []
		await Promise.all(
			keys.map((k) =>
				UserDraftDbSyncer.flush({ workspace: k.workspace, itemKind: opts.itemKind, path: k.path })
			)
		)
	}

	return {
		get draftPath() {
			return draftPath
		},
		adopt(workspace: string, path: string, value: V) {
			written = { workspace, path }
			writtenValue = JSON.stringify(value)
			autoPath = path
			draftPath = path
			adopted = true
		},
		async flush() {
			await commitPending()
			await settle()
		},
		async finish() {
			finished = true
			dropPending()
			const w = written
			written = undefined
			writtenValue = undefined
			draftPath = ''
			if (w) {
				// See `write`: an adopted key is a live handle key, so keep its cell.
				UserDraft.discard(opts.itemKind, w.path, opts.value(), { workspace: w.workspace })
				opts.onAbandonKey?.(w.workspace, w.path)
				markUnsettled(w.workspace, w.path)
			}
			await settle()
		},
		reset() {
			finished = false
			adopted = false
			abandoned.clear()
			dropPending()
			written = undefined
			writtenValue = undefined
			unsettled = []
			autoPath = undefined
			draftPath = ''
		}
	}
}
