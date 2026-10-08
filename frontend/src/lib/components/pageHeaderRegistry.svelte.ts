import type { Snippet } from 'svelte'
import type { WorkspaceItemKind } from './workspacePicker'

/** An item the page is working on: the breadcrumb walks its path and ends with its summary. */
export type PageHeaderItem = {
	/** Absent for an item the picker does not file under a kind — an agent's detail page. The
	 *  breadcrumb then names the item without claiming it is one of the three. */
	kind?: WorkspaceItemKind
	/** Shown whole. Clicking it opens whatever the page hangs off it with `pathTrigger`, and copies
	 *  the path where there is none. */
	path: string | undefined
	/** Wraps the band's own path segment in whatever the page wants behind it — its rename
	 *  popover, anchored there rather than beside the summary, so the editor opens under the half
	 *  of the name that was clicked. The band passes the path as it renders it and the classes its
	 *  own segment wears, so the page's control keeps the breadcrumb's look rather than inventing
	 *  one. Without this the segment copies the path on click. */
	pathTrigger?: Snippet<[Snippet, string]>
	summary?: string
	/** Rendered in place of the plain summary, for a page whose summary is itself a control —
	 *  a detail page's rename-and-labels popover, an editor's editable title. */
	summaryContent?: Snippet
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
	/** Orders these buttons among the other registrations', ascending; registration order breaks a
	 *  tie. For a menu that must stay at the far end however many pages register before it. */
	actionsOrder?: number
	/** These actions can give width back under pressure, so the bar lets their box shrink on a
	 *  phone. Two ways to earn it: a control that is narrower when it has to be (the Runs filter
	 *  row, whose search field yields), or a set that scrolls inside its own box (the app and
	 *  pipeline editors, whose rows are too wide for any bar). Without it the box holds its content
	 *  width, because squeezing a plain row of buttons only pushes them out of the bar. */
	actionsFlexible?: boolean
	/** These actions are not a row of buttons at the far end but a surface of their own, running
	 *  from the breadcrumb to the bar's right edge: a session in full screen, whose tab strip and
	 *  controls share one shape that opens into the page below. The bar gives up its right padding
	 *  to it, since the surface is what meets the edge. */
	actionsFill?: boolean
	/** Rendered right after the page's name in the breadcrumb: a mark that belongs to the name,
	 *  like a documentation tooltip, or a control that acts on the thing named — an app's Edit,
	 *  which at the far end of the bar would be a journey away from what it edits. */
	afterName?: Snippet
	/** The workspace this page acts on, when it is not the one the app is pointed at — a session
	 *  running in a fork. The breadcrumb then names that workspace, and its fork part shows only
	 *  when it really is a fork. */
	actingWorkspaceId?: string
	/** Width in px, at the right of the viewport, that this page owns from the top down — a
	 *  session's side panel. The band stops there instead of running over it, and the band floats
	 *  above the page rather than pushing it down, so that column starts at the top. */
	barRightInset?: number
	/** The bottom edge under the bar, which the bar owns so a page cannot draw a second one.
	 *  `onScroll` (the default) shows it only while something is passing under the bar — at the top
	 *  of a list there is nothing to divide. `always` is for a page whose content starts right
	 *  under the bar and needs the division at rest: an editor, a detail page, a settings pane.
	 *  `none` is for a page that genuinely draws its own first line, such as the script editor's
	 *  toolbar or the flow editor's canvas. */
	separator?: 'onScroll' | 'always' | 'none'
	/** Contexts this registration's snippets look up — its actions, what it hangs after the name,
	 *  and its summary. They all render under the header rather than under the page that wrote
	 *  them, so anything the page's tree provides has to travel with them. */
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

/**
 * Below this bar width a page keeps one button — the thing the page is for — and folds the rest
 * into the menu it already shows when compact. A phone in portrait is ~390px of bar and a tablet
 * in portrait ~720; the breadcrumb with its workspace disc takes ~150 of either, which is why the
 * line sits above the tablet rather than between the two.
 */
export const PHONE_BAR = 800

/** The bar's height, and the padding a page owes the band when the band floats over it rather than
 *  sitting in the flow (see `barRightInset`). They are one measurement in two forms: raise one and
 *  the other has to follow, or the page's own content disappears under the bar. Written as literal
 *  class names so Tailwind sees them. */
export const BAR_HEIGHT = 'h-12'
export const BAR_TOP_PAD = 'pt-12'
/** The same height in px, for the places that animate to the bar's bottom edge rather than
 *  standing under it — a transition's `css` and a measurement's fallback take a number. */
export const BAR_HEIGHT_PX = 48

// The bar's own width, written by the bar and read by the pages that fill it. A page's buttons
// share that row with the breadcrumb, so it is the width they crowd against — not the window's,
// which the sidebar and a page's side panel both take from.
let barWidth = $state(0)

/**
 * One registration's content, or undefined while it cannot answer.
 *
 * An entry is a getter closing over another component's props, evaluated here, in a tree of its
 * own. Nothing orders a parent clearing those props against the child's teardown: a route that
 * drops the `{#if}` around an editor and the value it binds into it does both in one flush, and
 * this derived re-runs on the same flush, before the editor's own cleanup releases its entry. The
 * entry has nothing to say in that window, which is not the same as the page having no header —
 * so it is skipped, and whatever registered before it still stands.
 */
function read(e: { get: () => PageHeaderContent }): PageHeaderContent | undefined {
	try {
		return e.get()
	} catch {
		return undefined
	}
}

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
			const c = read(e)
			if (!c) continue
			if (c.item !== undefined) merged.item = c.item
			if (c.section !== undefined) merged.section = c.section
			if (c.afterName !== undefined) merged.afterName = c.afterName
			if (c.contexts !== undefined) merged.contexts = c.contexts
			if (c.actingWorkspaceId !== undefined) merged.actingWorkspaceId = c.actingWorkspaceId
			if (c.barRightInset !== undefined) merged.barRightInset = c.barRightInset
			if (c.actionsFlexible !== undefined) merged.actionsFlexible = c.actionsFlexible
			if (c.actionsFill !== undefined) merged.actionsFill = c.actionsFill
			if (c.separator !== undefined) merged.separator = c.separator
		}
		return merged
	},
	/**
	 * Every registration's actions, in registration order: a page and something inside it can both
	 * put buttons in the band, and each renders with the contexts its own registration named.
	 */
	get actions(): { render: Snippet; contexts?: Map<any, any> }[] {
		return entries
			.flatMap((e, i) => {
				const c = read(e)
				return c?.actions
					? [{ render: c.actions, contexts: c.contexts, order: c.actionsOrder ?? 0, i }]
					: []
			})
			.sort((a, b) => a.order - b.order || a.i - b.i)
	},
	/** Width of the bar in px, or 0 before it has been measured. */
	get barWidth(): number {
		return barWidth
	},
	setBarWidth(w: number) {
		barWidth = w
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
