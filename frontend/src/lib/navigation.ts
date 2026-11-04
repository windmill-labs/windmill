import { goto } from '$app/navigation'
import { serializeParam } from '$lib/utils/serializeParam'

export { goto }

/**
 * Whether a click asks the browser to open its target in a new tab: a middle click, or a
 * ctrl/cmd-click. An `<a href>` whose `onclick` also opens something in place (a drawer, a
 * selection) must skip that work on such a click, since the browser opens the tab itself.
 */
export function isNewTabClick(e: MouseEvent): boolean {
	return e.button === 1 || e.ctrlKey || e.metaKey
}

/**
 * For a clickable element that cannot be an `<a href>` (it holds links or buttons of its own):
 * open `href` in a new tab on a middle click or, unless `modifiers` is false, a ctrl/cmd-click.
 * Returns whether it did, so the caller skips its own click action. A click that lands on a
 * nested link or button is left to that control, or both would open a tab.
 */
export function openOnNewTabClick(
	e: MouseEvent,
	href: string,
	{ modifiers = true }: { modifiers?: boolean } = {}
): boolean {
	if (!(e.button === 1 || (modifiers && (e.ctrlKey || e.metaKey)))) return false
	const control = (e.target as Element | null)?.closest('a[href], button')
	if (control && control !== e.currentTarget) return false
	e.preventDefault()
	window.open(href, '_blank')
	return true
}

/**
 * Build an in-app deep-link to `pathname` with query-param filters, encoded exactly
 * as the pages write them (via `serializeParam`) so `useUrlSyncedFilterInstance`
 * round-trips them back into filter state. Nullish/empty values are dropped; when
 * `validKeys` is provided, unknown keys are dropped too (structured output guarantees
 * shape, not truth). Returns an app path to pass to `goto`.
 */
export function buildFilterUrl(
	pathname: string,
	values: Record<string, unknown>,
	opts?: { validKeys?: Iterable<string>; hash?: string }
): string {
	const allow = opts?.validKeys ? new Set(opts.validKeys) : undefined
	const sp = new URLSearchParams()
	for (const [key, value] of Object.entries(values)) {
		if (value === undefined || value === null || value === '') continue
		if (allow && !allow.has(key)) continue
		sp.set(key, serializeParam(value))
	}
	const qs = sp.toString()
	const hash = opts?.hash ? `#${opts.hash}` : ''
	return qs ? `${pathname}?${qs}${hash}` : `${pathname}${hash}`
}

/**
 * Set (or, with an undefined value, drop) one query param on the current page and
 * navigate, leaving every other param untouched.
 *
 * The starting query must come from `window.location`, never from a URL held
 * elsewhere — above all `page.url`, which SvelteKit refreshes only on router-driven
 * navigations. Params written with shallow routing (a workspace switch's
 * `?workspace=`, anything `useSearchParams` writes) never reach `page.url`, so
 * rebuilding the query from it navigates back to their pre-write values.
 */
export async function setQuery(
	key: string,
	value: string | undefined,
	currentHash: string | undefined = undefined
): Promise<void> {
	const searchParams = new URLSearchParams(window.location.search)
	if (value !== undefined) {
		searchParams.set(key, value)
	} else {
		searchParams.delete(key)
	}

	await goto(currentHash ? `?${searchParams}${currentHash}` : `?${searchParams}`)
}
