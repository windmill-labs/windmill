import { UserDraftDbSyncer, type UserDraftLastSyncQuery } from '$lib/userDraftDbSyncer.svelte'
import { getLocalDraftHint } from '$lib/localDraftHints.svelte'
import type { UserDraftItemKind } from '$lib/gen'

/** Drop `?new_draft=true` from the current URL (preserving every other param),
 * mutating the address bar without a navigation. No-op when the flag is absent
 * or `window` is unavailable (SSR). */
export function stripNewDraftFlag(): void {
	if (typeof window === 'undefined') return
	const url = new URL(window.location.href)
	if (url.searchParams.get('new_draft') !== 'true') return
	url.searchParams.delete('new_draft')
	window.history.replaceState(window.history.state, '', url.toString())
}

/**
 * Whether an editor load should take the `?new_draft` seed-empty branch.
 *
 * The `/{scripts,flows,apps,apps_raw}/add` redirect lands the editor on
 * `u/<user>/draft_<uuid>?new_draft=true`; the seed branch bootstraps an empty
 * (or template/hub/fork-seeded) editor instead of fetching a row that doesn't
 * exist yet. But the flag can outlive the seed: exiting an AI session restores
 * the remembered nav route (still `?new_draft=true`) even though the draft was
 * already materialized — by the first autosave, or by "Open in AI session"'s
 * `forcePersist`. Re-running the seed branch then blanks the saved draft.
 *
 * So gate the seed branch on `new_draft=true` AND no persisted draft this
 * session (`getLocalDraftHint`): a never-saved draft still seeds empty (and a
 * pre-edit refresh re-seeds instead of 404-ing), while a re-entry after the
 * draft exists falls through to the normal load that reflects it.
 */
export function shouldSeedNewDraft(
	searchParams: URLSearchParams,
	workspace: string | undefined,
	itemKind: UserDraftItemKind,
	path: string
): boolean {
	if (searchParams.get('new_draft') !== 'true') return false
	return getLocalDraftHint(workspace, itemKind, path) !== true
}

/**
 * Defer stripping `?new_draft=true` from the URL until the draft's first save
 * is CONFIRMED by the backend. The `/{scripts,flows,apps,apps_raw}/add`
 * redirect lands the editor on an unsaved `u/<user>/draft_<uuid>` path;
 * stripping the flag on load (the old behavior) meant a refresh before any
 * edit fell through to the `getByPath` fetch for a row that doesn't exist yet
 * → 404 "not found". Keeping the flag until `onSaved` fires lets a pre-edit
 * refresh re-enter the new-draft seeding branch, while a post-save refresh
 * loads the now-persisted draft.
 *
 * Returns a cleanup to drop the listener on navigation / unmount; the listener
 * also self-unsubscribes after the first save.
 */
export function stripNewDraftFlagOnSave(query: UserDraftLastSyncQuery): () => void {
	let unsub: (() => void) | undefined
	unsub = UserDraftDbSyncer.onSaved(query, () => {
		unsub?.()
		unsub = undefined
		stripNewDraftFlag()
	})
	return () => {
		unsub?.()
		unsub = undefined
	}
}
