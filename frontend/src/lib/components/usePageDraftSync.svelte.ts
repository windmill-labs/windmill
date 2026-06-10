import { untrack } from 'svelte'
import { UserDraft, type UserDraftHandle, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'

/**
 * Page-level draft orchestration shared by the full-page editors
 * (scripts / flows / apps / apps_raw). The page editors each hand-rolled
 * this with three divergent handle-ownership models and an easy-to-forget
 * `recordRemoteSync`; this is the single model — the page analogue of
 * `useTriggerDraftSync` for the drawer editors.
 *
 * It owns:
 *   - the per-path autosave handle (re-keyed on navigation),
 *   - the live-editor-draft registry entry (home-page "edit draft" links),
 *   - `recordRemoteSync` (forgetting it silently downgrades conflict
 *     detection to overwrite-as-fresh — so it's a method here, not a
 *     thing each page remembers to call),
 *   - the deployed-baseline `seed` (via `UserDraft.seed`, no stopSync
 *     bracket to forget to resume) and the draft `remove`.
 *
 * The entity-specific backend load and the new-draft template stay in the
 * page — only the cross-cutting plumbing moves here.
 */
export interface PageDraftSyncOptions {
	itemKind: UserDraftItemKind
	/** Reactive draft storage path. `''` (e.g. viewing a historical hash)
	 *  releases the handle and skips registry/sync work. */
	path: () => string
	/** Reactive workspace (`$workspaceStore`). */
	workspace: () => string | undefined
	/** Reactive effective path for the live-editor-draft registry (the
	 *  draft's own `path`, used by home-page deep links). Omit to skip
	 *  registry registration entirely (e.g. read-only hash views). */
	effectivePath?: () => string | undefined
}

export interface PageDraftSync<V> {
	/** The live autosave handle. `draft` is the editor's bind target. */
	readonly handle: UserDraftHandle<V>
	get draft(): V | undefined
	set draft(value: V | undefined)
	/** Load `value` as the deployed/template baseline WITHOUT POSTing it
	 *  (see `UserDraft.seed`). */
	seedBaseline(value: V): void
	/** After a backend load, record the server's `draft_saved_at` so the
	 *  next autosave attaches a matching `last_sync` and the server can
	 *  reject stale writes. `undefined` clears it (no draft existed). */
	recordRemoteSync(draftSavedAt: string | undefined): void
	/** Drop the draft (server row + local cell) — restore-to-deployed and
	 *  post-deploy cleanup. */
	remove(): void
}

export function usePageDraftSync<V = unknown>(opts: PageDraftSyncOptions): PageDraftSync<V> {
	// One handle, re-keyed on (workspace, path) navigation — the single
	// ownership model. `''` path yields no spec, so the handle releases.
	const handle = UserDraft.useReactive<V>(() => ({
		itemKind: opts.itemKind,
		path: opts.path(),
		workspace: opts.workspace()
	}))

	// Live-editor-draft registry: lets the home-page "edit draft" deep
	// link resolve this open editor. Re-registers when the path or the
	// draft's effective path changes; cleared on teardown / re-key.
	$effect(() => {
		if (!opts.effectivePath) return
		const ws = opts.workspace()
		const p = opts.path()
		const eff = opts.effectivePath()
		if (!ws || !p) return
		UserDraft.setLiveEditorDraft({
			workspace: ws,
			itemKind: opts.itemKind,
			storagePath: p,
			effectivePath: eff || p
		})
		return () => UserDraft.clearLiveEditorDraft(opts.itemKind, { workspace: ws, storagePath: p })
	})

	return {
		get handle() {
			return handle
		},
		get draft() {
			return handle.draft
		},
		set draft(value: V | undefined) {
			handle.draft = value
		},
		seedBaseline(value: V) {
			const ws = opts.workspace()
			const p = opts.path()
			if (!ws || !p) return
			UserDraft.seed(opts.itemKind, p, value, { workspace: ws })
		},
		recordRemoteSync(draftSavedAt: string | undefined) {
			const ws = opts.workspace()
			const p = opts.path()
			if (!ws || !p) return
			untrack(() =>
				UserDraftDbSyncer.recordRemoteSync(
					{ workspace: ws, itemKind: opts.itemKind, path: p },
					draftSavedAt
				)
			)
		},
		remove() {
			const ws = opts.workspace()
			const p = opts.path()
			if (!ws || !p) return
			UserDraft.remove(opts.itemKind, p, { workspace: ws })
		}
	}
}
