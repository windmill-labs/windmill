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
	/** Whether nothing is deployed at `path` yet. Consulted only when a close
	 * forces a commit early, where `Path`'s own check may not have run. */
	pathIsFree?: (path: string) => Promise<boolean>
}

export interface NewItemDraftSync {
	/** Storage path of the persisted draft, `''` when none. */
	readonly draftPath: string
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
export function useNewItemDraftSync<V>(opts: NewItemDraftSyncOptions<V>): NewItemDraftSync {
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
		| { timer: ReturnType<typeof setTimeout>; key: string; value: V | undefined }
		| undefined
	let pendingWorkspace: string | undefined
	// `Path` auto-fills a unique name on mount, so a non-empty path is no
	// evidence the user did anything. Only a departure from the name it settled
	// on counts (`Path.dirty` can't: it flips on any keyup, tabbing included).
	let autoPath: string | undefined

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
			UserDraft.remove(opts.itemKind, written.path, { workspace: written.workspace })
			markUnsettled(written.workspace, written.path)
			written = undefined
			writtenValue = undefined
		}
		if (!workspace || !path || value === undefined) return
		const serialized = JSON.stringify(value)
		if (written && serialized === writtenValue) return
		UserDraft.save(opts.itemKind, path, value, { workspace })
		markUnsettled(workspace, path)
		written = { workspace, path }
		writtenValue = serialized
	}

	function dropPending(): void {
		if (!pending) return
		clearTimeout(pending.timer)
		pending = undefined
	}

	function commit(): void {
		const p = pending
		if (!p) return
		dropPending()
		draftPath = p.key
		write(pendingWorkspace, p.key, p.value)
	}

	$effect(() => {
		if (!opts.enabled() || finished) return
		const p = opts.path()
		const key = p !== '' && opts.pathError() === '' && touched() ? p : ''
		const workspace = opts.workspace()
		const value = opts.value()
		if (key === untrack(() => draftPath)) {
			// Back on the committed key: a transition away from it is stale.
			untrack(dropPending)
			return
		}
		untrack(() => {
			dropPending()
			pendingWorkspace = workspace
			pending = { timer: setTimeout(commit, COMMIT_DELAY_MS), key, value }
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
		async flush() {
			const p = pending
			if (p && p.key) {
				// Forced by a close, so the commit delay that lets `Path`'s debounced
				// existence check land was cut short. Re-check before keying on it:
				// a path that already holds an item would take this draft as an edit
				// of that item, and saving from there would overwrite its value.
				const free =
					opts.pathError() === '' && (opts.pathIsFree ? await opts.pathIsFree(p.key) : true)
				if (!free && pending === p) dropPending()
			}
			commit()
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
				UserDraft.remove(opts.itemKind, w.path, { workspace: w.workspace })
				markUnsettled(w.workspace, w.path)
			}
			await settle()
		},
		reset() {
			finished = false
			dropPending()
			written = undefined
			writtenValue = undefined
			unsettled = []
			autoPath = undefined
			draftPath = ''
		}
	}
}
