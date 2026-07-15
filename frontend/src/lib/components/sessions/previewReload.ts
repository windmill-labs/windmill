import type { SessionPreviewTab } from './sessionState.svelte'
import { stripBase, TRIGGER_PAGES, type TriggerKind } from './previewRouter'

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
export type ToolReloadEffect = { pages: string[] }
const NO_RELOAD: ToolReloadEffect = { pages: [] }

export function toolReloadEffect(name: string, args: any): ToolReloadEffect {
	switch (name) {
		case 'write_schedule':
			return { pages: ['/schedules'] }
		case 'write_trigger':
			return { pages: triggerPages(args?.kind) }
		case 'write_resource':
			return { pages: ['/resources'] }
		case 'write_variable':
			return { pages: ['/variables'] }
		case 'create_folder':
			return { pages: ['/folders'] }
		// Generic item tools carry a workspace-item `type`; refresh its list page
		// when it lives on one (schedule/resource/variable/trigger). script/flow/app
		// have their own live editor tab and no previewed list page → nothing.
		case 'delete_workspace_item':
		case 'discard_local_draft':
		case 'deploy_workspace_item':
		case 'rebase_draft':
			return { pages: pagesForItemType(args?.type, args) }
		default:
			return NO_RELOAD
	}
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
	return tabs.filter((t) => pages.has(stripBase(t.loc || t.url)))
}
