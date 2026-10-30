import type { Item } from '$lib/utils'
import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'

/** Home's header actions fold through two levels once the breadcrumb has given all it can: at
 *  the first the runs status shrinks to an icon, at the second every action joins the ⋮ menu. */
export const HOME_ACTION_FOLDS = 2

/** Bar width from which the fork badges say what they count. Below it they keep the icon and the
 *  number, which is the part a reader scans for; the tooltip names them either way. */
const BAR_FITS_COUNT_LABELS = 1280
/** Bar width from which CLI / MCP and the hub stand in the bar rather than in the ⋮ menu. Home's
 *  trail is ~250px and the rest of its actions ~400, so this leaves room for both labelled
 *  buttons without the trail starting to truncate. */
const BAR_FITS_SIDE_TRIPS = 1024

export function homeCountLabels(): boolean {
	return pageHeader.barWidth >= BAR_FITS_COUNT_LABELS
}

export function homeSideTripsInBar(): boolean {
	return pageHeader.barWidth >= BAR_FITS_SIDE_TRIPS && !homeActionsGathered()
}

/** The actions change width at these two widths on their own, not through a fold: the bar has to
 *  measure its fold levels afresh when one flips, or it keeps folding by the wider row. */
export function homeActionsVersion(): string {
	return `${homeCountLabels()}:${pageHeader.barWidth >= BAR_FITS_SIDE_TRIPS}:${renderers > 0}`
}

export function homeActionsCompact(): boolean {
	return pageHeader.actionFold >= 1
}

/** Only while the ⋮ menu is mounted to hold them (HomeAIChat, absent when the sessions beta is
 *  opted out): without it, gathered actions would be nowhere on the page. */
export function homeActionsGathered(): boolean {
	return pageHeader.actionFold >= 2 && renderers > 0
}

let providers = $state<{ id: number; order: number; get: () => Item[] }[]>([])
let nextId = 0
let renderers = $state(0)

/** The rows each of Home's header actions puts in the ⋮ menu once they are gathered there. */
export const homeHeaderMenu = {
	get items(): Item[] {
		return [...providers].sort((a, b) => a.order - b.order).flatMap((p) => p.get())
	},
	/** Registers on mount and returns the id to release on destroy. `order` is the action's own
	 *  `actionsOrder`, so the menu lists them as the bar does. */
	register(order: number, get: () => Item[]): number {
		const id = nextId++
		providers = [...providers, { id, order, get }]
		return id
	},
	release(id: number) {
		providers = providers.filter((p) => p.id !== id)
	},
	/** Called by the menu that lists `items`, on mount; returns its release for destroy. */
	mountRenderer(): () => void {
		renderers++
		return () => renderers--
	}
}
