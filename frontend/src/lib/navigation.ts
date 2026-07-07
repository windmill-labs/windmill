import { goto as svelteGoto } from '$app/navigation'
import { base as svelteBase } from '$app/paths'
import { serializeParam } from '$lib/svelte5UtilsKit.svelte'

export function goto(path: string, options = {}) {
	if (svelteBase == '' || path.startsWith('?')) {
		return svelteGoto(path, options)
	} else {
		const fullPath = path.startsWith(svelteBase) ? path : `${svelteBase}${path}`
		return svelteGoto(fullPath, options)
	}
}

/**
 * Build an in-app deep-link to `pathname` with query-param filters, encoded exactly
 * as the pages write them (via `serializeParam`) so `useUrlSyncedFilterInstance`
 * round-trips them back into filter state. Nullish/empty values are dropped; when
 * `validKeys` is provided, unknown keys are dropped too (structured output guarantees
 * shape, not truth). Returns an un-prefixed app path — pass it to `goto`, which adds
 * the SvelteKit base.
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

/** Current in-app pathname with the SvelteKit base stripped. */
function currentAppPathname(): string {
	const p = window.location.pathname
	return svelteBase && p.startsWith(svelteBase) ? p.slice(svelteBase.length) : p
}

/** True when the user is already on `appPath` (base-agnostic prefix match). */
export function isCurrentPage(appPath: string): boolean {
	return currentAppPathname().startsWith(appPath)
}

// Module-global navigation slot, mirroring the copilot handler-registration seams
// (e.g. setOpenPreviewHandler). A top-level logged-in layout registers the actual
// `goto`-based navigator at mount so callers that must stay router-free — notably AI
// chat tools — can trigger navigation without importing $app/navigation.
let chatNavigateHandler: ((appPath: string) => void) | undefined

export function setChatNavigateHandler(fn: ((appPath: string) => void) | undefined): void {
	chatNavigateHandler = fn
}

/** Navigate to an in-app path via the registered handler. Returns false if none is registered. */
export function chatNavigate(appPath: string): boolean {
	if (!chatNavigateHandler) return false
	chatNavigateHandler(appPath)
	return true
}

export async function setQuery(
	url: URL,
	key: string,
	value: string | undefined,
	currentHash: string | undefined = undefined
): Promise<void> {
	if (value !== undefined) {
		url.searchParams.set(key, value)
	} else {
		url.searchParams.delete(key)
	}

	let searchParams = url.searchParams.toString()

	await goto(currentHash ? `?${searchParams}${currentHash}` : `?${searchParams}`)
}
