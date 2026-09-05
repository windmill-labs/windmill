import type { SessionPreviewTab } from './sessionState.svelte'
import { whereIs } from './sessionPreviewTabs.svelte'
import { stripBase, TRIGGER_PAGES, type TriggerKind } from './previewPaths'

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
/** What a tool asks of a hosted entity editor (see previewRouter's
 * `IN_REALM_ENTITY_PAGES`) showing one of the affected pages. A plain write
 * reaches it on its own — it holds the draft cell the write seeds — but the
 * tools that clear or replace that cell go behind it, and the item can be gone
 * altogether. `none` is what makes the live-editing case live. */
export type EntityToolEffect = 'none' | 'refresh' | 'close'

export type ToolReloadEffect = {
	pages: string[]
	entity: EntityToolEffect
	/** The item the tool mutated, when its args name one. A hosted editor on
	 * another path is not affected by it — unlike a list page, which shows every
	 * row and so reloads for any mutation on it. */
	path?: string
}
const NO_RELOAD: ToolReloadEffect = { pages: [], entity: 'none' }

export function toolReloadEffect(name: string, args: any): ToolReloadEffect {
	switch (name) {
		case 'write_schedule':
			return { pages: ['/schedules'], entity: 'none' }
		case 'write_trigger':
			return { pages: triggerPages(args?.kind), entity: 'none' }
		case 'write_resource':
			return { pages: ['/resources'], entity: 'none' }
		case 'write_variable':
			return { pages: ['/variables'], entity: 'none' }
		case 'create_folder':
			return { pages: ['/folders'], entity: 'none' }
		// Generic item tools carry a workspace-item `type`; refresh its list page
		// when it lives on one (schedule/resource/variable/trigger). script/flow/app
		// have their own live editor tab and no previewed list page → nothing.
		// These all drop the draft the hosted editor is bound to: deploying or
		// discarding replaces it with the deployed value, and deleting removes the
		// item, so the editor is re-read from the server or left behind entirely.
		case 'discard_local_draft':
		case 'deploy_workspace_item':
		case 'rebase_draft':
			return { pages: pagesForItemType(args?.type, args), entity: 'refresh', path: itemPath(args) }
		case 'delete_workspace_item':
			return { pages: pagesForItemType(args?.type, args), entity: 'close', path: itemPath(args) }
		default:
			return NO_RELOAD
	}
}

function itemPath(args: any): string | undefined {
	return typeof args?.path === 'string' && args.path ? args.path : undefined
}

/** One item mutation from a chat round, as the preview needs to read it back. */
export type EntityMutation = {
	pages: string[]
	effect: EntityToolEffect
	path?: string
	/** The workspace the tool acted on — a session on a fork must not be moved by
	 * a mutation to the same path in its parent. */
	workspace: string
}

/** What a hosted entity editor showing `tab` must do about a round's mutations.
 * A mutation reaches it only in its own workspace, on its own list page, and —
 * when the tool named one — on its own path. */
export function entityEffectForTab(
	mutations: readonly EntityMutation[],
	tab: { listPage: string; path: string; workspace: string }
): EntityToolEffect {
	let effect: EntityToolEffect = 'none'
	for (const m of mutations) {
		if (m.workspace !== tab.workspace) continue
		if (!m.pages.includes(tab.listPage)) continue
		if (m.path !== undefined && m.path !== tab.path) continue
		effect = strongerEntityEffect(effect, m.effect)
	}
	return effect
}

/** The stronger of two effects, for a tab several of a round's mutations reach:
 * a delete outranks a refresh, which outranks nothing. */
export function strongerEntityEffect(a: EntityToolEffect, b: EntityToolEffect): EntityToolEffect {
	if (a === 'close' || b === 'close') return 'close'
	if (a === 'refresh' || b === 'refresh') return 'refresh'
	return 'none'
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

// The open tabs a round touched: those whose observed page path is in `pages`.
// Item-editor and pipeline routes are never list pages, so they never match. An
// entity-editor tab does — its location is a list page with the row in the hash,
// which the path comparison drops — and the caller tells the two apart, reloading
// the list frames and passing the rest through `entityEffectForTab`. Pure over a
// tab snapshot so the sessions page can act by id and this stays unit-testable.
export function tabsToReload(
	tabs: SessionPreviewTab[],
	pages: ReadonlySet<string>
): SessionPreviewTab[] {
	if (pages.size === 0) return []
	return tabs.filter((t) => pages.has(stripBase(whereIs(t))))
}
