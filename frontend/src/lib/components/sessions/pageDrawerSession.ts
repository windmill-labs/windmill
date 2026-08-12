import { tick } from 'svelte'
import { page } from '$app/state'
import { flushAllPendingEditorChanges } from '$lib/components/SimpleEditor.svelte'
import { buildFilterUrl } from '$lib/navigation'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import type { UserDraftItemKind } from '$lib/gen'
import { pageHref, stripBase, TRIGGER_PAGES, type TriggerKind } from './previewRouter'
import {
	RESOURCES_PATH,
	SCHEDULES_PATH,
	VARIABLES_PATH
} from '$lib/components/copilot/chat/global/pageNavigation'
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

// Push the drawer's pending autosave, and refuse to leave if it did not land: `flush`
// reports a failed or conflicting POST through its state rather than by throwing, so
// routing regardless would open the preview on the server's older draft while the
// drawer the user is looking at still holds the edit.
async function flushOrRefuse(query: Parameters<typeof UserDraftDbSyncer.flush>[0]): Promise<void> {
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
	const anchor = pagePath === RESOURCES_PATH ? `/resource/${itemPath}` : itemPath
	const href = pageHref(buildFilterUrl(pagePath, {}, { hash: anchor }))
	const itemKind = DRAWER_DRAFT_KIND[pagePath]
	return {
		page: () => href,
		workspaceId,
		// Autosave is debounced, and the preview reads the draft back through the server
		// from a document of its own — routing before the POST lands opens the drawer on a
		// value the user has already changed. The code editors hold their last keystrokes
		// behind a debounce of their own, so materialise those and let the bindings settle
		// before the draft is persisted.
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
