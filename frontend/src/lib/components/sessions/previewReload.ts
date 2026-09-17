import type { SessionPreviewTab } from './sessionState.svelte'
import { whereIs } from './sessionPreviewTabs.svelte'
import {
	pageItemListPath,
	pageItemUrl,
	parsePageItemRoute,
	stripBase,
	TRIGGER_PAGES,
	type PageItemRef,
	type TriggerKind
} from './previewPaths'

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
//
// Page items (variables, resources, schedules, triggers) are the exception among
// in-process tabs: their editors read a draft only when they open, so a write to
// one reloads its tab too. `items` names it when the tool's args do; without a
// path, every tab of that kind reloads.
export type ToolReloadEffect = { pages: string[]; items: PageItemRef[] }
const NO_RELOAD: ToolReloadEffect = { pages: [], items: [] }

export function toolReloadEffect(name: string, args: any): ToolReloadEffect {
	switch (name) {
		case 'write_schedule':
			return withItem(['/schedules'], itemRef('schedule', args))
		case 'write_trigger':
			return withItem(triggerPages(args?.kind), itemRef('trigger', args, args?.kind))
		case 'write_resource':
			return withItem(['/resources'], itemRef('resource', args))
		case 'write_variable':
			return withItem(['/variables'], itemRef('variable', args))
		case 'create_folder':
			return { pages: ['/folders'], items: [] }
		// Generic item tools carry a workspace-item `type`; refresh its list page
		// when it lives on one (schedule/resource/variable/trigger). script/flow/app
		// have their own live editor tab and no previewed list page → nothing.
		case 'delete_workspace_item':
		case 'discard_local_draft':
		case 'deploy_workspace_item':
		case 'rebase_draft':
			return withItem(
				pagesForItemType(args?.type, args),
				itemRef(args?.type, args, args?.trigger_kind)
			)
		default:
			return NO_RELOAD
	}
}

function withItem(pages: string[], item: PageItemRef | undefined): ToolReloadEffect {
	return { pages, items: item && pages.length ? [item] : [] }
}

function itemRef(type: unknown, args: any, triggerKind?: unknown): PageItemRef | undefined {
	const path = args?.path
	if (typeof path !== 'string' || !path) return undefined
	if (type === 'variable' || type === 'resource' || type === 'schedule') {
		return { kind: type, path }
	}
	if (type === 'trigger' && (triggerKind as string) in TRIGGER_PAGES) {
		return { kind: 'trigger', triggerKind: triggerKind as TriggerKind, path }
	}
	return undefined
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

// The open tabs a reload should refresh: list-page tabs whose observed page path is
// in `pages`, and page item tabs on those pages — only the named ones when a tool
// named its item. Item-editor and pipeline tab routes are never list pages, so they
// never match (see the self-sync invariant above). Pure over a tab snapshot so the
// sessions page can reload by id and this stays unit-testable.
export function tabsToReload(
	tabs: SessionPreviewTab[],
	pages: ReadonlySet<string>,
	items: ReadonlySet<string> = new Set()
): SessionPreviewTab[] {
	if (pages.size === 0) return []
	return tabs.filter((t) => {
		const pageItem = parsePageItemRoute(t.url)
		if (!pageItem) return pages.has(stripBase(whereIs(t)))
		const listPath = pageItemListPath(pageItem)
		if (!pages.has(listPath)) return false
		const named = [...items].some((u) => pageItemListPath(parsePageItemRoute(u)!) === listPath)
		return !named || items.has(pageItemUrl(pageItem))
	})
}
