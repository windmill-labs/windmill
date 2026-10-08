import { getContext, setContext } from 'svelte'

/**
 * Column template shared by the home table's header and every row, so the
 * columns line up across separately rendered rows (the tree nests them):
 * select · name · badges · last run · triggers · last edited · actions. Below `lg` the labels,
 * triggers and runs cells are hidden, and the template drops them too.
 */
export const HOME_TABLE_GRID =
	'grid items-center gap-x-5 grid-cols-[1rem_minmax(0,1fr)_6.5rem] lg:grid-cols-[1rem_minmax(0,2fr)_minmax(0,1.5fr)_6rem_13rem_5rem_6.5rem]'

/** Sub-columns of the badges area: labels · shared. */
export const HOME_TABLE_BADGE_GRID =
	'grid-cols-[minmax(0,1fr)_1.25rem] gap-x-2 items-center min-w-0'

/** Shows a muted dash in a table cell that renders no element. */
export const EMPTY_CELL_DASH =
	"[&:not(:has(*))]:after:content-['–'] [&:not(:has(*))]:after:text-hint [&:not(:has(*))]:after:text-2xs [&:not(:has(*))]:after:opacity-40"

const CONTEXT_KEY = 'homeTable'

/** Rows below the calling component render as home table rows. */
export function setHomeTable() {
	setContext(CONTEXT_KEY, true)
}

export function isHomeTable(): boolean {
	return getContext<boolean | undefined>(CONTEXT_KEY) ?? false
}
