/**
 * Optimistic "this item has a draft" hints for the list pages.
 *
 * The variable / resource / schedule / trigger list pages render a `*`
 * suffix from the server's `is_draft` flag, which only updates on a
 * refetch. While an editor drawer is open with unsaved changes (the
 * "You have unsaved changes" banner), the edited item should show the
 * asterisk immediately — the editors publish their dirty state here and
 * the pages OR it into the suffix condition.
 *
 * Backed by a `SvelteSet`, so a page template reading
 * `hasLocalDraftHint(...)` re-renders when the editor flips the hint.
 * Hints are transient: cleared when the editor reports clean (discard /
 * deploy) and on editor teardown; after that the server flag (refreshed
 * by the page's own reload-on-change paths) takes back over.
 */
import { SvelteSet } from 'svelte/reactivity'
import type { UserDraftItemKind } from '$lib/gen'

const hints = new SvelteSet<string>()

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
	if (on) {
		hints.add(key(workspace, kind, path))
	} else {
		hints.delete(key(workspace, kind, path))
	}
}

export function hasLocalDraftHint(
	workspace: string | undefined,
	kind: UserDraftItemKind,
	path: string
): boolean {
	if (!workspace) return false
	return hints.has(key(workspace, kind, path))
}
