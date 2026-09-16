import type { SessionPreviewTab } from './sessionState.svelte'
import { whereIs } from './sessionPreviewTabs.svelte'
import { parsePreviewItemRoute, stripBase, TRIGGER_PAGES, type TriggerKind } from './previewPaths'

// Which list pages a completed chat tool can change, as base-stripped paths
// (e.g. `/schedules`). This allowlist is the single source of truth for "does
// this tool change a list page a preview tab might show". A new mutating tool
// that surfaces on one of these pages must be added here or that tab silently
// goes stale — match by exact tool name, never a name regex, which mis-classifies
// purely-local tools (e.g. `update_user_instructions`) as page mutations.
//
// Item-editor writes (write_script / write_flow / init_app / write_app_*) are
// deliberately absent: every editable item is a live in-process editor that
// self-syncs from the store the chat mutates, so its tab needs no reload — and
// no list page we preview lists open drafts. They fall through to NO_RELOAD.
// This "live editors self-sync, only list pages reload" invariant is the reason
// the callers below and in the sessions page reload nothing for item tabs.
//
// `items` is the one exception to it. A tab can sit on the VIEW side of an item,
// which renders the DEPLOYED version read over the API — it shares no store with
// the chat, so a tool that changes what is deployed at a path has to name it here
// or that tab keeps showing the previous deployment. Only tools that change the
// deployed item belong in `items`; a draft write does not.
export type ToolReloadEffect = { pages: string[]; items: string[] }
const NO_RELOAD: ToolReloadEffect = { pages: [], items: [] }

export function toolReloadEffect(name: string, args: any): ToolReloadEffect {
	switch (name) {
		case 'write_schedule':
			return { pages: ['/schedules'], items: [] }
		case 'write_trigger':
			return { pages: triggerPages(args?.kind), items: [] }
		case 'write_resource':
			return { pages: ['/resources'], items: [] }
		case 'write_variable':
			return { pages: ['/variables'], items: [] }
		case 'create_folder':
			return { pages: ['/folders'], items: [] }
		// These two change what is deployed at a path, so a tab on that item's View side
		// is showing a version that no longer exists.
		case 'delete_workspace_item':
		case 'deploy_workspace_item':
			return { pages: pagesForItemType(args?.type, args), items: deployedItems(args) }
		// Generic item tools carry a workspace-item `type`; refresh its list page
		// when it lives on one (schedule/resource/variable/trigger). script/flow/app
		// have their own live editor tab and no previewed list page → nothing.
		case 'discard_local_draft':
		case 'rebase_draft':
			return { pages: pagesForItemType(args?.type, args), items: [] }
		default:
			return NO_RELOAD
	}
}

// The item path a deploy/delete changed, when it is one of the kinds a preview tab can
// show the deployed side of. Other types (schedule, resource, …) have list pages instead.
function deployedItems(args: any): string[] {
	const type = args?.type
	if (type !== 'script' && type !== 'flow' && type !== 'app') return []
	return typeof args?.path === 'string' && args.path ? [args.path] : []
}

function pagesForItemType(type: unknown, args: any): string[] {
	switch (type) {
		case 'schedule':
			return ['/schedules']
		case 'resource':
			return ['/resources']
		case 'variable':
			return ['/variables']
		case 'trigger':
			return triggerPages(args?.trigger_kind)
		default:
			return []
	}
}

function triggerPages(kind: unknown): string[] {
	const page = TRIGGER_PAGES[kind as TriggerKind]
	return page ? [page.path] : []
}

// The open tabs a page-reload should refresh: those whose observed page path is
// in `pages`. Item-editor and pipeline tab routes are never list pages, so they
// never match (see the self-sync invariant above). Pure over a tab snapshot so
// the sessions page can reload by id and this stays unit-testable.
export function tabsToReload(
	tabs: SessionPreviewTab[],
	pages: ReadonlySet<string>
): SessionPreviewTab[] {
	if (pages.size === 0) return []
	return tabs.filter((t) => pages.has(stripBase(whereIs(t))))
}

// The open tabs a deploy/delete of `items` should refresh: those sitting on the VIEW side
// of one of those paths. The Edit side is excluded on purpose — it is the live editor that
// self-syncs, and reloading it would throw away whatever is being edited.
export function viewerTabsToReload(
	tabs: SessionPreviewTab[],
	items: ReadonlySet<string>
): SessionPreviewTab[] {
	if (items.size === 0) return []
	return tabs.filter((t) => {
		const route = parsePreviewItemRoute(whereIs(t))
		return route?.mode === 'view' && items.has(route.itemPath)
	})
}
