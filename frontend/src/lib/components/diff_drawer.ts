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
	openDrawer: () => void
	closeDrawer: () => void
	setDiff: (diff: DiffDrawerDiff) => void
}
