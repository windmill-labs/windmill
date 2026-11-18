import { untrack } from 'svelte'
import { UserDraft, type UserDraftHandle, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'

/**
 * Page-level draft orchestration for the full-page editors (scripts / flows /
 * apps / apps_raw) — the page analogue of `useTriggerDraftSync`. Owns the
 * per-path autosave handle (re-keyed on navigation), the live-editor-draft
 * registry entry (home-page "edit draft" links), `recordRemoteSync` (a method
 * so pages can't forget it and silently downgrade conflict detection to
 * overwrite-as-fresh), the deployed-baseline `seed`, and the draft `remove`.
 * The entity-specific backend load and new-draft template stay in the page.
 */
export interface PageDraftSyncOptions<V = unknown> {
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
	/** Predicate: is the value about to autosave back at the deployed
	 *  baseline? When true the syncer POSTs a delete instead of a
	 *  baseline-equal draft, so editing back to deployed clears the draft
	 *  instead of leaving a no-op behind. MUST read the deployed baseline
	 *  reactively (it's captured once per re-keyed acquire) and use
	 *  `draftValuesEqual` so it can't disagree with the "unsaved changes"
	 *  banner. Return false for draft-only items (no deployed baseline). */
	discardIf?: (val: V) => boolean
	/** Seeds the cell on first acquire without POSTing (the syncer's seed
	 *  guard swallows it). Use when the value is already in hand at mount (an
	 *  embedder providing the item) instead of assigning `draft` after load.
	 *  Pass a STABLE reference (read it under `untrack`). */
	defaultValue?: V
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

export function usePageDraftSync<V = unknown>(opts: PageDraftSyncOptions<V>): PageDraftSync<V> {
	// One handle, re-keyed on (workspace, path); `''` path releases it.
	// `canBeDisabled` because these editors carry the "Enable auto-save" toggle.
	const handle = UserDraft.useReactive<V>(() => ({
		itemKind: opts.itemKind,
		path: opts.path(),
		workspace: opts.workspace(),
		canBeDisabled: true,
		defaultValue: opts.defaultValue,
		discardIf: opts.discardIf
	}))

	// Live-editor-draft registry: lets the home-page "edit draft" link resolve
	// this open editor. Re-registers on path change; cleared on teardown.
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
