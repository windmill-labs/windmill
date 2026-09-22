import type { Snippet } from 'svelte'
import type { WorkspaceItem, WorkspaceItemKind } from './workspacePicker'

/** An item the page is working on: the breadcrumb walks its path and ends with its summary. */
export type PageHeaderItem = {
	kind: WorkspaceItemKind
	path: string | undefined
	/** The item's saved path, so a mid-rename breadcrumb and picker stay coherent. */
	savedPath?: string
	summary?: string
	raw_app?: boolean
	/** Which workspace the breadcrumb's pickers read (a session editor acts on its fork). */
	workspaceId?: string
	pathEditable?: boolean
	summaryEditable?: boolean
	onBehalfOfEmail?: string
	onSummaryChange?: (summary: string) => void
	onPathChange?: (path: string) => void
	onNavigate?: (item: WorkspaceItem) => void
}

/** A page with no item of its own, such as a list: the breadcrumb is the section alone. */
export type PageHeaderSection = {
	label: string
	/** Scopes the segment's picker, when the section maps to a kind of item. */
	kind?: WorkspaceItemKind
}

export type PageHeaderContent = {
	item?: PageHeaderItem
	section?: PageHeaderSection
	/** Rendered at the right end of the bar — the page's own buttons. */
	actions?: Snippet
}

// The bar belongs to the root layout and is always on screen; a route page fills its breadcrumb
// and actions by registering here.
//
// A registration is a getter, not a snapshot: the layout calls it while rendering, so a path or
// summary the page edits reaches the bar through the layout's own dependency tracking, with no
// effect keeping two copies in step.
//
// Registrations stack and only the first shows. Pages register on mount, so the first entry is the
// outermost one — an editor nested in a session pane or a drawer is asked to register nothing, and
// a route mounting its successor before unmounting itself cannot flash the wrong path.
let entries = $state<{ id: number; get: () => PageHeaderContent }[]>([])
let nextId = 0

export const pageHeader = {
	get content(): PageHeaderContent | undefined {
		return entries[0]?.get()
	},
	/** Registers on mount and returns the id to release on destroy. */
	register(get: () => PageHeaderContent): number {
		const id = nextId++
		entries = [...entries, { id, get }]
		return id
	},
	release(id: number) {
		entries = entries.filter((e) => e.id !== id)
	}
}
