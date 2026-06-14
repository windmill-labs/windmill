/**
 * Optimistic "this item has a draft" overrides for the list pages.
 *
 * The variable / resource / schedule / trigger list pages render a `*`
 * suffix from the server's `is_draft` flag, which only updates on a
 * refetch. While an editor is open it knows the live truth — the
 * "unsaved changes" banner state — and publishes it here; the pages
 * read it with `getLocalDraftHint(...) ?? is_draft`, so the editor's
 * observation OVERRIDES the stale server flag in both directions:
 * editing sets the asterisk immediately, and discarding (or editing
 * back to the deployed value) clears it immediately instead of waiting
 * for the next refetch.
 *
 * Backed by a `SvelteMap`, so a page template reading
 * `getLocalDraftHint(...)` re-renders when the editor flips the hint.
 *
 * Lifetime: hints PERSIST past editor teardown — they record the truth
 * the editor last observed, which outlives the drawer (the divergence,
 * or its discard, is synced server-side). They are corrected, not
 * expired: the next time an editor settles on the same item it
 * re-publishes what it sees.
 */
import { SvelteMap } from 'svelte/reactivity'
import type { UserDraftItemKind } from '$lib/gen'

const hints = new SvelteMap<string, boolean>()

function key(workspace: string, kind: UserDraftItemKind, path: string): string {
	return `${workspace}/${kind}/${path}`
}

export function setLocalDraftHint(
	workspace: string,
	kind: UserDraftItemKind,
	path: string,
	on: boolean
): void {
	if (!workspace || !path) return
	hints.set(key(workspace, kind, path), on)
}

/** The editor-observed draft state, or `undefined` when no editor has
 * published an opinion — callers fall back to the server flag:
 * `getLocalDraftHint(...) ?? is_draft`. */
export function getLocalDraftHint(
	workspace: string | undefined,
	kind: UserDraftItemKind,
	path: string
): boolean | undefined {
	if (!workspace) return undefined
	return hints.get(key(workspace, kind, path))
}
