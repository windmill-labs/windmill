/**
 * Optimistic "this item has a draft" overrides for the list-page `*` suffix.
 *
 * The list pages render `*` from the server's `is_draft`, which only updates on
 * refetch. An open editor knows the live truth and publishes it here; pages read
 * `getLocalDraftHint(...) ?? is_draft`, so the editor overrides the stale flag in
 * both directions immediately. SvelteMap-backed, so readers re-render on a flip.
 *
 * Hints PERSIST past editor teardown (the divergence/discard is synced
 * server-side) and are corrected, not expired, on the next editor settle.
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
