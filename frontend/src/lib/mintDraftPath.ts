import { getUsernameForNamespace } from '$lib/userNamespace'
import { randomUUID } from '$lib/utils/uuid'

/**
 * Mint a fresh `u/<username>/draft_<uuid>` storage path for a brand-new
 * editor item. Shared by the `/{scripts,flows,apps,apps_raw}/add` route
 * redirects (`makeDraftAddLoad`) and the SDK builder wrappers
 * (`ScriptWrapper` / `FlowWrapper`) so the two can't diverge on format —
 * both need the autosave handle keyed under a real, unique path or the
 * draft detaches (local-only) and never POSTs.
 *
 * Underscores not dashes — path segments are `[a-zA-Z0-9_]` words and
 * downstream consumers treat `-` as foreign. `randomUUID` not
 * `crypto.randomUUID` (WebCrypto is absent on non-secure origins).
 */
export function mintDraftPath(): string {
	const username = getUsernameForNamespace()
	const uuid = randomUUID().replaceAll('-', '_')
	return `u/${username}/draft_${uuid}`
}

/**
 * Resolve the autosave storage key for an SDK builder wrapper. The first
 * caller-provided path wins (a consumer that supplies its own path — including
 * the temporary React-SDK stopgap — keeps it); otherwise, for a brand-new
 * editable item, mint a fresh draft path so autosave attaches. Returns `''`
 * (a detached, local-only handle that never POSTs) when there is no path and
 * the item isn't a new editable one — the intentionally-pathless read-only case.
 *
 * Shared by `ScriptWrapper` / `FlowWrapper` so the two can't diverge on this
 * precedence. Callers MUST capture the result once (`untrack`) so editing a
 * path field later can't re-key the handle and orphan the draft.
 */
export function selectDraftStoragePath(opts: {
	providedPaths: (string | undefined)[]
	isNewItem: boolean
}): string {
	for (const p of opts.providedPaths) {
		if (p) return p
	}
	return opts.isNewItem ? mintDraftPath() : ''
}
