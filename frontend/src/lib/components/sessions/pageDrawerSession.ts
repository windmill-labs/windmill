import { tick } from 'svelte'
import { page } from '$app/state'
import { goto } from '$lib/navigation'
import {
	anyEditorUnparseable,
	flushAllPendingEditorChanges
} from '$lib/components/pendingEditorFlush'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import type { UserDraftItemKind } from '$lib/gen'
// From the path leaf rather than `previewRouter`: these drawers mount inside script and
// flow editors, where pulling the filter schemas that module reads views from would make
// every trigger's save utils eager.
import {
	pageHref,
	stripBase,
	TRIGGER_PAGES,
	RESOURCES_PATH,
	SCHEDULES_PATH,
	VARIABLES_PATH,
	type TriggerKind
} from './previewPaths'
import type { OpenInSessionSource } from './OpenInSessionButton.svelte'

// The draft each page's drawer edits. The preview loads the page in its own
// document and reads the draft back from the server, so opening a session has to
// wait for the debounced autosave to land — see `beforeOpen` below.
const DRAWER_DRAFT_KIND: Record<string, UserDraftItemKind> = {
	[VARIABLES_PATH]: 'variable',
	[RESOURCES_PATH]: 'resource',
	[SCHEDULES_PATH]: 'trigger_schedule',
	...Object.fromEntries(
		Object.entries(TRIGGER_PAGES).map(([kind, p]) => [
			p.path,
			`trigger_${kind as TriggerKind}` as UserDraftItemKind
		])
	)
}

// Push the drawer's pending autosave and refuse to leave if it did not land: `flush`
// reports a failed or conflicting POST through its state rather than by throwing, so
// routing regardless would open the preview on the server's older draft while the drawer
// the user is looking at still holds the edit. The flush is the explicit kind, saving even
// with auto-save off: asking for a session is asking for the edit to come along.
async function flushOrRefuse(query: Parameters<typeof UserDraftDbSyncer.flush>[0]): Promise<void> {
	// Before the save, not after: text that does not parse never reached the draft, so
	// leaving now would open the session on the last value that did and drop the buffer
	// with the drawer — and saving first would write that stale value on the way to
	// refusing. The editors were materialised before this call, so the check is current.
	if (anyEditorUnparseable()) {
		throw new Error('This page has changes that are not valid JSON. Fix them first.')
	}
	await UserDraftDbSyncer.flush(query)
	if (UserDraftDbSyncer.getConflict(query).conflict) {
		throw new Error(
			'This draft has a newer conflicting version on the server. Resolve it here before opening a session.'
		)
	}
	const { state, failureMessage } = UserDraftDbSyncer.getState(query)
	if (state === 'failed') {
		throw new Error(
			`Saving the latest draft failed (${failureMessage ?? 'unknown error'}). Retry before opening a session.`
		)
	}
}

// How each page addresses a row in its hash. Resources route theirs through an extra
// segment; every other page names the path directly.
const drawerHashFor = (pagePath: string, itemPath: string) =>
	pagePath === RESOURCES_PATH ? `/resource/${itemPath}` : itemPath

/**
 * Deep-link the row whose drawer just opened, so the location says what is on screen — a
 * drawer opened from a row's Edit button is as open as one reached by link, and the chat
 * is told what the session shows through the location alone. No-op off that page, where
 * these drawers also open inside editors and pickers with no row convention to keep.
 *
 * Written straight to history rather than through the router: these pages open their
 * drawer *from* the hash, so a router-visible write would come back as a second open on
 * the row the user is already editing. The preview observes `replaceState` either way.
 */
export function setPageDrawerAnchor(pagePath: string, itemPath: string | undefined): void {
	if (!itemPath) return
	const { pathname, search, hash } = window.location
	if (stripBase(pathname) !== pagePath) return
	const anchor = `#${drawerHashFor(pagePath, itemPath)}`
	if (hash === anchor) return
	history.replaceState(history.state, '', `${pathname}${search}${anchor}`)
}

/**
 * Drop the row a list page deep-links, once its drawer closes. The hash is how the row was
 * requested; leaving it behind makes the location claim a drawer that is no longer open —
 * and the chat reports that location as what the user is looking at. No-op off that page,
 * where the same drawers open without a hash convention.
 */
export async function clearPageDrawerAnchor(pagePath: string): Promise<void> {
	// Read the location from the document, never from `page.url`: these pages write their
	// filters with shallow routing, which never reaches `page.url`, so rebuilding the query
	// from it would drop the filters the user typed along with the anchor.
	const { pathname, search, hash } = window.location
	if (stripBase(pathname) !== pagePath || !hash) return
	await goto(`${pathname}${search}`, { replaceState: true, noScroll: true })
}

/**
 * "Open in AI session" source for the edit drawer of a workspace list page (trigger
 * lists, schedules, resources, variables): none is an editable item the preview can
 * host, so the session opens the page with `itemPath`'s drawer deep-linked.
 *
 * Undefined — so the button renders nothing — unless that page's own route is on
 * screen: these drawers also open inside script/flow editors and pickers, which carry
 * their own entry point.
 */
export function pageDrawerSessionSource(
	pagePath: string,
	itemPath: string | undefined,
	workspaceId: string | undefined
): OpenInSessionSource | undefined {
	if (!itemPath || stripBase(page.url.pathname) !== pagePath) return undefined
	const anchor = drawerHashFor(pagePath, itemPath)
	const itemKind = DRAWER_DRAFT_KIND[pagePath]
	return {
		// The list behind the drawer is part of what the user is looking at, and its filters
		// are written with shallow routing — so the query has to come from the document at
		// click time, as the thunk exists for.
		page: () => `${pageHref(pagePath)}${window.location.search}#${anchor}`,
		workspaceId,
		// Autosave is debounced, and the preview reads the draft back through the server
		// from a document of its own — routing before the POST lands opens the drawer on a
		// value the user has already changed. Editors hold their last keystrokes behind a
		// debounce of their own, so materialise those and let the bindings settle first.
		// Text that does not parse never reaches the draft, so routing is refused instead.
		beforeOpen:
			itemKind && workspaceId
				? async () => {
						flushAllPendingEditorChanges()
						await tick()
						await flushOrRefuse({ workspace: workspaceId, itemKind, path: itemPath })
					}
				: undefined
	}
}
