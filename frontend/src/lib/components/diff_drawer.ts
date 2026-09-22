import type { Value } from '$lib/utils'

export type DiffVersionOption = {
	/** Opaque to the drawer — a script hash, a flow version id, an app version id. */
	id: string
	/** Identifies the version: its number and hash. Carries the weight in the list. */
	label: string
	/** Who deployed it, and when — rendered under the label in secondary text, so the
	 *  version reads first and the attribution second. */
	subtitle?: string
	/** The version currently deployed. */
	isHead?: boolean
}

/** How many deployed versions the diff picker asks for at a time. The history endpoints
 *  still answer whole when nobody asks — the panels and the CLI read them that way — so
 *  this is the picker's own appetite, not their default. */
export const VERSION_PAGE_SIZE = 20

export type DiffDrawerDiff =
	| {
			mode: 'normal'
			deployed: Value
			/** Names the deployed side, e.g. `Deployed d2154d55 by bob`. The version
			 *  identity is stripped from the displayed metadata, so without this the
			 *  reader cannot tell what their draft is being compared against. */
			deployedLabel?: string
			/** Deployed versions the reader can compare against, newest first. The
			 *  drawer only renders the picker; each editor supplies its own list and
			 *  fetcher because scripts, flows and apps identify versions differently. */
			versions?: DiffVersionOption[]
			/** Loads one version's payload. Returning `undefined` leaves the current
			 *  comparison in place rather than blanking the diff. */
			loadVersion?: (id: string) => Promise<Value | undefined>
			/** Fetches the next, older page of `versions`. `versions` holds one page so the
			 *  drawer opens without waiting on a path a pipeline has deployed thousands of
			 *  times; the reader asks for the rest. Returning nothing ends the list. */
			loadMoreVersions?: () => Promise<DiffVersionOption[] | undefined>
			/** Moves the draft's base to the head and keeps its content, rendered as a
			 *  header action so the user takes the latest with the diff in front of them.
			 *  Called with the version the drawer is showing as head (from `versions`), so
			 *  the base adopted is the one the reader just looked at. */
			onTakeLatest?: (head?: string) => void | Promise<void>
			/** The version the draft forked from. The drawer offers `onTakeLatest` only
			 *  while it differs from the head, and passes that head to it. */
			draftBase?: string
			/** The version the deployed payload passed here came from, used as the head when
			 *  no `versions` list loaded. Without either, the drawer cannot tell whether the
			 *  draft is behind and offers nothing. */
			deployedHead?: string
			draft?: Value | undefined
			current: Value
			defaultDiffType?: 'deployed' | 'draft'
			button?: { text: string; onClick: () => void }
	  }
	| {
			mode: 'simple'
			original: Value
			current: Value
			title: string
			button?: { text: string; onClick: () => void }
	  }

export interface DiffDrawerI {
	/** Pass the token from `beginOpening` to continue that opening; called without one,
	 *  the drawer claims a fresh opening, so any reuse invalidates a fetch still in
	 *  flight rather than being overwritten by it. */
	openDrawer: (token?: number) => void
	closeDrawer: () => void
	/** Takes the opening's token on the same terms as `openDrawer`. */
	setDiff: (diff: DiffDrawerDiff, token?: number) => void
	/** Claim the drawer for one opening. Filling it takes awaited fetches, and a path
	 *  change remounts the editor while this drawer stays, so the token lives here: an
	 *  editor checks `ownsOpening` before every write and drops the opening it started
	 *  when it goes away. */
	beginOpening: () => number
	ownsOpening: (token: number) => boolean
	/** Drop that opening and, when it still owns the drawer, what it put on screen. */
	abandonOpening: (token: number) => void
}
