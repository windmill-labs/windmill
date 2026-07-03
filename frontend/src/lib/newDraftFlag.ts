import { UserDraftDbSyncer, type UserDraftLastSyncQuery } from '$lib/userDraftDbSyncer.svelte'

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
		if (typeof window === 'undefined') return
		const url = new URL(window.location.href)
		if (url.searchParams.get('new_draft') !== 'true') return
		url.searchParams.delete('new_draft')
		window.history.replaceState(window.history.state, '', url.toString())
	})
	return () => {
		unsub?.()
		unsub = undefined
	}
}
