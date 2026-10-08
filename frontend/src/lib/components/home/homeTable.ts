import { getContext, setContext } from 'svelte'

/**
 * Column template shared by the home table's header and every row, so the
 * columns line up across separately rendered rows (the tree nests them):
 * select · name · badges · last run · triggers · last edited · actions. Below `lg` the labels,
 * triggers and runs cells are hidden, and the template drops them too.
 */
export const HOME_TABLE_GRID =
	'grid items-center gap-x-5 grid-cols-[1rem_minmax(0,1fr)_var(--home-actions-w,6.5rem)] lg:grid-cols-[1rem_minmax(0,3fr)_minmax(0,1fr)_6rem_13rem_5rem_var(--home-actions-w,6.5rem)]'

/** Actions column width when some row shows "Edit in fork" instead of Edit. */
export const HOME_TABLE_WIDE_ACTIONS = '--home-actions-w: 12.5rem'

/** Sub-columns of the badges area: labels · shared. */
export const HOME_TABLE_BADGE_GRID =
	'grid-cols-[minmax(0,1fr)_1.25rem] gap-x-2 items-center min-w-0'

/** The faint line between home table rows; the last row in the box draws none. */
export const HOME_TABLE_ROW_SEPARATOR =
	'border-b border-gray-100 dark:border-gray-700/40 last:border-b-0'

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
