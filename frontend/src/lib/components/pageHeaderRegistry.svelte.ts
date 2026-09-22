import type { Snippet } from 'svelte'
import type { WorkspaceItem, WorkspaceItemKind } from './workspacePicker'

/** An item the page is working on: the breadcrumb walks its path and ends with its summary. */
export type PageHeaderItem = {
	kind: WorkspaceItemKind
	path: string | undefined
	/** The item's saved path, so a mid-rename breadcrumb and picker stay coherent. */
	savedPath?: string
	summary?: string
	/** Rendered after the path instead of the editable summary — for a page whose summary is
	 *  itself a control, such as a detail page's rename-and-labels popover. */
	summaryContent?: Snippet
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
	/** Rendered in place of `label` when the name is itself a control, such as an editable title. */
	content?: Snippet
	/** Scopes the segment's picker, when the section maps to a kind of item. */
	kind?: WorkspaceItemKind
}

export type PageHeaderContent = {
	item?: PageHeaderItem
	section?: PageHeaderSection
	/** Rendered at the right end of the bar — the page's own buttons. */
	actions?: Snippet
	/** The workspace this page acts on, when it is not the one the app is pointed at — a session
	 *  running in a fork. The breadcrumb then names that workspace, and its fork part shows only
	 *  when it really is a fork. */
	actingWorkspaceId?: string
	/** False drops the band's bottom edge, for a page that draws its own first line. */
	barBorder?: boolean
	/** Width in px, at the right of the viewport, that this page owns from the top down — a
	 *  session's side panel. The band stops there instead of running over it, and the band floats
	 *  above the page rather than pushing it down, so that column starts at the top. */
	barRightInset?: number
	/** 'inline' when the route renders the bar itself — for a layout whose side panel must run to
	 *  the top of the viewport beside it, rather than starting under a full-width bar. */
	barPlacement?: 'layout' | 'inline'
	/** Contexts those buttons look up. They render under the header, not under the page that
	 *  wrote them, so anything the page's tree provides has to travel with them. */
	contexts?: Map<any, any>
}

// The bar belongs to the root layout and is always on screen; a route page fills its breadcrumb
// and actions by registering here.
//
// A registration is a getter, not a snapshot: the layout calls it while rendering, so a path or
// summary the page edits reaches the bar through the layout's own dependency tracking, with no
// effect keeping two copies in step.
//
// Registrations stack, and what the bar shows is their merge (see `content`). Pages register on
// mount, so the first entry is the outermost one — an editor nested in a session pane or a drawer
// is asked to register nothing.
let entries = $state<{ id: number; get: () => PageHeaderContent }[]>([])
let nextId = 0

export const pageHeader = {
	/**
	 * The registrations merged, later ones winning field by field: a route sets the frame (where
	 * the bar goes) and something nested in it — the active session, say — fills the breadcrumb
	 * and the actions.
	 */
	get content(): PageHeaderContent | undefined {
		if (entries.length === 0) return undefined
		const merged: PageHeaderContent = {}
		for (const e of entries) {
			const c = e.get()
			if (c.item !== undefined) merged.item = c.item
			if (c.section !== undefined) merged.section = c.section
			if (c.contexts !== undefined) merged.contexts = c.contexts
			if (c.barPlacement !== undefined) merged.barPlacement = c.barPlacement
			if (c.actingWorkspaceId !== undefined) merged.actingWorkspaceId = c.actingWorkspaceId
			if (c.barRightInset !== undefined) merged.barRightInset = c.barRightInset
			if (c.barBorder !== undefined) merged.barBorder = c.barBorder
		}
		return merged
	},
	/**
	 * Every registration's actions, in registration order: a page and something inside it can both
	 * put buttons in the band, and each renders with the contexts its own registration named.
	 */
	get actions(): { render: Snippet; contexts?: Map<any, any> }[] {
		return entries.flatMap((e) => {
			const c = e.get()
			return c.actions ? [{ render: c.actions, contexts: c.contexts }] : []
		})
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
